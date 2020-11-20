use super::Relation;

use crate::decode::{Decode, DecodeError, Decoder};
use crate::encode::{self, Encode};
use crate::kernel;

use std::io::IoSlice;

#[derive(Debug)]
pub struct OpInit<'b>(&'b kernel::fuse_init_in);

impl OpInit<'_> {
    pub fn major(&self) -> u32 {
        self.0.major
    }
    pub fn minor(&self) -> u32 {
        self.0.minor
    }
    pub fn max_readahead(&self) -> u32 {
        self.0.max_readahead
    }
    pub fn flags(&self) -> u32 {
        // FIXME: use bitflags
        self.0.flags
    }
}

#[derive(Debug, Default)]
pub struct ReplyInit(kernel::fuse_init_out);

macro_rules! setter {
    ($f:ident,$t:ty) => {
        pub fn $f(&mut self, $f: $t) -> &mut Self {
            self.0.$f = $f;
            self
        }
    };
}

impl ReplyInit {
    setter!(major, u32);
    setter!(minor, u32);
    setter!(max_readahead, u32);
    setter!(flags, u32);
    setter!(max_background, u16);
    setter!(congestion_threshold, u16);
    setter!(max_write, u32);
    setter!(time_gran, u32);
    setter!(max_pages, u16);
}

impl<'b> Decode<'b> for OpInit<'b> {
    fn decode(de: &mut Decoder<'b>) -> Result<Self, DecodeError> {
        let raw = de.fetch::<kernel::fuse_init_in>()?;
        Ok(Self(raw))
    }
}

impl Encode for ReplyInit {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let bytes = encode::as_abi_bytes(&self.0);
        container.extend(Some(IoSlice::new(bytes)))
    }
}

impl Relation for OpInit<'_> {
    type Reply = ReplyInit;
}
