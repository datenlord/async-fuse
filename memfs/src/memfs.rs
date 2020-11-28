use async_fuse::utils::ForceConvert;

use std::io;
use std::time::{Duration, SystemTime};

use async_fuse::ops::*;
use async_fuse::types::file::{AccessMode, FileMode, FileType, StMode};
use async_fuse::{Errno, FileSystem, FuseContext, Operation};

use once_cell::sync::Lazy;
use tracing::debug;

#[derive(Debug)]
pub struct MemFs; // hello_ll

const HELLO_STR: &str = "Hello World!\n";
const HELLO_NAME: &str = "hello";

async fn stat(ino: u64) -> Option<Attr> {
    let mut attr = Attr::default();

    let init_time: SystemTime = {
        static INIT_TIME: Lazy<SystemTime> = Lazy::new(SystemTime::now);
        *INIT_TIME
    };

    let _ = attr
        .ino(ino)
        .blocks(8)
        .blksize(4096)
        .uid(0)
        .gid(0)
        .atime(init_time)
        .ctime(init_time)
        .mtime(init_time);

    let root_mode = StMode::new(
        FileType::Directory,
        FileMode::RWXU | FileMode::RGRP | FileMode::ROTH,
    );
    let file_mode = StMode::new(
        FileType::Regular,
        FileMode::RUSR | FileMode::RGRP | FileMode::ROTH,
    );

    let file_size: u64 = HELLO_STR.len().force_convert();

    let _ = match ino {
        1 => attr.mode(root_mode).nlink(2).size(4096),
        2 => attr.mode(file_mode).nlink(1).size(file_size),
        _ => return None,
    };

    Some(attr)
}

async fn lookup(parent: u64, name: &[u8]) -> Option<Entry> {
    if parent != 1 {
        return None;
    }

    debug!(?name);
    if name != HELLO_NAME.as_bytes() {
        return None;
    }
    let mut entry = Entry::default();
    let _ = entry
        .attr_valid(Duration::from_secs(1))
        .entry_valid(Duration::from_secs(1))
        .attr(stat(2).await?)
        .nodeid(2);
    debug!(?entry);

    return Some(entry);
}

#[tracing::instrument]
async fn do_getattr(cx: FuseContext<'_>, op: OpGetAttr<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();

    match stat(ino).await {
        Some(attr) => {
            let mut reply = ReplyAttr::default();
            let _ = reply.attr(attr);
            cx.reply(&op, reply).await
        }
        None => cx.reply_err(Errno::ENOENT).await,
    }
}

#[tracing::instrument]
async fn do_lookup(cx: FuseContext<'_>, op: OpLookup<'_>) -> io::Result<()> {
    let parent = cx.header().nodeid();
    let name = op.name();

    match lookup(parent, name).await {
        Some(entry) => cx.reply(&op, ReplyEntry::new(entry)).await,
        None => cx.reply_err(Errno::ENOENT).await,
    }
}

#[tracing::instrument]
async fn do_readdir(cx: FuseContext<'_>, op: OpReadDir<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    if ino != 1 {
        return cx.reply_err(Errno::ENOTDIR).await;
    }

    let dir: &Directory<'_> = {
        static DIR: Lazy<Directory<'static>> = Lazy::new(|| {
            let mut dir = Directory::with_capacity(256);
            let _ = dir
                .add_entry(1, FileType::Directory, b".")
                .add_entry(1, FileType::Directory, b"..")
                .add_entry(2, FileType::Regular, HELLO_NAME.as_bytes());
            dir
        });

        &*DIR
    };

    let offset: usize = op.offset().force_convert();
    let size: usize = op.size().force_convert();

    let reply = ReplyDirectory::new(dir.by_ref(), offset, size);
    cx.reply(&op, reply).await
}

#[tracing::instrument]
async fn do_open(cx: FuseContext<'_>, op: OpOpen<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    if ino != 2 {
        return cx.reply_err(Errno::EISDIR).await;
    }

    debug!(open_flags = ?op.flags());

    let access_mode = AccessMode::from_raw(op.flags());
    if access_mode != AccessMode::ReadOnly {
        return cx.reply_err(Errno::EACCES).await;
    }

    let reply = ReplyOpen::default();
    cx.reply(&op, reply).await
}

#[tracing::instrument]
async fn do_opendir(cx: FuseContext<'_>, op: OpOpenDir<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    if ino != 1 {
        return cx.reply_err(Errno::ENOTDIR).await;
    }

    let access_mode = AccessMode::from_raw(op.flags());
    if access_mode != AccessMode::ReadOnly {
        return cx.reply_err(Errno::EACCES).await;
    }

    let reply = ReplyOpenDir::default();
    cx.reply(&op, reply).await
}

#[tracing::instrument]
async fn do_read(cx: FuseContext<'_>, op: OpRead<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    if ino != 2 {
        return cx.reply_err(Errno::EISDIR).await;
    }

    let offset: usize = op.offset().force_convert();
    let size: usize = op.size().force_convert();

    let reply = ReplyData::new(HELLO_STR.as_bytes(), offset, size);
    cx.reply(&op, reply).await
}

async fn do_releasedir(cx: FuseContext<'_>, op: OpReleaseDir<'_>) -> io::Result<()> {
    cx.reply(&op, ReplyEmpty::default()).await
}

async fn do_flush(cx: FuseContext<'_>, op: OpFlush<'_>) -> io::Result<()> {
    cx.reply(&op, ReplyEmpty::default()).await
}

async fn do_interrupt(cx: FuseContext<'_>, op: OpInterrupt<'_>) -> io::Result<()> {
    cx.reply(&op, ReplyEmpty::default()).await
}

#[async_trait::async_trait]
impl FileSystem for MemFs {
    #[inline]
    async fn dispatch<'b, 'a: 'b>(
        &'a self,
        cx: FuseContext<'b>,
        op: Operation<'b>,
    ) -> io::Result<()> {
        debug!(?op);

        #[allow(clippy::wildcard_enum_match_arm)]
        match op {
            Operation::Lookup(op) => do_lookup(cx, op).await,
            Operation::GetAttr(op) => do_getattr(cx, op).await,
            Operation::OpenDir(op) => do_opendir(cx, op).await,
            Operation::ReadDir(op) => do_readdir(cx, op).await,
            Operation::Open(op) => do_open(cx, op).await,
            Operation::Read(op) => do_read(cx, op).await,
            Operation::ReleaseDir(op) => do_releasedir(cx, op).await,
            Operation::Flush(op) => do_flush(cx, op).await,
            Operation::Interrupt(op) => do_interrupt(cx, op).await,
            _ => cx.reply_err(Errno::ENOSYS).await,
        }
    }
}
