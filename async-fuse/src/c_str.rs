use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::{io, ptr, slice};

use memchr::memchr;

pub fn with<T>(bytes: &[u8], f: impl FnOnce(&CStr) -> io::Result<T>) -> io::Result<T> {
    #[allow(clippy::as_conversions)]
    const STACK_BUF_SIZE: usize = libc::PATH_MAX as usize;

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

#[cfg(test)]
mod tests {

    use std::convert::TryFrom;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn path_max() {
        assert!(usize::try_from(libc::PATH_MAX).is_ok());
        assert!(libc::PATH_MAX >= 1024 && libc::PATH_MAX <= 8192);
    }
}
