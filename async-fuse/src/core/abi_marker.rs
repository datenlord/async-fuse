//! Internal marker for FUSE ABI types.

#[allow(clippy::wildcard_imports)]
use super::kernel::*;

/// FUSE ABI types.
/// It is safe to transmute a `&[u8]` to `&T` where `T: FuseAbiData + Sized`
pub unsafe trait FuseAbiData {}

macro_rules! mark_abi_type {
    ($t: ident) => {
        unsafe impl FuseAbiData for $t {}
    };
}

macro_rules! mark_sized_types {
    ($($t:ident,)+) => {
        $(
            mark_abi_type!($t);
        )+

        #[test]
        fn size_check(){
            use std::mem;

            $(
                assert!(mem::size_of::<$t>() > 0);
            )+

            $(
                assert!(mem::size_of::<$t>() <= 256);
            )+
        }
    };
}

mark_abi_type!(u8);
mark_abi_type!(u16);
mark_abi_type!(u32);
mark_abi_type!(u64);
mark_abi_type!(usize);
mark_abi_type!(i8);
mark_abi_type!(i16);
mark_abi_type!(i32);
mark_abi_type!(i64);
mark_abi_type!(isize);

mark_abi_type!(fuse_dirent); // DST
mark_abi_type!(fuse_direntplus); // DST

mark_sized_types!(
    fuse_attr,
    fuse_kstatfs,
    fuse_file_lock,
    fuse_entry_out,
    fuse_forget_in,
    fuse_forget_one,
    fuse_batch_forget_in,
    fuse_getattr_in,
    fuse_attr_out,
    fuse_mknod_in,
    fuse_mkdir_in,
    fuse_rename_in,
    fuse_rename2_in,
    fuse_link_in,
    fuse_setattr_in,
    fuse_open_in,
    fuse_create_in,
    fuse_open_out,
    fuse_release_in,
    fuse_flush_in,
    fuse_read_in,
    fuse_write_in,
    fuse_write_out,
    fuse_statfs_out,
    fuse_fsync_in,
    fuse_setxattr_in,
    fuse_getxattr_in,
    fuse_getxattr_out,
    fuse_lk_in,
    fuse_lk_out,
    fuse_access_in,
    fuse_init_in,
    fuse_init_out,
    cuse_init_in,
    cuse_init_out,
    fuse_interrupt_in,
    fuse_bmap_in,
    fuse_bmap_out,
    fuse_ioctl_in,
    fuse_ioctl_iovec,
    fuse_ioctl_out,
    fuse_poll_in,
    fuse_poll_out,
    fuse_notify_poll_wakeup_out,
    fuse_fallocate_in,
    fuse_in_header,
    fuse_out_header,
    // fuse_dirent, // DST
    // fuse_direntplus, // DST
    fuse_notify_inval_inode_out,
    fuse_notify_inval_entry_out,
    fuse_notify_delete_out,
    fuse_notify_store_out,
    fuse_notify_retrieve_out,
    fuse_notify_retrieve_in,
    fuse_lseek_in,
    fuse_lseek_out,
    fuse_copy_file_range_in,
);
