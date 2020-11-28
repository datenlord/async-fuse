// TODO: extract into another crate

#![allow(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

pub trait WrappingCast {
    type Target;
    fn wrapping_cast(self) -> Self::Target;
}

macro_rules! impl_cast {
    ($($lhs:ty => $rhs:ty, f = $f:ident;)+) => {
        $(
        impl WrappingCast for $lhs {
            type Target = $rhs;

            #[inline]
            fn wrapping_cast(self) -> Self::Target {
                $f(self)
            }
        }
        )+

        #[test]
        fn size_eq(){
            $(
            assert_eq!(std::mem::size_of::<$lhs>(), std::mem::size_of::<$rhs>());
            )+
        }

        $(
        #[inline]
        #[must_use]
        pub const fn $f(x: $lhs)->$rhs{
            x as $rhs
        }
        )+
    };
}

impl_cast!(
    u8 => i8,       f = wrapping_cast_i8;
    u16 => i16,     f = wrapping_cast_i16;
    u32 => i32,     f = wrapping_cast_i32;
    u64 => i64,     f = wrapping_cast_i64;
    usize => isize, f = wrapping_cast_isize;
    i8 => u8,       f = wrapping_cast_u8;
    i16 => u16,     f = wrapping_cast_u16;
    i32 => u32,     f = wrapping_cast_u32;
    i64 => u64,     f = wrapping_cast_u64;
    isize => usize, f = wrapping_cast_usize;
);

pub trait TruncatingCast<U> {
    fn truncating_cast(self) -> U;
}

impl TruncatingCast<u8> for u32 {
    #[inline]
    fn truncating_cast(self) -> u8 {
        self as u8
    }
}

pub mod extending_cast {
    #[must_use]
    #[inline]
    pub const fn u8_to_u32(x: u8) -> u32 {
        x as u32
    }
}
