use std::any::type_name;
use std::convert::TryFrom;
use std::fmt::Debug;

pub fn force_convert<T, U>(x: T) -> U
where
    U: TryFrom<T> + Debug + Copy + 'static,
    U::Error: std::error::Error,
    T: Debug + Copy + 'static,
{
    match U::try_from(x) {
        Ok(y) => y,
        Err(e) => panic!(
            "failed to convert {} to {}, value = {:?}, error = {}",
            type_name::<T>(),
            type_name::<U>(),
            x,
            e
        ),
    }
}
