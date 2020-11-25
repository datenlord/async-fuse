mod buffer_pool;
mod c_str;
mod conn;
mod mount;
mod server;

mod fs {
    use std::io;

    use async_fuse::{Errno, FileSystem, FuseContext, Operation};
    use tracing::debug;

    #[derive(Debug)]
    pub struct MemFs;

    #[async_trait::async_trait]
    impl FileSystem for MemFs {
        #[allow(clippy::unit_arg)]
        #[tracing::instrument(err)]
        async fn dispatch<'b, 'a: 'b>(
            &'a self,
            cx: FuseContext<'b>,
            op: Operation<'b>,
        ) -> io::Result<()> {
            let _ = op;
            debug!(errno = ?Errno::ENOSYS);
            cx.reply_err(Errno::ENOSYS).await?;
            Ok(())
        }
    }
}

pub use self::fs::MemFs;
pub use self::server::{Server, ServerBuilder};
