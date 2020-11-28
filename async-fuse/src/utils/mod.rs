pub mod c_str;
pub mod c_bytes;
pub mod better_as;

use std::any::type_name;
use std::convert::TryFrom;
use std::fmt::{self, Debug, Display};
use std::{ascii, mem, slice};

#[allow(clippy::as_conversions)]
#[inline]
pub fn to_address<T: ?Sized>(ptr: *const T) -> usize {
    ptr as *const () as usize
}

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

#[derive(Debug)]
pub struct DisplayBytes<'a>(pub &'a [u8]);

impl Display for DisplayBytes<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0 {
            for c in ascii::escape_default(b) {
                write!(f, "{}", char::from(c))?;
            }
        }
        write!(f, "\"")
    }
}
