pub trait InputStream<T> {
    fn optional_next(&mut self, closure: &mut dyn FnMut(&mut dyn InputStream<T>) -> bool);
    fn peek(&self) -> Option<T>;
    fn next(&mut self) -> Option<T>;
}

use alloc::string::String;

pub struct CharStream {
    string: String,
    idx: usize,
}
impl CharStream {
    pub fn new(string: String) -> Self {
        assert!(string.is_ascii());
        Self { string, idx: 0 }
    }
}

impl InputStream<char> for CharStream {
    fn optional_next(&mut self, closure: &mut dyn FnMut(&mut dyn InputStream<char>) -> bool) {
        let oldidx = self.idx;
        let b = closure(self);
        if !b {
            self.idx = oldidx;
        }
    }
    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.idx += 1;
        Some(c)
    }
    fn peek(&self) -> Option<char> {
        Some(*self.string.as_bytes().get(self.idx)? as char)
    }
}
