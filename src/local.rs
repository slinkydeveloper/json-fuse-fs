use super::*;

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Hash)]
pub struct LocalFSFileType {
    pub file_path: String
}

impl FSFileTypeOps for LocalFSFileType {
    fn get_attributes(&self, inode: u64) -> FileAttr {
        unimplemented!()
    }
    fn read(&self, offset: i64) -> Option<&[u8]> {
        unimplemented!()
    }
}

impl LocalFSFileType {
    pub fn new(pointer: String) -> LocalFSFileType {
        LocalFSFileType {
            file_path: pointer
        }
    }
}