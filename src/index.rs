use std::path::PathBuf;
use ignore::Walk;

pub struct ReferenceIndex {
    pub root_path: PathBuf,
}

impl ReferenceIndex {
    pub fn new(path: &PathBuf) -> ReferenceIndex {
        // for result in Walk::new(path) {
        //     println!("{:?}", result);
        // }

        ReferenceIndex { root_path: path.clone() }
    }
}
