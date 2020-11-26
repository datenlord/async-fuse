use crate::buffer_pool::BufferPool;
use crate::conn::{connect, ConnReader, ConnWriter};
use crate::mount::mount;

use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

use async_fuse::kernel;
use async_fuse::ops;
use async_fuse::FileSystem;
use async_fuse::FuseContext;
use async_fuse::Operation;

use aligned_bytes::AlignedBytes;
use async_std::task;
use blocking::unblock;
use futures::pin_mut;
use tracing::{debug, error};

const PAGE_SIZE: usize = 4096;
const MAX_WRITE_SIZE: usize = 128 * 1024;
const MAX_BACKGROUND: usize = 10;
const BUFFER_SIZE: usize = MAX_WRITE_SIZE + 512;

pub struct ServerBuilder<F> {
    mount_point: PathBuf,
    fs: F,
}

impl<F> ServerBuilder<F>
where
    F: FileSystem + Send + 'static,
{
    pub fn new(mount_point: PathBuf, fs: F) -> Self {
        Self { mount_point, fs }
    }

    pub async fn initialize(self) -> io::Result<Server<F>> {
        let ((reader, writer), mount_point) = {
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

        let (header, op) = FuseContext::parse(&buf[..nread]).expect("failed to parse fuse request");

        let cx_writer = writer.clone();
        pin_mut!(cx_writer);

        let cx = FuseContext::new(cx_writer, header);

        debug!(opcode = cx.header().opcode(), "got first request");

        if let Operation::Init(op) = op {
            // FIXME: how to set the init config?

            let mut rep = ops::ReplyInit::default();
            rep.major(kernel::FUSE_KERNEL_VERSION)
                .minor(kernel::FUSE_KERNEL_MINOR_VERSION)
                .max_readahead(op.max_readahead())
                .flags(0)
                .max_background(MAX_BACKGROUND as u16)
                .congestion_threshold(10)
                .max_write(MAX_WRITE_SIZE as u32)
                .time_gran(1)
                .max_pages(0);

            debug!(?op, ?rep);
            cx.reply(&op, rep).await?;
        } else {
            panic!("failed to initialize memfs: first request is not FUSE_INIT");
        }

        let buffer_pool = BufferPool::new(MAX_BACKGROUND, BUFFER_SIZE, PAGE_SIZE);

        debug!("initialized");

        let server = Server {
            reader,
            writer,
            mount_point,
            buffer_pool: Arc::new(buffer_pool),
            fs: Arc::new(self.fs),
        };
        Ok(server)
    }
}

pub struct Server<F> {
    writer: ConnWriter,
    reader: ConnReader,
    buffer_pool: Arc<BufferPool>,
    fs: Arc<F>,
    mount_point: PathBuf,
}

impl<F> Server<F>
where
    F: FileSystem + Send + 'static,
{
    pub fn mount(mount_point: PathBuf, fs: F) -> ServerBuilder<F> {
        ServerBuilder::new(mount_point, fs)
    }

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

                debug!("spawn task");
                task::spawn(async move {
                    let (header, op) =
                        FuseContext::parse(buf.as_ref()).expect("failed to parse fuse request");

                    debug!(
                        opcode = header.opcode(),
                        unique = header.unique(),
                        "got request"
                    );

                    pin_mut!(cx_writer);

                    let cx = FuseContext::new(cx_writer, header);

                    let ret = fs.dispatch(cx, op).await;
                    if let Err(err) = ret {
                        // FIXME: how to handle the error
                        error!(%err);
                    }

                    pool.release(buf);
                });
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

        Ok(())
    }
}
