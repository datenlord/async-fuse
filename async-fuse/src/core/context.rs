//! The context of a FUSE request

use super::decode::{Decode, DecodeError, Decoder};
use super::encode::{self, Encode};
use super::errno::Errno;
use super::kernel;
use super::ops::{FuseInHeader, IsReplyOf, Operation};
use super::write::FuseWrite;

#[allow(clippy::wildcard_imports)]
use super::kernel::fuse_opcode::*;

use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::io::{self, IoSlice};
use std::mem;
use std::pin::Pin;

use futures_util::future::poll_fn;
use smallvec::SmallVec;

/// The context of a FUSE request
pub struct FuseContext<'b> {
    /// a writer of FUSE connection
    writer: Pin<&'b mut (dyn FuseWrite + Send)>,
    /// request header
    header: FuseInHeader<'b>,
    /// protocol version
    #[allow(dead_code)]
    proto: ProtocolVersion,
}

/// protocol version
#[derive(Debug, Clone, Copy)]
pub struct ProtocolVersion {
    /// major version number
    pub major: u32,
    /// minor version number
    pub minor: u32,
}

impl Debug for FuseContext<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuseContext")
            .field("writer", &"<Pin<&'b mut (dyn FuseWrite + Send)>>")
            .field("header", &self.header)
            .finish()
    }
}

/// decode
fn decode<'b, T: Decode<'b>>(
    de: &mut Decoder<'b>,
    proto: ProtocolVersion,
) -> Result<T, DecodeError> {
    T::decode(de, proto)
}

/// parse
fn parse(
    buf: &'_ [u8],
    proto: ProtocolVersion,
) -> Result<(FuseInHeader<'_>, Operation<'_>), DecodeError> {
    let mut de = Decoder::new(buf);
    de.all_consuming(|de| {
        let header: FuseInHeader<'_> = decode(de, proto)?;
        let opcode = header.0.opcode;

        assert_eq!(usize::try_from(header.0.len), Ok(buf.len()));

        let op = match opcode {
            FUSE_FLUSH => Operation::Flush(decode(de, proto)?),
            FUSE_FORGET => Operation::Forget(decode(de, proto)?),
            FUSE_FSYNC => Operation::FSync(decode(de, proto)?),
            FUSE_GETATTR => Operation::GetAttr(decode(de, proto)?),
            FUSE_GETXATTR => Operation::GetXAttr(decode(de, proto)?),
            FUSE_INIT => Operation::Init(decode(de, proto)?),
            FUSE_INTERRUPT => Operation::Interrupt(decode(de, proto)?),
            FUSE_LOOKUP => Operation::Lookup(decode(de, proto)?),
            FUSE_MKDIR => Operation::MkDir(decode(de, proto)?),
            FUSE_MKNOD => Operation::MkNod(decode(de, proto)?),
            FUSE_OPEN => Operation::Open(decode(de, proto)?),
            FUSE_OPENDIR => Operation::OpenDir(decode(de, proto)?),
            FUSE_READ => Operation::Read(decode(de, proto)?),
            FUSE_READDIR => Operation::ReadDir(decode(de, proto)?),
            FUSE_READLINK => Operation::ReadLink(decode(de, proto)?),
            FUSE_RELEASE => Operation::Release(decode(de, proto)?),
            FUSE_RELEASEDIR => Operation::ReleaseDir(decode(de, proto)?),
            FUSE_RMDIR => Operation::RmDir(decode(de, proto)?),
            FUSE_SETATTR => Operation::SetAttr(decode(de, proto)?),
            FUSE_SYMLINK => Operation::SymLink(decode(de, proto)?),
            FUSE_STATFS => Operation::StatFs(decode(de, proto)?),
            FUSE_UNLINK => Operation::Unlink(decode(de, proto)?),
            FUSE_WRITE => Operation::Write(decode(de, proto)?),
            // TODO: add more operations
            _ => {
                tracing::error!(%opcode, "unimplemented operation");
                return Err(DecodeError::InvalidValue);
            }
        };
        Ok((header, op))
    })
}

impl<'b> FuseContext<'b> {
    /// Creates a [`FuseContext`]
    /// # Errors
    /// Returns `DecodeError`
    #[inline]
    pub fn new(
        buf: &'b [u8],
        writer: Pin<&'b mut (dyn FuseWrite + Send)>,
        proto: ProtocolVersion,
    ) -> Result<(Self, Operation<'b>), DecodeError> {
        let (header, op) = parse(buf, proto)?;
        let cx = Self {
            writer,
            header,
            proto,
        };
        Ok((cx, op))
    }

    /// Gets the request header
    #[must_use]
    #[inline]
    pub const fn header(&self) -> &FuseInHeader<'_> {
        &self.header
    }

    /// Sends reply
    /// # Errors
    /// Returns [`io::Error`] when failed to write bytes to the connection
    #[allow(clippy::future_not_send)]
    #[inline]
    pub async fn reply<T, R>(mut self, _: &T, mut reply: R) -> io::Result<()>
    where
        R: IsReplyOf<T> + Encode,
    {
        reply.set_version(self.proto);

        let header;
        let header_len = mem::size_of::<kernel::fuse_out_header>();

        let mut bufs: SmallVec<[IoSlice<'_>; 8]> = SmallVec::new();

        bufs.push(IoSlice::new(&[]));

        reply.collect_bytes(&mut bufs);

        let body_len: usize = bufs.iter().map(|b| b.len()).fold(0, |acc, x| {
            let (ans, is_overflow) = acc.overflowing_add(x);
            if is_overflow {
                panic!("iov length overflow: acc = {}, x = {}", acc, x)
            }
            ans
        });

        let handle_overflow = || {
            panic!(
                "number overflow: header_len = {}, body_len = {}",
                header_len, body_len
            )
        };

        let total_len: u32 = header_len
            .checked_add(body_len)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or_else(handle_overflow);

        header = kernel::fuse_out_header {
            len: total_len,
            unique: self.header.0.unique,
            error: 0,
        };
        bufs[0] = IoSlice::new(encode::as_abi_bytes(&header));

        poll_fn(|cx| self.writer.as_mut().poll_reply(cx, &*bufs)).await?;

        Ok(())
    }

    /// Sends errno
    /// # Errors
    /// Returns [`io::Error`] when failed to write bytes to the connection
    #[inline]
    pub async fn reply_err(mut self, errno: Errno) -> io::Result<()> {
        let header_len: usize = mem::size_of::<kernel::fuse_out_header>();

        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        // the size of a single kernel struct can not overflow u32
        // `try_from` has runtime cost
        let len: u32 = header_len as u32;

        let header;

        header = kernel::fuse_out_header {
            len,
            unique: self.header.0.unique,
            error: errno.as_raw().wrapping_neg(),
        };
        let bufs = [IoSlice::new(encode::as_abi_bytes(&header))];
        poll_fn(|cx| self.writer.as_mut().poll_reply(cx, &bufs)).await?;

        Ok(())
    }
}
