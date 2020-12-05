//! [`FuseWrite`]

use std::io::{self, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Writes the bytes of a reply to FUSE connection
pub trait FuseWrite {
    /// Writes the bytes of a reply to FUSE connection
    fn poll_reply(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<()>>;
}
