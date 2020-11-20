use crate::buffer_pool::BufferPool;

use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::atomic::{self, AtomicU8};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread;

use async_fuse::{FuseDesc, FuseWrite};

use aligned_bytes::AlignedBytes;
use atomic_waker::AtomicWaker;
use blocking::unblock;
use crossbeam_queue::SegQueue;
use futures::Stream;

#[derive(Debug, Clone)]
pub struct ConnWriter {
    fd: Arc<FuseDesc>,
}

impl FuseWrite for ConnWriter {
    fn poll_reply(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<()>> {
        // libc::writev is atomic, so we don't need to lock the fd.
        // fuse fd is always writable.
        let len: usize = bufs.iter().map(|s| s.len()).sum();
        let nwrite = (&*self.fd).write_vectored(bufs)?;
        assert_eq!(len, nwrite);
        Poll::Ready(Ok(()))
    }
}

struct ReaderInner {
    pool: BufferPool,
    queue: SegQueue<io::Result<(AlignedBytes, usize)>>,
    waker: AtomicWaker,
    state: AtomicU8,
    fd: Arc<FuseDesc>,
}

const INIT: u8 = 0;
const RUNNING: u8 = 1;
const CLOSED: u8 = 2;

pub struct ConnReader(Arc<ReaderInner>);

impl Stream for ConnReader {
    type Item = io::Result<(AlignedBytes, usize)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &*self.0;

        if let Some(item) = this.queue.pop() {
            return Poll::Ready(Some(item));
        }
        match this.state.load(atomic::Ordering::Acquire) {
            INIT => panic!("reader daemon is not running"),
            RUNNING => this.waker.register(cx.waker()),
            CLOSED => return Poll::Ready(None),
            _ => unreachable!(),
        }
        if let Some(item) = this.queue.pop() {
            return Poll::Ready(Some(item));
        }
        Poll::Pending
    }
}

impl Drop for ConnReader {
    fn drop(&mut self) {
        self.0.state.store(CLOSED, atomic::Ordering::Release);
    }
}

impl ConnReader {
    pub fn get_fd(&self) -> &FuseDesc {
        &*self.0.fd
    }

    pub fn spawn_daemon(&mut self) -> thread::JoinHandle<()> {
        struct Guard<'a>(&'a AtomicU8);

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.store(CLOSED, atomic::Ordering::Release);
            }
        }

        if self
            .0
            .state
            .compare_and_swap(INIT, RUNNING, atomic::Ordering::SeqCst)
            != INIT
        {
            panic!("reader daemon is already spawned")
        }

        let daemon = Arc::clone(&self.0);
        thread::spawn(move || {
            let _guard = Guard(&daemon.state);
            loop {
                let mut buf = daemon.pool.acquire();
                match (&*daemon.fd).read(&mut buf) {
                    Ok(nread) => daemon.queue.push(Ok((buf, nread))),
                    Err(err) => daemon.queue.push(Err(err)),
                }
                daemon.waker.wake();

                if daemon.state.load(atomic::Ordering::Acquire) == CLOSED {
                    break;
                }
            }
        })
    }
}

pub async fn connect(pool: BufferPool) -> io::Result<(ConnReader, ConnWriter)> {
    let fd = Arc::new(unblock(FuseDesc::open).await?);

    let writer = ConnWriter { fd };

    let reader = ConnReader(Arc::new(ReaderInner {
        pool,
        queue: SegQueue::new(),
        waker: AtomicWaker::new(),
        state: INIT.into(),
        fd: Arc::clone(&writer.fd),
    }));

    Ok((reader, writer))
}
