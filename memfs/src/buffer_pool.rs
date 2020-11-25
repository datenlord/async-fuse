use aligned_bytes::AlignedBytes;
use crossbeam_queue::ArrayQueue;

#[derive(Debug)]
pub struct BufferPool {
    queue: ArrayQueue<AlignedBytes>,
    buf_size: usize,
    align: usize,
}

pub struct Buffer {
    inner: AlignedBytes,
    len: usize,
}

impl Buffer {
    pub fn capacity(&self) -> usize {
        self.inner.len()
    }
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
    }
    pub fn reset(&mut self) {
        self.len = self.inner.len();
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.inner[..self.len]
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner[..self.len]
    }
}

impl BufferPool {
    pub fn new(cap: usize, buf_size: usize, align: usize) -> Self {
        let queue = ArrayQueue::new(cap);
        Self {
            queue,
            buf_size,
            align,
        }
    }

    pub fn create(&self) -> Buffer {
        Buffer {
            inner: AlignedBytes::new_zeroed(self.buf_size, self.align),
            len: self.buf_size,
        }
    }

    pub fn acquire(&self) -> Buffer {
        match self.queue.pop() {
            Some(buf) => Buffer {
                len: buf.len(),
                inner: buf,
            },
            None => self.create(),
        }
    }

    pub fn release(&self, buf: Buffer) {
        if buf.inner.len() == self.buf_size && buf.inner.alignment() == self.align {
            let _ = self.queue.push(buf.inner);
        }
    }
}
