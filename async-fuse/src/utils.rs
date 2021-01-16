use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_longlong};
use std::{io, ptr, slice};

use memchr::memchr;

/// Casts [`usize`] to [`c_int`]
#[track_caller]
pub fn usize_to_c_int(x: usize) -> c_int {
    match c_int::try_from(x) {
        Ok(r) => r,
        Err(e) => panic!(
            "failed to convert usize to c_int: value = {}, error = {}",
            x, e
        ),
    }
}

#[track_caller]
pub fn usize_to_c_longlong(x: usize) -> c_longlong {
    match c_longlong::try_from(x) {
        Ok(r) => r,
        Err(e) => panic!(
            "failed to convert usize to c_longlong: value = {}, error = {}",
            x, e
        ),
    }
}

/// Stores short bytes on stack, stores long bytes on heap and provides [`CStr`].
///
/// The threshold of allocation is [`libc::PATH_MAX`] (4096 on linux).
///
/// # Errors
/// Returns [`io::Error`]
///
/// Generates `InvalidInput` if the input bytes contain an interior nul byte
#[cfg(target_os = "linux")]
#[inline]
pub fn with_c_str<T>(bytes: &[u8], f: impl FnOnce(&CStr) -> io::Result<T>) -> io::Result<T> {
    /// The threshold of allocation
    #[allow(clippy::as_conversions)]
    const STACK_BUF_SIZE: usize = libc::PATH_MAX as usize; // 4096

    if memchr(0, bytes).is_some() {
        let err = io::Error::new(
            io::ErrorKind::InvalidInput,
            "input bytes contain an interior nul byte",
        );
        return Err(err);
    }

    if bytes.len() >= STACK_BUF_SIZE {
        let c_string = unsafe { CString::from_vec_unchecked(Vec::from(bytes)) };
        return f(&c_string);
    }

    let mut buf: MaybeUninit<[u8; STACK_BUF_SIZE]> = MaybeUninit::uninit();

    unsafe {
        let buf: *mut u8 = buf.as_mut_ptr().cast();
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
        buf.add(bytes.len()).write(0);

        let bytes_with_nul = slice::from_raw_parts(buf, bytes.len().wrapping_add(1));
        let c_str = CStr::from_bytes_with_nul_unchecked(bytes_with_nul);

        f(c_str)
    }
}

pub struct FreezedBuf<B>(B, usize);

impl<B: AsRef<[u8]>> FreezedBuf<B> {
    pub fn new(buf: B, len: usize) -> Self {
        Self(buf, len)
    }
}

impl<B: AsRef<[u8]>> AsRef<[u8]> for FreezedBuf<B> {
    fn as_ref(&self) -> &[u8] {
        &self.0.as_ref()[..self.1]
    }
}
