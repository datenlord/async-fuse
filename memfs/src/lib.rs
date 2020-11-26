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

mod buffer_pool;
mod c_str;
mod io;
mod memfs;
mod mount;
mod server;

pub use self::memfs::MemFs;
pub use self::server::{Server, ServerBuilder};
