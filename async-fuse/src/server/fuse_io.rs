//! FUSE connection

use crate::core::{FuseDesc, FuseWrite};

use std::io::{self, Read};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_io::Async;
use blocking::unblock;
use futures_util::io::AsyncWrite;
use futures_util::ready;

/// The writer of a FUSE connection
#[derive(Debug, Clone)]
pub struct ConnWriter {
    /// Arc fd
    fd: Arc<Async<FuseDesc>>,
}

impl FuseWrite for ConnWriter {
    fn poll_reply(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<()>> {
        // libc::writev is atomic, so we don't need to lock the fd.
        // fuse fd is always writable.
        let len: usize = bufs.iter().map(|s| s.len()).sum();
        let mut fd = &*self.fd;
        let nwrite = ready!(Pin::new(&mut fd).poll_write_vectored(cx, bufs)?);
        assert_eq!(len, nwrite);
        Poll::Ready(Ok(()))
    }
}

/// The reader of a FUSE connection
#[derive(Debug)]
pub struct ConnReader {
    /// Arc fd
    fd: Arc<Async<FuseDesc>>,
}

impl ConnReader {
    /// Gets the underlying fd
    pub fn get_fd(&self) -> &FuseDesc {
        self.fd.get_ref()
    }

    /// Reads a request into the buffer, using a thread pool.
    pub async fn read<B>(&mut self, mut buf: B) -> io::Result<(B, usize)>
    where
        B: AsMut<[u8]> + Send + 'static,
    {
        let nread = self.fd.read_with(|fd| (&*fd).read(buf.as_mut())).await?;
        Ok((buf, nread))
    }
}

/// Connects to `/dev/fuse`
pub async fn connect() -> io::Result<(ConnReader, ConnWriter)> {
    let fd = Arc::new(Async::new(unblock(FuseDesc::open).await?)?);

    let writer = ConnWriter { fd };

    let reader = ConnReader {
        fd: Arc::clone(&writer.fd),
    };

    Ok((reader, writer))
}
