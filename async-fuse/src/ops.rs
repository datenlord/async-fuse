use crate::c_bytes::{self, CBytes, NulError};
use crate::decode::{Decode, DecodeError, Decoder};
use crate::encode::{self, Encode};
use crate::kernel::*;

#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'b> {
    Init(OpInit<'b>),
    Lookup(OpLookup<'b>),
    Forget(OpForget<'b>),
    GetAttr(OpGetAttr<'b>),
    SetAttr(OpSetAttr<'b>),
    ReadLink(OpReadLink<'b>),
    SymLink(OpSymLink<'b>),
    Unlink(OpUnlink<'b>),
    MkNod(OpMkNod<'b>),
    MkDir(OpMkDir<'b>),
    RmDir(OpRmDir<'b>),
    Open(OpOpen<'b>),
    Read(OpRead<'b>),
    Write(OpWrite<'b>),
    StatFs(OpStatFs<'b>),
    Release(OpRelease<'b>),
    FSync(OpFSync<'b>),
}

pub trait IsReplyOf<T> {}

pub struct ReplyEmpty(());

impl Encode for ReplyEmpty {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        let _ = container;
    }
}

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

#[derive(Debug)]
pub struct OpInit<'b>(&'b fuse_init_in);

#[derive(Debug, Default)]
pub struct ReplyInit(fuse_init_out);

derive_Decode!(OpInit<'b>);

derive_Encode!(ReplyInit);

declare_relation!(OpInit<'_> => ReplyInit);

impl OpInit<'_> {
    getters!(
        major: u32,
        minor: u32,
        max_readahead: u32,
        flags: u32, // FIXME: use bitflags
    );
}

impl ReplyInit {
    setters!(
        major: u32,
        minor: u32,
        max_readahead: u32,
        flags: u32,
        max_background: u16,
        congestion_threshold: u16,
        max_write: u32,
        time_gran: u32,
        max_pages: u16,
    );
}

#[derive(Debug)]
pub struct OpLookup<'b> {
    name: CBytes<'b>,
}

derive_Decode!(@c_bytes OpLookup<'b>, name);

declare_relation!(OpLookup<'_> => ReplyEntry);

pub struct FuseAttr<'b>(&'b fuse_attr);

pub struct ReplyAttr(fuse_attr_out);

#[derive(Debug)]
pub struct OpGetAttr<'b>(&'b fuse_getattr_in);

#[derive(Debug)]
pub struct OpSetAttr<'b>(&'b fuse_setattr_in);

derive_Decode!(FuseAttr<'b>);
derive_Decode!(OpGetAttr<'b>);
derive_Decode!(OpSetAttr<'b>);

derive_Encode!(ReplyAttr);

declare_relation!(OpGetAttr<'_> => ReplyAttr);
declare_relation!(OpSetAttr<'_> => ReplyAttr);

#[derive(Debug, Default)]
pub struct ReplyEntry(pub(crate) fuse_entry_out);

derive_Encode!(ReplyEntry);

#[derive(Debug)]
pub struct OpForget<'b>(&'b fuse_forget_in);

derive_Decode!(OpForget<'b>);

use std::io::IoSlice;

#[derive(Debug)]
pub struct OpReadLink<'b>(&'b ());

derive_Decode!(@empty OpReadLink<'b>);

pub struct ReplyReadLink<'a>(&'a [u8]); // bytes without NUL (?)

impl<'a> ReplyReadLink<'a> {
    pub fn new(link_name: &'a [u8]) -> Result<Self, NulError> {
        c_bytes::check_bytes(link_name)?;
        Ok(Self(link_name))
    }
}

impl Encode for ReplyReadLink<'_> {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        encode::add_bytes(container, self.0);
    }
}

declare_relation!(OpReadLink<'_> => ReplyReadLink<'a>);

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

#[derive(Debug)]
pub struct OpOpen<'b>(&'b fuse_open_in);

derive_Decode!(OpOpen<'b>);

pub struct ReplyOpen(fuse_open_out);

derive_Encode!(ReplyOpen);

declare_relation!(OpOpen<'_> => ReplyOpen);

#[derive(Debug)]
pub struct OpRead<'b>(&'b fuse_read_in);

derive_Decode!(OpRead<'b>);

pub struct ReplyData<'a>(&'a [u8]);

impl<'a> ReplyData<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }
}

impl Encode for ReplyData<'_> {
    fn collect_bytes<'c, C>(&'c self, container: &mut C)
    where
        C: Extend<IoSlice<'c>>,
    {
        encode::add_bytes(container, self.0)
    }
}

declare_relation!(OpRead<'_> => ReplyData<'_>);

#[derive(Debug)]
pub struct OpWrite<'b> {
    arg: &'b fuse_write_in,
    data: &'b [u8],
}

derive_Decode!(@data OpWrite<'b>, arg, data);

pub struct ReplyWrite(fuse_write_out);

derive_Encode!(ReplyWrite);

declare_relation!(OpWrite<'_> => ReplyWrite);

#[derive(Debug)]
pub struct OpStatFs<'b>(&'b ());

derive_Decode!(@empty OpStatFs<'b>);

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

// TODO: add more operations
