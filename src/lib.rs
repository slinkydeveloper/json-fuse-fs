pub mod jsonfs {
    use std::error::Error;
    use std::fmt::{Display, Formatter, Debug};
    use std::{fmt, iter};
    use std::path::{Path, Component};
    use std::ffi::OsStr;
    use std::collections::HashMap;
    use raw::RawFSFileType;
    use file::LocalFSFileType;

    #[derive(Debug)]
    pub enum FSFileType {
        Raw(raw::RawFSFileType),
        Local(file::LocalFSFileType)
    }

    pub trait FSFileTypeOps {
        fn read(&self, offset: i64) -> &[u8];
    }

    impl FSFileType {

        fn parse_file_type(type_descriptor: &str, pointer: String) -> Result<FSFileType, DescriptorError> {
            match type_descriptor {
                "raw" => Ok(FSFileType::Raw(raw::RawFSFileType::new(pointer))),
                "file" => Ok(FSFileType::Local(LocalFSFileType::new(pointer))),
                _ => Err(DescriptorError)
            }
        }

        fn ops(&self) -> &FSFileTypeOps {
            match self {
                FSFileType::Raw(s) => s,
                FSFileType::Local(s) => s
            }
        }
    }

    #[derive(Debug)]
    pub enum FSEntry {
        File {
            name: String,
            file_type: FSFileType
        },
        Dir {
            name: String,
            entries: Vec<FSEntry>
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

    impl FSEntry {

        pub fn new(descriptor: serde_json::Value) -> Result<FSEntry, DescriptorError> {
            FSEntry::_new(String::new(), descriptor)
        }

        fn _new(name: String, descriptor: serde_json::Value) -> Result<FSEntry, DescriptorError> {
            use serde_json::value::Value::*;

            match descriptor {
                Object(m) => FSEntry::create_directory(name, m),
                String(s) => FSEntry::create_file(name, s),
                _ => Err(DescriptorError)
            }
        }

        fn create_file(filename: String, file_descriptor: String) -> Result<FSEntry, DescriptorError> {
            let (descriptor_type, descriptor_pointer) = file_descriptor.split_at(file_descriptor.find(':').ok_or(DescriptorError)?);

            let fs_entry_type = FSFileType::parse_file_type(descriptor_type, descriptor_pointer[1..].to_string())?;

            Ok(FSEntry::File {
                name: filename,
                file_type: fs_entry_type
            })
        }

        fn create_directory(dir_name: String, dir_descriptor: serde_json::Map<String, serde_json::Value>) -> Result<FSEntry, DescriptorError> {
            let entries_result: Result<Vec<FSEntry>, DescriptorError> = dir_descriptor.into_iter().map(|(k, v)| FSEntry::_new(k, v)).collect();

            Ok(FSEntry::Dir {
                name: dir_name.to_string(),
                entries: entries_result?
            })
        }

        pub fn name(&self) -> &String {
            match self {
                FSEntry::Dir {name, entries: _} => name,
                FSEntry::File {name,  file_type: _ } => name
            }
        }

        pub fn walk(&self, path: String) -> Option<&FSEntry> {
            Path::new(&path)
                .components()
                .skip(1)
                .fold(Some(self), |o, c| o.and_then(|e| e._walk(c)))
        }

        pub fn flatten(&self) -> Vec<&FSEntry> {
            match self {
                FSEntry::Dir {name: _, entries} =>
                    iter::once(self)
                        .chain(entries.iter().flat_map(|e| e.flatten()))
                        .collect(),
                FSEntry::File {name: _, file_type: _} => vec![self]
            }
        }

        pub fn generate_inode_map(&self) -> HashMap<u64, &FSEntry> {
            let mut map: HashMap<u64, &FSEntry> = HashMap::new();
            let flattened_tree = self.flatten();
            for i in 0 as usize..flattened_tree.len() {
                map.insert((i as u64) + 1, flattened_tree[i]);
            }
            map
        }

        fn _walk(&self, component: Component) -> Option<&FSEntry> {
            match (component, self) {
                (Component::Normal(c), FSEntry::Dir { name: _, entries }) =>
                    entries
                        .iter()
                        .find(|e| OsStr::new(e.name()) == c),
                (_, _) => None
            }
        }
    }

    mod raw {
        use super::*;

        #[derive(Debug)]
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
            fn read(&self, offset: i64) -> &[u8] {
                &self.data.as_bytes()[offset as usize..]
            }
        }
    }

    mod file {
        use super::*;

        #[derive(Debug)]
        pub struct LocalFSFileType {
            pub file_path: String
        }

        impl FSFileTypeOps for LocalFSFileType {
            fn read(&self, offset: i64) -> &[u8] {
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
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use super::file::LocalFSFileType;
        use super::raw::RawFSFileType;

        macro_rules! assert_file_name {
            ($entry:expr, $filename:expr) => ({
                let e = $entry;
                if let FSEntry::File { name, file_type: _ } = e {
                    assert_eq!(name, ($filename))
                } else {
                    panic!("Entry is not a FSEntry::File")
                }
            });
        }

        macro_rules! assert_file_local_file_path {
            ($entry:expr, $file_name:expr) => ({
                let (e, f) = ($entry, $file_name);
                if let FSEntry::File { name, file_type: FSFileType::Local(loc) } = e {
                        assert_eq!(loc.file_path, f);

                } else {
                    panic!("Entry is not a FSEntry::File")
                }
            });
        }

        macro_rules! assert_file_raw_data {
            ($entry:expr, $data:expr) => ({
                let (e, f) = ($entry, $data);
                if let FSEntry::File { name, file_type: FSFileType::Raw(loc) } = e {
                        assert_eq!(loc.data, f);

                } else {
                    panic!("Entry is not a FSEntry::File")
                }
            });
        }

        macro_rules! assert_dir_name {
            ($entry:expr, $dirname:expr) => ({
                let e = $entry;
                if let FSEntry::Dir { name, entries: _} = e {
                    assert_eq!(name, ($dirname))
                } else {
                    panic!("Entry is not a FSEntry::Dir")
                }
            });
        }

        fn nested_structure() -> FSEntry {
            FSEntry::Dir {
                name: "".to_string(),
                entries: vec![
                    FSEntry::Dir {
                        name: "bla".to_string(),
                        entries: vec![
                            FSEntry::File {
                                name: "file.txt".to_string(),
                                file_type: FSFileType::Raw(RawFSFileType::new("abc".to_string()))
                            }
                        ]
                    }
                ]
            }
        }

        #[test]
        fn walk_to_root() {
            let structure = FSEntry::Dir {
                name: String::new(),
                entries: vec![
                    FSEntry::File {
                        name: "file.txt".to_string(),
                        file_type: FSFileType::Raw(RawFSFileType::new("abc".to_string()))
                    }
                ]
            };

            let found = structure.walk("/".to_string()).unwrap();

            assert_dir_name!(found, "")
        }

        #[test]
        fn walk_to_file() {
            let structure = FSEntry::Dir {
                name: String::new(),
                entries: vec![
                    FSEntry::File {
                        name: "file.txt".to_string(),
                        file_type: FSFileType::Raw(RawFSFileType::new("abc".to_string()))
                    }
                ]
            };

            let found = structure.walk("/file.txt".to_string()).unwrap();

            assert_file_name!(found, "file.txt")
        }

        #[test]
        fn walk_to_dir() {
            let structure = FSEntry::Dir {
                name: String::new(),
                entries: vec![
                    FSEntry::File {
                        name: "file.txt".to_string(),
                        file_type: FSFileType::Raw(RawFSFileType::new("abc".to_string()))
                    },
                    FSEntry::Dir {
                        name: "bla".to_string(),
                        entries: vec![]
                    }
                ]
            };

            let found = structure.walk("/bla".to_string()).unwrap();

            assert_dir_name!(found, "bla")
        }

        #[test]
        fn walk_to_nested() {
            let fs_tree = nested_structure();

            let found = fs_tree.walk("/bla/file.txt".to_string()).unwrap();

            assert_file_name!(found, "file.txt")
        }

        #[test]
        fn flatten() {
            let fs_tree = nested_structure();

            let found: Vec<&FSEntry> = fs_tree.flatten();

            assert_eq!(found.len(), 3);

            assert_dir_name!(found[0], "");
            assert_dir_name!(found[1], "bla");
            assert_file_name!(found[2], "file.txt");
        }

        #[test]
        fn generate_inode_map() {
            let fs_tree = nested_structure();

            let map: HashMap<u64, &FSEntry> = fs_tree.generate_inode_map();

            assert_eq!(map.len(), 3);

            assert_dir_name!(map.get(&(1 as u64)).unwrap(), "");
            assert_dir_name!(map.get(&(2 as u64)).unwrap(), "bla");
            assert_file_name!(map.get(&(3 as u64)).unwrap(), "file.txt");
        }

        #[test]
        fn load_raw_file_type() {
            let json = r#"
                {
                    "file.txt": "raw:abc"
                }"#;

            let result = FSEntry::new(serde_json::from_str(json).unwrap());
            assert!(result.is_ok());

            let fs_tree = result.unwrap();

            assert_dir_name!(fs_tree.walk("/".to_string()).unwrap(), "");
            assert_file_name!(fs_tree.walk("/file.txt".to_string()).unwrap(), "file.txt");
            assert_file_raw_data!(fs_tree.walk("/file.txt".to_string()).unwrap(), "abc");
        }

        #[test]
        fn load_local_file_type() {
            let json = r#"
                {
                    "file.txt": "file:/my_file.txt"
                }"#;

            let result = FSEntry::new(serde_json::from_str(json).unwrap());
            assert!(result.is_ok());

            let fs_tree = result.unwrap();

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

            let result = FSEntry::new(serde_json::from_str(json).unwrap());
            assert!(result.is_ok());

            let fs_tree = result.unwrap();

            assert_dir_name!(fs_tree.walk("/nested".to_string()).unwrap(), "nested");
            assert_file_name!(fs_tree.walk("/nested/nested.txt".to_string()).unwrap(), "nested.txt");
        }
    }
}