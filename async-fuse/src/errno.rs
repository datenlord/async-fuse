pub struct Errno(i32);

impl Errno {
    pub fn as_raw(&self) -> i32 {
        self.0
    }
}

impl Errno {
    pub const NOSYS: Self = Self(libc::ENOSYS);
}
