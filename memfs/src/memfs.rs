use std::io;
use std::time::{Duration, SystemTime};

#[allow(clippy::wildcard_imports)]
use async_fuse::ops::*;
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

    attr.ino(ino)
        .blocks(8)
        .blksize(4096)
        .uid(0)
        .gid(0)
        .atime(init_time)
        .ctime(init_time)
        .mtime(init_time);

    match ino {
        1 => attr.mode(libc::S_IFDIR | 0o755).nlink(2).size(4096),

        2 => attr
            .mode(libc::S_IFREG | 0o444)
            .nlink(1)
            .size(HELLO_STR.len() as u64),

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
    entry.attr_valid(Duration::from_secs(1));
    entry.entry_valid(Duration::from_secs(1));
    entry.attr(stat(2).await.unwrap());
    entry.nodeid(2);
    debug!(?entry);

    return Some(entry);
}

#[tracing::instrument]
async fn do_getattr(cx: FuseContext<'_>, op: OpGetAttr<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();

    match stat(ino).await {
        Some(attr) => {
            let mut reply = ReplyAttr::default();
            reply.attr(attr);
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

    let dir: &Directory = {
        static DIR: Lazy<Directory> = Lazy::new(|| {
            let mut dir = Directory::with_capacity(256);
            dir.add_entry(1, u32::from(libc::DT_DIR), b".").unwrap();
            dir.add_entry(1, u32::from(libc::DT_DIR), b"..").unwrap();
            dir.add_entry(2, u32::from(libc::DT_REG), HELLO_NAME.as_bytes())
                .unwrap();
            dir
        });

        &*DIR
    };

    #[allow(clippy::cast_possible_truncation)]
    let offset = op.offset() as usize;

    let reply = ReplyDirectory::new(dir.by_ref(), offset, op.size() as usize);
    cx.reply(&op, reply).await
}

#[tracing::instrument]
async fn do_open(cx: FuseContext<'_>, op: OpOpen<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    if ino != 2 {
        return cx.reply_err(Errno::EISDIR).await;
    }

    debug!(open_flags = ?op.flags());

    #[allow(clippy::cast_possible_wrap)]
    if (op.flags() as i32) & libc::O_ACCMODE != libc::O_RDONLY {
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

    #[allow(clippy::cast_possible_wrap)]
    if (op.flags() as i32) & libc::O_ACCMODE != libc::O_RDONLY {
        return cx.reply_err(Errno::EACCES).await;
    }

    let reply = ReplyOpenDir::default();
    cx.reply(&op, reply).await
}

#[tracing::instrument]
async fn do_read(cx: FuseContext<'_>, op: OpRead<'_>) -> io::Result<()> {
    let ino = cx.header().nodeid();
    assert_eq!(ino, 2);

    #[allow(clippy::cast_possible_truncation)]
    let offset = op.offset() as usize;

    let reply = ReplyData::new(HELLO_STR.as_bytes(), offset, op.size() as usize);
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
    async fn dispatch<'b, 'a: 'b>(
        &'a self,
        cx: FuseContext<'b>,
        op: Operation<'b>,
    ) -> io::Result<()> {
        debug!(?op);

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
