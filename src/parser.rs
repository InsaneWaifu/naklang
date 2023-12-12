use crate::Tok;

pub trait Parser<I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O), E>;
    
}

impl<F, I, O, E> Parser<I, O, E> for F
where
    F: Fn(I) -> Result<(I, O), E>,
{
    fn parse(&self, input: I) -> Result<(I, O), E> {
        self(input)
    }
}

pub type Token<'a> = (Tok, logos::Span, &'a str);

pub fn lit<'a, 'b>(tomatch: &'a str) -> impl Parser<&'b [Token<'b>], (), ()>
where
    'a: 'b,
{
    move |input: &'b [Token<'b>]| match input.get(0) {
        Some((Tok::Ident, _, ref st)) if st == &tomatch => Ok((&input[1..], ())),
        _ => Err(()),
    }
}
