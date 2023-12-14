use logos::Logos;
use naklang::{reserved, tok, BoxedParser, Parser, ParserErr, Range, Tok, Token, TokenStream};

#[derive(Debug)]
pub enum TypeSize {
    _8,
    _16,
    _32,
    _64,
}

#[derive(Debug)]
pub enum Type<'a> {
    I(TypeSize),
    U(TypeSize),
    F(TypeSize),
    Ref(&'a str),
    Unresolved(&'a str),
}

#[derive(Debug)]
pub enum AstNode<'a> {
    Err(ParserErr, Range),
    Local(&'a str, Range),
    Global(&'a str, Range),
    Const(Type<'a>, &'a str, Range),
    CPtrOffset(Type<'a>, &'a str, Range),
    SPtrOffset(Type<'a>, Vec<&'a str>, Range),
    Cpy(Box<AstNode<'a>>, Range),
    Add(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Sub(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Div(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Mul(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Call(
        Type<'a>,
        Box<AstNode<'a>>,
        Vec<(Type<'a>, AstNode<'a>)>,
        Range,
    ),
    Ret(Type<'a>, Box<AstNode<'a>>, Range),
    Stalloc(Type<'a>, Box<AstNode<'a>>, Range),
    Ptroffset(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Load(Type<'a>, Box<AstNode<'a>>, Range),
    Store(Type<'a>, Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
    Dbg(Type<'a>, Box<AstNode<'a>>, Range),
    Equals(Box<AstNode<'a>>, Box<AstNode<'a>>, Range),
}

impl AstNode<'_> {
    pub fn span(&self) -> Range {
        match self {
            AstNode::Local(_, r) => r.clone(),
            AstNode::Global(_, r) => r.clone(),
            AstNode::Const(_, _, r) => r.clone(),
            AstNode::CPtrOffset(_, _, r) => r.clone(),
            AstNode::SPtrOffset(_, _, r) => r.clone(),
            AstNode::Cpy(_, r) => r.clone(),
            AstNode::Add(_, _, _, r) => r.clone(),
            AstNode::Sub(_, _, _, r) => r.clone(),
            AstNode::Div(_, _, _, r) => r.clone(),
            AstNode::Mul(_, _, _, r) => r.clone(),
            AstNode::Call(_, _, _, r) => r.clone(),
            AstNode::Ret(_, _, r) => r.clone(),
            AstNode::Stalloc(_, _, r) => r.clone(),
            AstNode::Ptroffset(_, _, _, r) => r.clone(),
            AstNode::Load(_, _, r) => r.clone(),
            AstNode::Store(_, _, _, r) => r.clone(),
            AstNode::Dbg(_, _, r) => r.clone(),
            AstNode::Equals(_, _, r) => r.clone(),
            AstNode::Err(_, r) => r.clone(),
        }
    }

    pub fn is_var(&self) -> bool {
        matches!(self, AstNode::Local(..) | AstNode::Global(..))
    }
}

pub fn display_parse_err(x: ParserErr, src: &str, stream: &[Token]) {
    // ok so basically we need to get the line that the gobbled indexes into
    let mut lines = src.lines();
    let mut count = 0;
    let mut diff = 0;
    let mut line = None;
    let mut prevline;
    // gobbled is in token count. lets get it in chars
    let mut gobbled = x.end_idx;
    loop {
        prevline = line;
        line = lines.next();
        if line.is_none() {
            break;
        }
        diff = gobbled - count;
        count += line.as_ref().unwrap().len();
        if count > gobbled {
            break;
        }
    }
    if let Some(l) = prevline {
        eprintln!("{}", l);
    }
    eprintln!("{}", line.unwrap());
    let mut ptr = " ".repeat(diff);
    ptr.push('^');
    ptr.push_str("  Expected: ");
    ptr.push_str(&x.expected);
    eprintln!("{}", ptr);
    if let Some((t, e)) = x.next {
        eprintln!("Next token is {t:?} ({e})");
    }
}

#[allow(clippy::type_complexity)]
pub fn path<'a>(
) -> BoxedParser<'a, &'a [Token<'a>], Vec<(std::ops::Range<usize>, &'a str)>, ParserErr> {
    tok(Tok::Ident)
        .map_range(|x, r| (r, x))
        .repeated_sep(tok(Tok::Comma))
}

pub fn atom<'a>() -> BoxedParser<'a, &'a [Token<'a>], AstNode<'a>, ParserErr> {
    let local = tok(Tok::Ampersand)
        .chain(tok(Tok::Ident))
        .map_range(|x, r| AstNode::Local(x.1, r));
    let global = tok(Tok::Dollar)
        .chain(tok(Tok::Ident))
        .map_range(|x, r| AstNode::Global(x.1, r));
    let cst = tok(Tok::Bang)
        .ignore_then(tok(Tok::Ident).delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)))
        .chain(tok(Tok::Number))
        .map_range(|x, r| AstNode::Const(Type::Unresolved(x.0), x.1, r));
    let cptroffset = reserved("cptroffset")
        .ignore_then(tok(Tok::Ident))
        .then_ignore(tok(Tok::Comma))
        .chain(tok(Tok::Number))
        .map_range(|x, r| AstNode::CPtrOffset(Type::Unresolved(x.0), x.1, r));

    let sptroffset = reserved("sptroffset")
        .ignore_then(tok(Tok::Ident))
        .then_ignore(tok(Tok::Comma))
        .chain(path())
        .map_range(|x, r| {
            AstNode::SPtrOffset(
                Type::Unresolved(x.0),
                x.1.into_iter().map(|y| y.1).collect(),
                r,
            )
        });
    local.or(global).or(cst).or(cptroffset).or(sptroffset)
}

pub fn op<'a>(standalone: bool) -> BoxedParser<'a, &'a [Token<'a>], AstNode<'a>, ParserErr> {
    macro_rules! binop {
        ($x:ident $y:ident) => {
            let $x = reserved(stringify!($x))
                .ignore_then(
                    tok(Tok::Ident)
                        .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket))
                        .map(Type::Unresolved),
                )
                .chain(atom().then_ignore(tok(Tok::Comma)).chain(atom()))
                .map_range(|x, r| AstNode::$y(x.0, Box::new(x.1 .0), Box::new(x.1 .1), r));
        };
    }

    binop!(add Add);
    binop!(sub Sub);
    binop!(div Div);
    binop!(mul Mul);
    let cpy = reserved("cpy")
        .ignore_then(atom())
        .map_range(|x, r| AstNode::Cpy(Box::new(x), r));
    let call = reserved("call")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .chain(atom())
        .chain(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .chain(atom())
                .repeated_sep(tok(Tok::Comma)),
        )
        .map_range(|x, r| AstNode::Call(x.0 .0, Box::new(x.0 .1), x.1, r));
    let ret = reserved("ret")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .chain(atom())
        .map_range(|x, r| AstNode::Ret(x.0, Box::new(x.1), r));
    let stalloc = reserved("stalloc")
        .ignore_then(tok(Tok::Ident).map(Type::Unresolved))
        .then_ignore(reserved("times"))
        .chain(atom())
        .map_range(|x, r| AstNode::Stalloc(x.0, Box::new(x.1), r));
    let ptroffset = reserved("ptroffset")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .then_ignore(reserved("ptr"))
        .chain(atom().chain(atom()))
        .map_range(|x, r| AstNode::Ptroffset(x.0, Box::new(x.1 .0), Box::new(x.1 .1), r));
    let load = reserved("load")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .then_ignore(reserved("ptr"))
        .chain(atom())
        .map_range(|x, r| AstNode::Load(x.0, Box::new(x.1), r));
    let store = reserved("store")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .then_ignore(reserved("ptr"))
        .chain(atom().then_ignore(tok(Tok::Comma)).chain(atom()))
        .map_range(|x, r| AstNode::Store(x.0, Box::new(x.1 .0), Box::new(x.1 .1), r));
    let dbg = reserved("dbg")
        .ignore_then(
            tok(Tok::Ident)
                .map(Type::Unresolved)
                .delimited(tok(Tok::OpenBracket), tok(Tok::CloseBracket)),
        )
        .chain(atom())
        .map_range(|x, r| AstNode::Dbg(x.0, Box::new(x.1), r));
    if !standalone {
        call.or(stalloc)
            .or(ptroffset)
            .or(load)
            .or(add)
            .or(sub)
            .or(div)
            .or(mul)
            .or(cpy)
    } else {
        call.or(ret).or(store).or(dbg)
    }
}

pub fn stmt<'a>() -> BoxedParser<'a, &'a [Token<'a>], AstNode<'a>, ParserErr> {
    let justop = op(true);
    let var = atom().check(|x| {
        if x.is_var() {
            None
        } else {
            Some("Variable".to_owned())
        }
    });
    let eq = var
        .then_ignore(tok(Tok::Equals))
        .chain(op(false))
        .map_range(|x, r| AstNode::Equals(Box::new(x.0), Box::new(x.1), r));
    justop.or(eq)
}

fn main() {
    let src = r#"&num1 = cpy !(u32)9
    &num2 = cpy r!(u32)11
    &result = add(u32) &num1, &num2
    dbg(u32) &result

"#;
    let tl = Tok::lexer(src);
    let ts = TokenStream::new(tl);

    let patom = stmt().repeated().eoi().parse(ts.slice());
    if let Err(x) = patom {
        display_parse_err(x, src, ts.slice());
    } else {
        dbg!(patom.unwrap());
    }
}
