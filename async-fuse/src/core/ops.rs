//! FUSE operations

use super::decode::{Decode, DecodeError, Decoder};
use super::encode::{self, Encode};

#[allow(clippy::wildcard_imports)]
use super::kernel::*;

#[allow(clippy::wildcard_imports)]
use super::kernel::fuse_init_flags::*;

use crate::types::file::{FileType, StMode};
use crate::utils::as_bytes_unchecked;
use crate::utils::c_bytes::{self, CBytes, NulError};

use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::IoSlice;
use std::mem;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitflags::bitflags;

/// Self is a reply of T
pub trait IsReplyOf<T> {}

/// FUSE operations
#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'b> {
    Flush(OpFlush<'b>),
    FSync(OpFSync<'b>),
    Forget(OpForget<'b>),
    GetAttr(OpGetAttr<'b>),
    GetXAttr(OpGetXAttr<'b>),
    Init(OpInit<'b>),
    Interrupt(OpInterrupt<'b>),
    Lookup(OpLookup<'b>),
    MkDir(OpMkDir<'b>),
    MkNod(OpMkNod<'b>),
    Open(OpOpen<'b>),
    OpenDir(OpOpenDir<'b>),
    Read(OpRead<'b>),
    ReadDir(OpReadDir<'b>),
    ReadLink(OpReadLink<'b>),
    Release(OpRelease<'b>),
    ReleaseDir(OpReleaseDir<'b>),
    RmDir(OpRmDir<'b>),
    SetAttr(OpSetAttr<'b>),
    StatFs(OpStatFs<'b>),
    SymLink(OpSymLink<'b>),
    Unlink(OpUnlink<'b>),
    Write(OpWrite<'b>),
}

macro_rules! derive_Decode {
    ($t:ty) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(
                de: &mut Decoder<'b>,
            ) -> Result<Self, DecodeError> {
                Ok(Self(de.fetch()?))
            }
        }
    };

    (@c_bytes $t:ty,$($f:ident),+) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(
                de: &mut Decoder<'b>,
            ) -> Result<Self, DecodeError> {
                $(let $f = de.fetch_c_bytes()?;)+
                Ok(Self{$($f),+})
            }
        }
    };

    (@header $t:ty, $h:ident, $b: ident) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(
                de: &mut Decoder<'b>,
            ) -> Result<Self, DecodeError> {
                let $h = de.fetch()?;
                let $b = de.fetch_c_bytes()?;
                Ok(Self{$h, $b})
            }
        }
    };

    (@data $t:ty, $h:ident, $b: ident) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(
                de: &mut Decoder<'b>,
            ) -> Result<Self, DecodeError> {
                let $h = de.fetch()?;
                let $b = de.fetch_all_bytes()?;
                Ok(Self{$h, $b})
            }
        }
    };

    (@empty $t:ty) => {
        impl<'b> Decode<'b> for $t {
            #[inline]
            fn decode(
                _: &mut Decoder<'b>,
            ) -> Result<Self, DecodeError> {
                Ok(Self(&()))
            }
        }
    }
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
// OpInit
// ReplyInit

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
// ReplyEmpty

#[derive(Debug, Clone, Copy, Default)]
pub struct ReplyEmpty(());

impl Encode for ReplyEmpty {
    #[inline]
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let _ = container;
    }
}

// ----------------------------------------------------------------------------
// Attr
// ReplyAttr
// OpGetAttr
// OpSetAttr

#[derive(Debug, Default)]
pub struct Attr(fuse_attr);

impl Attr {
    setters!(
        ino: u64,
        size: u64,
        blocks: u64,
        nlink: u32,
        uid: u32,
        gid: u32,
        rdev: u32,
        blksize: u32,
    );

    #[inline]
    pub fn mode(&mut self, mode: StMode) -> &mut Self {
        self.0.mode = mode.as_raw();
        self
    }

    #[inline]
    pub fn atime(&mut self, time: SystemTime) -> &mut Self {
        let time = time.duration_since(UNIX_EPOCH).unwrap_or_default();

        self.0.atime = time.as_secs();
        self.0.atimensec = time.subsec_nanos();
        self
    }

    #[inline]
    pub fn mtime(&mut self, time: SystemTime) -> &mut Self {
        let time = time.duration_since(UNIX_EPOCH).unwrap_or_default();

        self.0.mtime = time.as_secs();
        self.0.mtimensec = time.subsec_nanos();
        self
    }

    #[inline]
    pub fn ctime(&mut self, time: SystemTime) -> &mut Self {
        let time = time.duration_since(UNIX_EPOCH).unwrap_or_default();

        self.0.ctime = time.as_secs();
        self.0.ctimensec = time.subsec_nanos();
        self
    }
}

#[derive(Debug, Default)]
pub struct ReplyAttr(fuse_attr_out);

impl ReplyAttr {
    #[inline]
    pub fn attr_valid(&mut self, timeout: Duration) -> &mut Self {
        self.0.attr_valid = timeout.as_secs();
        self.0.attr_valid_nsec = timeout.subsec_nanos();
        self
    }

    #[inline]
    pub fn attr(&mut self, attr: Attr) -> &mut Self {
        self.0.attr = attr.0;
        self
    }
}

#[derive(Debug)]
pub struct OpGetAttr<'b>(&'b fuse_getattr_in);

#[derive(Debug)]
pub struct OpSetAttr<'b>(&'b fuse_setattr_in);

derive_Decode!(OpGetAttr<'b>);
derive_Decode!(OpSetAttr<'b>);

derive_Encode!(ReplyAttr);

declare_relation!(OpGetAttr<'_> => ReplyAttr);
declare_relation!(OpSetAttr<'_> => ReplyAttr);

// ----------------------------------------------------------------------------
// Entry
// ReplyEntry

#[derive(Debug, Default)]
pub struct Entry(fuse_entry_out);

impl Entry {
    setters!(nodeid: u64, generation: u64,);

    #[inline]
    pub fn attr(&mut self, attr: Attr) -> &mut Self {
        self.0.attr = attr.0;
        self
    }

    #[inline]
    pub fn entry_valid(&mut self, timeout: Duration) -> &mut Self {
        self.0.entry_valid = timeout.as_secs();
        self.0.entry_valid_nsec = timeout.subsec_nanos();
        self
    }

    #[inline]
    pub fn attr_valid(&mut self, timeout: Duration) -> &mut Self {
        self.0.attr_valid = timeout.as_secs();
        self.0.attr_valid_nsec = timeout.subsec_nanos();
        self
    }
}

#[derive(Debug)]
pub struct ReplyEntry(fuse_entry_out);

impl ReplyEntry {
    #[must_use]
    #[inline]
    pub const fn new(entry: Entry) -> Self {
        Self(entry.0)
    }
}

derive_Encode!(ReplyEntry);

// ----------------------------------------------------------------------------
// OpLookup

#[derive(Debug)]
pub struct OpLookup<'b> {
    name: CBytes<'b>,
}

impl<'b> OpLookup<'b> {
    #[must_use]
    #[inline]
    pub fn name(&self) -> &'b [u8] {
        self.name.as_bytes()
    }
}

derive_Decode!(@c_bytes OpLookup<'b>, name);

declare_relation!(OpLookup<'_> => ReplyEntry);

// ----------------------------------------------------------------------------
// OpForget

#[derive(Debug)]
pub struct OpForget<'b>(&'b fuse_forget_in);

derive_Decode!(OpForget<'b>);

// ----------------------------------------------------------------------------
// OpReadLink

#[derive(Debug)]
pub struct OpReadLink<'b>(&'b ());

derive_Decode!(@empty OpReadLink<'b>);

#[derive(Debug)]
pub struct ReplyReadLink<'a>(&'a [u8]); // bytes without NUL (?)

impl<'a> ReplyReadLink<'a> {
    /// # Errors
    /// Returns [`io::Error`]
    #[inline]
    pub fn new(link_name: &'a [u8]) -> Result<Self, NulError> {
        c_bytes::check_bytes(link_name)?;
        Ok(Self(link_name))
    }
}

impl Encode for ReplyReadLink<'_> {
    #[inline]
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        encode::add_bytes(container, self.0);
    }
}

declare_relation!(OpReadLink<'_> => ReplyReadLink<'a>);

// ----------------------------------------------------------------------------
// OpSymLink
// OpUnlink
// OpLink

#[derive(Debug)]
pub struct OpSymLink<'b> {
    name: CBytes<'b>,
    link: CBytes<'b>,
}

derive_Decode!(@c_bytes OpSymLink<'b>, name, link);

declare_relation!(OpSymLink<'_> => ReplyEntry);

#[derive(Debug)]
pub struct OpUnlink<'b> {
    name: CBytes<'b>,
}

derive_Decode!(@c_bytes OpUnlink<'b>, name);

declare_relation!(OpUnlink<'_> => ReplyEmpty);

#[derive(Debug)]
pub struct OpLink<'b> {
    arg: &'b fuse_link_in,
    name: CBytes<'b>,
}

derive_Decode!(@header OpLink<'b>, arg, name);

// ----------------------------------------------------------------------------
// OpMkNod
// OpMkDir
// OpRmDir

#[derive(Debug)]
pub struct OpMkNod<'b> {
    arg: &'b fuse_mknod_in,
    name: CBytes<'b>,
}

derive_Decode!(@header OpMkNod<'b>, arg, name);

declare_relation!(OpMkNod<'_> => ReplyEntry);

#[derive(Debug)]
pub struct OpMkDir<'b> {
    arg: &'b fuse_mkdir_in,
    name: CBytes<'b>,
}

derive_Decode!(@header OpMkDir<'b>, arg, name);

declare_relation!(OpMkDir<'_> => ReplyEntry);

#[derive(Debug)]
pub struct OpRmDir<'b> {
    name: CBytes<'b>,
}

derive_Decode!(@c_bytes OpRmDir<'b>, name);

declare_relation!(OpRmDir<'_> => ReplyEmpty);

// ----------------------------------------------------------------------------
// OpOpen
// ReplyOpen

#[derive(Debug)]
pub struct OpOpen<'b>(&'b fuse_open_in);

derive_Decode!(OpOpen<'b>);

impl OpOpen<'_> {
    getters!(flags: u32,); // FIXME: use bitflags
}

#[derive(Debug, Default)]
pub struct ReplyOpen(fuse_open_out);

derive_Encode!(ReplyOpen);

declare_relation!(OpOpen<'_> => ReplyOpen);

impl ReplyOpen {
    setters!(
        fh: u64,
        open_flags: u32, // FIXME: use bitflags
    );
}

// ----------------------------------------------------------------------------
// ReplyData

#[derive(Debug)]
pub struct ReplyData<'a> {
    buf: &'a [u8],
    offset: usize,
    max_write_size: usize,
}

impl<'a> ReplyData<'a> {
    #[must_use]
    #[inline]
    pub const fn new(buf: &'a [u8], offset: usize, max_write_size: usize) -> Self {
        Self {
            buf,
            offset,
            max_write_size,
        }
    }
}

impl Encode for ReplyData<'_> {
    #[inline]
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let start = self.offset.min(self.buf.len());
        let end = self
            .offset
            .saturating_add(self.max_write_size)
            .min(self.buf.len());
        encode::add_bytes(container, &self.buf[start..end])
    }
}

// ----------------------------------------------------------------------------
// OpRead

#[derive(Debug)]
pub struct OpRead<'b>(&'b fuse_read_in);

derive_Decode!(OpRead<'b>);

impl OpRead<'_> {
    getters!(
        fh: u64,
        offset: u64,
        size: u32,
        read_flags: u32,
        lock_owner: u64,
        flags: u32,
    );
}

declare_relation!(OpRead<'_> => ReplyData<'_>);

// ----------------------------------------------------------------------------
// OpWrite

#[derive(Debug)]
pub struct OpWrite<'b> {
    arg: &'b fuse_write_in,
    data: &'b [u8],
}

derive_Decode!(@data OpWrite<'b>, arg, data);

#[derive(Debug)]
pub struct ReplyWrite(fuse_write_out);

derive_Encode!(ReplyWrite);

declare_relation!(OpWrite<'_> => ReplyWrite);

// ----------------------------------------------------------------------------
// OpStatFs
// OpRelease
// OpFSync

#[derive(Debug)]
pub struct OpStatFs<'b>(&'b ());

derive_Decode!(@empty OpStatFs<'b>);

#[derive(Debug)]
pub struct ReplyStatFs(fuse_statfs_out);

derive_Encode!(ReplyStatFs);

declare_relation!(OpStatFs<'_> => ReplyStatFs);

#[derive(Debug)]
pub struct OpRelease<'b>(&'b fuse_release_in);

derive_Decode!(OpRelease<'b>);

declare_relation!(OpRelease<'_> => ReplyEmpty);

#[derive(Debug)]
pub struct OpFSync<'b>(&'b fuse_fsync_in);

derive_Decode!(OpFSync<'b>);

declare_relation!(OpFSync<'_> => ReplyEmpty);

// ----------------------------------------------------------------------------
// Directory
// ReplyDirectory

#[derive(Debug, Default)]
pub struct Directory<'a> {
    buf: Cow<'a, [u8]>,
}

impl Directory<'_> {
    #[must_use]
    #[inline]
    pub fn by_ref(&self) -> Directory<'_> {
        Directory {
            buf: Cow::Borrowed(&*self.buf),
        }
    }

    #[must_use]
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Cow::Owned(Vec::with_capacity(cap)),
        }
    }

    #[inline]
    pub fn add_entry(&mut self, ino: u64, file_type: FileType, name: &[u8]) -> &mut Self {
        let handle = |err| {
            panic!(
                "input bytes contain an interior nul byte: err = {}, name = {:?}",
                err, name
            )
        };

        self.try_add_entry(ino, file_type, name)
            .unwrap_or_else(handle);
        self
    }

    /// # Errors
    /// Returns [`NulError`]
    #[inline]
    pub fn try_add_entry(
        &mut self,
        ino: u64,
        file_type: FileType,
        name: &[u8],
    ) -> Result<(), NulError> {
        /// <https://doc.rust-lang.org/std/alloc/struct.Layout.html#method.padding_needed_for>
        ///
        /// <https://doc.rust-lang.org/src/core/alloc/layout.rs.html#226-250>
        const fn round_up(len: usize, align: usize) -> usize {
            len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1)
        }

        #[repr(C)]
        struct DirEntry {
            ino: u64,
            off: u64,
            namelen: u32,
            r#type: u32,
        }

        c_bytes::check_bytes(name)?;

        // FIXME: what is the proper length limit?

        let handle_namelen_overflow =
            |_| panic!("name.len() overflows u32: name.len() = {}", name.len());
        let namelen: u32 = u32::try_from(name.len()).unwrap_or_else(handle_namelen_overflow);

        let entry_len = fuse_dirent::offset_of_name().wrapping_add(name.len());
        let entry_len_padded = round_up(entry_len, mem::size_of::<u64>());

        let handle_offset_overflow = |_| {
            panic!(
                "offset overflows u64: buf.len() = {}, entry_len_padded = {}",
                self.buf.len(),
                entry_len_padded
            )
        };
        let offset: u64 = u64::try_from(self.buf.len().wrapping_add(entry_len_padded))
            .unwrap_or_else(handle_offset_overflow);

        let entry_type = file_type.as_u32();

        let entry = DirEntry {
            ino,
            off: offset, // the offset of next entry
            namelen,
            r#type: entry_type,
        };

        let buf = self.buf.to_mut();
        buf.reserve(entry_len_padded);

        unsafe {
            let bytes = as_bytes_unchecked(&entry);
            buf.extend_from_slice(bytes);
        }

        buf.extend_from_slice(name);

        unsafe {
            let end_ptr = buf.as_mut_ptr().add(buf.len());
            let pad_len = entry_len_padded.wrapping_sub(entry_len);
            end_ptr.write_bytes(0, pad_len);
            let new_len = buf.len().wrapping_add(pad_len);
            buf.set_len(new_len);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ReplyDirectory<'a> {
    dir: Directory<'a>,
    offset: usize,
    max_write_size: usize,
}

impl<'a> ReplyDirectory<'a> {
    #[must_use]
    #[inline]
    pub const fn new(dir: Directory<'a>, offset: usize, max_write_size: usize) -> Self {
        Self {
            dir,
            offset,
            max_write_size,
        }
    }
}

impl Encode for ReplyDirectory<'_> {
    #[inline]
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let buf: &[u8] = &*self.dir.buf;
        let start = self.offset.min(buf.len());
        let end = self
            .offset
            .saturating_add(self.max_write_size)
            .min(buf.len());
        let bytes = &buf[start..end];
        encode::add_bytes(container, bytes);
    }
}

// ----------------------------------------------------------------------------
// OpReadDir
// OpOpenDir
// ReplyOpenDir
// OpReleaseDir

#[derive(Debug)]
pub struct OpReadDir<'b>(&'b fuse_read_in);

derive_Decode!(OpReadDir<'b>);

impl OpReadDir<'_> {
    getters!(
        fh: u64,
        offset: u64,
        size: u32,
        read_flags: u32, // FIXME: use bitflags
        lock_owner: u64,
        flags: u32, // FIXME: use bitflags
    );
}

declare_relation!(OpReadDir<'_> => ReplyDirectory<'_>);

#[derive(Debug)]
pub struct OpOpenDir<'b>(&'b fuse_open_in);

derive_Decode!(OpOpenDir<'b>);

impl OpOpenDir<'_> {
    getters!(flags: u32,); // FIXME: use bitflags
}

#[derive(Debug, Default)]
pub struct ReplyOpenDir(fuse_open_out);

derive_Encode!(ReplyOpenDir);

declare_relation!(OpOpenDir<'_> => ReplyOpenDir);

impl ReplyOpenDir {
    setters!(
        fh: u64,
        open_flags: u32, // FIXME: use bitflags
    );
}

#[derive(Debug)]
pub struct OpReleaseDir<'b>(&'b fuse_release_in);

derive_Decode!(OpReleaseDir<'b>);

declare_relation!(OpReleaseDir<'_> => ReplyEmpty);

// ----------------------------------------------------------------------------
// OpFlush

#[derive(Debug)]
pub struct OpFlush<'b>(&'b fuse_flush_in);

derive_Decode!(OpFlush<'b>);

declare_relation!(OpFlush<'_> => ReplyEmpty);

// ----------------------------------------------------------------------------
// OpInterrupt

#[derive(Debug)]
pub struct OpInterrupt<'b>(&'b fuse_interrupt_in);

derive_Decode!(OpInterrupt<'b>);

declare_relation!(OpInterrupt<'_> => ReplyEmpty);

// ----------------------------------------------------------------------------
// OpGetXAttr

#[derive(Debug)]
pub struct OpGetXAttr<'b> {
    arg: &'b fuse_getxattr_in,
    name: CBytes<'b>,
}

derive_Decode!(@header OpGetXAttr<'b>, arg, name);

#[derive(Debug)]
pub struct ReplyGetXAttr<'a> {
    out: fuse_getxattr_out,
    buf: &'a [u8],
}

impl<'a> ReplyGetXAttr<'a> {
    #[must_use]
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        let buf_len =
            u32::try_from(buf.len()).unwrap_or_else(|e| panic!("buf is too large: {}", e));

        Self {
            out: fuse_getxattr_out {
                size: buf_len,
                padding: Default::default(),
            },
            buf,
        }
    }
}

impl Encode for ReplyGetXAttr<'_> {
    #[inline]
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let bufs = [encode::as_abi_bytes(&self.out), self.buf];
        container.extend(bufs.iter().map(|&b| IoSlice::new(b)));
    }
}
