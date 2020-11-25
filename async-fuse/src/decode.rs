use crate::abi_marker::FuseAbiData;
use crate::c_bytes::CBytes;

use std::mem;
use std::slice;

use memchr::memchr;

#[derive(Debug)]
pub struct Decoder<'b> {
    bytes: &'b [u8],
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("NotEnough")]
    NotEnough,

    #[error("TooMuchData")]
    TooMuchData,

    #[error("AlignMismatch")]
    AlignMismatch,

    #[error("NumOverflow")]
    NumOverflow,

    #[error("InvalidValue")]
    InvalidValue,
}

pub trait Decode<'b>: Sized {
    fn decode(de: &mut Decoder<'b>) -> Result<Self, DecodeError>;
}

fn to_address<T: ?Sized>(ptr: *const T) -> usize {
    ptr as *const () as usize
}

impl<'b> Decoder<'b> {
    pub fn new(bytes: &'b [u8]) -> Self {
        Self { bytes }
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn decode<T: Decode<'b>>(&mut self) -> Result<T, DecodeError> {
        <T as Decode>::decode(self)
    }

    unsafe fn pop_bytes_unchecked(&mut self, len: usize) -> &'b [u8] {
        let bytes = self.bytes.get_unchecked(..len);
        self.bytes = self.bytes.get_unchecked(len..);
        bytes
    }

    pub(crate) fn fetch<T: FuseAbiData + Sized>(&mut self) -> Result<&'b T, DecodeError> {
        let ty_size: usize = mem::size_of::<T>();
        let ty_align: usize = mem::align_of::<T>();
        debug_assert!(ty_size > 0 && ty_size.wrapping_rem(ty_align) == 0);

        if self.bytes.len() < ty_size {
            return Err(DecodeError::NotEnough);
        }

        let addr = to_address(self.bytes);
        if addr.wrapping_rem(ty_align) != 0 {
            return Err(DecodeError::AlignMismatch);
        }

        unsafe {
            let bytes = self.pop_bytes_unchecked(ty_size);
            let ret = &*(bytes.as_ptr().cast());
            Ok(ret)
        }
    }

    pub(crate) fn fetch_slice<T: FuseAbiData + Sized>(
        &mut self,
        len: usize,
    ) -> Result<&'b [T], DecodeError> {
        let ty_size: usize = mem::size_of::<T>();
        let ty_align: usize = mem::align_of::<T>();
        debug_assert!(ty_size > 0 && ty_size.wrapping_rem(ty_align) == 0);

        let (slice_size, is_overflow) = ty_size.overflowing_mul(len);
        if is_overflow {
            return Err(DecodeError::NumOverflow);
        }

        if self.bytes.len() < slice_size {
            return Err(DecodeError::NotEnough);
        }

        let addr = to_address(self.bytes);
        if addr.wrapping_rem(ty_align) != 0 {
            return Err(DecodeError::AlignMismatch);
        }

        unsafe {
            let bytes = self.pop_bytes_unchecked(slice_size);
            let ret = slice::from_raw_parts(bytes.as_ptr().cast(), len);
            Ok(ret)
        }
    }

    pub fn fetch_all_bytes(&mut self) -> Result<&'b [u8], DecodeError> {
        unsafe {
            let bytes = self.bytes;
            self.bytes = slice::from_raw_parts(self.bytes.as_ptr(), 0);
            Ok(bytes)
        }
    }

    pub(crate) fn fetch_c_bytes(&mut self) -> Result<CBytes<'b>, DecodeError> {
        let idx = memchr(0, self.bytes).ok_or(DecodeError::NotEnough)?;
        let len = idx.wrapping_add(1);
        assert!(len <= self.bytes.len());

        unsafe {
            let bytes = self.pop_bytes_unchecked(len);
            let ret = bytes.get_unchecked(..len);
            Ok(CBytes::new_unchecked(ret))
        }
    }

    pub fn all_consuming<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<T, DecodeError> {
        let ret = f(self)?;
        if !self.is_empty() {
            return Err(DecodeError::TooMuchData);
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aligned_bytes::stack;

    #[test]
    fn decode_integer_ok() {
        let data: stack::Align16<[u8; 4]> = stack::align16([1, 2, 3, 4]);
        let mut decoder = Decoder::new(&*data);

        let ret = decoder.fetch::<u32>().unwrap();
        assert_eq!(ret, &u32::from_ne_bytes(data.into_inner()));

        assert!(decoder.bytes.is_empty())
    }

    #[test]
    fn decode_integer_align_mismatch() {
        let data: stack::Align16<[u8; 5]> = stack::align16([1, 2, 3, 4, 5]);
        let mut decoder = Decoder::new(&data.as_ref()[1..]);

        let ret = decoder.fetch::<u32>().unwrap_err();
        assert_eq!(ret, DecodeError::AlignMismatch);

        assert!(decoder.bytes.len() == 4);
    }

    #[test]
    fn decode_integer_not_enough() {
        let data: stack::Align16<[u8; 4]> = stack::align16([1, 2, 3, 4]);
        let mut decoder = Decoder::new(&*data);

        let ret = decoder.fetch::<u64>().unwrap_err();
        assert_eq!(ret, DecodeError::NotEnough);

        assert!(decoder.bytes.len() == 4);
    }

    #[test]
    fn decode_slice_ok() {
        let data: stack::Align16<[u8; 4]> = stack::align16([1, 2, 3, 4]);
        let mut decoder = Decoder::new(&*data);

        let ret = decoder.fetch_slice::<u16>(2).unwrap();
        assert_eq!(
            ret,
            &[u16::from_ne_bytes([1, 2]), u16::from_ne_bytes([3, 4])]
        );

        assert!(decoder.bytes.is_empty())
    }
}
