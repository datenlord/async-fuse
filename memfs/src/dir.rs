//! The implementation of directory related functionalities

use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::ptr::NonNull;
use std::{io, ptr, slice};

use memchr::memchr;

/// Directory meta-data
pub struct Dir(NonNull<libc::DIR>);

unsafe impl Send for Dir {}
unsafe impl Sync for Dir {}

impl Drop for Dir {
    fn drop(&mut self) {
        let dirp = self.0.as_ptr();
        unsafe {
            let _ = libc::closedir(dirp);
        }
    }
}

impl Dir {
    /// fdopendir(3)
    pub unsafe fn fdopendir(fd: RawFd) -> io::Result<Self> {
        let dirp = libc::fdopendir(fd);
        if dirp.is_null() {
            let err = io::Error::last_os_error();
            return Err(err);
        }
        Ok(Self(NonNull::new_unchecked(dirp)))
    }

    /// readdir
    pub fn readdir(&mut self) -> ReadDir<'_> {
        ReadDir {
            dir: self,
            end_of_stream: false,
        }
    }
}

/// Iterator over the entries in a directory.
pub struct ReadDir<'a> {
    /// &'a mut Dir
    dir: &'a mut Dir,
    /// end flag
    end_of_stream: bool,
}

/// Entries returned by the [`ReadDir`] iterator.
pub struct DirEntry {
    /// entry
    entry: libc::dirent64,
    /// name len (without NUL)
    name_len: usize,
}

/// cast `&[c_char]` to `&[u8]`
fn cstr_to_bytes(s: &[c_char]) -> &[u8] {
    unsafe { slice::from_raw_parts(s.as_ptr().cast(), s.len()) }
}

impl DirEntry {
    /// get entry name
    pub fn name(&self) -> &OsStr {
        let name_bytes = cstr_to_bytes(&self.entry.d_name);
        let name = unsafe { name_bytes.get_unchecked(..self.name_len) };
        OsStrExt::from_bytes(name)
    }
}

impl ReadDir<'_> {
    /// get dir pointer
    const fn dirp(&self) -> *mut libc::DIR {
        self.dir.0.as_ptr()
    }
}

impl Drop for ReadDir<'_> {
    fn drop(&mut self) {
        unsafe { libc::rewinddir(self.dirp()) }
    }
}

impl Iterator for ReadDir<'_> {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        if self.end_of_stream {
            return None;
        }
        unsafe {
            let mut entry: libc::dirent64 = MaybeUninit::zeroed().assume_init();
            let mut entry_ptr = ptr::null_mut();
            let dirp = self.dirp();
            loop {
                let ret = libc::readdir64_r(dirp, &mut entry, &mut entry_ptr);
                if entry_ptr.is_null() {
                    self.end_of_stream = true;
                    if ret == 0 {
                        return None;
                    } else {
                        return Some(Err(io::Error::last_os_error()));
                    }
                }
                let name_bytes = cstr_to_bytes(&entry.d_name);
                let name_len = match memchr(0, name_bytes) {
                    None => panic!("entry name has no NUL byte: {:?}", name_bytes),
                    Some(idx) => idx,
                };
                debug_assert!(name_len < 256);

                match name_bytes {
                    [b'.', 0, ..] | [b'.', b'.', 0, ..] => continue,
                    _ => return Some(Ok(DirEntry { entry, name_len })),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() -> io::Result<()> {
        let fd = unsafe {
            let ret = libc::open(
                b".\0".as_ptr().cast(),
                libc::O_RDONLY | libc::O_DIRECTORY,
                0,
            );
            if ret < 0 {
                return Err(io::Error::last_os_error());
            }
            ret
        };

        let mut dir = unsafe {
            Dir::fdopendir(fd).map_err(|err| {
                let _ = libc::close(fd);
                err
            })
        }?;

        for entry in dir.readdir() {
            let entry = entry?;
            dbg!(&entry.name());
        }

        for entry in dir.readdir() {
            let entry = entry?;
            dbg!(&entry.name());
        }

        Ok(())
    }
}
