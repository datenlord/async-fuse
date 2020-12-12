//! FUSE file system server

mod buffer_pool;
mod fuse_io;
mod mount;

use self::buffer_pool::BufferPool;
use self::fuse_io::{connect, ConnReader, ConnWriter};
use self::mount::mount;

use crate::core::kernel;
use crate::core::ops;
use crate::core::{FileSystem, FuseContext, Operation, ProtocolVersion};

use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

use aligned_bytes::AlignedBytes;
use async_std::task;
use blocking::unblock;
use futures_util::pin_mut;
use tracing::{debug, error};

/// page size
/// FIXME: use libc?
const PAGE_SIZE: usize = 4096;

/// max background
/// FIXME
const MAX_BACKGROUND: u16 = 10;

/// max write size
/// FIXME
const MAX_WRITE_SIZE: u32 = 128 * 1024;

/// request buffer size
/// FIXME
const BUFFER_SIZE: usize = 128 * 1024 + 512;

/// The builder of [`Server`]
#[derive(Debug)]
pub struct ServerBuilder<F> {
    /// mount point
    mount_point: PathBuf,
    /// file system
    fs: F,
}

impl<F> ServerBuilder<F>
where
    F: FileSystem + Send + 'static,
{
    /// Starts to build a [`Server`]
    #[inline]
    pub fn new(mount_point: PathBuf, fs: F) -> Self {
        Self { mount_point, fs }
    }

    /// Initializes a FUSE connection and returns a [`Server`]
    /// # Errors
    /// Returns [`io::Error`]
    #[inline]
    pub async fn initialize(self) -> io::Result<Server<F>> {
        let ((reader, writer), _ /*mount_point*/) = {
            debug!("connecting to /dev/fuse");
            let conn = connect().await?;

            debug!("connected");

            let mount_point = self.mount_point;
            debug!(?mount_point, "mounting");

            let do_mount = move || {
                mount(conn.0.get_fd(), &mount_point)?;
                <io::Result<_>>::Ok((conn, mount_point))
            };

            unblock(do_mount).await?
        };

        debug!(fd = ?reader.get_fd(), "mounted");

        debug!("initializing");

        let (reader, buf, nread) = unblock(|| {
            let mut buf = AlignedBytes::new_zeroed(BUFFER_SIZE, PAGE_SIZE);
            let nread = reader.get_fd().read(&mut buf)?;
            <io::Result<_>>::Ok((reader, buf, nread))
        })
        .await?;

        let cx_writer = writer.clone();
        pin_mut!(cx_writer);

        let (cx, op) = FuseContext::new(
            &buf[..nread],
            cx_writer,
            ProtocolVersion {
                major: kernel::FUSE_KERNEL_VERSION,
                minor: kernel::FUSE_KERNEL_MINOR_VERSION,
            },
        )
        .unwrap_or_else(|err| {
            debug!(buf = ?buf.as_ref());
            panic!("failed to parse fuse request: {}", err);
        });

        debug!(opcode = cx.header().opcode(), "got first request");

        let proto;
        if let Operation::Init(op) = op {
            // FIXME: how to set the init config?

            proto = ProtocolVersion {
                major: op.major(),
                minor: op.minor(),
            };

            let mut reply = ops::ReplyInit::default();
            let _ = reply
                .major(kernel::FUSE_KERNEL_VERSION)
                .minor(kernel::FUSE_KERNEL_MINOR_VERSION)
                .max_readahead(op.max_readahead())
                .flags(ops::FuseInitFlags::empty())
                .max_background(MAX_BACKGROUND)
                .congestion_threshold(10)
                .max_write(MAX_WRITE_SIZE)
                .time_gran(1)
                .max_pages(0);

            debug!(?op, ?reply);
            cx.reply(&op, reply).await?;
        } else {
            panic!("failed to initialize memfs: first request is not FUSE_INIT");
        }

        let buffer_pool = BufferPool::new(MAX_BACKGROUND.into(), BUFFER_SIZE, PAGE_SIZE);

        debug!("initialized");

        let server = Server {
            reader,
            writer,
            // mount_point,
            buffer_pool: Arc::new(buffer_pool),
            fs: Arc::new(self.fs),
            proto,
        };
        Ok(server)
    }
}

/// FUSE file system server
#[derive(Debug)]
pub struct Server<F> {
    /// conn writer
    writer: ConnWriter,
    /// conn reader
    reader: ConnReader,
    /// request buffer pool (lockfree)
    buffer_pool: Arc<BufferPool>,
    /// Arc file system
    fs: Arc<F>,
    // mount_point: PathBuf,
    /// kernel prototol version
    proto: ProtocolVersion,
}

impl<F> Server<F>
where
    F: FileSystem + Send + 'static,
{
    /// Starts to build a [`Server`] with the given mount point
    #[inline]
    pub fn mount(mount_point: PathBuf, fs: F) -> ServerBuilder<F> {
        ServerBuilder::new(mount_point, fs)
    }

    /// Runs the file system until it is un-mounted
    /// # Errors
    /// Returns [`io::Error`]
    #[inline]
    pub async fn run(mut self) -> io::Result<()> {
        loop {
            let result = async {
                debug!("waiting for fuse request");

                let buf = self.buffer_pool.acquire();
                let pool = Arc::clone(&self.buffer_pool);
                let (mut buf, nread) = self.reader.read(buf).await?;
                buf.set_len(nread);

                let cx_writer = self.writer.clone();
                let fs = Arc::clone(&self.fs);
                let proto = self.proto;

                debug!("spawn task");

                let _ = task::spawn(async move {
                    pin_mut!(cx_writer);
                    let (cx, op) = match FuseContext::new(buf.as_ref(), cx_writer, proto) {
                        Ok(r) => r,
                        Err(e) => {
                            debug!(buf = ?buf.as_ref());
                            panic!("failed to parse fuse request: {}", e);
                        }
                    };
                    debug!(
                        opcode = cx.header().opcode(),
                        unique = cx.header().unique(),
                        "got request"
                    );

                    let ret = fs.dispatch(cx, op).await;
                    if let Err(err) = ret {
                        // FIXME: how to handle the error
                        error!(%err);
                    }

                    pool.release(buf);
                }); // task is detached here

                <io::Result<_>>::Ok(())
            }
            .await;

            if let Err(err) = result {
                let errno = err
                    .raw_os_error()
                    .unwrap_or_else(|| panic!("failed to read fuse connection: {}", err));

                match errno {
                    libc::ENODEV => {
                        break;
                    }
                    libc::EAGAIN => {
                        continue;
                    }
                    _ => panic!("unrecoverable os error: {}", err),
                }
            }
        }

        debug!("shutdown");

        Ok(())
    }
}
