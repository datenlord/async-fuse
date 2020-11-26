use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait FuseWrite {
    fn poll_reply(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<()>>;
}
