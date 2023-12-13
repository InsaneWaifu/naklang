use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
pub enum Tok {
    #[regex(r"[ \t\n\f]*;.*\n?", logos::skip)]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Err,
    #[token("$")]
    Dollar,
    #[token("&")]
    Ampersand,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("!")]
    Bang,
    #[regex("[_a-zA-Z][_a-zA-Z0-9]*")]
    Ident,
    #[regex("\\(")]
    OpenBracket,
    #[regex("\\)")]
    CloseBracket,
    #[regex("(0x[a-fA-F0-9]+)|(0b[10]+)|([0-9]+)+")]
    Number,
    #[regex("=")]
    Equals,
}
