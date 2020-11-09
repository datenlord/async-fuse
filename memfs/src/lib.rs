mod buffer_pool;
mod c_str;
mod conn;
mod mount;

use crate::buffer_pool::BufferPool;
use crate::conn::Connection;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use async_fuse::kernel;
use async_fuse::ops;
use async_fuse::Errno;
use async_fuse::FileSystem;
use async_fuse::FuseContext;
use async_fuse::Operation;

use async_std::task;
use blocking::unblock;
use futures::io::AsyncReadExt;
use tracing::{debug, error};

const PAGE_SIZE: usize = 4096;
const MAX_WRITE_SIZE: usize = 128 * 1024;
const MAX_BACKGROUND: usize = 10;
const BUFFER_SIZE: usize = MAX_WRITE_SIZE + 512;

pub struct MemFsBuilder {
    mount_point: PathBuf,
}

impl MemFsBuilder {
    pub fn new(mount_point: PathBuf) -> Self {
        Self { mount_point }
    }

    pub async fn initialize(self) -> io::Result<MemFs> {
        let (mut conn, mount_point) = {
            debug!("connecting to /dev/fuse");
            let conn = Connection::open().await?;

            debug!("connected");

            let mount_point = self.mount_point;
            debug!(?mount_point, "mounting");

            let do_mount = move || {
                mount::mount(conn.get_fd(), &mount_point)?;
                <io::Result<_>>::Ok((conn, mount_point))
            };

            unblock(do_mount).await?
        };

        debug!(fd = ?conn.get_fd(), "mounted");

        let buffer_pool = BufferPool::new(MAX_BACKGROUND, BUFFER_SIZE, PAGE_SIZE);

        {
            debug!("initializing");

            let mut buf = buffer_pool.acquire();
            let len = conn.read(&mut buf).await?;

            let (cx, op) =
                FuseContext::parse(&buf[..len], &mut conn).expect("failed to parse fuse request");

            debug!(opcode = cx.opcode(), "got first request");

            if let Operation::Init(op) = op {
                // FIXME: how to set the init config?

                let mut rep = ops::ReplyInit::default();
                rep.major(kernel::FUSE_KERNEL_VERSION)
                    .minor(kernel::FUSE_KERNEL_MINOR_VERSION)
                    .max_readahead(op.max_readahead())
                    .flags(op.flags())
                    .max_background(MAX_BACKGROUND as u16)
                    .congestion_threshold(10)
                    .max_write(MAX_WRITE_SIZE as u32)
                    .time_gran(1)
                    .max_pages(0);

                debug!(?op, ?rep);
                op.reply(cx, rep).await?;
            } else {
                panic!("failed to initialize memfs: first request is not FUSE_INIT");
            }

            buffer_pool.release(buf)
        }

        debug!("initialized");

        let fs = MemFs {
            inner: Arc::new(SharedInner {
                conn,
                buffer_pool,
                mount_point,
            }),
        };
        Ok(fs)
    }
}

struct SharedInner {
    conn: Connection,
    buffer_pool: BufferPool,
    #[allow(dead_code)]
    mount_point: PathBuf,
}

pub struct MemFs {
    inner: Arc<SharedInner>,
}

impl MemFs {
    pub fn mount(mount_point: PathBuf) -> MemFsBuilder {
        MemFsBuilder::new(mount_point)
    }

    pub async fn run(self) -> io::Result<()> {
        loop {
            let result = async {
                debug!("waiting for fuse request");
                let mut buf = self.inner.buffer_pool.acquire();
                let len = (&self.inner.conn).read(&mut buf).await?;
                // let len = loop {
                //     use std::io::Read;
                //     use std::thread;
                //     use std::time::Duration;
                //     match self.inner.conn.get_fd().read(&mut buf) {
                //         Ok(len) => break len,
                //         Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                //             thread::sleep(Duration::from_secs(1))
                //         }
                //         Err(err) => return Err(err),
                //     }
                // };
                let this = Arc::clone(&self.inner);

                debug!("spawn task");
                task::spawn(async move {
                    let mut conn = &this.conn;
                    let (cx, op) = FuseContext::parse(&buf[..len], &mut conn)
                        .expect("failed to parse fuse request");

                    debug!(opcode = cx.opcode(), "got request");

                    let ret = this.dispatch(cx, op).await;
                    if let Err(err) = ret {
                        // FIXME: how to handle the error
                        error!(%err);
                    }
                });
                <io::Result<_>>::Ok(())
            }
            .await;

            if let Err(err) = result {
                let errno = err
                    .raw_os_error()
                    .unwrap_or_else(|| panic!("failed to read fuse connection: {}", err));

                match errno {
                    libc::ENODEV => {
                        break;
                    }
                    libc::EAGAIN => {
                        continue;
                    }
                    _ => panic!("unrecoverable os error: {}", err),
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl FileSystem for SharedInner {
    async fn dispatch<'b, 'a: 'b>(
        &'a self,
        cx: FuseContext<'b>,
        op: Operation<'b>,
    ) -> io::Result<()> {
        let _ = op;
        cx.reply_err(Errno::NOSYS).await?;
        Ok(())
    }
}
