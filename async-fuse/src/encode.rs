use crate::abi_marker::FuseAbiData;

use std::io::IoSlice;
use std::mem;
use std::slice;

pub trait Encode {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>;
}

pub(crate) fn as_abi_bytes<T: FuseAbiData + Sized>(raw: &T) -> &[u8] {
    let ty_size = mem::size_of::<T>();
    unsafe { slice::from_raw_parts(raw as *const T as *const u8, ty_size) }
}

pub(crate) fn add_bytes<'c, C>(container: &mut C, bytes: &'c [u8])
where
    C: Extend<IoSlice<'c>>,
{
    container.extend(Some(IoSlice::new(bytes)))
}
