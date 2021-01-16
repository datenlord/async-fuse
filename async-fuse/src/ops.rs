#[allow(clippy::wildcard_imports)]
use crate::kernel::fuse_init_flags::*;

use bitflags::bitflags;

bitflags! {
    pub struct FuseInitFlags: u32{
        const ASYNC_READ            = FUSE_ASYNC_READ;
        const POSIX_LOCKS           = FUSE_POSIX_LOCKS;
        const FILE_OPS              = FUSE_FILE_OPS;
        const ATOMIC_O_TRUNC        = FUSE_ATOMIC_O_TRUNC;
        const EXPORT_SUPPORT        = FUSE_EXPORT_SUPPORT;
        const BIG_WRITES            = FUSE_BIG_WRITES;
        const DONT_MASK             = FUSE_DONT_MASK;
        const SPLICE_WRITE          = FUSE_SPLICE_WRITE;
        const SPLICE_MOVE           = FUSE_SPLICE_MOVE;
        const SPLICE_READ           = FUSE_SPLICE_READ;
        const FLOCK_LOCKS           = FUSE_FLOCK_LOCKS;
        const HAS_IOCTL_DIR         = FUSE_HAS_IOCTL_DIR;
        const AUTO_INVAL_DATA       = FUSE_AUTO_INVAL_DATA;
        const DO_READDIRPLUS        = FUSE_DO_READDIRPLUS;
        const READDIRPLUS_AUTO      = FUSE_READDIRPLUS_AUTO;
        const ASYNC_DIO             = FUSE_ASYNC_DIO;
        const WRITEBACK_CACHE       = FUSE_WRITEBACK_CACHE;
        const NO_OPEN_SUPPORT       = FUSE_NO_OPEN_SUPPORT;
        const PARALLEL_DIROPS       = FUSE_PARALLEL_DIROPS;
        const HANDLE_KILLPRIV       = FUSE_HANDLE_KILLPRIV;
        const POSIX_ACL             = FUSE_POSIX_ACL;
        const ABORT_ERROR           = FUSE_ABORT_ERROR;
        const MAX_PAGES             = FUSE_MAX_PAGES;
        const CACHE_SYMLINKS        = FUSE_CACHE_SYMLINKS;
        const NO_OPENDIR_SUPPORT    = FUSE_NO_OPENDIR_SUPPORT;
        const EXPLICIT_INVAL_DATA   = FUSE_EXPLICIT_INVAL_DATA;
    }
}
