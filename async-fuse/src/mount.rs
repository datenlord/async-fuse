//! mount

use crate::fd::FuseDesc;
use crate::syscall;
use crate::utils;

use std::io;
use std::os::raw::{c_char, c_int, c_uint};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

macro_rules! ensure_type {
    ($lhs:expr, $rhs:ty) => {{
        const fn __check(_: $rhs) {}
        __check($lhs)
    }};
}

/// Mounts a FUSE fd to the given mount point
pub fn mount(fd: &FuseDesc, mount_point: &Path) -> io::Result<()> {
    utils::with_c_str(mount_point.as_os_str().as_bytes(), |target| {
        let stat = syscall::stat(target)?;
        let rootmode = stat.st_mode & libc::S_IFMT;
        let user_id = unsafe { libc::getuid() };
        let group_id = unsafe { libc::getgid() };
        let fd = fd.as_raw_fd();

        ensure_type!(fd, c_int);
        ensure_type!(rootmode, c_uint);
        ensure_type!(user_id, c_uint);
        ensure_type!(group_id, c_uint);

        let mut opts: [c_char; 128] = [0; 128];
        unsafe {
            let format = b"fd=%d,rootmode=%o,user_id=%u,group_id=%u\0";
            let ret = libc::sprintf(
                opts.as_mut_ptr(),
                format.as_ptr().cast(),
                fd,
                rootmode,
                user_id,
                group_id,
            );
            assert!(ret > 0);
        }

        let fstype = b"fuse\0";
        let source = b"/dev/fuse\0";

        unsafe {
            let ret = libc::mount(
                source.as_ptr().cast(),
                target.as_ptr(),
                fstype.as_ptr().cast(),
                libc::MS_NOSUID | libc::MS_NODEV,
                opts.as_ptr().cast(),
            );
            if ret < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    })
}

// pub fn umount(mount_point: &Path) -> io::Result<()> {
//     c_str::with(mount_point.as_os_str().as_bytes(), |target| unsafe {
//         let ret = libc::umount2(target.as_ptr(), libc::MNT_FORCE);
//         if ret < 0 {
//             return Err(io::Error::last_os_error());
//         }
//         Ok(())
//     })
// }
