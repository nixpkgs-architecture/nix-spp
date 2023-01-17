use crate::unit::attr_shard_dir;
use std::path::PathBuf;
use std::collections::HashSet;
use crate::all_packages::AllPackages;
mod args;
mod index;
mod unit;
use index::GlobalIndex;
use args::Args;
use clap::Parser;
use unit::check_unit_dir;

mod all_packages;
mod line_index;

fn main() {

    let cli = Args::parse();

    let reference_index = GlobalIndex::new(&cli.path);

    let unit_dir = cli.path.join("pkgs/unit");
    if ! unit_dir.exists() {
        eprintln!("Unit directory doesn't exist, skipping check");
    } else {
        check_unit_dir(unit_dir, &reference_index);
        eprintln!("Unit directory is valid");
    }

    // println!("{:#?}", reference_index);

    // Function that parses all-packages.nix, returning a struct for every identifier assignment
    // that could be migrated, without looking at the file references

    let mut ap = AllPackages::new(&cli.path.join("pkgs/top-level/all-packages.nix"), &reference_index);

    'attrs: for (key, value) in ap.entries.to_owned() {
        // if ! key.starts_with("zarc") {
        //     continue
        // }
        //let mut movable_ancestor : PathBuf = value.path.clone();
        let mut stack = vec![value.path.clone()];
        let mut seen : HashSet<PathBuf> = HashSet::new();
        seen.insert(value.path.clone());
        let old_dir = value.path.parent().unwrap();

        while let Some(next) = stack.pop() {
            for reference in reference_index.path_indices.get(&next).unwrap().references.to_owned() {
                // println!("Reference: {:#?}", reference);
                if ! reference.movable_ancestor.starts_with(old_dir) {
                    eprintln!("Cannot move attribute {:?} pointing to file {:?}, because it transitively references file {:?} which in line {:?} contains a path reference {:?} which would break", key, value.path, next, reference.line, reference.text);
                    continue 'attrs;
                }
                if seen.insert(reference.rel_to_root.clone()) {
                    stack.push(reference.rel_to_root);
                }
            }
        }

        for file in seen.clone() {
            for (referenced_by, index) in reference_index.path_indices.get(&file).unwrap().referenced_by.to_owned() {
                let reference = reference_index.path_indices.get(&referenced_by).unwrap().references[index].clone();
                if referenced_by == PathBuf::from("./pkgs/top-level/all-packages.nix") && reference.line == value.line {
                    // println!("Attribute {:?} pointing to file {:?} is referenced by another file {:?} on line {:?}", key, value.path, referenced_by, reference.line);
                    continue
                } else if seen.contains(&referenced_by) {
                    // println!("Attribute {:?} pointing to file {:?} is referenced by another file {:?} on line {:?}", key, value.path, referenced_by, reference.line);
                    continue
                } else {
                    eprintln!("Cannot move attribute {:?} pointing to file {:?}, because one of its transitively referenced files {:?} is referenced by file {:?} on line {:?}", key, value.path, file, referenced_by, reference.line);
                    continue 'attrs;
                }
            }
        }

        let shard_dir = attr_shard_dir(&key);
        let unit_dir = cli.path.join("pkgs/unit").join(shard_dir).join(&key);
        std::fs::create_dir_all(&unit_dir).unwrap();
        // println!("Moving attribute {:?} pointing to file {:?} to unit directory {:?}", key, value.path, unit_dir);

        for old in seen {
            let base = old.strip_prefix(old_dir).unwrap();
            let mut new = unit_dir.join(base);
            if old == value.path {
                new.pop();
                new.push("pkg-fun.nix");
            }
            std::fs::create_dir_all(new.parent().unwrap()).unwrap();
            // println!("Moving {:?} to {:?}", old, new);
            std::fs::rename(&old, new).unwrap();
        }

        ap.remove(&key);
    }

    ap.render();

}





































