use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};
use bimap::BiHashMap;
use super::*;
use std::time::{Duration, SystemTime};
use libc::{ENOENT, ENOSYS};

const TTL: Duration = Duration::from_secs(1);

pub struct JsonFS<'a> {
    fs_tree_root: FSEntry,
    inode: BiHashMap<u64, &'a FSEntry>
}

impl JsonFS<'_> {
    pub fn new(fs_tree_root: FSEntry, inode: BiHashMap<u64, &FSEntry>) -> JsonFS {
        JsonFS { fs_tree_root, inode }
    }

    fn generate_dir_attr(&self, inode: u64) -> FileAttr {
        FileAttr {
            ino: inode,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: nix::unistd::getuid().into(),
            gid: nix::unistd::getgid().into(),
            rdev: 0,
            flags: 0
        }
    }
}

// https://github.com/libfuse/libfuse/blob/e16fdc06d7473f00499b6b03fb7bd06259a22135/include/fuse.h#L290
impl Filesystem for JsonFS<'_> {

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("Lookup for name: {}, parent: {}", name.to_str().unwrap(), parent);
        reply.error(ENOSYS);
    }

    /** Get file attributes.
     *
     * Similar to stat().  The 'st_dev' and 'st_blksize' fields are
     * ignored. The 'st_ino' field is ignored except if the 'use_ino'
     * mount option is given.
     *
     * `fi` will always be NULL if the file is not currently open, but
     * may also be NULL if the file is open.
     */
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if let Some(entry) = self.inode.get_by_left(&ino) {
            reply.attr(
                &TTL,
                &match entry {
                    FSEntry::File {name, file_type} => file_type.ops().get_attributes(ino),
                    FSEntry::Dir {name, entries } => self.generate_dir_attr(ino)
                }
            );
        }
        reply.error(ENOENT);
    }

    /** Read data from an open file
     *
     * Read should return exactly the number of bytes requested except
     * on EOF or error, otherwise the rest of the data will be
     * substituted with zeroes.	 An exception to this is when the
     * 'direct_io' mount option is specified, in which case the return
     * value of the read system call will reflect the return value of
     * this operation.
     */
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, _size: u32, reply: ReplyData) {
        if let Some(FSEntry::File {name, file_type}) = self.inode.get_by_left(&ino) {
            if let Some(data) = file_type.ops().read(offset) {
                reply.data(data);
                return;
            }
        }
        reply.error(ENOENT);
    }

    /** Read directory
     *
     * The filesystem may choose between two modes of operation:
     *
     * 1) The readdir implementation ignores the offset parameter, and
     * passes zero to the filler function's offset.  The filler
     * function will not return '1' (unless an error happens), so the
     * whole directory is read in a single readdir operation.
     *
     * 2) The readdir implementation keeps track of the offsets of the
     * directory entries.  It uses the offset parameter and always
     * passes non-zero offset to the filler function.  When the buffer
     * is full (or an error happens) the filler function will return
     * '1'.
     */
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if let Some(FSEntry::Dir {name, entries}) = self.inode.get_by_left(&ino) {
            reply.add(ino, 0, FileType::Directory, ".");
            entries
                .iter()
                .map(|e| (self.inode.get_by_right(&e).unwrap(), e))
                .for_each(|(i, e)| {
                    match e {
                        FSEntry::Dir {name, entries} => reply.add(*i, 0, FileType::Directory, name),
                        FSEntry::File {name, file_type} => reply.add(*i, 0, FileType::RegularFile, name)
                    };
                });
            reply.ok();
        }
        reply.error(ENOENT);
    }

}
