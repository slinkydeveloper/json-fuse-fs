use super::*;
use std::time::SystemTime;
use fuse::{FileType, FileAttr};
use reqwest::StatusCode;
use std::io::Read;
use log::info;

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Hash)]
pub struct HttpFSFileType {
    pub address: String
}

impl HttpFSFileType {
    pub fn new(pointer: String) -> HttpFSFileType {
        HttpFSFileType {
            address: pointer
        }
    }
}

impl FSFileTypeOps for HttpFSFileType {
    fn get_attributes(&self, inode: u64) -> FileAttr {
        let client = reqwest::Client::new();
        let res = client.head(&self.address).send().unwrap();

        let size: u64 = res.content_length().unwrap_or(0);

        FileAttr {
            ino: inode,
            size,
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

    fn read(&self, offset: i64, buffer: &mut [u8]) -> io::Result<()> {
        let mut resp = reqwest::get(&self.address).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        if resp.status() ==  StatusCode::OK {
            let off: usize = offset as usize;
            let mut body: Vec<u8> = vec![];
            resp.read_to_end(&mut body);

            info!("Received response of length {:?}, content-length: {:?}", body.len(), resp.content_length());

            if buffer.len() > body.len() - off {
                buffer[..body.len() - off].copy_from_slice(&body[offset as usize..])
            } else {
                buffer.copy_from_slice(&body[offset as usize..buffer.len()])
            }

            Ok(())
        } else {
            info!("Response received, but with status code {:?}", resp.status());
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Shit happens"))
        }
    }
}
