use std::any::type_name;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::{mem, slice};

pub trait ForceConvert<U> {
    fn force_convert(self) -> U;
}

impl<T, U> ForceConvert<U> for T
where
    T: Debug + Copy + 'static,
    U: TryFrom<T> + 'static,
    U::Error: std::error::Error,
{
    #[inline]
    fn force_convert(self) -> U {
        match U::try_from(self) {
            Ok(u) => u,
            Err(err) => panic!(
                "failed to convert {} to {}, self = {:?}, error = {}",
                type_name::<Self>(),
                type_name::<U>(),
                self,
                err,
            ),
        }
    }
}

/// # Safety
/// T muse have no internal mutability
#[inline]
pub unsafe fn as_bytes_unchecked<T: Sized + Sync>(raw: &T) -> &[u8] {
    let ty_size = mem::size_of::<T>();
    let base: *const u8 = <*const T>::cast(raw);
    slice::from_raw_parts(base, ty_size)
}
