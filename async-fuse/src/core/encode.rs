//! Types which can be converted to some bytes without copy

use super::abi_marker::FuseAbiData;
use crate::utils::as_bytes_unchecked;

use std::io::IoSlice;

/// Types which can be converted to some bytes without copy
pub trait Encode {
    /// Collects bytes from Self
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>;
}

/// convert a FUSE ABI reference to bytes
pub fn as_abi_bytes<T: FuseAbiData + Sized>(raw: &T) -> &[u8] {
    unsafe { as_bytes_unchecked(raw) }
}

/// add bytes to a [`IoSlice`] container
pub fn add_bytes<'c, C>(container: &mut C, bytes: &'c [u8])
where
    C: Extend<IoSlice<'c>>,
{
    container.extend(Some(IoSlice::new(bytes)))
}
