use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::Tok;
pub type Range = core::ops::Range<usize>;

pub trait Parser<I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O, Range), E>;
}

#[derive(Debug)]
pub struct ParserErr {
    pub end_idx: usize,
    pub expected: String,
    pub next: Option<(Tok, String)>,
}

impl<F, I, O, E> Parser<I, O, E> for F
where
    F: Fn(I) -> Result<(I, O, Range), E>,
{
    fn parse(&self, input: I) -> Result<(I, O, Range), E> {
        self(input)
    }
}

impl<I, O, E> Parser<I, O, E> for &dyn Parser<I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O, Range), E> {
        (*self).parse(input)
    }
}

pub struct BoxedParser<'a, I, O, E>(Box<dyn Parser<I, O, E> + 'a>);

impl<'a, I, O, E> BoxedParser<'a, I, O, E> {
    #[allow(clippy::should_implement_trait)]
    pub fn clone(&'a self) -> Self {
        BoxedParser(Box::new(self.0.as_ref()))
    }
}

impl<'a, I, O, E> Parser<I, O, E> for BoxedParser<'a, I, O, E> {
    fn parse(&self, input: I) -> Result<(I, O, Range), E> {
        self.0.parse(input)
    }
}
impl<'a, A, B, C> BoxedParser<'a, A, B, C> {
    fn new(x: impl Parser<A, B, C> + 'a) -> Self {
        BoxedParser(Box::new(x))
    }
}

pub trait SliceHelper<T> {
    fn finished(&self) -> bool;
    fn first(&self) -> &T;
}

impl<T> SliceHelper<T> for &[T] {
    fn finished(&self) -> bool {
        self.is_empty()
    }
    fn first(&self) -> &T {
        &self[0]
    }
}

// combinators
impl<'a, I, O> BoxedParser<'a, I, O, ParserErr> {
    pub fn map<F, O2>(self, f: F) -> BoxedParser<'a, I, O2, ParserErr>
    where
        F: Fn(O) -> O2 + 'a,
        I: 'a,
        O: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i);
            sp.map(|(i, o, z)| (i, f(o), z))
        }))
    }
    pub fn map_range<F, O2>(self, f: F) -> BoxedParser<'a, I, O2, ParserErr>
    where
        F: Fn(O, Range) -> O2 + 'a,
        I: 'a,
        O: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i);
            sp.map(|(i, o, z)| (i, f(o, z.clone()), z))
        }))
    }
    pub fn chain<O2>(
        self,
        ting: impl Parser<I, O2, ParserErr> + 'a,
    ) -> BoxedParser<'a, I, (O, O2), ParserErr>
    where
        I: 'a,
        O: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i)?;
            // TODO map_err to make sure sp2 error gobbled includes what sp parsed
            let sp2 = ting.parse(sp.0)?;
            Ok((sp2.0, (sp.1, sp2.1), sp.2.start..sp2.2.end))
        }))
    }

    pub fn ignore_then<O2>(
        self,
        ting: impl Parser<I, O2, ParserErr> + 'a,
    ) -> BoxedParser<'a, I, O2, ParserErr>
    where
        I: 'a,
        O: 'a,
        O2: 'a,
    {
        self.chain(ting).map(|x| x.1)
    }

    pub fn then_ignore<O2>(
        self,
        ting: impl Parser<I, O2, ParserErr> + 'a,
    ) -> BoxedParser<'a, I, O, ParserErr>
    where
        I: 'a,
        O: 'a,
        O2: 'a,
    {
        self.chain(ting).map(|x| x.0)
    }

    pub fn check<F>(self, check: F) -> BoxedParser<'a, I, O, ParserErr>
    where
        I: 'a,
        O: 'a,
        F: Fn(&O) -> Option<String> + 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i)?;
            let res = check(&sp.1);
            if let Some(res) = res {
                Err(ParserErr {
                    end_idx: sp.2.end,
                    expected: res,
                    next: None,
                })
            } else {
                Ok(sp)
            }
        }))
    }

    pub fn or(self, ting: impl Parser<I, O, ParserErr> + 'a) -> BoxedParser<'a, I, O, ParserErr>
    where
        I: 'a + Clone,
        O: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let selfparse = self.parse(i.clone());
            if selfparse.is_ok() {
                selfparse
            } else {
                ting.parse(i).map_err(|e| {
                    let sp = unsafe { selfparse.unwrap_err_unchecked() };
                    #[allow(clippy::comparison_chain)]
                    if e.end_idx > sp.end_idx {
                        e
                    } else if sp.end_idx > e.end_idx {
                        return sp;
                    } else {
                        return ParserErr {
                            end_idx: e.end_idx,
                            expected: e.expected + ", " + &sp.expected,
                            next: e.next.or(sp.next),
                        };
                    }
                })
            }
        }))
    }

    fn repeated_optsep<O2>(
        self,
        sep: Option<impl Parser<I, O2, ParserErr> + 'a>,
    ) -> BoxedParser<'a, I, Vec<O>, ParserErr>
    where
        I: 'a + Clone,
        O: 'a,
        O2: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let mut rangestart = None;
            let mut rangeend = None;
            let mut input = i;
            let mut matched = false;
            let mut ve = vec![];
            let mut parsed;
            loop {
                parsed = self.parse(input.clone());
                if parsed.is_err() {
                    if !matched {
                        return Err(unsafe { parsed.unwrap_err_unchecked() });
                    }
                    break;
                }
                matched = true;
                let x = parsed.unwrap();
                if rangestart.is_none() {
                    rangestart = Some(x.2.start);
                }
                rangeend = Some(x.2.end);
                input = x.0;
                ve.push(x.1);
                if sep.is_some() {
                    let sp = sep.as_ref().unwrap().parse(input.clone());
                    if sp.is_err() {
                        break;
                    }

                    let sp = unsafe { sp.unwrap_unchecked() };
                    rangeend = Some(sp.2.end);
                    input = sp.0;
                }
            }

            Ok((input, ve, rangestart.unwrap()..rangeend.unwrap()))
        }))
    }

    pub fn repeated_sep<O2>(
        self,
        sep: impl Parser<I, O2, ParserErr> + 'a,
    ) -> BoxedParser<'a, I, Vec<O>, ParserErr>
    where
        I: 'a + Clone,
        O: 'a,
        O2: 'a,
    {
        self.repeated_optsep(Some(sep))
    }

    pub fn repeated(self) -> BoxedParser<'a, I, Vec<O>, ParserErr>
    where
        I: 'a + Clone,
        O: 'a,
    {
        self.repeated_optsep::<()>(None::<BoxedParser<'a, I, (), ParserErr>>)
    }

    pub fn delimited<IDC, IDC2>(
        self,
        a: impl Parser<I, IDC, ParserErr> + 'a,
        b: impl Parser<I, IDC2, ParserErr> + 'a,
    ) -> BoxedParser<'a, I, O, ParserErr>
    where
        I: 'a,
        O: 'a,
        IDC: 'a,
        IDC2: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let ap = a.parse(i);
            if let Ok(ap) = ap {
                let me = self.parse(ap.0)?;
                let bp = b.parse(me.0);
                if let Ok(bp) = bp {
                    Ok((bp.0, me.1, ap.2.start..bp.2.end))
                } else {
                    let bpe = unsafe { bp.unwrap_err_unchecked() };
                    Err(bpe)
                }
            } else {
                unsafe { Err(ap.unwrap_err_unchecked()) }
            }
        }))
    }

    pub fn eoi(self) -> BoxedParser<'a, I, O, ParserErr>
    where
        I: SliceHelper<Token<'a>> + 'a,
        O: 'a,
    {
        BoxedParser(Box::new(move |i: I| {
            let sp = self.parse(i)?;
            if sp.0.finished() {
                Ok(sp)
            } else {
                Err(ParserErr {
                    end_idx: sp.2.end,
                    expected: "EOF".to_owned(),
                    next: Some((sp.0.first().0.clone(), sp.0.first().2.to_owned())),
                })
            }
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

pub fn reserved<'a, 'b>(tomatch: &'a str) -> BoxedParser<'b, &'b [Token<'b>], (), ParserErr>
where
    'a: 'b,
{
    BoxedParser::new(move |input: &'b [Token<'b>]| match input.first().cloned() {
        Some((Tok::Ident, sp, st)) if st == tomatch => Ok((&input[1..], (), sp)),
        _ => Err(ParserErr {
            end_idx: input.first().map_or(0, |x| x.1.start),
            expected: tomatch.to_string(),
            next: if input.first().is_some() {
                Some((
                    input.first().unwrap().0.clone(),
                    input.first().unwrap().2.to_owned(),
                ))
            } else {
                None
            },
        }),
    })
}

pub fn tok<'a>(tomatch: Tok) -> BoxedParser<'a, &'a [Token<'a>], &'a str, ParserErr> {
    BoxedParser::new(move |input: &'a [Token<'a>]| match input.first().cloned() {
        Some((x, sp, st)) if x == tomatch => Ok((&input[1..], st, sp)),
        _ => Err(ParserErr {
            end_idx: input.first().map_or(0, |x| x.1.start),
            expected: format!("{:?}", tomatch),
            next: if input.first().is_some() {
                Some((
                    input.first().unwrap().0.clone(),
                    input.first().unwrap().2.to_owned(),
                ))
            } else {
                None
            },
        }),
    })
}

pub fn match_until<'a, O>(
    p2: BoxedParser<'a, &'a [Token<'a>], O, ParserErr>,
) -> BoxedParser<'a, &'a [Token<'a>], O, ParserErr>
where
    O: 'a,
{
    BoxedParser(Box::new(move |i: &'a [Token]| {
        let mut eidx = 0;
        let mut expected = "?".to_string();
        for j in 0..i.len() {
            eidx = i[j].1.end;
            let pp = p2.parse(&i[j..]);
            if pp.is_ok() {
                return Ok(pp.unwrap());
            } else {
                expected = pp.map(|_| ()).unwrap_err().expected;
            }
        }
        Err(ParserErr {
            end_idx: eidx,
            expected: expected,
            next: None,
        })
    }))
}
