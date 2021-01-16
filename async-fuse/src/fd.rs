//! The file desciptor of `/dev/fuse`

use crate::syscall;

use std::ops::Deref;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, mem};

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
                syscall::read(self.0, buf)
            }

            #[inline]
            fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
                syscall::read_vectored(self.0, bufs)
            }
        }
    };
}

macro_rules! impl_Write {
    ($ty:ty) => {
        impl io::Write for $ty {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                syscall::write(self.0, buf)
            }

            #[inline]
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }

            #[inline]
            fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
                syscall::write_vectored(self.0, bufs)
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
