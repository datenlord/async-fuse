//! FUSE file system server

use crate::conn::{connect, ConnReader, ConnWriter};
use crate::context::{FuseContext, FuseInHeader, ProtocolVersion};
use crate::fs::FileSystem;
use crate::mount::mount;
use crate::payload::PayloadPool;
use crate::utils::FreezedBuf;
use crate::{abi, kernel, ops, utils};

use std::io::{self, Read};
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use aligned_utils::bytes::AlignedBytes;
use aligned_utils::stack::Align8;
use async_std::task;
use better_as::number::TruncatingCast;
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

    async fn mount(
        conn: ConnReader,
        mount_point: PathBuf,
    ) -> (ConnReader, PathBuf, io::Result<()>) {
        unblock(move || {
            let ret = mount(conn.get_fd(), &mount_point);
            (conn, mount_point, ret)
        })
        .await
    }

    /// Initializes a FUSE connection and returns a [`Server`]
    /// # Errors
    /// Returns [`io::Error`]
    #[inline]
    pub async fn initialize(self) -> io::Result<Server<F>> {
        debug!("connecting to /dev/fuse");
        let (reader, mut writer) = connect().await?;
        debug!("connected");

        let mut reader = {
            let mount_point = self.mount_point;
            debug!(?mount_point, "mounting");

            let (reader, _, ret) = Self::mount(reader, mount_point).await;
            ret?;

            debug!(fd = ?reader.get_fd(), "mounted");
            reader
        };

        debug!("initializing");

        let request_buf = {
            let buf = AlignedBytes::new_zeroed(512, 8);
            let (buf, ret) = reader.read(buf).await;
            let nread = ret?;
            FreezedBuf::new(buf, nread)
        };

        let (in_header, init_in) = {
            let mut bytes = request_buf.as_ref();
            let header_ref: &kernel::fuse_in_header = abi::fetch_ref(&mut bytes)
                .unwrap_or_else(|err| panic!("failed to fetch header: {}", err));

            assert_eq!(
                header_ref.opcode,
                kernel::fuse_opcode::FUSE_INIT,
                "first request is not FUSE_INIT: opcode = {}",
                header_ref.opcode
            );

            let init_in_ref: &kernel::fuse_init_in = abi::fetch_ref(&mut bytes)
                .unwrap_or_else(|err| panic!("failed to fetch fuse_init_in: {}", err));

            (header_ref, init_in_ref)
        };

        let proto = ProtocolVersion {
            major: init_in.major,
            minor: init_in.minor,
        };

        {
            type ReplyInit = abi::Tuple2<kernel::fuse_out_header, kernel::fuse_init_out>;
            let mut reply_buf = AlignedBytes::new_zeroed(512, 8);

            let nvalid = abi::with_mut(&mut *reply_buf, |out: &mut ReplyInit| {
                let out_header = &mut out.first;
                let init_out = &mut out.second;

                #[allow(clippy::as_conversions, clippy::clippy::cast_possible_truncation)]
                {
                    out_header.len = mem::size_of::<ReplyInit>() as u32;
                    out_header.error = 0;
                    out_header.unique = in_header.unique;
                }

                {
                    init_out.major = kernel::FUSE_KERNEL_VERSION;
                    init_out.minor = kernel::FUSE_KERNEL_MINOR_VERSION;
                    init_out.max_readahead = init_in.max_readahead;
                    init_out.flags = ops::FuseInitFlags::empty().bits();
                    init_out.max_background = MAX_BACKGROUND;
                    init_out.congestion_threshold = 10;
                    init_out.max_write = MAX_WRITE_SIZE;
                    init_out.time_gran = 1;
                    init_out.max_pages = 0;
                }
            })
            .unwrap_or_else(|err| panic!("failed to build reply: {}", err));

            let (_, ret) = writer.write(FreezedBuf::new(reply_buf, nvalid)).await;
            let nwrite = ret?;
            assert_eq!(nwrite, nvalid);
        }

        let payload_pool = PayloadPool::new(MAX_BACKGROUND.into(), BUFFER_SIZE, PAGE_SIZE);

        debug!("initialized");

        let server = Server {
            reader,
            writer,
            payload_pool: Arc::new(payload_pool),
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
    payload_pool: Arc<PayloadPool>,
    /// Arc file system
    fs: Arc<F>,
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
        let mut handles = Vec::new();
        for _ in 0..16 {
            let reader = self.reader.clone();
            let writer = self.writer.clone();
            let pool = Arc::clone(&self.payload_pool);
            let fs = Arc::clone(&self.fs);
            let proto = self.proto;
            let handle = task::spawn(Self::main_loop(reader, writer, pool, fs, proto));
            handles.push(handle)
        }
        let results = futures_util::future::join_all(handles).await;
        for result in results {
            result?;
        }
        Ok(())
    }

    async fn main_loop(
        mut reader: ConnReader,
        writer: ConnWriter,
        pool: Arc<PayloadPool>,
        fs: Arc<F>,
        proto: ProtocolVersion,
    ) -> io::Result<()> {
        loop {
            let result = async {
                debug!("waiting for fuse request");

                let mut payload = pool.create()?;

                #[allow(clippy::unwrap_used)]
                {
                    let pipe_tx = payload.pipe_tx.take().unwrap();
                    let (pipe_tx, ret) = reader.splice_to(pipe_tx, BUFFER_SIZE).await;
                    payload.pipe_tx = Some(pipe_tx);
                    payload.pipe_data_len = ret?;
                }

                let in_header = {
                    let header_len = mem::size_of::<kernel::fuse_in_header>();
                    let mut bytes = payload.load_data(header_len).await?;
                    assert_eq!(bytes.len(), header_len);
                    let header_ref = abi::fetch_ref::<kernel::fuse_in_header>(&mut bytes)
                        .unwrap_or_else(|err| panic!("failed to fetch header: {}", err));

                    FuseInHeader {
                        len: header_ref.len,
                        opcode: header_ref.opcode,
                        unique: header_ref.unique,
                        nodeid: header_ref.nodeid,
                        uid: header_ref.uid,
                        gid: header_ref.gid,
                        pid: header_ref.pid,
                    }
                };

                let cx = FuseContext {
                    header: in_header,
                    payload,
                    proto,
                    writer: writer.clone(),
                };

                fs.dispatch(cx).await?;

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
