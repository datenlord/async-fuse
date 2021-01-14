//! FUSE async implementation

#![deny(
    // The following are allowed by default lints according to
    // https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html
    anonymous_parameters,
    bare_trait_objects,
    // box_pointers,
    elided_lifetimes_in_paths,
    missing_copy_implementations,
    missing_debug_implementations,
    // missing_docs,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    // unreachable_pub, allow clippy::redundant_pub_crate lint instead
    // unsafe_code,
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    variant_size_differences,

    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    // clippy::cargo
)]
#![allow(
    // Some explicitly allowed Clippy lints, must have clear reason to allow
    clippy::blanket_clippy_restriction_lints, // allow clippy::restriction
    clippy::implicit_return, // actually omitting the return keyword is idiomatic Rust code
    clippy::module_name_repetitions, // repeation of module name in a struct name is not big deal
    clippy::panic, // allow debug_assert, panic in production code
    clippy::panic_in_result_fn, // allow debug_assert, panic in production code
    clippy::indexing_slicing,
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::redundant_pub_crate,
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod abi;
mod conn;
mod context;
mod fd;
mod kernel;
mod payload;
mod proactor;
