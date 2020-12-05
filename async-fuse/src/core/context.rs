//! The context of a FUSE request

use super::decode::{DecodeError, Decoder};
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

impl<'b> FuseContext<'b> {
    /// Creates a [`FuseContext`]
    #[must_use]
    #[inline]
    pub fn new(writer: Pin<&'b mut (dyn FuseWrite + Send)>, header: FuseInHeader<'b>) -> Self {
        Self { writer, header }
    }

    /// Parses a buffer
    /// # Errors
    /// Returns [`DecodeError`]
    #[inline]
    pub fn parse(buf: &'b [u8]) -> Result<(FuseInHeader<'b>, Operation<'b>), DecodeError> {
        let mut de = Decoder::new(buf);
        de.all_consuming(|de| {
            let header = de.decode::<FuseInHeader<'b>>()?;
            let opcode = header.0.opcode;

            assert_eq!(usize::try_from(header.0.len), Ok(buf.len()));

            #[allow(clippy::single_match_else)]
            let op = match opcode {
                FUSE_INIT => Operation::Init(de.decode()?),
                // TODO: add more operations
                _ => {
                    tracing::error!(%opcode, "unimplemented operation");
                    return Err(DecodeError::InvalidValue);
                }
            };
            Ok((header, op))
        })
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
    pub async fn reply<T, R>(mut self, _: &T, reply: R) -> io::Result<()>
    where
        R: IsReplyOf<T> + Encode,
    {
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
