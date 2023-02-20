use ignore::Walk;
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
        // Not needed any more, can do in Nix
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
        let old_dir = value.path.parent().unwrap().to_path_buf();

        while let Some(next) = stack.pop() {
            for reference in reference_index.path_indices.get(&next).unwrap().references.to_owned() {
                // println!("Reference: {:#?}", reference);
                if ! reference.movable_ancestor.starts_with(&old_dir) {
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

        std::env::set_current_dir(&cli.path).unwrap();
        for result in Walk::new(&old_dir) {
            let old = result.unwrap().into_path();
            let old_dir_ref_bys = &reference_index.path_indices.get(&old_dir).unwrap().referenced_by;
            if old.is_dir() {
                continue
            }
            if seen.contains(&old) {
                // println!("Moving {:?} to {:?} because it's being transitively referenced", old, new);
            } else if
                // There can only be one reference from all-packages.nix, a bit hacky
                old_dir_ref_bys.iter().filter(|(path, _)| path == &PathBuf::from("./pkgs/top-level/all-packages.nix")).count() == 1 &&
                // And all the other references must come from the file itself
                old_dir_ref_bys.iter().all(|(path, _)| path == &PathBuf::from("./pkgs/top-level/all-packages.nix") || path == &value.path) &&
                // And the file to be moved must not be referenced from anywhere else
                reference_index.path_indices.get(&old).unwrap().referenced_by.is_empty() {
                eprintln!("For attribute {:?}, only all-packages.nix and its file {:?} may reference the containing directory {:?}, which also contains the file {:?} which is not referenced from anywhere else. Also moving that to the unit directory", key, value.path, old_dir, old);
            } else {
                continue
            }
            let base = old.strip_prefix(&old_dir).unwrap();
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





































