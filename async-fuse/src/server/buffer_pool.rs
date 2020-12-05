//! lockfree buffer pool

use aligned_bytes::AlignedBytes;
use crossbeam_queue::ArrayQueue;

/// buffer pool
#[derive(Debug)]
pub struct BufferPool {
    /// lockfree queue
    queue: ArrayQueue<AlignedBytes>,
    /// buffer size
    buf_size: usize,
    /// buffer alignment
    align: usize,
}

/// aligned buffer
pub struct Buffer {
    /// buf
    inner: AlignedBytes,
    /// data length
    len: usize,
}

impl Buffer {
    // pub fn capacity(&self) -> usize {
    //     self.inner.len()
    // }
    /// Sets data length
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
    }
    // pub fn reset(&mut self) {
    //     self.len = self.inner.len();
    // }
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
    /// Creates a new [`BufferPool`]
    pub fn new(cap: usize, buf_size: usize, align: usize) -> Self {
        let queue = ArrayQueue::new(cap);
        Self {
            queue,
            buf_size,
            align,
        }
    }

    /// Creates a new [`Buffer`]
    pub fn create(&self) -> Buffer {
        Buffer {
            inner: AlignedBytes::new_zeroed(self.buf_size, self.align),
            len: self.buf_size,
        }
    }

    /// Acquires a [`Buffer`]. Creates a new one if the pool is empty
    pub fn acquire(&self) -> Buffer {
        match self.queue.pop() {
            Some(buf) => Buffer {
                len: buf.len(),
                inner: buf,
            },
            None => self.create(),
        }
    }

    /// Releases a [`Buffer`]. Drop the buffer if the pool is full or the size and aligment mismatches.
    pub fn release(&self, buf: Buffer) {
        if buf.inner.len() == self.buf_size && buf.inner.alignment() == self.align {
            drop(self.queue.push(buf.inner));
        }
    }
}
