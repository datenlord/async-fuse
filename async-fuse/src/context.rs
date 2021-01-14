//!

#![allow(clippy::missing_docs_in_private_items)]

use crate::abi::RawBytes;
use crate::conn::ConnWriter;
use crate::kernel;
use crate::payload::Payload;

use std::{io, mem};

use nix::errno::Errno;

#[derive(Debug)]
pub struct FuseContext {
    header: FuseInHeader,
    payload: Payload,
    writer: ConnWriter,
}

#[derive(Debug)]
struct FuseInHeader {
    len: u32,
    opcode: u32,
    unique: u64,
    nodeid: u64,
    uid: u32,
    gid: u32,
    pid: u32,
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
