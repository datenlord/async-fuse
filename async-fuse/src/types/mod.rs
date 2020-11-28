pub mod file;

#[allow(clippy::as_conversions)]
pub const PATH_MAX: usize = libc::PATH_MAX as usize;

#[allow(clippy::assertions_on_constants)]
#[test]
fn path_max() {
    use std::convert::TryFrom;
    assert!(usize::try_from(libc::PATH_MAX).is_ok());
    assert!(libc::PATH_MAX >= 1024 && libc::PATH_MAX <= 8192);
}
