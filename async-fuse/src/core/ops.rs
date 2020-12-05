//! FUSE operations

use super::decode::{Decode, DecodeError, Decoder};
use super::encode::{self, Encode};

#[allow(clippy::wildcard_imports)]
use super::kernel::*;

#[allow(clippy::wildcard_imports)]
use super::kernel::fuse_init_flags::*;

use std::io::IoSlice;

use bitflags::bitflags;

/// Self is a reply of T
pub trait IsReplyOf<T> {}

/// FUSE operations
#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'b> {
    Init(OpInit<'b>),
    __TODO,
}

macro_rules! derive_Decode {
    ($t:ty) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(de: &mut Decoder<'b>) -> Result<Self, DecodeError> {
                Ok(Self(de.fetch()?))
            }
        }
    };
}

macro_rules! derive_Encode {
    ($t:ty) => {
        impl Encode for $t {
            #[inline]
            fn collect_bytes<'c, C>(&'c self, container: &mut C)
            where
                C: Extend<IoSlice<'c>>,
            {
                let bytes = encode::as_abi_bytes(&self.0);
                container.extend(Some(IoSlice::new(bytes)))
            }
        }
    };
}

macro_rules! declare_relation {
    ($op:ty => $reply:ty) => {
        #[allow(single_use_lifetimes)]
        impl<'a> IsReplyOf<$op> for $reply {}
    };
}

macro_rules! getters {
    ($($f:ident: $t:ty,)+) => {$(
        #[must_use]
        #[inline]
        pub const fn $f(&self) -> $t {
            self.0.$f
        }
    )+};
}

macro_rules! setters {
    ($($f:ident: $t:ty,)+) => {$(
        #[inline]
        pub fn $f(&mut self, $f: $t) -> &mut Self {
            self.0.$f = $f;
            self
        }
    )+};
}

macro_rules! flags_getter {
    ($f:ident: $t:ty) => {
        #[must_use]
        #[inline]
        pub const fn $f(&self) -> $t {
            <$t>::from_bits_truncate(self.0.flags)
        }
    };
}

macro_rules! flags_setter {
    ($f:ident: $t:ty) => {
        #[inline]
        pub fn $f(&mut self, $f: $t) -> &mut Self {
            self.0.$f = $f.bits();
            self
        }
    };
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct FuseInHeader<'b>(pub(crate) &'b fuse_in_header);

derive_Decode!(FuseInHeader<'b>);

impl FuseInHeader<'_> {
    getters!(
        opcode: u32,
        unique: u64,
        nodeid: u64,
        uid: u32,
        gid: u32,
        pid: u32,
    );
}

// ----------------------------------------------------------------------------

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

#[derive(Debug)]
pub struct OpInit<'b>(&'b fuse_init_in);

#[derive(Debug, Default)]
pub struct ReplyInit(fuse_init_out);

derive_Decode!(OpInit<'b>);

derive_Encode!(ReplyInit);

declare_relation!(OpInit<'_> => ReplyInit);

impl OpInit<'_> {
    getters!(major: u32, minor: u32, max_readahead: u32,);

    flags_getter!(flags: FuseInitFlags);
}

impl ReplyInit {
    setters!(
        major: u32,
        minor: u32,
        max_readahead: u32,
        max_background: u16,
        congestion_threshold: u16,
        max_write: u32,
        time_gran: u32,
        max_pages: u16,
    );

    flags_setter!(flags: FuseInitFlags);
}

// ----------------------------------------------------------------------------
