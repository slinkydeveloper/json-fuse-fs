use super::*;
use std::time::SystemTime;
use fuse::{FileType, FileAttr};

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Hash)]
pub struct RawFSFileType {
    pub data: String
}

impl RawFSFileType {
    pub fn new(pointer: String) -> RawFSFileType {
        RawFSFileType {
            data: pointer
        }
    }
}

impl FSFileTypeOps for RawFSFileType {
    fn get_attributes(&self, inode: u64) -> FileAttr {
        FileAttr {
            ino: inode,
            size: self.data.bytes().len() as u64,
            blocks: 1,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: nix::unistd::getuid().into(),
            gid: nix::unistd::getgid().into(),
            rdev: 0,
            flags: 0
        }
    }

    fn read(&self, offset: i64) -> Option<&[u8]> {
        Some(&self.data.as_bytes()[offset as usize..])
    }
}
