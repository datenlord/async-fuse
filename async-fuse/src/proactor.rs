#[cfg(not(feature = "use-uring"))]
pub use self::unblock_proactor::{global_proactor, Proactor};
#[cfg(feature = "use-uring")]
pub use self::uring_proactor::{global_proactor, Proactor};

mod unblock_proactor {
    use std::io::{self, IoSlice};
    use std::os::unix::io::AsRawFd;

    use blocking::unblock;
    use nix::fcntl::SpliceFFlags;
    use once_cell::sync::Lazy;

    pub struct Proactor {
        _priv: (),
    }

    pub fn global_proactor() -> &'static Proactor {
        static GLOBAL_PROACTOR: Lazy<Proactor> = Lazy::new(|| Proactor { _priv: () });
        &*GLOBAL_PROACTOR
    }

    impl Proactor {
        pub async fn read<H, B>(&self, handle: H, mut buf: B) -> (H, B, io::Result<usize>)
        where
            H: AsRawFd + Send + 'static,
            B: AsMut<[u8]> + Send + 'static,
        {
            unblock(move || {
                let fd = handle.as_raw_fd();
                let ret = crate::fd::read(fd, buf.as_mut());
                (handle, buf, ret)
            })
            .await
        }

        pub async fn write<H, B>(&self, handle: H, buf: B) -> (H, B, io::Result<usize>)
        where
            H: AsRawFd + Send + 'static,
            B: AsRef<[u8]> + Send + 'static,
        {
            unblock(move || {
                let fd = handle.as_raw_fd();
                let ret = crate::fd::write(fd, buf.as_ref());
                (handle, buf, ret)
            })
            .await
        }

        #[allow(single_use_lifetimes)]
        pub async fn write_vectored<H, S>(&self, handle: H, bufs: S) -> (H, S, io::Result<usize>)
        where
            H: AsRawFd + Send + 'static,
            S: for<'a> AsRef<[IoSlice<'a>]> + Send + 'static,
        {
            unblock(move || {
                let fd = handle.as_raw_fd();
                let ret = crate::fd::write_vectored(fd, bufs.as_ref());
                (handle, bufs, ret)
            })
            .await
        }

        pub async fn splice<H1, H2>(
            handle_in: H1,
            off_in: Option<usize>,
            handle_out: H2,
            off_out: Option<usize>,
            len: usize,
            flags: SpliceFFlags,
        ) -> (H1, H2, io::Result<usize>)
        where
            H1: AsRawFd + Send + 'static,
            H2: AsRawFd + Send + 'static,
        {
            unblock(move || {
                let fd_in = handle_in.as_raw_fd();
                let fd_out = handle_out.as_raw_fd();
                let ret = crate::fd::splice(fd_in, off_in, fd_out, off_out, len, flags);
                (handle_in, handle_out, ret)
            })
            .await
        }
    }
}

mod uring_proactor {
    use once_cell::sync::Lazy;

    pub struct Proactor {
        _ring: (),
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn global_proactor() -> &'static Proactor {
        static GLOBAL_PROACTOR: Lazy<Proactor> = Lazy::new(|| Proactor { _ring: () });
        &*GLOBAL_PROACTOR
    }

    // TODO: impl uring proactor
}
