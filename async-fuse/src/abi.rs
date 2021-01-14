//! Internal unsafe marker for FUSE ABI types

use std::{mem, slice};

/// FUSE ABI types.
///
/// It is safe to transmute a `&[u8]` to `&T` where `T: FuseAbiData + Sized`.
///
/// [`FuseAbiData`] can not be implemented for ZSTs.
///
/// [`FuseAbiData`] can be implemented for DSTs but there is no way to construct a custom DST reference.
///
pub unsafe trait FuseAbiData {}

/// # Safety
/// T muse not be changed during the lifetime of `&[u8]`
#[allow(dead_code)] // TODO
#[inline]
pub unsafe fn as_bytes_unchecked<T: Sized>(raw: &T) -> &[u8] {
    let ty_size = mem::size_of::<T>();
    let base: *const u8 = <*const T>::cast(raw);
    slice::from_raw_parts(base, ty_size)
}

/// Transmutes `&T` to `&[u8]` where `T: FuseAbiData + Sized`
#[allow(dead_code)] // TODO
#[inline]
pub fn as_abi_bytes<T: FuseAbiData + Sized>(raw: &T) -> &[u8] {
    unsafe { as_bytes_unchecked(raw) }
}

pub struct RawBytes<T: FuseAbiData + Sized>(T);

impl<T: FuseAbiData + Sized> RawBytes<T> {
    pub fn wrap(raw: T) -> Self {
        Self(raw)
    }
}

impl<T: FuseAbiData + Sized> AsRef<[u8]> for RawBytes<T> {
    fn as_ref(&self) -> &[u8] {
        as_abi_bytes(&self.0)
    }
}

macro_rules! mark_abi_type {
    ($ty: ty) => {
        unsafe impl FuseAbiData for $ty {}
    };
}

macro_rules! mark_sized_types {
    (@kernel $($ty:ident,)+) => {
        $(
            mark_abi_type!(super::kernel::$ty);
        )+

        #[test]
        fn size_check() {
            $(
                assert!(mem::size_of::<super::kernel::$ty>() > 0); // ZST makes no sense
            )+
            $(
                assert!(mem::size_of::<super::kernel::$ty>() <= 256); // detect large types
            )+
        }
    };

    (@primitive $($ty:ty,)+) => {
        $(
            mark_abi_type!($ty);
        )+
    }
}

mark_sized_types!(@primitive
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
);

mark_abi_type!(super::kernel::fuse_dirent); // DST
mark_abi_type!(super::kernel::fuse_direntplus); // DST

mark_sized_types!(@kernel
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
