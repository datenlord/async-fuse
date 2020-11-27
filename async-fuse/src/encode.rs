use crate::abi_marker::FuseAbiData;
use crate::utils::as_bytes_unchecked;

use std::io::IoSlice;

pub trait Encode {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>;
}

pub fn as_abi_bytes<T: FuseAbiData + Sized>(raw: &T) -> &[u8] {
    unsafe { as_bytes_unchecked(raw) }
}

pub fn add_bytes<'c, C>(container: &mut C, bytes: &'c [u8])
where
    C: Extend<IoSlice<'c>>,
{
    container.extend(Some(IoSlice::new(bytes)))
}
