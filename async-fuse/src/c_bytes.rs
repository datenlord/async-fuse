use std::ascii;
use std::fmt::{self, Debug};

use memchr::memchr;

pub struct CBytes<'b>(&'b [u8]);

impl Debug for CBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0 {
            for c in ascii::escape_default(b) {
                write!(f, "{}", c as char)?;
            }
        }
        write!(f, "\"")
    }
}

impl<'b> CBytes<'b> {
    pub unsafe fn new_unchecked(bytes: &'b [u8]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &'b [u8] {
        debug_assert!(!self.0.is_empty());
        debug_assert!(self.0[self.0.len() - 1] == 0);
        unsafe { self.0.get_unchecked(..self.0.len() - 1) }
    }
}

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
    pub fn nul_position(&self) -> usize {
        self.pos
    }
}
