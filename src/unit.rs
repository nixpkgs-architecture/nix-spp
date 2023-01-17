use std::path::PathBuf;
use std::ffi::OsString;
use std::collections::HashSet;
use crate::index::GlobalIndex;

const MAX_SHARD_LENGTH : usize = 2;

// - insert: Takes a path that should be moved into the unit directory,
//   checks that the path doesn't contain any references to outside, and
//   nothing references that path (transitively). Also takes a subpath to use as the pkg-fun.nix
//   file, and optionally a Nix expression to use as the args.nix
// - Provides an iterator over all the unit directories

pub fn check_unit_dir(unit_dir: PathBuf, _global_index: &GlobalIndex) -> HashSet<String> {
    let mut result = HashSet::new();
    for unit_result in unit_dir.read_dir().unwrap() {
        let shard_dir = unit_result.unwrap();
        // Every entry in the root must be a directory
        if ! shard_dir.file_type().unwrap().is_dir() {
            eprintln!("Unit directory entry {:?} is not a directory", shard_dir.file_name());
            std::process::exit(1)
        }

        let mut attributes = shard_dir.path().read_dir().unwrap().peekable();

        // All shard directories must be non-empty
        if attributes.peek().is_none() {
            eprintln!("Shard directory {:?} is empty", shard_dir.file_name());
            std::process::exit(1)
        }

        for attr_result in attributes {
            let entry = attr_result.unwrap();
            let attr = entry.file_name().into_string().unwrap();
            result.insert(attr.clone());

            // All unit directories must be a directory
            if ! entry.path().is_dir() {
                eprintln!("Path {:?}/{:?} is not a directory", shard_dir.file_name(), entry.file_name());
                std::process::exit(1)
            }

            // All unit directories must contain a pkg-fun.nix file
            if ! entry.path().join("pkg-fun.nix").exists() {
                eprintln!("Path {:?}/{:?} doesn't contain a pkg-fun.nix file", shard_dir.file_name(), entry.file_name());
                std::process::exit(1)
            }

            // All unit directories must be in the correct shard directory
            if attr_shard_dir(&attr) != shard_dir.file_name() {
                eprintln!("Shard directory {:?} doesn't match shard entry {:?}", shard_dir.file_name(), entry.file_name());
                std::process::exit(1)
            }

            // Unit directories can only contain a limited set of characters
            for c in attr.chars() {
                if ! (c >= 'a' && c <= 'z'       // lowercase
                      || c >= 'A' && c <= 'Z'    // uppercase
                      || c >= '0' && c <= '9'    // numbers
                      || c == '-' || c == '_'    // - and _
                      ) {
                    eprintln!("Unit directory entry {:?} contains in invalid attribute character {:?}", shard_dir.file_name(), c);
                    std::process::exit(1)
                }
            }
        }
    }
    result
}

pub fn attr_shard_dir(attr: &String) -> OsString {
    let mut padded = attr.to_owned();
    // padded.push_str("----");
    let mut result = String::new();
    let first : String = padded.chars().take(2).collect();
    // let second : String = padded.chars().skip(2).take(2).collect();
    result.push_str(&first);
    // result.push_str("/");
    // result.push_str(&second);
    result.into()
    
}
