use crate::line_index::LineIndex;
use ignore::{DirEntry, Walk};
use rnix::{Root, SyntaxKind::NODE_PATH};
use rowan::ast::AstNode;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Reference {
    pub line: usize,

    // The most longest ancestor of the referenced path that can be moved
    // around without breaking the reference
    // E.g. if the reference is `./foo`, then this is `./.`, since we can move the current
    // directory without breaking this reference. It can't be `./foo` because moving `./foo` around
    // would break the reference
    // Another example: If the reference is `../bar`, then movable_ancestor is `..`. It's not `./.`
    // because if we moved the current directory around we could break this reference.
    pub movable_ancestor: PathBuf,

    pub rel_to_root: PathBuf,

    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PathIndex {
    pub references: Vec<Reference>,
    pub referenced_by: Vec<(PathBuf, usize)>,
}

impl PathIndex {
    fn new() -> PathIndex {
        PathIndex {
            references: Vec::new(),
            referenced_by: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalIndex {
    // For each Nix file, what paths it references
    pub path_indices: HashMap<PathBuf, PathIndex>,
}

enum Tree {
    Dir(HashMap<String, Tree>),
    File(Vec<Reference>),
}
//////

enum Edge {
    Reference,
    DirEntry(String),
}


// Arena
// https://crates.io/crates/atree
// pkgs/development/libraries/readline/update-patch-set.sh -> pkgs/shells/bash/update-packag-set.sh



// Nodes: Paths
// Edges: Contains (directory listing)
//        References

// Move all files from one directory to another

impl GlobalIndex {
    pub fn new(path: impl AsRef<Path>) -> GlobalIndex {
        std::env::set_current_dir(path).unwrap();
        let subpaths: Vec<_> = Walk::new(".")
            .filter_map(Result::ok)
            .map(DirEntry::into_path)
            .collect();

        let mut path_indices = subpaths
            .iter()
            .map(|p| (p.clone(), PathIndex::new()))
            .collect();

        subpaths
            .iter()
            .filter(|p| !p.is_dir() && p.extension() == Some(OsStr::new("nix")))
            .for_each(|subpath| {

            let contents = read_to_string(&subpath).unwrap();

            let root = match Root::parse(&contents).ok() {
                Ok(root) => root,
                Err(err) => {
                    eprintln!(
                        "Warning: Couldn't parse file {:?}, ignoring it: {}",
                        subpath, err
                    );
                    return;
                }
            };

            let line_index = LineIndex::new(&contents);

            'nodes: for node in root.syntax().descendants() {
                if node.kind() != NODE_PATH {
                    continue 'nodes;
                }
                let text = node.text().to_string();
                let line = line_index.line(node.text_range().start().into());

                // Filters out ./foo/${bar}/baz
                if node.children().count() != 0 {
                    eprintln!("Note: File {:?} on line {:?} contains a path with a subexpressions, ignoring it: {}", subpath, line, text);
                    continue 'nodes;
                }
                // Filters out search paths like <nixpkgs>
                if str::starts_with(&text, "<") {
                    eprintln!("Warning: File {:?} on line {:?} refers to Nix search path, ignoring it: {:?}", subpath, line, text);
                    continue 'nodes;
                }

                let (rel_to_source, movable_ancestor, rel_to_root) = if let Some(resolved) =
                    resolve_reference(&subpath, line, &PathBuf::from(&text), &path_indices)
                {
                    resolved
                } else {
                    continue 'nodes;
                };

                let reference = Reference {
                    line,
                    movable_ancestor,
                    rel_to_root,
                    text,
                };
                let path_index = path_indices.get_mut(&*subpath).unwrap();
                let current_length = path_index.references.len();
                let pointer = (subpath.clone(), current_length);

                // Insert the reference
                path_index.references.push(reference);
                // We can't move the file that contains the reference itself without breaking the
                // reference contained in it
                path_index.referenced_by.push(pointer.clone());

                let mut focused_dir = subpath.parent().unwrap().to_path_buf();
                // The directory of the file is referenced by the file
                path_indices
                    .get_mut(&focused_dir)
                    .unwrap()
                    .referenced_by
                    .push(pointer.clone());
                for component in rel_to_source.components() {
                    match component {
                        Component::CurDir => {}
                        Component::ParentDir => {
                            path_indices
                                .get_mut(&focused_dir)
                                .unwrap()
                                .referenced_by
                                .push(pointer.clone());
                            focused_dir = focused_dir.parent().unwrap().to_path_buf();
                        }
                        Component::Normal(osstr) => {
                            focused_dir = focused_dir.join(osstr).to_path_buf();
                            path_indices
                                .get_mut(&focused_dir)
                                .unwrap()
                                .referenced_by
                                .push(pointer.clone());
                        }
                        _ => panic!("Should not occur!"),
                    }
                }
            }
        });

        GlobalIndex { path_indices }
    }
}

// Absolute project root path
// Source path is where the reference is, relative to project root
// reference is the reference string, any format
pub fn resolve_reference(
    source: &PathBuf,
    line: usize,
    reference: &PathBuf,
    known_files: &HashMap<PathBuf, PathIndex>,
) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let mut rel_to_source = reference.clone();
    let mut movable_ancestor = source.parent().unwrap().to_path_buf();
    let mut rel_to_root = movable_ancestor.clone();
    let mut ascending = true;
    for component in reference.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !ascending {
                    eprintln!("Warning: File {:?} on line {:?} contains a path with an interleaved `..` segment, ignoring it: {:?}", source, line, reference);
                    return None;
                }
                movable_ancestor = match movable_ancestor.parent() {
                    None => {
                        eprintln!("Parent doesn't exist");
                        return None;
                    }
                    Some(parent) => {
                        if !parent.starts_with(".") {
                            eprintln!("Warning: File {:?} on line {:?} refers to a path that escapes the project root, ignoring it: {:?}", source, line, reference);
                            return None;
                        }
                        parent.to_path_buf()
                    }
                };
                rel_to_root = movable_ancestor.clone();
            }
            Component::Normal(segment) => {
                ascending = false;
                rel_to_root = rel_to_root.join(segment);
                if !known_files.contains_key(&rel_to_root) {
                    if rel_to_root.exists() {
                        eprintln!("Warning: File {:?} on line {:?} refers to an ignored path, ignoring it: {:?}", source, line, reference);
                    } else {
                        eprintln!("Warning: File {:?} on line {:?} refers to non-existent path, ignoring it {:?}", source, line, reference);
                    }
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                eprintln!(
                    "Warning: File {:?} on line {:?} refers to absolute path, ignoring it: {:?}",
                    source, line, reference
                );
                return None;
            }
        }
    }

    // This should only be done for the top-level
    if rel_to_root.is_dir() && known_files.contains_key(&rel_to_root.join("default.nix")) {
        rel_to_root = rel_to_root.join("default.nix");
        rel_to_source = rel_to_source.join("default.nix");
    }
    Some((rel_to_source, movable_ancestor, rel_to_root))
}
