use crate::lexer::Tok;
use crate::InputStream;
use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};
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
      {

        $name.optional_next(&mut |x| {
            parse!($name x $b {$($p=$v),*});
            false
        })}
    };

    ($name:ident $x:ident $b:tt {$p:pat=$v:ident, $($pt:pat=$vt:ident),*}) => {
        {
            let $v = $x.next();
            if matches!($v, Some($p)) {
                #[allow(unused_variables)]
                let $v = $v.unwrap();
                parse!($name $x $b {$($pt=$vt),*})
            }
        }
    };

    ($name:ident $x:ident $b:tt {$p:pat=$v:ident} ) => {
        {
        let $v = $x.next();
        #[allow(unused_variables)]
        let $name = $x;
        if matches!($v, Some($p)) {
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

#[derive(Debug, Clone)]
pub enum Atom<'a> {
    GlobalIdent(Token<'a>),
    LocalIdent(Token<'a>),
    Const(Token<'a>, Token<'a>),
    CPtroffset(Token<'a>, Box<Atom<'a>>),
    SPtroffset(Token<'a>, Vec<Token<'a>>),
}

macro_rules! parseinto {
    ($is:ident, $id:ident @ $($x:tt)*) => {
      let mut $id = None;
      parse!($is @ $($x)*);
      retok!($id);
    };
}

impl<'a> Parse<&Token<'a>, Atom<'a>> for Atom<'a> {
    fn parse(is: &mut dyn InputStream<&Token<'a>>) -> Result<Atom<'a>, String> {
        parseinto!(is, gi @ {Token(Tok::Dollar,..)=dl,Token(Tok::Ident,..)=pi} => {
            gi = Some(Atom::GlobalIdent(pi.clone()));
        });
        parseinto!(is, li @ {Token(Tok::Ampersand,..)=amp,Token(Tok::Ident,..)=pi} => {
            li = Some(Atom::LocalIdent(pi.clone()));
        });
        parseinto!(is, co @ {Token(Tok::Bang,..)=_b,Token(Tok::OpenBracket,..)=_o,Token(Tok::Ident,..)=ty,Token(Tok::CloseBracket,..)=_c,Token(Tok::Number,..)=num} => {
          co = Some(Atom::Const(ty.clone(), num.clone()));
        });
        parseinto!(is, cpt @ {
          Token(Tok::Ident, _, "cptroffset")=_c,
          Token(Tok::Ident, ..)=ty,
          Token(Tok::Comma, ..)=_cc
        } => {
          let num = Atom::parse(is);
          if ! matches!(num, Ok(Atom::Const(..))) {
            return false;
          }
          cpt = Some(Atom::CPtroffset(ty.clone(), Box::new(num.unwrap())));
        });
        parseinto!(is, spt @ {
          Token(Tok::Ident, _, "sptroffset")=_s,
          Token(Tok::Ident, ..)=ty,
          Token(Tok::Comma, ..)=_cc
        } => {
          let mut v = Vec::new();
          let mut need_dot = false;
          while let Some(x) = is.peek() {
            match x.0 {
                Tok::Ident if !need_dot => {
                    need_dot = true;
                    v.push(is.next().unwrap().clone())
                },
                Tok::Dot if need_dot => {
                    is.next().unwrap();
                    need_dot = false;
                },
                _ => {
                    if need_dot {
                        break
                    } else {
                        return false
                    }
                },
            };
          }
          spt = Some(Atom::SPtroffset(ty.clone(), v))
        });
        Err("Nah".to_owned())
    }
}

impl Atom<'_> {
    fn is_var(&self) -> bool {
        matches!(self, Atom::LocalIdent(_) | Atom::GlobalIdent(_))
    }
}

#[derive(Debug)]
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
impl<'a> Parse<&Token<'a>, Type<'a>> for Type<'a> {
    fn parse(is: &mut dyn InputStream<&Token<'a>>) -> Result<Type<'a>, String> {
        macro_rules! typ {
            ($t:expr) => {{
                is.next();
                Ok($t)
            }};
        }
        if is.peek().is_none() {
            return Err("EOF".to_owned());
        }
        match is.peek().unwrap().2 {
            "s8" => typ!(Type::S8),
            "u8" => typ!(Type::U8),
            "s16" => typ!(Type::S16),
            "u16" => typ!(Type::U16),
            "s32" => typ!(Type::S32),
            "u32" => typ!(Type::U32),
            "f32" => typ!(Type::F32),
            "s64" => typ!(Type::S64),
            "u64" => typ!(Type::U64),
            "f64" => typ!(Type::F64),
            _ => {
                let atom = Atom::parse(is);
                if let Ok(Atom::GlobalIdent(x)) = atom {
                    Ok(Type::Global(Atom::GlobalIdent(x)))
                } else {
                    Err("No".to_owned())
                }
            }
        }
    }
}

#[derive(Debug)]
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

impl AstNode<'_> {
    fn is_assignable(&self) -> bool {
        use AstNode as A;
        match self {
            A::JustIdent(..) => true,
            A::Const(..) => true,
            A::Ptroffset(..) => true,
            A::Load(..) => true,
            A::Store(..) => false,
            A::Add(..) => true,
            A::Sub(..) => true,
            A::Div(..) => true,
            A::Mul(..) => true,
            A::Call(..) => true,
            A::Ret(..) => false,
            A::Stalloc(..) => true,
            A::Dbg(..) => false,
            A::Assign(..) => false,
        }
    }
}

macro_rules! match_token {
    ($t:ident $x:ident $e:block) => {
        if let Some(Token(Tok::$t, ..)) = $x.next() {
            $e
        }
    };
}

macro_rules! match_reserved {
    ($t:ident $x:ident $e:block) => {
        if let Some(Token(Tok::Ident, _, stringify!($t))) = $x.next() {
            $e
        }
    };
}

macro_rules! binary_op {
    ($is:ident $name:ident $t:ident $e:ident) => {
        let mut $name = None;
        $is.optional_next(&mut |x| {
            match_reserved!($t x {
                match_token!(OpenBracket x {
                    let ty = Type::parse(x);
                    match_token!(CloseBracket x {
                        if let Ok(atom1) = Atom::parse(x) {
                            match_token!(Comma x {
                                if let Ok(atom2) = Atom::parse(x) {
                                    $name = Some(AstNode::$e(ty.unwrap(), atom1, atom2));
                                    return true;
                                }
                            })
                        }
                    })
                })
            });
            false
        });
        retok!($name);
    };
}

impl<'a> Parse<&Token<'a>, AstNode<'a>> for AstNode<'a> {
    fn parse(is: &mut dyn InputStream<&Token<'a>>) -> Result<AstNode<'a>, String> {
        let mut assign = None;
        is.optional_next(&mut |x| {
            let atom = Atom::parse(x);
            if let Ok(i) = atom {
                println!("assing got first2 atom");
                if i.is_var() {
                    match_token!(Equals x {
                        if let Ok(rhs) = AstNode::parse(x) {
                            if rhs.is_assignable() {
                                assign = Some(AstNode::Assign(i, Box::new(rhs)));
                                return true;
                            }
                        }
                    })
                }
            };
            false
        });
        retok!(assign);
        let mut stalloc = None;
        is.optional_next(&mut |x| {
            match_reserved!(stalloc x {
                if let Ok(t) = Type::parse(x) {
                    match_reserved!(times x {
                        if let Ok(a) = Atom::parse(x) {
                            stalloc = Some(AstNode::Stalloc(t, a));
                            return true;
                        }
                    })
                }
            });
            false
        });
        retok!(stalloc);
        let mut ptroffset = None;
        is.optional_next(&mut |x| {
            match_reserved!(ptroffset x {
                match_token!(OpenBracket x {
                    if let Ok(ty) = Type::parse(x) {
                        match_token!(CloseBracket x {
                            match_reserved!(ptr x {
                                if let Ok(originalptr) = Atom::parse(x) {
                                    if let Ok(offset) = Atom::parse(x) {
                                        ptroffset = Some(AstNode::Ptroffset(ty, originalptr, offset));
                                        return true;
                                    }
                                }
                            })
                        })
                    }
                })
            });
            false
        });
        retok!(ptroffset);
        binary_op!(is add add Add);
        binary_op!(is sub sub Sub);
        binary_op!(is mul mul Mul);
        binary_op!(is div div Div);
        let mut load = None;
        is.optional_next(&mut |x| {
            match_reserved!(load x {
                match_token!(OpenBracket x {
                    if let Ok(ty) = Type::parse(x) {
                        match_token!(CloseBracket x {
                            match_reserved!(ptr x {
                                if let Ok(atom) = Atom::parse(x) {
                                    load = Some(AstNode::Load(ty, atom));
                                    return true;
                                }
                            })

                        })
                    }
                })
            });
            false
        });
        retok!(load);
        let mut store = None;
        is.optional_next(&mut |x| {
            match_reserved!(store x {
                match_token!(OpenBracket x {
                    if let Ok(ty) = Type::parse(x) {
                        match_token!(CloseBracket x {
                            match_reserved!(ptr x {
                                if let Ok(atom1) = Atom::parse(x) {
                                    match_token!(Comma x {
                                        if let Ok(atom2) = Atom::parse(x) {
                                            store = Some(AstNode::Store(ty, atom1, atom2));
                                            return true;
                                        }
                                    })
                                }
                            })
                        })
                    }
                })
            });
            false
        });
        retok!(store);
        let mut dbg = None;
        is.optional_next(&mut |x| {
            match_reserved!(dbg x {
                match_token!(OpenBracket x {
                    if let Ok(ty) = Type::parse(x) {
                        match_token!(CloseBracket x {
                            if let Ok(atom) = Atom::parse(x) {
                                dbg = Some(AstNode::Dbg(ty, atom));
                                return true;
                            }
                        })
                    }
                })
            });
            false
        });

        Err("Nah".to_owned())
    }
}
