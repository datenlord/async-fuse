#![deny(
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    variant_size_differences,

    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    // clippy::cargo
)]
#![allow(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc
)]

#[macro_use]
mod internel_macros;

// unsafe modules
mod abi_marker;
mod c_bytes;
mod de;
mod encode;
mod fd;

// safe modules
mod ctx;
mod errno;
mod fs;
mod io;

pub mod kernel;
pub mod ops;

pub use self::ctx::FuseContext;
pub use self::errno::Errno;
pub use self::fd::FuseDesc;
pub use self::fs::FileSystem;
pub use self::io::FuseWrite;
pub use self::ops::Operation;
