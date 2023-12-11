//#![no_std]
extern crate alloc;
mod inputstream;
mod lexer;
mod parser;

pub use inputstream::*;
pub use lexer::Tok;
pub use parser::*;

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned;

    use crate::InputStream;

    #[test]
    fn charstream() {
        use crate::CharStream;
        let str = "Hello World!".to_owned();
        let mut cs = CharStream::new(str);
        assert!(cs.next().unwrap() == 'H');
        assert!(cs.peek().unwrap() == 'e');
        let mut rest = None;
        cs.optional_next(&mut |x| {
            rest = x.next();
            while x.next().is_some() {}
            false
        });
        assert!(cs.next().unwrap() == 'e');
        assert!(cs.next().unwrap() == 'l');
    }
}
