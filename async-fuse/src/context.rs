//!

use crate::abi::RawBytes;
use crate::conn::ConnWriter;
use crate::kernel;
use crate::payload::Payload;

use std::{io, mem};

use nix::errno::Errno;

#[derive(Debug)]
pub struct FuseContext {
    pub(crate) header: FuseInHeader,
    pub(crate) payload: Payload,
    pub(crate) writer: ConnWriter,
    pub(crate) proto: ProtocolVersion,
}

#[derive(Debug)]
pub struct FuseInHeader {
    pub len: u32,
    pub opcode: u32,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
}

/// protocol version
#[derive(Debug, Clone, Copy)]
pub struct ProtocolVersion {
    /// major version number
    pub major: u32,
    /// minor version number
    pub minor: u32,
}

impl FuseContext {
    /// Sends errno
    /// # Errors
    /// Returns [`io::Error`] when failed to write bytes to the connection
    #[inline]
    pub async fn reply_err(mut self, errno: Errno) -> io::Result<()> {
        let header_len: usize = mem::size_of::<kernel::fuse_out_header>();

        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        // the size of a single kernel struct can not overflow u32
        let len: u32 = header_len as u32;

        #[allow(clippy::as_conversions)]
        // Errno is correct
        let error = (errno as i32).wrapping_neg();

        let out_header = RawBytes::wrap(kernel::fuse_out_header {
            len,
            unique: self.header.unique,
            error,
        });

        let len = out_header.as_ref().len();
        let (_, ret) = self.writer.write(out_header).await;
        let nwrite = ret?;

        assert_eq!(nwrite, len);

        Ok(())
    }
}
