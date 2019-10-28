use super::*;

use nix::sys::stat::{FileStat, stat};
use fuse::{FileType, FileAttr};
use std::time::SystemTime;
use std::time::Duration;
use std::fs::{Metadata, File};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::io::{Seek, SeekFrom, Read};

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Hash)]
pub struct LocalFSFileType {
    pub file_path: String
}

impl LocalFSFileType {
    pub fn new(pointer: String) -> LocalFSFileType {
        LocalFSFileType {
            file_path: pointer
        }
    }
}

macro_rules! stat_time_to_SystemTime {
    ($msec:expr, $nsec:expr) => { SystemTime::UNIX_EPOCH + Duration::new($msec as u64, $nsec as u32) };
}

impl FSFileTypeOps for LocalFSFileType {
    fn get_attributes(&self, inode: u64) -> FileAttr {
        let stat: FileStat = stat(OsStr::new(&self.file_path)).unwrap();
        let meta: Metadata = fs::metadata(&self.file_path).unwrap();
        FileAttr {
            ino: inode,
            size: stat.st_size as u64,
            blocks: stat.st_blocks as u64,
            atime: stat_time_to_SystemTime!(stat.st_atime, stat.st_atime_nsec),
            mtime: stat_time_to_SystemTime!(stat.st_mtime, stat.st_mtime_nsec),
            ctime: stat_time_to_SystemTime!(stat.st_ctime, stat.st_ctime_nsec),
            crtime: stat_time_to_SystemTime!(stat.st_ctime, stat.st_ctime_nsec),
            kind: FileType::RegularFile,
            perm: meta.permissions().mode() as u16,
            nlink: stat.st_nlink as u32,
            uid: stat.st_uid,
            gid: stat.st_gid,
            rdev: stat.st_rdev as u32,
            flags: 0
        }
    }
    fn read(&self, offset: i64, buffer: &mut [u8]) -> io::Result<()> {
        let mut file = File::open(&self.file_path)?;

        file.seek(SeekFrom::Start(offset as u64))?;
        file.read(buffer)?;
        Ok(())
    }
}