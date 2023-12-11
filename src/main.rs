use logos::Logos;
use naklang::{AstNode, Atom, CharStream, InputStream, Parse, Tok, TokStream};

fn main() {
    let tl = Tok::lexer(
        r#"

        ; Numbers to add
        &num1 = !(u32)9
        &num2 = !(u32)11
        &result = add(u32) &num1, &num2
        dbg(u32) &result
      
"#,
    );
    dbg!(tl.clone().collect::<Vec<_>>());
    let mut tokstream = TokStream::new(tl);
    while let Ok(x) = AstNode::parse(&mut tokstream) {
        println!("{:?}", x);
    }
}
