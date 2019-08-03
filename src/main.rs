#[macro_use] extern crate log;

mod fs;

use std::fs::File;
use std::io::{BufReader, Error};
use std::env;
use serde_json::Value;
use json_fuse_fs::{FSEntry, FSNode};
use std::ffi::{OsStr, OsString};
use fs::JsonFS;

fn load_json(path: &str) -> Result<Value, Error> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let u: Value = serde_json::from_reader(reader)?;

    Ok(u)
}

fn main() {
    env_logger::init();

    let args: Vec<OsString> = env::args_os().collect();
    let executable_name = args[0].to_str().unwrap();

    if let (Some(filename), Some(mountpoint)) = (args.get(1).and_then(|s| s.to_str()), args.get(2)) {
        let j = load_json(filename).expect(format!("Cannot load {}", filename).as_str());

        let (parsed_fs_tree, inode_map) = FSNode::new(j).unwrap();

        info!("Parsed FS Tree: {:?}", parsed_fs_tree);

        let fs = JsonFS::new(parsed_fs_tree, inode_map);

        let options = ["-o", "ro", "-o", "fsname=jsonfs"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();

        fuse::mount(fs, mountpoint, &options).unwrap();
    } else {
        panic!("Usage: {} [json_descriptor] [mountpoint]", executable_name)
    }

}
