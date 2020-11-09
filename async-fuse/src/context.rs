use crate::conn::FuseConn;
use crate::decode::{DecodeError, Decoder};
use crate::encode::{self, Encode};
use crate::errno::Errno;
use crate::kernel::{self, fuse_in_header};
use crate::ops::{self, Operation};

use std::convert::TryFrom;
use std::io::{self, IoSlice};
use std::mem;

use futures_util::AsyncWriteExt;
use smallvec::SmallVec;

pub struct FuseContext<'a> {
    conn: &'a mut (dyn FuseConn + Send),
    header: &'a fuse_in_header,
}

impl<'b> FuseContext<'b> {
    pub fn parse(
        buf: &'b [u8],
        conn: &'b mut (dyn FuseConn + Send),
    ) -> Result<(Self, Operation<'b>), DecodeError> {
        use kernel::fuse_opcode::*;

        let mut de = Decoder::new(buf);
        let (header, op) = de.all_consuming(|de| {
            let header = de.fetch::<fuse_in_header>()?;
            let op = match header.opcode {
                FUSE_INIT => {
                    let args = de.decode::<ops::OpInit>()?;
                    Operation::Init(args)
                }
                // TODO: add more operations
                _ => {
                    tracing::error!(opcode = header.opcode, "unimplemented operation");
                    return Err(DecodeError::InvalidValue);
                }
            };
            Ok((header, op))
        })?;

        let cx = Self { conn, header };

        Ok((cx, op))
    }

    pub fn opcode(&self) -> u32 {
        self.header.opcode
    }

    pub(crate) async fn reply(self, reply: impl Encode) -> io::Result<()> {
        let header;
        let header_len = mem::size_of::<kernel::fuse_out_header>();

        let mut iov_buf: SmallVec<[IoSlice; 8]> = SmallVec::new();

        iov_buf.push(IoSlice::new(&[]));

        reply.collect_bytes(&mut iov_buf);

        let body_len: usize = iov_buf.iter().map(|b| b.len()).sum();

        let total_len: usize = header_len.checked_add(body_len).unwrap();

        header = kernel::fuse_out_header {
            len: u32::try_from(total_len).unwrap(),
            unique: self.header.unique,
            error: 0,
        };
        iov_buf[0] = IoSlice::new(encode::as_abi_bytes(&header));

        let nwrite = self.conn.write_vectored(&*iov_buf).await?; // FIXME: impl write_all_vectored
        assert_eq!(nwrite, total_len);
        Ok(())
    }

    pub async fn reply_err(self, errno: Errno) -> io::Result<()> {
        let header;
        let header_len = mem::size_of::<kernel::fuse_out_header>();
        let total_len = header_len;

        header = kernel::fuse_out_header {
            len: u32::try_from(total_len).unwrap(),
            unique: self.header.unique,
            error: errno.as_raw().wrapping_neg(),
        };
        let buf = encode::as_abi_bytes(&header);

        self.conn.write_all(&*buf).await?;
        Ok(())
    }
}
