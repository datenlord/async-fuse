use std::convert::TryFrom;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, mem};

#[derive(Debug)]
pub struct FuseDesc {
    fd: RawFd,
}

unsafe impl Send for FuseDesc {}
unsafe impl Sync for FuseDesc {}

impl FuseDesc {
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
            Ok(Self { fd: ret })
        }
    }

    pub fn close(self) -> io::Result<()> {
        let fd = self.fd;
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

impl Drop for FuseDesc {
    fn drop(&mut self) {
        unsafe {
            let ret = libc::close(self.fd);
            debug_assert_eq!(ret, 0);
        }
    }
}

impl AsRawFd for FuseDesc {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl FromRawFd for FuseDesc {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd }
    }
}

#[allow(clippy::as_conversions, clippy::cast_sign_loss)]
const fn force_cast(x: isize) -> usize {
    x as usize
}

fn read(fd: &'_ FuseDesc, buf: &mut [u8]) -> io::Result<usize> {
    unsafe {
        let buf_ptr: *mut c_void = buf.as_mut_ptr().cast();
        let ret: isize = libc::read(fd.fd, buf_ptr, buf.len());
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(force_cast(ret))
    }
}

fn read_vectored(fd: &'_ FuseDesc, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSliceMut is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = c_int::try_from(bufs.len()).unwrap();

        let ret: isize = libc::readv(fd.fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(force_cast(ret))
    }
}

impl io::Read for &'_ FuseDesc {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        read_vectored(self, bufs)
    }
}

impl io::Read for FuseDesc {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        read_vectored(self, bufs)
    }
}

fn write(fd: &'_ FuseDesc, buf: &[u8]) -> io::Result<usize> {
    unsafe {
        let buf_ptr: *const c_void = buf.as_ptr().cast();
        let ret: isize = libc::write(fd.fd, buf_ptr, buf.len());
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        Ok(force_cast(ret))
    }
}

fn write_vectored(fd: &'_ FuseDesc, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    unsafe {
        // IoSlice is guaranteed to be ABI compatible with `iovec`
        let iov: *const libc::iovec = bufs.as_ptr().cast();

        let iovcnt: c_int = c_int::try_from(bufs.len()).unwrap();

        let ret: isize = libc::writev(fd.fd, iov, iovcnt);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        // a non-negative `ssize_t` value can not overflow `usize`
        #[allow(clippy::as_conversions)]
        Ok(force_cast(ret))
    }
}

impl io::Write for &'_ FuseDesc {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        write_vectored(self, bufs)
    }
}

impl io::Write for FuseDesc {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        write_vectored(self, bufs)
    }
}
