use std::fmt::{self, Debug};

use crate::utils::DisplayBytes;

use memchr::memchr;

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

    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &'b [u8] {
        debug_assert!(!self.0.is_empty());
        debug_assert!(self.0[self.0.len().wrapping_sub(1)] == 0);
        unsafe { self.0.get_unchecked(..self.0.len().wrapping_sub(1)) }
    }
}

#[inline]
pub fn check_bytes(bytes: &[u8]) -> Result<(), NulError> {
    match memchr(0, bytes) {
        None => Ok(()),
        Some(pos) => Err(NulError { pos }),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("NulError: nul position = {}",.pos)]
pub struct NulError {
    pos: usize,
}

impl NulError {
    #[inline]
    #[must_use]
    pub const fn nul_position(&self) -> usize {
        self.pos
    }
}
