//! FUSE filesystem

use super::context::FuseContext;
use super::ops::Operation;

use std::io;

/// FUSE filesystem
#[async_trait::async_trait]
pub trait FileSystem: Sync {
    /// dispatch operations
    async fn dispatch<'b, 'a: 'b>(
        &'a self,
        cx: FuseContext<'b>,
        op: Operation<'b>,
    ) -> io::Result<()>;
}
