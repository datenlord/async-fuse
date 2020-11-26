#![deny(clippy::all)]

#[macro_use]
mod internel_macros;

// unsafe modules
mod abi_marker;
mod c_bytes;
mod decode;
mod encode;
mod fd;

// safe modules
mod context;
mod errno;
mod fs;
mod write;

pub mod kernel;
pub mod ops;

pub use self::context::FuseContext;
pub use self::errno::Errno;
pub use self::fd::FuseDesc;
pub use self::fs::FileSystem;
pub use self::ops::Operation;
pub use self::write::FuseWrite;
