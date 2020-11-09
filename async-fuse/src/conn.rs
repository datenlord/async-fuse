use futures_io::{AsyncRead, AsyncWrite};

pub trait FuseConn: AsyncRead + AsyncWrite + Unpin {}
