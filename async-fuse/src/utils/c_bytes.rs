//! Nul-terminated bytes

use crate::utils::DisplayBytes;

use std::fmt::{self, Debug};

use memchr::memchr;

/// Nul-terminated bytes
pub struct CBytes<'b>(&'b [u8]);

impl Debug for CBytes<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", DisplayBytes(self.0))
    }
}

impl<'b> CBytes<'b> {
    /// # Safety
    /// The bytes must not contain any interior nul byte
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(bytes: &'b [u8]) -> Self {
        Self(bytes)
    }

    /// Returns bytes without NUL. Time: O(1).
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &'b [u8] {
        debug_assert!(!self.0.is_empty());
        debug_assert!(self.0[self.0.len().wrapping_sub(1)] == 0);
        unsafe { self.0.get_unchecked(..self.0.len().wrapping_sub(1)) }
    }
}

/// Checks whether there is any NUL in the bytes
#[inline]
pub fn check_bytes(bytes: &[u8]) -> Result<(), NulError> {
    match memchr(0, bytes) {
        None => Ok(()),
        Some(pos) => Err(NulError { pos }),
    }
}

/// The error returned by [`check_bytes`]
#[derive(Debug, thiserror::Error)]
#[error("NulError: nul position = {}",.pos)]
pub struct NulError {
    /// nul position
    pos: usize,
}

impl NulError {
    /// Returns the position of NUL
    #[inline]
    #[must_use]
    pub const fn nul_position(&self) -> usize {
        self.pos
    }
}
