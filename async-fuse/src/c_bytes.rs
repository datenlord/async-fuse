use memchr::memchr;

#[derive(Debug)]
pub struct CBytes<'b>(&'b [u8]);

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

pub fn check_bytes(bytes: &[u8]) -> Result<&[u8], NulError> {
    match memchr(0, bytes) {
        None => Ok(bytes),
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
