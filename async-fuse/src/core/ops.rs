//! FUSE operations

use super::decode::{Decode, DecodeError, Decoder};
use super::kernel::fuse_in_header;

/// Self is a reply of T
pub trait IsReplyOf<T> {}

/// FUSE operations
#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'b> {
    /// TODO
    __TODO(&'b [u8]),
}

macro_rules! derive_Decode {
    ($t:ty) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(de: &mut Decoder<'b>) -> Result<Self, DecodeError> {
                Ok(Self(de.fetch()?))
            }
        }
    };
}

/// `fuse_in_header`
#[derive(Debug)]
pub struct FuseInHeader<'b>(pub(crate) &'b fuse_in_header);

derive_Decode!(FuseInHeader<'b>);
