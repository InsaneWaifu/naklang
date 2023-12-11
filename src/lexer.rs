use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Tok {
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
    #[regex("(0[xb])?[0-9_a-fA-F]+", priority = 2)]
    Number,
}
