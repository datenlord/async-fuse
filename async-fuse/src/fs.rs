use crate::context::FuseContext;
use crate::ops::Operation;

use std::io;

#[async_trait::async_trait]
pub trait FileSystem: Sync {
    async fn dispatch<'b, 'a: 'b>(
        &'a self,
        cx: FuseContext<'b>,
        op: Operation<'b>,
    ) -> io::Result<()>;
}
