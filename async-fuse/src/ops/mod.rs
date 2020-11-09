mod init;
pub use self::init::*;

#[non_exhaustive]
pub enum Operation<'a> {
    Init(OpInit<'a>),
    // TODO: add more operations
}
