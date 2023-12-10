use logos::Logos;
use naklang::{Atom, CharStream, InputStream, Parse, Tok, TokStream};

fn main() {
    let tl = Tok::lexer("&result, $fn");
    let mut tokstream = TokStream::new(tl);
    let c: Result<Atom, String> = Atom::parse(&mut tokstream);
    let _ = dbg!(c);
    let a = tokstream.next().unwrap().0;
    println!("token {:?}", a);
    assert!(a == Tok::Comma);
    let c: Result<Atom, String> = Atom::parse(&mut tokstream);
    let _ = dbg!(c);
}
