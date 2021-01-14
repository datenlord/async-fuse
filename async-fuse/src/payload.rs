use std::io;
use std::sync::Arc;

use crate::fd::{create_pipe, PipeReader, PipeWriter};

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
    buf: Option<AlignedBytes>,
    pipe_rx: Option<PipeReader>,
    pipe_tx: Option<PipeWriter>,
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
        };
        Ok(payload)
    }
}
