use crate::line_index::LineIndex;
use rowan::ast::AstNode;
use rnix::{
    Root,
    SyntaxKind::NODE_PATH
};
use std::fs::read_to_string;
use std::ffi::OsStr;
use std::path::Component;
use std::collections::HashMap;
use std::path::PathBuf;
use ignore::Walk;

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
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct GlobalIndex {
    // For each Nix file, what paths it references
    pub path_indices: HashMap<PathBuf, PathIndex>,
}



impl GlobalIndex {
    pub fn new(path: &PathBuf) -> GlobalIndex {
        let mut path_indices = HashMap::new();

        std::env::set_current_dir(path).unwrap();
        for result in Walk::new(".") {
            let subpath = result.unwrap().into_path();
            path_indices.insert(subpath, PathIndex::new());
        }

        for result in Walk::new(".") {
            let subpath = result.unwrap().into_path();
            if subpath.is_dir() {
                continue
            }

            if subpath.extension() != Some(OsStr::new("nix")) {
                continue
            }

            let contents = read_to_string(&subpath).unwrap();
            
            let root = match Root::parse(&contents).ok() {
                Ok(root) => root,
                Err(err) => {
                    eprintln!("Warning: Couldn't parse file {:?}, ignoring it: {}", subpath, err);
                    continue
                },
            };

            let line_index = LineIndex::new(&contents);

            'nodes: for node in root.syntax().descendants() {
                if node.kind() != NODE_PATH {
                    continue 'nodes
                }
                let text = node.text().to_string();
                let line = line_index.line(node.text_range().start().into());

                if node.children().count() != 0 {
                    eprintln!("Note: File {:?} on line {:?} contains a path with a subexpressions, ignoring it: {}", subpath, line, text);
                    continue 'nodes
                }
                if str::starts_with(&text, "<") {
                    eprintln!("Warning: File {:?} on line {:?} refers to Nix search path, ignoring it: {:?}", subpath, line, text);
                    continue 'nodes
                }

                let mut node_path = PathBuf::from(&text);

                let mut movable_ancestor = subpath.parent().unwrap().to_path_buf();
                let mut referenced_path = movable_ancestor.clone();
                let mut ascending = true;

                for component in node_path.components() {
                    match component {
                        Component::CurDir => {}
                        Component::ParentDir => {
                            if ! ascending {
                                eprintln!("Warning: File {:?} on line {:?} contains a path with an interleaved `..` segment, ignoring it: {:?}", subpath, line, text);
                                continue 'nodes;
                            }
                            movable_ancestor = match movable_ancestor.parent() {
                                None => {
                                    eprintln!("Parent doesn't exist");
                                    continue 'nodes;
                                },
                                Some(parent) => {
                                    if ! parent.starts_with(".") {
                                        eprintln!("Warning: File {:?} on line {:?} refers to a path that escapes the project root, ignoring it: {:?}", subpath, line, text);
                                        continue 'nodes;
                                    }
                                    parent.to_path_buf()
                                },

                            };
                            referenced_path = movable_ancestor.clone();
                        }
                        Component::Normal(segment) => {
                            ascending = false;
                            referenced_path = referenced_path.join(segment);
                            if ! path_indices.contains_key(&referenced_path) {
                                if referenced_path.exists() {
                                    eprintln!("Warning: File {:?} on line {:?} refers to an ignored path, ignoring it: {:?}", subpath, line, text);
                                } else {
                                    eprintln!("Warning: File {:?} on line {:?} refers to non-existent path, ignoring it {:?}", subpath, line, text);
                                }
                                continue 'nodes;
                            }
                        }
                        Component::RootDir | Component::Prefix(_) => {
                            eprintln!("Warning: File {:?} on line {:?} refers to absolute path, ignoring it: {:?}", subpath, line, text);
                            continue 'nodes;
                        }
                    }
                }

                if referenced_path.join("default.nix").exists() {
                    node_path = node_path.join("default.nix");
                }

                let reference = Reference { line, movable_ancestor };
                let path_index = path_indices.get_mut(&subpath).unwrap();
                let current_length = path_index.references.len();
                let pointer = (subpath.clone(), current_length);

                // Insert the reference
                path_index.references.push(reference);
                // We can't move the file that contains the reference itself without breaking the
                // reference contained in it
                path_index.referenced_by.push(pointer.clone());

                let mut focused_dir = subpath.parent().unwrap().to_path_buf();
                for component in node_path.components() {
                    match component {
                        Component::CurDir => {}
                        Component::ParentDir => {
                            path_indices.get_mut(&focused_dir).unwrap().referenced_by.push(pointer.clone());
                            focused_dir = focused_dir.parent().unwrap().to_path_buf();
                        }
                        Component::Normal(osstr) => {
                            focused_dir = focused_dir.join(osstr).to_path_buf(); 
                            path_indices.get_mut(&focused_dir).unwrap().referenced_by.push(pointer.clone());
                        }
                        _ => panic!("Should not occur!"),
                    }
                }

            }

        }

        GlobalIndex { path_indices }
    }
}
