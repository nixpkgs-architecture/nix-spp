use std::io::Write;
use std::fs::File;
use rowan::GreenToken;
use rnix::SyntaxKind;
use rnix::NodeOrToken::Token;
use std::cmp::Reverse;
use rnix::SyntaxNode;
use rowan::api::Language;
use rnix::NixLanguage;
use rnix::ast::AstToken;
use rowan::ast::AstNode;
use rnix::ast::{Attr, Expr, AttrSet, HasEntry, InterpolPart};
use rnix::Root;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;
use crate::line_index::LineIndex;

#[derive(Debug, Clone)]
pub struct Entry {
    pub index: usize,
    pub line: usize,
    pub path: String,
}

#[derive(Debug)]
pub struct AllPackages {
    path: PathBuf,
    syntax_node: SyntaxNode,
    attributes_to_remove: Vec<String>,
    pub entries: HashMap<String, Entry>,
}

impl AllPackages {
    pub fn new(path: &PathBuf) -> AllPackages {
        let contents = read_to_string(path).unwrap();
        let line_index = LineIndex::new(&contents);
        let mut entries = HashMap::new();

        let root = match Root::parse(&contents).ok() {
            Ok(root) => root,
            Err(err) => {
                eprintln!("Couldn't parse all-packages.nix file {:?}: {:?}", path, err);
                std::process::exit(1);
            },
        };


        let attribute_set = resulting_attrs(root.expr().unwrap()).unwrap();

        for attribute_definition in attribute_set.attrpath_values() {
            let line = line_index.line(attribute_definition.syntax().text_range().start().into());
            let index = attribute_definition.syntax().index();
            let attribute = {
                let attribute_path = attribute_definition.attrpath().unwrap();
                let mut iterator = attribute_path.attrs();
                let first = iterator.next().unwrap();
                if let Some(_) = iterator.next() {
                    eprintln!("Warning: all-packages.nix attribute {:?} defined on line {:?} is an attribute path, ignoring", attribute_path.syntax().to_string(), line);
                    continue;
                }
                match first {
                    Attr::Ident(it) => it.ident_token().unwrap().text().to_string(),
                    _ => {
                        eprintln!("Warning: all-packages.nix attribute {:?} defined on line {:?} is not an identifier, ignoring", attribute_path.syntax().to_string(), line);
                        continue;
                    }
                }
            };

            match &unwrap_apply_chain(attribute_definition.value().unwrap())[..] {
                [Expr::Ident(ident_expr), Expr::Path(path_expr), Expr::AttrSet(args_expr)] => {
                    let ident_text = ident_expr.ident_token().unwrap().text().to_string();
                    if ident_text != "callPackage" {
                        continue;
                    }
                    let path = {
                        let mut iterator = path_expr.parts();
                        let part = iterator.next().unwrap();
                        if let Some(_) = iterator.next() {
                            continue;
                        }
                        match part {
                            InterpolPart::Literal(path) => path.syntax().text().to_string(),
                            _ => continue
                        }
                    };

                    if let Some(_) = args_expr.entries().next() {
                        continue;
                    }

                    entries.insert(attribute, Entry { index, line, path })
                },
                _ => continue,
            };

        }

        AllPackages {
            path: path.to_owned(),
            syntax_node: attribute_set.syntax().to_owned(),
            attributes_to_remove: vec![],
            entries,
        }
    }


    pub fn remove(&mut self, attribute: &String) -> bool {
        if let Some(_) = self.entries.get(attribute) {
            self.attributes_to_remove.push(attribute.to_owned());
            // self.entries.remove(attribute);
            true
        } else {
            false
        }
    }

    pub fn render(&self) {
        let mut green = self.syntax_node.green().into_owned();
        let mut sorted_indices_to_remove : Vec<(usize, String)> = vec![];
        for attr in self.attributes_to_remove.iter() {
            sorted_indices_to_remove.push((self.entries.get(attr).unwrap().index, attr.to_owned()));
        }
        sorted_indices_to_remove.sort_by_key(|(index, _)| Reverse(*index));
        for (index, attr) in sorted_indices_to_remove.iter() {

            let mut potential_comment = false;
            // Go back until the previous node is found
            //
            let mut previous_offset = 1;
            while let Some(Token(previous)) = green.children().nth(index - previous_offset) {
                match NixLanguage::kind_from_raw(previous.kind()) {
                    SyntaxKind::TOKEN_WHITESPACE => {
                        if previous.text().contains("\n") {
                            break
                        } else {
                            previous_offset += 1;
                        }
                    },
                    SyntaxKind::TOKEN_COMMENT => {
                        potential_comment = true;
                        previous_offset += 1;
                    },
                    _ => break
                }
            }
            let mut next_offset = 1;
            while let Some(Token(next)) = green.children().nth(index + next_offset) {
                match NixLanguage::kind_from_raw(next.kind()) {
                    SyntaxKind::TOKEN_WHITESPACE => {
                        if next.text().contains("\n") {
                            break
                        } else {
                            next_offset += 1;
                        }
                    },
                    SyntaxKind::TOKEN_COMMENT => {
                        potential_comment = true;
                        next_offset += 1;
                    },
                    _ => break
                }
            }

            if potential_comment {
                green = green.replace_child(*index, Token(GreenToken::new(NixLanguage::kind_to_raw(SyntaxKind::TOKEN_COMMENT), &format!("/* {} = <moved> */", attr))));
                continue;
            }
            if let Some(Token(previous)) = green.children().nth(index - previous_offset) {
                if NixLanguage::kind_from_raw(previous.kind()) == SyntaxKind::TOKEN_WHITESPACE {
                    if let Some(Token(next)) = green.children().nth(index + next_offset) {
                        if NixLanguage::kind_from_raw(next.kind()) == SyntaxKind::TOKEN_WHITESPACE {
                        // if Language::kind_from_raw(next.kind()) == SyntaxKind::TOKEN_WHITESPACE {
                            let mut prev_iter = previous.text().chars().rev().peekable();
                            let mut prev_count = 0;
                            // Remove leading spaces
                            while prev_iter.peek() == Some(&' ') {
                                prev_iter.next();
                            }
                            while prev_iter.peek() == Some(&'\n') {
                                prev_iter.next();
                                prev_count += 1;
                            }
                            let mut next_iter = next.text().chars().peekable();
                            let mut next_count = 0;
                            // Remove trailing spaces (shouldn't be needed, there should be a
                            // newline)
                            while next_iter.peek() == Some(&' ') {
                                next_iter.next();
                            }
                            while next_iter.peek() == Some(&'\n') {
                                next_iter.next();
                                next_count += 1;
                            }

                            let mut new : String = prev_iter.rev().collect();
                            new += &"\n".repeat(prev_count.max(next_count));

                            let x : String = next_iter.collect();
                            new += &x;

                            green = green.splice_children(index - previous_offset ..= index + next_offset, [Token(GreenToken::new(previous.kind(), &new))]);
                            continue
                        }
                    }
                }
            }
            eprintln!("Couldn't properly strip space around {:?}", green.children().nth(*index).unwrap().to_string());
            green = green.remove_child(*index);
        }
        let mut file = File::create(&self.path).unwrap();
        file.write(&self.syntax_node.replace_with(green).to_string().into_bytes()).unwrap();
        // println!("{:#?}", SyntaxNode::new_root(self.syntax_node.replace_with(green)));
    }

}

fn unwrap_apply_chain(expr: Expr) -> Vec<Expr> {
    match expr {
        Expr::Apply(it) => {
            let mut x = unwrap_apply_chain(it.lambda().unwrap());
            x.push(it.argument().unwrap());
            x
        },
        other => vec![other],
    }
}

fn resulting_attrs(expr: Expr) -> Option<AttrSet> {
    match expr {
        Expr::Lambda(it) => resulting_attrs(it.body()?),
        Expr::With(it) => resulting_attrs(it.body()?),
        Expr::AttrSet(it) => Some(it),
        _ => None
    }
}
