use crate::decode::{DecodeError, Decoder};
use crate::encode::{self, Encode};
use crate::errno::Errno;
use crate::kernel;
use crate::ops::{self, FuseInHeader, Operation, Relation};
use crate::write::FuseWrite;

use std::convert::TryFrom;
use std::io::{self, IoSlice};
use std::mem;
use std::pin::Pin;

use futures_util::future::poll_fn;
use smallvec::SmallVec;

pub struct FuseContext<'b> {
    writer: Pin<&'b mut (dyn FuseWrite + Send)>,
    header: FuseInHeader<'b>,
}

impl<'b> FuseContext<'b> {
    pub fn new(writer: Pin<&'b mut (dyn FuseWrite + Send)>, header: FuseInHeader<'b>) -> Self {
        Self { writer, header }
    }

    pub fn parse(buf: &'b [u8]) -> Result<(FuseInHeader<'b>, Operation<'b>), DecodeError> {
        use kernel::fuse_opcode::*;

        let mut de = Decoder::new(buf);
        de.all_consuming(|de| {
            let header = de.decode::<FuseInHeader>()?;
            let opcode = header.0.opcode;

            let op = match opcode {
                FUSE_INIT => {
                    let args = de.decode::<ops::OpInit>()?;
                    Operation::Init(args)
                }
                // TODO: add more operations
                _ => {
                    tracing::error!(%opcode, "unimplemented operation");
                    return Err(DecodeError::InvalidValue);
                }
            };
            Ok((header, op))
        })
    }

    pub fn header(&self) -> &FuseInHeader<'_> {
        &self.header
    }

    pub async fn reply<T>(mut self, _: &T, reply: T::Reply) -> io::Result<()>
    where
        T: Relation,
        T::Reply: Encode,
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
            unique: self.header.0.unique,
            error: 0,
        };
        bufs[0] = IoSlice::new(encode::as_abi_bytes(&header));

        poll_fn(|cx| self.writer.as_mut().poll_reply(cx, &*bufs)).await?;

        Ok(())
    }

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
