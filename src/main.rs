use logos::Logos;
use naklang::{reserved, tok, Parser, Tok, Token, TokenStream};
fn main() {
    /*let tl = Tok::lexer(
            r#"

            ; Numbers to add
            &num1 = !(u32)9
            &num2 = !(u32)11
            &result = add(u32) &num1, &num2
            dbg(u32) &result

    "#,
        );*/
    let tl = Tok::lexer("hello 0x00");
    let ts = TokenStream::new(tl);
    let p = reserved("hello").chain(tok(Tok::Number)).parse(ts.slice());
    dbg!(p);
}
