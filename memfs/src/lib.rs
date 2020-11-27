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
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    // clippy::cargo
)]
#![allow(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::implicit_return,
    clippy::blanket_clippy_restriction_lints,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::panic_in_result_fn,
    clippy::wildcard_imports,
    clippy::module_name_repetitions
)]

mod memfs;

pub use self::memfs::MemFs;
