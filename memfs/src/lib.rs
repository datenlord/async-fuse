mod buffer_pool;
mod c_str;
mod conn;
mod mount;
mod server;

mod fs {
    use std::io;

    use async_fuse::{Errno, FileSystem, FuseContext, Operation};

    pub struct MemFs;

    #[async_trait::async_trait]
    impl FileSystem for MemFs {
        async fn dispatch<'b, 'a: 'b>(
            &'a self,
            cx: FuseContext<'b>,
            op: Operation<'b>,
        ) -> io::Result<()> {
            let _ = op;
            cx.reply_err(Errno::ENOSYS).await?;
            Ok(())
        }
    }
}

pub use self::fs::MemFs;
pub use self::server::{Server, ServerBuilder};
