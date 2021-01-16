//! FUSE filesystem

use crate::context::FuseContext;

use std::future::Future;
use std::io;

/// FUSE filesystem
pub trait FileSystem: Sync {
    type Future: Future<Output = io::Result<()>> + Send + 'static;
    fn dispatch(&self, cx: FuseContext) -> Self::Future;
}
