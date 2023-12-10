use crate::lexer::Tok;
use crate::InputStream;
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use logos::{Lexer, Span};

macro_rules! retok {
    ($x:ident) => {
        if let Some(i) = $x {
            return Ok(i);
        }
    };
}

macro_rules! parse {
    ($name:ident @ {$($p:pat=$v:ident),*} => $b:tt) => {
        $name.optional_next(&mut |x| {
            parse!(x $b {$($p=$v),*});
            false
        })
    };

    ($x:ident $b:tt {$p:pat=$v:ident, $($pt:pat=$vt:ident),*}) => {
        {
            let $v = $x.next();
            if matches!($v, Some(Token($p, ..))) {
                #[allow(unused_variables)]
                let $v = $v.unwrap();
                parse!($x $b {$($pt=$vt),*})
            }
        }
    };

    ($x:ident $b:tt {$p:pat=$v:ident} ) => {
        {
        let $v = $x.next();
        if matches!($v, Some(Token($p, ..))) {
            #[allow(unused_variables)]
            let $v = $v.unwrap();
            {$b}
            return true
        }
    }
    }
}

#[derive(Clone, Debug)]
pub struct Token<'a>(pub Tok, pub Span, pub &'a str);

pub struct TokStream<'a>(Vec<Token<'a>>, usize);

impl<'a> TokStream<'a> {
    pub fn new(mut t: Lexer<'a, Tok>) -> Self {
        let mut v = Vec::new();
        while let Some(Ok(i)) = t.next() {
            v.push(Token(i, t.span().clone(), t.slice()))
        }
        TokStream(v, 0)
    }
}

impl<'a, 'b> InputStream<&'b Token<'a>> for TokStream<'a> {
    fn peek(&self) -> Option<&'b Token<'a>> {
        // SAFETY: I have no idea why rust freaks out without this transmute but it does
        unsafe { core::mem::transmute(self.0.get(self.1)) }
    }
    fn next(&mut self) -> Option<&'b Token<'a>> {
        let c = self.peek()?;
        self.1 += 1;
        Some(c)
    }
    fn optional_next(
        &mut self,
        closure: &mut dyn FnMut(&mut dyn InputStream<&'b Token<'a>>) -> bool,
    ) {
        let oidx = self.1;
        if !closure(self) {
            self.1 = oidx;
        }
    }
}

pub trait Parse<I, O> {
    fn parse(is: &mut dyn InputStream<I>) -> Result<O, String>;
}

#[derive(Debug)]
pub enum Atom<'a> {
    GlobalIdent(Token<'a>),
    LocalIdent(Token<'a>),
}

impl<'a> Parse<&Token<'a>, Atom<'a>> for Atom<'a> {
    fn parse(is: &mut dyn InputStream<&Token<'a>>) -> Result<Atom<'a>, String> {
        let mut gi = None;
        let mut li = None;

        parse!(is @ {Tok::Dollar=dl,Tok::Ident=pi} => {
            gi = Some(Atom::GlobalIdent(pi.clone()));
        });
        retok!(gi);
        parse!(is @ {Tok::Ampersand=amp,Tok::Ident=pi} => {
            li = Some(Atom::LocalIdent(pi.clone()));
        });
        retok!(li);
        Err("Nah".to_owned())
    }
}

pub enum Type<'a> {
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    F32,
    S64,
    U64,
    F64,
    Global(Atom<'a>),
}

pub enum AstNode<'a> {
    Assign(Atom<'a>, Box<AstNode<'a>>),
    JustIdent(Box<AstNode<'a>>),
    Const(Atom<'a>),
    Ptroffset(Type<'a>, Atom<'a>, Atom<'a>),
    Load(Type<'a>, Atom<'a>),
    Store(Type<'a>, Atom<'a>, Atom<'a>),
    Add(Type<'a>, Atom<'a>, Atom<'a>),
    Sub(Type<'a>, Atom<'a>, Atom<'a>),
    Div(Type<'a>, Atom<'a>, Atom<'a>),
    Mul(Type<'a>, Atom<'a>, Atom<'a>),
    Call(Type<'a>, Atom<'a>, Vec<(Type<'a>, Atom<'a>)>),
    Ret(Type<'a>, Atom<'a>),
    Stalloc(Type<'a>, Atom<'a>),
    Dbg(Type<'a>, Atom<'a>),
}
