use aligned_bytes::AlignedBytes;
use crossbeam_queue::ArrayQueue;

#[derive(Debug)]
pub struct BufferPool {
    queue: ArrayQueue<AlignedBytes>,
    buf_size: usize,
    align: usize,
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

    pub fn create(&self) -> AlignedBytes {
        AlignedBytes::new_zeroed(self.buf_size, self.align)
    }

    pub fn acquire(&self) -> AlignedBytes {
        match self.queue.pop() {
            Some(buf) => buf,
            None => self.create(),
        }
    }

    pub fn release(&self, buf: AlignedBytes) {
        if buf.len() == self.buf_size && buf.alignment() == self.align {
            let _ = self.queue.push(buf);
        }
    }
}
