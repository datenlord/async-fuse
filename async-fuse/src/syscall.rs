use crate::utils;

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int, c_longlong, c_void};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, mem, ptr};

use better_as::number::WrappingCast;
use nix::fcntl::SpliceFFlags;

/// Calls `read(2)`
pub fn read(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    unsafe {
        let buf_ptr: *mut c_void = buf.as_mut_ptr().cast();
        let ret: isize = libc::read(fd, buf_ptr, buf.len());
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

/// Calls `readv(2)`
pub fn read_vectored(fd: RawFd, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSliceMut is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = utils::usize_to_c_int(bufs.len());

        let ret: isize = libc::readv(fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

/// Calls `write(2)`
pub fn write(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
    unsafe {
        let buf_ptr: *const c_void = buf.as_ptr().cast();
        let ret: isize = libc::write(fd, buf_ptr, buf.len());
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

/// Calls `writev(2)`
pub fn write_vectored(fd: RawFd, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSlice is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = utils::usize_to_c_int(bufs.len());

        let ret: isize = libc::writev(fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

pub fn splice(
    fd_in: RawFd,
    off_in: Option<usize>,
    fd_out: RawFd,
    off_out: Option<usize>,
    len: usize,
    flags: SpliceFFlags,
) -> io::Result<usize> {
    let mut raw_off_in: libc::c_longlong;
    let mut raw_off_out: libc::c_longlong;
    let p_off_in = match off_in {
        None => ptr::null_mut(),
        Some(n) => {
            raw_off_in = utils::usize_to_c_longlong(n);
            &mut raw_off_in
        }
    };
    let p_off_out = match off_out {
        None => ptr::null_mut(),
        Some(n) => {
            raw_off_out = utils::usize_to_c_longlong(n);
            &mut raw_off_out
        }
    };

    unsafe {
        let ret = libc::splice(fd_in, p_off_in, fd_out, p_off_out, len, flags.bits());
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret.wrapping_cast())
        }
    }
}

/// Calls `stat(2)`
pub fn stat(path: &CStr) -> io::Result<libc::stat> {
    unsafe {
        let mut stat: libc::stat = MaybeUninit::zeroed().assume_init();
        let ret = libc::stat(path.as_ptr(), &mut stat);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        debug_assert_eq!(ret, 0);
        Ok(stat)
    }
}
