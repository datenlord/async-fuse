//! FUSE connection

use crate::core::{FuseDesc, FuseWrite};

use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use blocking::unblock;

/// The writer of a FUSE connection
#[derive(Debug, Clone)]
pub struct ConnWriter {
    /// Arc fd
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

/// The reader of a FUSE connection
#[derive(Debug)]
pub struct ConnReader {
    /// Arc fd
    fd: Arc<FuseDesc>,
}

impl ConnReader {
    /// Gets the underlying fd
    pub fn get_fd(&self) -> &FuseDesc {
        &*self.fd
    }

    /// Reads a request into the buffer, using a thread pool.
    pub async fn read<B>(&mut self, mut buf: B) -> io::Result<(B, usize)>
    where
        B: AsMut<[u8]> + Send + 'static,
    {
        let fd = Arc::clone(&self.fd);
        unblock(move || (&*fd).read(buf.as_mut()).map(|nread| (buf, nread))).await
    }
}

/// Connects to `/dev/fuse`
pub async fn connect() -> io::Result<(ConnReader, ConnWriter)> {
    let fd = Arc::new(unblock(FuseDesc::open).await?);

    let writer = ConnWriter { fd };

    let reader = ConnReader {
        fd: Arc::clone(&writer.fd),
    };

    Ok((reader, writer))
}
