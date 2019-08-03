pub mod raw;
pub mod local;

use std::error::Error;
use std::fmt::{Display, Formatter, Debug};
use std::{fmt, iter};
use std::path::{Path, Component};
use std::ffi::OsStr;
use raw::RawFSFileType;
use local::LocalFSFileType;
use fuse::FileAttr;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;
use std::borrow::Borrow;

#[derive(Debug)]
pub struct FSNode {
    pub inode: u64,
    pub name: String,
    pub parent: RefCell<Weak<FSNode>>,
    pub entry: FSEntry
}

#[derive(Debug)]
pub enum FSEntry {
    File(FSFileType),
    Dir(Vec<Rc<FSNode>>)
}

#[derive(Debug)]
pub enum FSFileType {
    Raw(RawFSFileType),
    Local(LocalFSFileType),
}

impl FSNode {
    pub fn new(descriptor: serde_json::Value) -> Result<(Rc<FSNode>, HashMap<u64, Weak<FSNode>>), DescriptorError> {
        let fs_tree = FSNode::_new(&mut 0, String::new(), descriptor)?;
        let map: HashMap<u64, Weak<FSNode>> = fs_tree
            .flatten()
            .into_iter()
            .map(|e| {
                let inode = e.upgrade().unwrap().inode;
                (inode, e)
            })
            .collect();

        Ok((fs_tree, map))
    }

    fn _new(parent_inode: &mut u64, name: String, descriptor: serde_json::Value) -> Result<Rc<FSNode>, DescriptorError> {
        use serde_json::value::Value::*;

        *parent_inode = *parent_inode + 1;
        let this_node_inode = *parent_inode;

        // Create the entry of this node
        let entry = match descriptor {
            Object(m) => FSEntry::create_directory(parent_inode, m),
            String(s) => FSEntry::create_file(s),
            _ => Err(DescriptorError)
        }?;

        // Create this node
        let node = Rc::new(FSNode {
            inode: this_node_inode,
            name,
            parent: RefCell::new(Weak::new()),
            entry
        });

        // Link the parents
        if let FSEntry::Dir(childs) = &node.entry {
            for child in childs {
                *child.parent.borrow_mut() = Rc::downgrade(&node)
            }
        }

        Ok(node)
    }

    pub fn walk(&self, path: String) -> Option<&FSNode> {
        Path::new(&path)
            .components()
            .skip(1)
            .fold(Some(self), |o, c| o.and_then(|e| e._walk(c)))
    }

    fn _walk(&self, component: Component) -> Option<&FSNode> {
        match (component, self) {
            (Component::Normal(c), FSNode { inode:_, name: _, parent: _, entry: FSEntry::Dir(entries) }) =>
                entries
                    .iter()
                    .find(|e| OsStr::new(&e.name) == c)
                    .map(|r| r.borrow()),
            (_, _) => None
        }
    }
}

pub trait Flatten<T> {
    fn flatten(&self) -> Vec<Weak<T>>;
}

impl Flatten<FSNode> for Rc<FSNode> {

    fn flatten(&self) -> Vec<Weak<FSNode>> {
        match &self.entry {
            FSEntry::Dir (entries) =>
                iter::once(Rc::downgrade(self))
                    .chain(entries.iter().flat_map(|e| e.flatten()))
                    .collect(),
            FSEntry::File(_) => vec![Rc::downgrade(self)]
        }
    }

}

impl FSEntry {

    fn create_file(file_descriptor: String) -> Result<FSEntry, DescriptorError> {
        let (descriptor_type, descriptor_pointer) = file_descriptor
            .split_at(file_descriptor.find(':').ok_or(DescriptorError)?);

        let fs_entry_type = FSFileType::parse_file_type(descriptor_type, descriptor_pointer[1..].to_string())?;

        Ok(FSEntry::File(fs_entry_type))
    }

    fn create_directory<'a>(parent_inode: &mut u64, dir_descriptor: serde_json::Map<String, serde_json::Value>) -> Result<FSEntry, DescriptorError> {
        let entries_result: Result<Vec<Rc<FSNode>>, DescriptorError> =
            dir_descriptor
                .into_iter()
                .map(|(k, v)| FSNode::_new(parent_inode, k, v))
                .collect();

        Ok(FSEntry::Dir(entries_result?))
    }
}

pub trait FSFileTypeOps {
    fn get_attributes(&self, inode: u64) -> FileAttr;
    fn read(&self, offset: i64) -> Option<&[u8]>;
}

impl FSFileType {
    fn parse_file_type(type_descriptor: &str, pointer: String) -> Result<FSFileType, DescriptorError> {
        match type_descriptor {
            "raw" => Ok(FSFileType::Raw(raw::RawFSFileType::new(pointer))),
            "file" => Ok(FSFileType::Local(LocalFSFileType::new(pointer))),
            _ => Err(DescriptorError)
        }
    }

    pub fn ops(&self) -> &FSFileTypeOps {
        match self {
            FSFileType::Raw(s) => s,
            FSFileType::Local(s) => s
        }
    }
}

pub struct DescriptorError;

impl Debug for DescriptorError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "JSON descriptor error")
    }
}

impl Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JSON descriptor error")
    }
}

impl Error for DescriptorError {}
