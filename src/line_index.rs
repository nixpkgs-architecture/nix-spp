// A simple utility for calculating the line for a string offset
pub struct LineIndex {
    newlines: Vec<usize>,
}

impl LineIndex {
    pub fn new(s: &String) -> LineIndex {
        let mut newlines = vec![];
        let mut index = 0;
        for split in s.split_inclusive("\n") {
            index += split.len();
            newlines.push(index);
        }
        LineIndex { newlines }
    }

    pub fn line(&self, index: usize) -> usize {
        match self.newlines.binary_search(&index) {
            Ok(x) => x + 1,
            Err(x) => x + 1,
        }
    }
}
