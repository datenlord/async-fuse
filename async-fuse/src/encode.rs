use crate::abi_marker::FuseAbiData;

use std::io::IoSlice;
use std::mem;
use std::slice;

pub trait Encode {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>;
}

impl Encode for () {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let _ = container;
    }
}

impl Encode for [u8] {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        container.extend(Some(IoSlice::new(self)))
    }
}

impl Encode for [IoSlice<'_>] {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        container.extend(self.iter().copied())
    }
}

pub(crate) fn as_abi_bytes<T: FuseAbiData + Sized>(raw: &T) -> &[u8] {
    let ty_size = mem::size_of::<T>();
    unsafe { slice::from_raw_parts(raw as *const T as *const u8, ty_size) }
}
