use std::path::Path;
use std::collections::HashMap;
use crate::index::GlobalIndex;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Result, Context};

// - insert: Takes a path that should be moved into the unit directory,
//   checks that the path doesn't contain any references to outside, and
//   nothing references that path (transitively). Also takes a subpath to use as the pkg-fun.nix
//   file, and optionally a Nix expression to use as the args.nix
// - Provides an iterator over all the unit directories

pub fn check_unit_dir(nixpkgs_dir: impl AsRef<Path>, global_index: &GlobalIndex) -> Result<()> {
    let unit_dir = nixpkgs_dir.as_ref().join("./pkgs/unit");
    if !unit_dir.exists() {
        eprintln!("Unit directory doesn't exist, skipping check");
        return Ok(())
    } 
    eprintln!("Unit directory is valid");

    let mut result = HashMap::new();
    for unit_result in unit_dir.read_dir()? {
        let shard_dir = unit_result?;
        // Every entry in the root must be a directory
        if !shard_dir.file_type()?.is_dir() {
            anyhow::bail!(
                "Unit directory entry {:?} is not a directory",
                shard_dir.file_name()
            );
        }

        let mut attributes = shard_dir.path().read_dir()?.peekable();

        // All shard directories must be non-empty
        if attributes.peek().is_none() {
            anyhow::bail!("Shard directory {:?} is empty", shard_dir.file_name());
        }

        for attr_result in attributes {
            let entry = attr_result?;
            let attr = entry.file_name().into_string().map_err
                (|o| anyhow::anyhow!("weird filename: {}", o.to_string_lossy()))?;
            let relative_path = PathBuf::from(".").join(entry.path().strip_prefix(&nixpkgs_dir)?.to_owned());
            result.insert(attr.clone(), relative_path);

            // All unit directories must be a directory
            if !entry.path().is_dir() {
                anyhow::bail!(
                    "Path {:?}/{:?} is not a directory",
                    shard_dir.file_name(),
                    entry.file_name()
                );
            }

            // All unit directories must contain a pkg-fun.nix file
            if !entry.path().join("pkg-fun.nix").exists() {
                anyhow::bail!(
                    "Path {:?}/{:?} doesn't contain a pkg-fun.nix file",
                    shard_dir.file_name(),
                    entry.file_name()
                );
            }

            // All unit directories must be in the correct shard directory
            if attr_shard_dir(&attr) != shard_dir.file_name() {
                anyhow::bail!(
                    "Shard directory {:?} doesn't match shard entry {:?}",
                    shard_dir.file_name(),
                    entry.file_name()
                );
            }

            // Unit directories can only contain a limited set of characters
            for c in attr.chars() {
                if !(
                    c >= 'a' && c <= 'z'       // lowercase
                      || c >= 'A' && c <= 'Z'    // uppercase
                      || c >= '0' && c <= '9'    // numbers
                      || c == '-' || c == '_'
                    // - and _
                ) {
                    anyhow::bail!(
                        "Unit directory entry {:?} contains in invalid attribute character {:?}",
                        shard_dir.file_name(),
                        c
                    );
                }
            }
        }
    }

    // global_index.path_indices
    // path like pkgs/unit/he/hello
    //
    //
    // A reference is not just a single reference, but also a reference to all the intermediate
    // directories traversed to get to the final path
    for (attr, unitDir) in result {
        println!("{:?}", unitDir);
        println!("{:?}", global_index.path_indices[&unitDir]);
        for (referenced_by_file, index) in &global_index.path_indices[&unitDir].referenced_by {
            let reference = dbg!(dbg!(&global_index.path_indices[dbg!(referenced_by_file)].references)[*index].clone());
            // Example: Movable ancestor is ./pkgs, then ./pkgs/unit/he/hello
            if ! reference.movable_ancestor.starts_with(&unitDir) {
                anyhow::bail!(
                    "File {:?} is being referenced by {:?}, which crosses the unit directory bound of {:?}",
                    reference.rel_to_root,
                    referenced_by_file,
                    unitDir,
                );
            }
        }

        // 
    }

    std::process::exit(1);

    Ok(())
    // result
}

pub fn attr_shard_dir(attr: &String) -> OsString {
    let str: String = attr.to_lowercase().chars().take(2).collect();
    str.into()
}
