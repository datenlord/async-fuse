//! File system node

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use async_fuse::core::ops::Attr;
use async_fuse::types::file::FileType;

/// File node
pub struct Node {
    /// Parent node i-number
    pub parent: u64,

    /// i-number
    pub ino: u64,

    /// name (without NUL)
    pub name: OsString,

    /// attributes
    pub attr: Attr,
}

/// Directory entry
pub struct Entry {
    /// i-number
    ino: u64,
    /// file type
    file_type: FileType,
    /// name
    name: OsString,
}

/// Node data
pub enum NodeData {
    /// Directory
    Directory(BTreeMap<OsString, Entry>),
    /// Regular file
    Regular(Vec<u8>),
    /// Symbolic link
    SymLink(PathBuf),
}
