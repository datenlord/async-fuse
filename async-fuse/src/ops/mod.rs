mod init;
pub use self::init::*;

mod header {
    use crate::decode::{Decode, DecodeError, Decoder};
    use crate::kernel;

    pub struct FuseInHeader<'b>(pub(crate) &'b kernel::fuse_in_header);

    impl<'b> Decode<'b> for FuseInHeader<'b> {
        fn decode(de: &mut Decoder<'b>) -> Result<Self, DecodeError> {
            let raw = de.fetch::<kernel::fuse_in_header>()?;
            Ok(Self(raw))
        }
    }

    impl FuseInHeader<'_> {
        pub fn opcode(&self) -> u32 {
            self.0.opcode
        }
    }
}

pub use self::header::FuseInHeader;

#[non_exhaustive]
pub enum Operation<'b> {
    Init(OpInit<'b>),
    // TODO: add more operations
}

pub trait Relation {
    type Reply;
}
