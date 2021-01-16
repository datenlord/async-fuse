use std::io;
use std::sync::Arc;

use crate::fd::{create_pipe, PipeReader, PipeWriter};
use crate::proactor::global_proactor;

use aligned_utils::bytes::AlignedBytes;
use crossbeam_queue::ArrayQueue;

#[derive(Debug)]
struct Resource {
    buf: AlignedBytes,
    pipe_rx: PipeReader,
    pipe_tx: PipeWriter,
}

impl Resource {
    fn new(buf_len: usize, buf_align: usize) -> io::Result<Self> {
        let buf = AlignedBytes::new_zeroed(buf_len, buf_align);
        let (rx, tx) = create_pipe()?;
        Ok(Self {
            buf,
            pipe_rx: rx,
            pipe_tx: tx,
        })
    }
}

#[derive(Debug)]
pub struct Payload {
    pub(crate) buf: Option<AlignedBytes>,
    pub(crate) pipe_rx: Option<PipeReader>,
    pub(crate) pipe_tx: Option<PipeWriter>,
    pub(crate) buf_data_len: usize,
    pub(crate) pipe_data_len: usize,
    queue: Arc<ArrayQueue<Resource>>,
}

impl Drop for Payload {
    fn drop(&mut self) {
        if let Some(buf) = self.buf.take() {
            if let Some(pipe_rx) = self.pipe_rx.take() {
                if let Some(pipe_tx) = self.pipe_tx.take() {
                    let res = Resource {
                        buf,
                        pipe_rx,
                        pipe_tx,
                    };
                    drop(self.queue.push(res));
                }
            }
        }
    }
}

impl Payload {
    #[allow(clippy::unwrap_used, clippy::clippy::integer_arithmetic)]
    pub(crate) async fn load_data(&mut self, len: usize) -> io::Result<&[u8]> {
        struct OffsetBuf<B> {
            buf: B,
            offset: usize,
            len: usize,
        }

        impl<B: AsMut<[u8]>> AsMut<[u8]> for OffsetBuf<B> {
            fn as_mut(&mut self) -> &mut [u8] {
                let buf = self.buf.as_mut();
                let upper = self.offset.saturating_add(self.len).min(buf.len());
                &mut self.buf.as_mut()[self.offset..upper]
            }
        }

        let proactor = global_proactor();
        let pipe_rx = self.pipe_rx.take().unwrap();
        let buf = self.buf.take().unwrap();

        let offset_buf = OffsetBuf {
            buf,
            offset: self.buf_data_len,
            len,
        };

        let (pipe_rx, offset_buf, ret) = proactor.read(pipe_rx, offset_buf).await;
        self.pipe_rx = Some(pipe_rx);
        self.buf = Some(offset_buf.buf);
        let nread = ret?;
        assert_eq!(nread, len);

        self.buf_data_len += nread;
        self.pipe_data_len -= nread;

        Ok(&self.buf.as_ref().unwrap()[self.buf_data_len - nread..self.buf_data_len])
    }
}

#[derive(Debug)]
pub struct PayloadPool {
    queue: Arc<ArrayQueue<Resource>>,
    buf_len: usize,
    buf_align: usize,
}

impl PayloadPool {
    pub fn new(pool_size: usize, buf_len: usize, buf_align: usize) -> Self {
        let queue = Arc::new(ArrayQueue::new(pool_size));
        Self {
            queue,
            buf_len,
            buf_align,
        }
    }

    pub fn create(&self) -> io::Result<Payload> {
        let res = match self
            .queue
            .pop()
            .filter(|res| res.buf.len() == self.buf_len || res.buf.align() == self.buf_align)
        {
            Some(res) => res,
            None => Resource::new(self.buf_len, self.buf_align)?,
        };
        let payload = Payload {
            queue: Arc::clone(&self.queue),
            buf: Some(res.buf),
            pipe_rx: Some(res.pipe_rx),
            pipe_tx: Some(res.pipe_tx),
            buf_data_len: 0,
            pipe_data_len: 0,
        };
        Ok(payload)
    }
}
