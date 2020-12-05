use better_as::number::{ExtendingCast, TruncatingCast, WrappingCast};

use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl AccessMode {
    #[must_use]
    #[inline]
    pub fn as_raw(self) -> u32 {
        match self {
            Self::ReadOnly => libc::O_RDONLY,
            Self::WriteOnly => libc::O_WRONLY,
            Self::ReadWrite => libc::O_RDWR,
        }
        .wrapping_cast()
    }

    #[must_use]
    #[inline]
    pub fn from_raw(o_flags: u32) -> Self {
        match o_flags.wrapping_cast() & libc::O_ACCMODE {
            libc::O_RDONLY => Self::ReadOnly,
            libc::O_WRONLY => Self::WriteOnly,
            libc::O_RDWR => Self::ReadWrite,
            _ => panic!("invalid o_flags: {:o}", o_flags),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Unknown,
    NamedPipe,
    CharacterDevice,
    Directory,
    BlockDevice,
    Regular,
    SymbolicLink,
    Socket,
}

impl FileType {
    #[inline]
    #[must_use]
    pub fn from_raw(d_type: u8) -> Self {
        match d_type {
            libc::DT_UNKNOWN => Self::Unknown,
            libc::DT_FIFO => Self::NamedPipe,
            libc::DT_CHR => Self::CharacterDevice,
            libc::DT_DIR => Self::Directory,
            libc::DT_BLK => Self::BlockDevice,
            libc::DT_REG => Self::Regular,
            libc::DT_LNK => Self::SymbolicLink,
            libc::DT_SOCK => Self::Socket,
            _ => panic!("unexpected d_type: {}", d_type),
        }
    }

    #[must_use]
    #[inline]
    pub const fn as_raw(self) -> u8 {
        match self {
            Self::Unknown => libc::DT_UNKNOWN,
            Self::NamedPipe => libc::DT_FIFO,
            Self::CharacterDevice => libc::DT_CHR,
            Self::Directory => libc::DT_DIR,
            Self::BlockDevice => libc::DT_BLK,
            Self::Regular => libc::DT_REG,
            Self::SymbolicLink => libc::DT_LNK,
            Self::Socket => libc::DT_SOCK,
        }
    }

    #[must_use]
    #[inline]
    pub fn as_u32(self) -> u32 {
        u32::from(self.as_raw())
    }
}

bitflags! {
    pub struct FileMode: u32 {
        const RWXO = 0o0007;
        const ROTH = 0o0004;
        const WOTH = 0o0002;
        const XOTH = 0o0001;

        const RWXG = 0o0070;
        const RGRP = 0o0040;
        const WGRP = 0o0020;
        const XGRP = 0o0010;

        const RWXU = 0o0700;
        const RUSR = 0o0400;
        const WUSR = 0o0200;
        const XUSR = 0o0100;

        const SUID = 0o4000;
        const SGID = 0o2000;
        const SVTX = 0o1000;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StMode(u32);

impl StMode {
    #[must_use]
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    #[must_use]
    #[inline]
    pub fn new(ty: FileType, mode: FileMode) -> Self {
        let ty_u32: u32 = ty.as_raw().extending_cast();
        Self(ty_u32.wrapping_shl(12) | mode.bits())
    }

    #[must_use]
    #[inline]
    pub fn file_type(self) -> FileType {
        FileType::from_raw(self.0.wrapping_shr(12).truncating_cast())
    }

    #[must_use]
    #[inline]
    pub const fn file_mode(self) -> FileMode {
        FileMode::from_bits_truncate(self.0)
    }
}
