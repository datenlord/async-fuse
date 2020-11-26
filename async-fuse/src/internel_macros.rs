macro_rules! derive_Decode {
    ($t:ty) => {
        impl<'b> crate::de::Decode<'b> for $t {
            fn decode(
                de: &mut crate::de::Decoder<'b>,
            ) -> Result<Self, crate::de::DecodeError> {
                Ok(Self(de.fetch()?))
            }
        }
    };

    (@c_bytes $t:ty,$($f:ident),+) => {
        impl<'b> crate::de::Decode<'b> for $t {
            fn decode(
                de: &mut crate::de::Decoder<'b>,
            ) -> Result<Self, crate::de::DecodeError> {
                $(let $f = de.fetch_c_bytes()?;)+
                Ok(Self{$($f),+})
            }
        }
    };

    (@header $t:ty, $h:ident, $b: ident) => {
        impl<'b> crate::de::Decode<'b> for $t {
            fn decode(
                de: &mut crate::de::Decoder<'b>,
            ) -> Result<Self, crate::de::DecodeError> {
                let $h = de.fetch()?;
                let $b = de.fetch_c_bytes()?;
                Ok(Self{$h, $b})
            }
        }
    };

    (@data $t:ty, $h:ident, $b: ident) => {
        impl<'b> crate::de::Decode<'b> for $t {
            fn decode(
                de: &mut crate::de::Decoder<'b>,
            ) -> Result<Self, crate::de::DecodeError> {
                let $h = de.fetch()?;
                let $b = de.fetch_all_bytes()?;
                Ok(Self{$h, $b})
            }
        }
    };

    (@empty $t:ty) => {
        impl<'b> crate::de::Decode<'b> for $t {
            fn decode(
                _: &mut crate::de::Decoder<'b>,
            ) -> Result<Self, crate::de::DecodeError> {
                Ok(Self(&()))
            }
        }
    }
}

macro_rules! derive_Encode {
    ($t:ty) => {
        #[allow(unused_qualifications)]
        impl crate::encode::Encode for $t {
            fn collect_bytes<'c, C>(&'c self, container: &mut C)
            where
                C: Extend<std::io::IoSlice<'c>>,
            {
                let bytes = crate::encode::as_abi_bytes(&self.0);
                container.extend(Some(std::io::IoSlice::new(bytes)))
            }
        }
    };
}

macro_rules! declare_relation {
    ($op:ty => $reply:ty) => {
        #[allow(unused_qualifications, single_use_lifetimes)]
        impl<'a> crate::ops::IsReplyOf<$op> for $reply {}
    };
}

macro_rules! getters {
    ($($f:ident: $t:ty,)+) => {$(
        #[must_use]
        pub const fn $f(&self) -> $t {
            self.0.$f
        }
    )+};
}

macro_rules! setters {
    ($($f:ident: $t:ty,)+) => {$(
        pub fn $f(&mut self, $f: $t) -> &mut Self {
            self.0.$f = $f;
            self
        }
    )+};
}
