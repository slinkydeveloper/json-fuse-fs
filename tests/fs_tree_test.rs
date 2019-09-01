extern crate json_fuse_fs;

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use json_fuse_fs::*;
use json_fuse_fs::raw::RawFSFileType;
use std::borrow::Borrow;

macro_rules! assert_file_name {
    ($entry:expr, $name:expr) => ({
        let e = $entry;
        if let FSNode { name, entry: FSEntry::File(_), .. } = e {
            assert_eq!(name, ($name))
        } else {
            panic!("FSNode.entry is not a FSEntry::Dir")
        }
    });
}

macro_rules! assert_file_local_file_path {
    ($entry:expr, $file_name:expr) => ({
        let (e, f) = ($entry, $file_name);
        if let FSNode { entry: FSEntry::File(FSFileType::Local(loc)),  .. } = e {
            assert_eq!(loc.file_path, f);
        } else {
            panic!("FSNode.entry is not a FSEntry::File(FSFileType::Local(_))")
        }
    });
}

macro_rules! assert_file_raw_data {
    ($entry:expr, $data:expr) => ({
        let (e, f) = ($entry, $data);
        if let FSNode { entry: FSEntry::File(FSFileType::Raw(raw)), .. } = e {
            assert_eq!(raw.data, f);
        } else {
            panic!("FSNode.entry is not a FSEntry::File(FSFileType::Raw(_))")
        }
    });
}

macro_rules! assert_dir_name {
    ($entry:expr, $name:expr) => ({
        let e = $entry;
        if let FSNode { name, entry: FSEntry::Dir(_), .. } = e {
            assert_eq!(name, ($name))
        } else {
            panic!("FSNode.entry is not a FSEntry::Dir(_)")
        }
    });
}

fn nested_structure() -> Rc<FSNode> {
    Rc::new(FSNode {
        inode: 1,
        name: String::new(),
        parent: RefCell::new(Weak::new()),
        entry: FSEntry::Dir(
            vec![
                Rc::new(FSNode {
                    inode: 2,
                    name: String::from("bla"),
                    parent: RefCell::new(Weak::new()),
                    entry: FSEntry::Dir(
                        vec![
                            Rc::new(FSNode {
                                inode: 3,
                                name: "file.txt".to_string(),
                                parent: RefCell::new(Weak::new()),
                                entry: FSEntry::File(FSFileType::Raw(RawFSFileType::new("abc".to_string())))
                            })
                        ]
                    )
                })
            ]
        )
    })
}

#[test]
fn walk_to_root() {
    let fs_tree = nested_structure();
    let found = fs_tree.walk("/".to_string()).unwrap();

    assert_dir_name!(found, "");
}

#[test]
fn walk_to_file() {
    let structure = Rc::new(FSNode {
        inode: 1,
        name: String::new(),
        parent: RefCell::new(Weak::new()),
        entry: FSEntry::Dir(
            vec![
                Rc::new(FSNode {
                    inode: 2,
                    name: "file.txt".to_string(),
                    parent: RefCell::new(Weak::new()),
                    entry: FSEntry::File(FSFileType::Raw(RawFSFileType::new("abc".to_string())))
                })
            ]
        )
    });

    let found = structure.walk("/file.txt".to_string()).unwrap();

    assert_file_name!(found, "file.txt");
}

#[test]
fn walk_to_dir() {
    let structure = Rc::new(FSNode {
        inode: 1,
        name: String::new(),
        parent: RefCell::new(Weak::new()),
        entry: FSEntry::Dir(
            vec![
                Rc::new(FSNode {
                    inode: 2,
                    name: "file.txt".to_string(),
                    parent: RefCell::new(Weak::new()),
                    entry: FSEntry::File(FSFileType::Raw(RawFSFileType::new("abc".to_string())))
                }),
                Rc::new(FSNode {
                    inode: 3,
                    name: String::from("bla"),
                    parent: RefCell::new(Weak::new()),
                    entry: FSEntry::Dir(vec![])
                })
            ]
        )
    });

    let found = structure.walk("/bla".to_string()).unwrap();

    assert_dir_name!(found, "bla");
}

#[test]
fn walk_to_nested() {
    let fs_tree = nested_structure();

    let found = fs_tree.walk("/bla/file.txt".to_string()).unwrap();

    assert_file_name!(found, "file.txt");
}

#[test]
fn flatten() {
    let fs_tree = nested_structure();

    let found: Vec<Weak<FSNode>> = fs_tree.flatten();

    assert_eq!(found.len(), 3);

    let root = found[0].upgrade().unwrap();
    assert_dir_name!(root.borrow(), "");

    let bla = found[1].upgrade().unwrap();
    assert_dir_name!(bla.borrow(), "bla");

    let file = found[2].upgrade().unwrap();

    assert_file_name!(file.borrow(), "file.txt");
}

#[test]
fn load_raw_file_type() {
    let json = r#"
            {
                "file.txt": "raw:abc"
            }"#;

    let result = FSNode::new(serde_json::from_str(json).unwrap());
    assert!(result.is_ok());

    let (fs_tree, inode_map) = result.unwrap();

    assert_dir_name!(fs_tree.walk("/".to_string()).unwrap(), "");
    assert_file_name!(fs_tree.walk("/file.txt".to_string()).unwrap(), "file.txt");
    assert_file_raw_data!(fs_tree.walk("/file.txt".to_string()).unwrap(), "abc");

    let root_from_inode = inode_map.get(&1).unwrap().upgrade().unwrap();
    assert_dir_name!(root_from_inode.borrow(), "");

    let file_from_inode = inode_map.get(&2).unwrap().upgrade().unwrap();
    assert_file_name!(file_from_inode.borrow(), "file.txt");
}

#[test]
fn load_local_file_type() {
    let json = r#"
            {
                "file.txt": "file:/my_file.txt"
            }"#;

    let result = FSNode::new(serde_json::from_str(json).unwrap());
    assert!(result.is_ok());

    let (fs_tree, _) = result.unwrap();

    assert_dir_name!(fs_tree.walk("/".to_string()).unwrap(), "");
    assert_file_name!(fs_tree.walk("/file.txt".to_string()).unwrap(), "file.txt");
    assert_file_local_file_path!(fs_tree.walk("/file.txt".to_string()).unwrap(),  "/my_file.txt");
}

#[test]
fn load_nested() {
    let json = r#"
            {
                "file.txt": "file:/my_file.txt",
                "nested": {
                    "nested.txt": "raw:cba"
                }
            }"#;

    let result = FSNode::new(serde_json::from_str(json).unwrap());
    assert!(result.is_ok());

    let (fs_tree, _) = result.unwrap();

    assert_file_name!(fs_tree.walk("/nested/nested.txt".to_string()).unwrap(), "nested.txt");

    let nested = fs_tree.walk("/nested".to_string()).unwrap();
    assert_dir_name!(nested, "nested");
    assert_eq!(1, nested.parent.borrow().upgrade().unwrap().inode);
}
