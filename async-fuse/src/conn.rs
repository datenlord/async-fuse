//! FUSE connection

use crate::fd::{FuseDesc, OwnedFd};
use crate::proactor::global_proactor;

use std::io::{self, IoSlice};
use std::sync::Arc;

use blocking::unblock;

/// Connects to `/dev/fuse`
pub async fn connect() -> io::Result<(ConnReader, ConnWriter)> {
    let fd = Arc::new(unblock(FuseDesc::open).await?);

    let writer = ConnWriter { fd };

    let reader = ConnReader {
        fd: Arc::clone(&writer.fd),
    };

    Ok((reader, writer))
}

/// The reader of a FUSE connection
#[derive(Debug)]
pub struct ConnReader {
    /// Arc fd
    fd: Arc<FuseDesc>,
}

/// The writer of a FUSE connection
#[derive(Debug, Clone)]
pub struct ConnWriter {
    /// Arc fd
    fd: Arc<FuseDesc>,
}

impl ConnReader {
    /// Gets the underlying fd
    pub fn get_fd(&self) -> &FuseDesc {
        &*self.fd
    }

    /// Reads a request into the buffer, using a thread pool.
    pub async fn read<B>(&mut self, buf: B) -> (B, io::Result<usize>)
    where
        B: AsMut<[u8]> + Send + 'static,
    {
        let fd = OwnedFd(Arc::clone(&self.fd));
        let proactor = global_proactor();
        let (_, buf, ret) = proactor.read(fd, buf).await;
        (buf, ret)
    }
}

impl ConnWriter {
    #[allow(single_use_lifetimes)]
    pub async fn write_vectored<S>(&mut self, bufs: S) -> (S, io::Result<usize>)
    where
        S: for<'a> AsRef<[IoSlice<'a>]> + Send + 'static,
    {
        let fd = OwnedFd(Arc::clone(&self.fd));
        let proactor = global_proactor();
        let (_, buf, ret) = proactor.write_vectored(fd, bufs).await;
        (buf, ret)
    }

    pub async fn write<B>(&mut self, buf: B) -> (B, io::Result<usize>)
    where
        B: AsRef<[u8]> + Send + 'static,
    {
        let fd = OwnedFd(Arc::clone(&self.fd));
        let proactor = global_proactor();
        let (_, buf, ret) = proactor.write(fd, buf).await;
        (buf, ret)
    }
}
