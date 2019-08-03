use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};
use super::*;
use std::time::{Duration, SystemTime};
use libc::ENOENT;
use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::{Rc, Weak};
use std::borrow::Borrow;
use json_fuse_fs::Flatten;

const TTL: Duration = Duration::from_secs(1);

pub struct JsonFS {
    fs_tree_root: Rc<FSNode>,
    inode: HashMap<u64, Weak<FSNode>>,
    dir_listing: HashMap<u64, Vec<(u64, FileType, OsString)>>
}

impl JsonFS {
    pub fn new(fs_tree_root: Rc<FSNode>, inode: HashMap<u64, Weak<FSNode>>) -> JsonFS {
        let dir_listing = JsonFS::generate_dir_listing(fs_tree_root.flatten());
        info!("Inode map: {:?}", inode);
        JsonFS { fs_tree_root, inode, dir_listing }
    }

    fn generate_dir_listing(nodes: Vec<Weak<FSNode>>) -> HashMap<u64, Vec<(u64, FileType, OsString)>> {
        let mut result = HashMap::new();

        for weak_node in nodes.iter() {
            let node: Rc<FSNode> = weak_node.upgrade().unwrap();
            if let FSNode { inode, parent, entry: FSEntry::Dir(entries), .. } = node.borrow() {
                let mut dir_listing: Vec<(u64, FileType, OsString)> = vec![
                    (*inode, FileType::Directory, OsString::from("."))
                ];
                if let Some(parent_rc) = parent.borrow().upgrade() {
                    dir_listing.push((parent_rc.inode, FileType::Directory, OsString::from("..")))
                };
                dir_listing.extend(
                    entries
                        .iter()
                        .map(|node| {
                            match node.borrow() {
                                FSNode { inode, name, entry: FSEntry::Dir(_), .. } => (*inode, FileType::Directory, OsString::from(name)),
                                FSNode { inode, name, entry: FSEntry::File(_), .. } => (*inode, FileType::RegularFile, OsString::from(name))
                            }
                        })
                        .collect::<Vec<(u64, FileType, OsString)>>()
                );
                result.insert(*inode, dir_listing);
            }
        }

        info!("Generated dir listing: {:?}", result);

        result
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

    fn get_node_attr(&self, entry: &FSNode) -> FileAttr {
        match entry {
            FSNode { inode, entry: FSEntry::File(file), .. } => file.ops().get_attributes(*inode),
            FSNode { inode, entry: FSEntry::Dir(_), .. } => self.generate_dir_attr(*inode)
        }
    }
}

// https://github.com/libfuse/libfuse/blob/e16fdc06d7473f00499b6b03fb7bd06259a22135/include/fuse.h#L290
impl Filesystem for JsonFS {

    fn lookup(&mut self, _req: &Request, parent: u64, lookup_name: &OsStr, reply: ReplyEntry) {
        info!("lookup for name: {} parent: {}", lookup_name.to_str().unwrap(), parent);
        if let FSNode { name, entry: FSEntry::Dir(entries), .. } = self.inode.get(&parent).unwrap().upgrade().unwrap().borrow() {
            info!("lookup in dir: {:?}, {:?}", name, entries);
            if let Some(entry) = entries
                .iter()
                .find(|e| e.name == lookup_name.to_str().unwrap()) {
                reply.entry(&TTL, &self.get_node_attr(&*entry), 0);
                return;
            }
        }
        reply.error(ENOENT);
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
        info!("getattr for {}", ino);
        if let Some(entry) = self.inode.get(&ino).unwrap().upgrade() {
            reply.attr(
                &TTL,
                &self.get_node_attr(&*entry)
            );
            return;
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
        info!("read for {} at offset {}", ino, offset);
        if let FSNode {entry: FSEntry::File(file_type), .. }  = self.inode.get(&ino).unwrap().upgrade().unwrap().borrow() {
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
        info!("readdir for {} and offset {}", ino, offset);
        if let Some(dir_entries) = self.dir_listing.get(&ino) {
            if offset < dir_entries.len().try_into().unwrap() {
                dir_entries
                    .iter()
                    .skip(offset as usize)
                    .enumerate()
                    .find(|(i, (inode, f, s))| reply.add(*inode, *i as i64 + 1, *f, s));
            }
            reply.ok();
            return;
        }
        reply.error(ENOENT);
    }

}
