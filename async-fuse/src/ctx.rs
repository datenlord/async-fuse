use crate::de::{DecodeError, Decoder};
use crate::encode::{self, Encode};
use crate::errno::Errno;
use crate::io::FuseWrite;
use crate::kernel;
use crate::ops::{FuseInHeader, IsReplyOf, Operation};

use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::io::{self, IoSlice};
use std::mem;
use std::pin::Pin;

use futures_util::future::poll_fn;
use smallvec::SmallVec;

pub struct FuseContext<'b> {
    writer: Pin<&'b mut (dyn FuseWrite + Send)>,
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
    #[must_use]
    #[inline]
    pub fn new(writer: Pin<&'b mut (dyn FuseWrite + Send)>, header: FuseInHeader<'b>) -> Self {
        Self { writer, header }
    }

    #[inline]
    pub fn parse(buf: &'b [u8]) -> Result<(FuseInHeader<'b>, Operation<'b>), DecodeError> {
        use kernel::fuse_opcode::*;

        let mut de = Decoder::new(buf);
        de.all_consuming(|de| {
            let header = de.decode::<FuseInHeader>()?;
            let opcode = header.0.opcode;

            assert_eq!(usize::try_from(header.0.len).unwrap(), buf.len());

            let op = match opcode {
                FUSE_FLUSH => Operation::Flush(de.decode()?),
                FUSE_FORGET => Operation::Forget(de.decode()?),
                FUSE_FSYNC => Operation::FSync(de.decode()?),
                FUSE_GETATTR => Operation::GetAttr(de.decode()?),
                FUSE_GETXATTR => Operation::GetXAttr(de.decode()?),
                FUSE_INIT => Operation::Init(de.decode()?),
                FUSE_INTERRUPT => Operation::Interrupt(de.decode()?),
                FUSE_LOOKUP => Operation::Lookup(de.decode()?),
                FUSE_MKDIR => Operation::MkDir(de.decode()?),
                FUSE_MKNOD => Operation::MkNod(de.decode()?),
                FUSE_OPEN => Operation::Open(de.decode()?),
                FUSE_OPENDIR => Operation::OpenDir(de.decode()?),
                FUSE_READ => Operation::Read(de.decode()?),
                FUSE_READDIR => Operation::ReadDir(de.decode()?),
                FUSE_READLINK => Operation::ReadLink(de.decode()?),
                FUSE_RELEASE => Operation::Release(de.decode()?),
                FUSE_RELEASEDIR => Operation::ReleaseDir(de.decode()?),
                FUSE_RMDIR => Operation::RmDir(de.decode()?),
                FUSE_SETATTR => Operation::SetAttr(de.decode()?),
                FUSE_SYMLINK => Operation::SymLink(de.decode()?),
                FUSE_STATFS => Operation::StatFs(de.decode()?),
                FUSE_UNLINK => Operation::Unlink(de.decode()?),
                FUSE_WRITE => Operation::Write(de.decode()?),
                // TODO: add more operations
                _ => {
                    tracing::error!(%opcode, "unimplemented operation");
                    return Err(DecodeError::InvalidValue);
                }
            };
            Ok((header, op))
        })
    }

    #[must_use]
    #[inline]
    pub const fn header(&self) -> &FuseInHeader<'_> {
        &self.header
    }

    #[allow(clippy::future_not_send)]
    #[inline]
    pub async fn reply<T, R>(mut self, _: &T, reply: R) -> io::Result<()>
    where
        R: IsReplyOf<T> + Encode,
    {
        let header;
        let header_len = mem::size_of::<kernel::fuse_out_header>();

        let mut bufs: SmallVec<[IoSlice; 8]> = SmallVec::new();

        bufs.push(IoSlice::new(&[]));

        reply.collect_bytes(&mut bufs);

        let body_len: usize = bufs.iter().map(|b| b.len()).sum();

        let total_len: usize = header_len.checked_add(body_len).unwrap();

        header = kernel::fuse_out_header {
            len: u32::try_from(total_len).unwrap(),
            unique: self.header.unique(),
            error: 0,
        };
        bufs[0] = IoSlice::new(encode::as_abi_bytes(&header));

        poll_fn(|cx| self.writer.as_mut().poll_reply(cx, &*bufs)).await?;

        Ok(())
    }

    #[inline]
    pub async fn reply_err(mut self, errno: Errno) -> io::Result<()> {
        let header;
        let header_len = mem::size_of::<kernel::fuse_out_header>();
        let total_len = header_len;

        header = kernel::fuse_out_header {
            len: u32::try_from(total_len).unwrap(),
            unique: self.header.0.unique,
            error: errno.as_raw().wrapping_neg(),
        };
        let bufs = [IoSlice::new(encode::as_abi_bytes(&header))];
        poll_fn(|cx| self.writer.as_mut().poll_reply(cx, &bufs)).await?;

        Ok(())
    }
}
