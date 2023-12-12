use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::Tok;

pub trait Parser<I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O, u32), E>;
}

#[derive(Debug)]
pub struct ParserErr {
    pub gobbled: u32,
    pub expected: String,
}

impl<F, I, O, E> Parser<I, O, E> for F
where
    F: Fn(I) -> Result<(I, O, u32), E>,
{
    fn parse(&self, input: I) -> Result<(I, O, u32), E> {
        self(input)
    }
}

pub struct BoxedParser<'a, I, O, E>(pub Box<dyn Parser<I, O, E> + 'a>);
impl<'a, I, O, E> Parser<I, O, E> for BoxedParser<'a, I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O, u32), E> {
        self.0.parse(input)
    }
}
impl<'a, A, B, C> BoxedParser<'a, A, B, C> {
    fn new(x: impl Parser<A, B, C> + 'a) -> Self {
        BoxedParser(Box::new(x))
    }
}

// combinators
impl<'a, I, O, E> BoxedParser<'a, I, O, E> {
    pub fn map<F, O2>(self, f: F) -> BoxedParser<'a, I, O2, E>
    where
        F: Fn(O) -> O2 + 'a,
        I: 'a,
        O: 'a,
        E: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i);
            sp.map(|(i, o, z)| (i, f(o), z))
        }))
    }

    pub fn chain<O2>(self, ting: impl Parser<I, O2, E> + 'a) -> BoxedParser<'a, I, (O, O2), E>
    where
        I: 'a,
        O: 'a,
        E: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i)?;
            let sp2 = ting.parse(sp.0)?;
            Ok((sp2.0, (sp.1, sp2.1), sp.2 + sp2.2))
        }))
    }
}

pub type Token<'a> = (Tok, logos::Span, &'a str);

pub struct TokenStream<'a>(Vec<Token<'a>>);

impl<'a> TokenStream<'a> {
    pub fn new(mut lex: logos::Lexer<'a, Tok>) -> TokenStream<'a> {
        let mut v = Vec::new();
        while let Some(i) = lex.next() {
            let i = i.unwrap_or(Tok::Err);
            v.push((i, lex.span(), lex.slice()));
        }
        TokenStream(v)
    }

    pub fn slice(&self) -> &[Token<'a>] {
        &self.0
    }
}

pub fn reserved<'a, 'b>(
    tomatch: &'a str,
) -> BoxedParser<'b, &'b [Token<'b>], (logos::Span, &'b str), ParserErr>
where
    'a: 'b,
{
    BoxedParser::new(move |input: &'b [Token<'b>]| match input.first().cloned() {
        Some((Tok::Ident, sp, st)) if st == tomatch => Ok((&input[1..], (sp, st), 1)),
        _ => Err(ParserErr {
            gobbled: 0,
            expected: tomatch.to_string(),
        }),
    })
}

pub fn tok<'a>(
    tomatch: Tok,
) -> BoxedParser<'a, &'a [Token<'a>], (logos::Span, &'a str), ParserErr> {
    BoxedParser::new(move |input: &'a [Token<'a>]| match input.first().cloned() {
        Some((x, sp, st)) if x == tomatch => Ok((&input[1..], (sp, st), 1)),
        _ => Err(ParserErr {
            gobbled: 0,
            expected: format!("{:?}", tomatch),
        }),
    })
}
