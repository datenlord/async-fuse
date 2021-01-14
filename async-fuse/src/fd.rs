//! The file desciptor of `/dev/fuse`

use std::convert::TryFrom;
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_longlong, c_void};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, mem, ptr};

use better_as::number::WrappingCast;
use nix::fcntl::SpliceFFlags;

/// The file desciptor of `/dev/fuse`
#[derive(Debug)]
pub struct FuseDesc(RawFd);

unsafe impl Send for FuseDesc {}
unsafe impl Sync for FuseDesc {}

impl FuseDesc {
    /// Opens the connection to `/dev/fuse`
    /// # Errors
    /// Returns an error if the underlying syscalls failed
    #[inline]
    pub fn open() -> io::Result<Self> {
        unsafe {
            let dev_path = b"/dev/fuse\0";
            let pathname: *const c_char = dev_path.as_ptr().cast();

            let oflag: c_int = libc::O_RDWR;
            let ret: c_int = libc::open(pathname, oflag);
            if ret < 0 {
                return Err(io::Error::last_os_error());
            }
            debug_assert!(ret > 0);
            Ok(Self(ret))
        }
    }

    /// Closes the connection to `/dev/fuse`
    /// # Errors
    /// Returns an error if the underlying syscalls failed
    #[inline]
    pub fn close(self) -> io::Result<()> {
        let fd = self.0;

        #[allow(clippy::mem_forget)]
        mem::forget(self);

        unsafe {
            let ret: c_int = libc::close(fd);
            if ret < 0 {
                return Err(io::Error::last_os_error());
            }
            debug_assert_eq!(ret, 0);
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct PipeReader(RawFd);

#[derive(Debug)]
pub struct PipeWriter(RawFd);

pub fn create_pipe() -> io::Result<(PipeReader, PipeWriter)> {
    let mut fds: [RawFd; 2] = [0; 2];
    unsafe {
        let ret = libc::pipe(fds.as_mut_ptr());
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok((PipeReader(fds[0]), PipeWriter(fds[1])))
}

/// Casts [`usize`] to [`c_int`]
#[track_caller]
fn usize_to_c_int(x: usize) -> c_int {
    match c_int::try_from(x) {
        Ok(r) => r,
        Err(e) => panic!(
            "failed to convert usize to c_int: value = {}, error = {}",
            x, e
        ),
    }
}

/// Calls `read(2)`
pub(crate) fn read(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
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
pub(crate) fn read_vectored(fd: RawFd, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSliceMut is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = usize_to_c_int(bufs.len());

        let ret: isize = libc::readv(fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

/// Calls `write(2)`
pub(crate) fn write(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
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
pub(crate) fn write_vectored(fd: RawFd, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSlice is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = usize_to_c_int(bufs.len());

        let ret: isize = libc::writev(fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(ret.wrapping_cast())
    }
}

macro_rules! impl_fd_wrapper {
    ($ty: ty) => {
        impl Drop for $ty {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    let ret = libc::close(self.0);
                    debug_assert_eq!(ret, 0);
                }
            }
        }

        impl AsRawFd for $ty {
            #[inline]
            fn as_raw_fd(&self) -> RawFd {
                self.0
            }
        }

        impl FromRawFd for $ty {
            #[inline]
            unsafe fn from_raw_fd(fd: RawFd) -> Self {
                Self(fd)
            }
        }
    };
}

macro_rules! impl_Read {
    ($ty: ty) => {
        impl io::Read for $ty {
            #[inline]
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                read(self.0, buf)
            }

            #[inline]
            fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
                read_vectored(self.0, bufs)
            }
        }
    };
}

macro_rules! impl_Write {
    ($ty:ty) => {
        impl io::Write for $ty {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                write(self.0, buf)
            }

            #[inline]
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }

            #[inline]
            fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
                write_vectored(self.0, bufs)
            }
        }
    };
}

impl_fd_wrapper!(FuseDesc);
impl_fd_wrapper!(PipeReader);
impl_fd_wrapper!(PipeWriter);

impl_Read!(FuseDesc);
impl_Read!(&'_ FuseDesc);
impl_Read!(PipeReader);

impl_Write!(FuseDesc);
impl_Write!(&'_ FuseDesc);
impl_Write!(PipeWriter);

pub struct OwnedFd<P>(pub P);

impl<F, P> AsRawFd for OwnedFd<P>
where
    P: Deref<Target = F>,
    F: AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.0.deref().as_raw_fd()
    }
}

#[track_caller]
fn usize_to_c_longlong(x: usize) -> c_longlong {
    match c_longlong::try_from(x) {
        Ok(r) => r,
        Err(e) => panic!(
            "failed to convert usize to c_longlong: value = {}, error = {}",
            x, e
        ),
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
            raw_off_in = usize_to_c_longlong(n);
            &mut raw_off_in
        }
    };
    let p_off_out = match off_out {
        None => ptr::null_mut(),
        Some(n) => {
            raw_off_out = usize_to_c_longlong(n);
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
