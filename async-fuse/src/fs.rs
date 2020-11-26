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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{ReplyData, ReplyReadLink};

    #[test]
    fn lifetime() {
        struct MockFs;

        #[async_trait::async_trait]
        impl FileSystem for MockFs {
            async fn dispatch<'b, 'a: 'b>(
                &'a self,
                cx: FuseContext<'b>,
                op: Operation<'b>,
            ) -> io::Result<()> {
                match op {
                    Operation::ReadLink(readlink) => {
                        let bytes: &'b [u8] = b"asd";
                        let reply = ReplyReadLink::new(bytes).unwrap();
                        cx.reply(&readlink, reply).await
                    }
                    Operation::Read(read) => {
                        let bytes: &'b [u8] = b"asd";
                        let reply = ReplyData::new(bytes, 0, 3);
                        cx.reply(&read, reply).await
                    }
                    _ => panic!("untested"),
                }
            }
        }
    }
}
