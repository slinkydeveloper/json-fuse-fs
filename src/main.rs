use std::fs::File;
use std::io::{BufReader, Error};
use std::env;
use serde_json::Value;
use json_fuse_fs::jsonfs::FSEntry;

fn load_json(path: &str) -> Result<Value, Error> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let u: Value = serde_json::from_reader(reader)?;

    Ok(u)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = args.get(1).expect(format!("Usage: {} [json_descriptor]", args[0]).as_str());

    let j = load_json(filename).expect(format!("Cannot load {}", filename).as_str());

    println!("{:?}", j);

    let parsed_fs_tree = FSEntry::new(j);

    println!("{:?}", parsed_fs_tree);
}
