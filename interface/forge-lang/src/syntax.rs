//! The lexer and the LL(1) parser: source text to an AST whose every node carries its byte span.
//! The parser runs the table generated from grammar.ll1 (table.rs); the actions here build the tree.

use crate::error::{Error, SyntaxError};
use table::{P, t};

#[rustfmt::skip]
pub mod table;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(lo: usize, hi: usize) -> Span {
        Span {
            lo: lo as u32,
            hi: hi as u32,
        }
    }
    pub fn to(self, other: Span) -> Span {
        Span {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }
    /// 1-based line and column of the start, and of the end.
    pub fn lines(self, src: &str) -> (usize, usize, usize, usize) {
        let at = |off: usize| {
            let off = off.min(src.len());
            let before = &src[..off];
            let line = before.matches('\n').count() + 1;
            let col = off - before.rfind('\n').map_or(0, |k| k + 1) + 1;
            (line, col)
        };
        let (a, b) = at(self.lo as usize);
        let (c, d) = at(self.hi as usize);
        (a, b, c, d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    None,
    Mm,
    Deg,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokKind {
    Num(f64, Unit),
    Str(String),
    Id(String),
    Kw(&'static str),
    Op(&'static str),
    Nl,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tok {
    pub kind: TokKind,
    pub span: Span,
}

pub const KEYWORDS: [&str; 19] = [
    "shared",
    "param",
    "fn",
    "for",
    "in",
    "if",
    "else",
    "return",
    "expect",
    "and",
    "or",
    "not",
    "true",
    "false",
    "sketch",
    "path",
    "at",
    "component",
    "drawing",
];

pub const OPS: [&str; 26] = [
    "..=", "+-", "..", "==", "!=", "<=", ">=", "+", "-", "*", "/", "^", "%", "(", ")", "[", "]", "{", "}",
    ",", ":", ".", "=", "<", ">", ";",
];

/// For each ASCII byte: 1 + the index in OPS of the one-byte operator it is (0: none), and
/// whether a longer operator starts with it.
const OP_BYTES: ([u8; 128], [bool; 128]) = {
    let (mut one, mut long) = ([0; 128], [false; 128]);
    let mut k = 0;
    while k < OPS.len() {
        let o = OPS[k].as_bytes();
        if o.len() == 1 {
            one[o[0] as usize] = k as u8 + 1;
        } else {
            long[o[0] as usize] = true;
        }
        k += 1;
    }
    (one, long)
};

/// OPS starts with its operators of more than one byte.
const LONG_OPS: usize = {
    let mut k = 0;
    while OPS[k].len() > 1 {
        k += 1;
    }
    let mut j = k;
    while j < OPS.len() {
        assert!(OPS[j].len() == 1, "OPS lists its longer operators first");
        j += 1;
    }
    k
};

/// The operator at `b[i..]`, longest first, as its index in OPS.
fn op_at(b: &[u8], i: usize) -> Option<usize> {
    let c = b[i] as usize;
    if c >= 128 {
        return None;
    }
    if OP_BYTES.1[c] {
        let n = b.get(i + 1).copied().unwrap_or(0);
        for (k, o) in OPS[..LONG_OPS].iter().enumerate() {
            let o = o.as_bytes();
            if o[0] as usize == c && o[1] == n && (o.len() == 2 || b.get(i + 2) == Some(&o[2])) {
                return Some(k);
            }
        }
    }
    OP_BYTES.0[c].checked_sub(1).map(usize::from)
}

fn keyword(w: &str) -> Option<usize> {
    if !(2..=9).contains(&w.len()) || !w.as_bytes()[0].is_ascii_lowercase() {
        return None;
    }
    KEYWORDS.iter().position(|k| *k == w)
}

const fn word_term(w: &str) -> u8 {
    let mut k = 0;
    while k < table::NT {
        let (a, b) = (table::TERMS[k].as_bytes(), w.as_bytes());
        if a.len() == b.len() {
            let mut i = 0;
            while i < a.len() && a[i] == b[i] {
                i += 1;
            }
            if i == a.len() {
                return k as u8;
            }
        }
        k += 1;
    }
    panic!("a keyword or operator that is not a terminal of grammar.ll1")
}

/// Each keyword's and operator's terminal, found when the crate compiles.
const KW_TERM: [u8; KEYWORDS.len()] = {
    let mut out = [0; KEYWORDS.len()];
    let mut k = 0;
    while k < out.len() {
        out[k] = word_term(KEYWORDS[k]);
        k += 1;
    }
    out
};
const OP_TERM: [u8; OPS.len()] = {
    let mut out = [0; OPS.len()];
    let mut k = 0;
    while k < out.len() {
        out[k] = word_term(OPS[k]);
        k += 1;
    }
    out
};

/// What the lexer skips between tokens. With the tokens, trivia tile the source: every byte is in
/// exactly one token or one trivia, so the text is rebuilt exactly from them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    /// Spaces, tabs and carriage returns.
    Space,
    /// `#` to the end of the line, the line break not included.
    Comment,
    /// A line break inside `( )` or `[ ]`, which is not an NL token.
    Newline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Tok>, Error> {
    lex_into::<false>(src, &mut Vec::new(), &mut Vec::new())
}

/// The tokens and the trivia between them.
pub fn lex_lossless(src: &str) -> Result<(Vec<Tok>, Vec<Trivia>), Error> {
    let mut trivia = Vec::new();
    let toks = lex_into::<true>(src, &mut trivia, &mut Vec::new())?;
    Ok((toks, trivia))
}

/// Lexes, and writes each token's terminal of grammar.ll1 to `la`.
fn lex_into<const KEEP: bool>(
    src: &str,
    trivia: &mut Vec<Trivia>,
    la: &mut Vec<u8>,
) -> Result<Vec<Tok>, Error> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut depth: i32 = 0;
    let mut keep = |kind: TriviaKind, lo: usize, hi: usize| {
        if KEEP {
            trivia.push(Trivia {
                kind,
                span: Span::new(lo, hi),
            });
        }
    };
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            if depth == 0 {
                la.push(t::NL);
                out.push(Tok {
                    kind: TokKind::Nl,
                    span: Span::new(i, i + 1),
                });
            } else {
                keep(TriviaKind::Newline, i, i + 1);
            }
            i += 1;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            let s = i;
            while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\r') {
                i += 1;
            }
            keep(TriviaKind::Space, s, i);
            continue;
        }
        if c == b'#' {
            let s = i;
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            keep(TriviaKind::Comment, s, i);
            continue;
        }
        let neg = c == b'-'
            && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())
            && !matches!(
                out.last().map(|t: &Tok| &t.kind),
                Some(TokKind::Num(..) | TokKind::Str(_) | TokKind::Id(_) | TokKind::Kw("true" | "false"))
                    | Some(TokKind::Op(")" | "]" | "}"))
            );
        if c.is_ascii_digit() || neg {
            let s = i;
            if neg {
                i += 1;
            }
            while i < b.len() && (b[i].is_ascii_digit() || (b[i] == b'.' && b.get(i + 1) != Some(&b'.'))) {
                i += 1;
            }
            if i < b.len()
                && (b[i] == b'e' || b[i] == b'E')
                && b.get(i + 1)
                    .is_some_and(|d| d.is_ascii_digit() || *d == b'-' || *d == b'+')
            {
                i += 2;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text = &src[s..i];
            let v: f64 = text.parse().map_err(|_| {
                Error::Syntax(SyntaxError::BadNumber {
                    span: Span::new(s, i),
                })
            })?;
            let u0 = i;
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit = match &src[u0..i] {
                "" => Unit::None,
                "mm" => Unit::Mm,
                "deg" => Unit::Deg,
                _ => {
                    return Err(Error::Syntax(SyntaxError::UnknownUnit {
                        span: Span::new(u0, i),
                    }));
                }
            };
            la.push(t::NUM);
            out.push(Tok {
                kind: TokKind::Num(v, unit),
                span: Span::new(s, i),
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let w = &src[s..i];
            let kind = match keyword(w) {
                Some(k) => {
                    la.push(KW_TERM[k]);
                    TokKind::Kw(KEYWORDS[k])
                }
                None => {
                    la.push(t::ID);
                    TokKind::Id(w.to_string())
                }
            };
            out.push(Tok {
                kind,
                span: Span::new(s, i),
            });
            continue;
        }
        if c == b'"' {
            let s = i;
            i += 1;
            while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
                i += 1;
            }
            if i >= b.len() || b[i] != b'"' {
                return Err(Error::Syntax(SyntaxError::OpenString {
                    span: Span::new(s, i),
                }));
            }
            i += 1;
            la.push(t::STR);
            out.push(Tok {
                kind: TokKind::Str(src[s + 1..i - 1].to_string()),
                span: Span::new(s, i),
            });
            continue;
        }
        match op_at(b, i) {
            Some(k) => {
                let op = OPS[k];
                la.push(OP_TERM[k]);
                match op {
                    "(" | "[" => depth += 1,
                    ")" | "]" => depth = (depth - 1).max(0),
                    _ => {}
                }
                out.push(Tok {
                    kind: TokKind::Op(op),
                    span: Span::new(i, i + op.len()),
                });
                i += op.len();
            }
            None => {
                let ch = src[i..].chars().next().map_or(1, char::len_utf8);
                return Err(Error::Syntax(SyntaxError::BadChar {
                    span: Span::new(i, i + ch),
                }));
            }
        }
    }
    la.push(t::EOF);
    out.push(Tok {
        kind: TokKind::Eof,
        span: Span::new(b.len(), b.len()),
    });
    Ok(out)
}

// AST -----------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Num(f64, Unit),
    Str(String),
    Bool(bool),
    Name(String),
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    /// `{a: e, b: e}`: values keyed by labels.
    Map(Vec<(String, Expr)>),
    Comp {
        item: Box<Expr>,
        vars: Vec<String>,
        iter: Box<Expr>,
        cond: Option<Box<Expr>>,
    },
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
        inclusive: bool,
    },
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Call(Box<Expr>, Vec<Arg>),
    Attr(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        other: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<(String, Span)>,
    pub value: Expr,
}

/// A loop instance's key: an index over a range, or a label of a keyed loop. Never a value
/// and never a position in a list.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Ix(i64),
    Label(String),
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Ix(i) => write!(f, "{i}"),
            Key::Label(l) => write!(f, "{l}"),
        }
    }
}

/// One segment of an id or a param's owner path: a name and, for one instance of a loop, its keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Seg {
    pub name: String,
    pub index: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Shared {
        name: String,
        value: Expr,
    },
    Param {
        owner: Vec<Seg>,
        name: String,
        value: Expr,
    },
    Fn {
        name: String,
        params: Vec<FnParam>,
        body: Vec<Stmt>,
    },
    For {
        vars: Vec<String>,
        iter: Expr,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        other: Option<Vec<Stmt>>,
    },
    Return(Expr),
    Expect {
        metric: String,
        args: Vec<Expr>,
        cmp: Option<ExpectCmp>,
        /// `at P = v, ..`: params held at these values for this check (an assembly's pose).
        at: Vec<(Vec<Seg>, Expr)>,
    },
    /// `sketch NAME(args) { rows }` or `path NAME(args) { segments }`.
    Block {
        kind: BlockKind,
        name: String,
        args: Vec<Arg>,
        body: Vec<Stmt>,
    },
    Assign {
        name: String,
        value: Expr,
    },
    Expr(Expr),
}

/// An expect line's comparison: how, against what, and a tolerance (true: a percentage).
pub type ExpectCmp = (Cmp, Expr, Option<(Expr, bool)>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Sketch,
    Path,
    /// `component NAME(at: ..) { statements }`: a part of its own, placed in this document.
    Component,
    /// `drawing NAME(..) { statements }`: a drawing of this document, in the drawing language;
    /// the model's evaluation skips it.
    Drawing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnParam {
    pub name: String,
    pub ty: Option<String>,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    /// The whole statement.
    pub span: Span,
    /// The statement and the line break after it, for edits that take a statement out or move it.
    pub full: Span,
}

/// A set of terminals of grammar.ll1 (bit k: `table::TERMS[k]`): what a syntax error expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct TermSet(pub u128);

impl TermSet {
    pub fn contains(self, t: u8) -> bool {
        self.0 >> t & 1 == 1
    }
    /// The terminals as written: `)`, `in`, NUM, ID, NL, EOF.
    pub fn names(self) -> Vec<&'static str> {
        (0..table::NT as u8)
            .filter(|t| self.contains(*t))
            .map(|t| table::TERMS[t as usize])
            .collect()
    }
}

/// A value on the parser's stack: a token, or what a finished rule built. Lists made by
/// right-recursive rules are built last item first and turned round by the rule that owns them.
enum V {
    None,
    Tok(u32),
    Expr(Expr),
    Kind(ExprKind),
    Exprs(Vec<Expr>),
    /// A statement, put on `Parser::stmts`.
    Stmt,
    /// How many statements of a list are on `Parser::stmts`, the last ones.
    Count(u32),
    Stmts(Vec<Stmt>),
    /// stmtsrest: the end of the separator, and the statements after it.
    Rest(u32, u32),
    Path(Vec<Seg>, String),
    Segs(Vec<Seg>),
    Keys(Vec<Key>),
    Key(Key),
    Param(Box<FnParam>),
    Params(Vec<FnParam>),
    Name(String),
    Names(Vec<String>),
    Arg(Arg),
    Args(Vec<Arg>),
    Cmp(Cmp),
    ExpCmp(Box<ExpectCmp>),
    Ats(Vec<(Vec<Seg>, Expr)>),
    Tol(Expr, bool),
    Pct(bool),
    IfPart(Box<(Expr, Expr)>),
    Op(BinOp),
    /// The operators or postfix ops after an operand, as the tree they make with a hole where
    /// that operand goes: the hole is `depth` left children down.
    Chain(Expr, u32),
    RangeTail(bool, Expr),
    Comp(Box<(Vec<String>, Expr, Option<Box<Expr>>)>),
    Map(Vec<(String, Expr)>),
    /// drawtail: a drawing block's name, args and statements.
    Draw(Box<(String, Vec<Arg>, Vec<Stmt>)>),
}

type R<T> = Result<T, Error>;

fn err<T>(e: SyntaxError) -> R<T> {
    Err(Error::Syntax(e))
}

/// Takes the value at a stack slot apart: `get!(self, i, V::Expr(e) => e)`.
macro_rules! get {
    ($p:expr, $i:expr, $pat:pat => $out:expr) => {
        match $p.take($i) {
            $pat => $out,
            _ => unreachable!("rule action and grammar.ll1 disagree"),
        }
    };
}

/// A list at a stack slot; an empty rule left `V::None` there.
macro_rules! list {
    ($p:expr, $i:expr, V::$v:ident) => {
        match $p.take($i) {
            V::$v(l) => l,
            V::None => Vec::new(),
            _ => unreachable!("rule action and grammar.ll1 disagree"),
        }
    };
}

/// A stack entry at or above this is "reduce by rule (entry - REDUCE)".
const REDUCE: u16 = 0x100;

struct Parser {
    toks: Vec<Tok>,
    /// Finished statements of the lists being parsed, innermost last.
    stmts: Vec<Stmt>,
    /// Each token's terminal.
    la: Vec<u8>,
    k: usize,
    vals: Vec<V>,
}

fn nonterm(name: &str) -> usize {
    table::NONTERMS
        .iter()
        .position(|n| *n == name)
        .unwrap_or(usize::MAX)
}

/// How an error names what it wanted, from the symbol the parser stopped at.
fn what(stop: u16) -> &'static str {
    if (stop as u8) < table::NT_BASE {
        return match stop as u8 {
            t::ID => "a name",
            t::NUM => "a number",
            t::STR => "a string",
            t::NL => "a line break",
            t::EOF => "end of input",
            x => table::TERMS[x as usize],
        };
    }
    match table::NONTERMS[stop as usize - table::NT_BASE as usize] {
        "block" | "elsebody" => "{",
        "path" => "a name",
        "key" => "an instance index",
        _ => "a value",
    }
}

impl Parser {
    fn take(&mut self, i: usize) -> V {
        std::mem::replace(&mut self.vals[i], V::None)
    }
    fn expr(&mut self, i: usize) -> Expr {
        get!(self, i, V::Expr(e) => e)
    }
    fn tok(&self, i: usize) -> &Tok {
        match self.vals[i] {
            V::Tok(k) => &self.toks[k as usize],
            _ => unreachable!("rule action and grammar.ll1 disagree"),
        }
    }
    fn text(&mut self, i: usize) -> String {
        let V::Tok(k) = self.vals[i] else {
            unreachable!("rule action and grammar.ll1 disagree")
        };
        match &mut self.toks[k as usize].kind {
            TokKind::Id(s) | TokKind::Str(s) => std::mem::take(s),
            _ => unreachable!("a name or a string"),
        }
    }
    /// Pops to `n` values; what is left above is spent (empty, or a token).
    #[inline]
    fn drop_to(&mut self, n: usize) {
        while self.vals.len() > n {
            match self.vals.pop() {
                Some(V::None | V::Tok(_)) => {}
                other => drop(other),
            }
        }
    }
    fn last_hi(&self) -> u32 {
        self.toks[self.k.saturating_sub(1)].span.hi
    }

    fn run(&mut self) -> R<Vec<Stmt>> {
        let expand = expansions();
        let chains = chains();
        let mut stack: Vec<u16> = Vec::with_capacity(128);
        stack.push(table::NT_BASE as u16);
        // After the first lookahead no rule takes: what was expected there, and whether a
        // statement could have ended (stmtsrest) or begun (stmts) at it. Rules that can be empty
        // are then taken empty until a terminal or a rule that cannot be, so the actions on the
        // way see the same input the hand parser saw before its error.
        let mut missed: Option<(TermSet, bool, bool)> = None;
        while let Some(s) = stack.pop() {
            if s >= REDUCE {
                let p = (s - REDUCE) as usize;
                let base = self.vals.len() - table::RHS[p].len();
                if KEEPS[p] && self.vals[base + 1..].iter().all(|v| matches!(v, V::None)) {
                    self.drop_to(base + 1);
                    continue;
                }
                let v = self.reduce(table::RULES[p], base)?;
                self.drop_to(base);
                self.vals.push(v);
                continue;
            }
            let la = self.la[self.k];
            if (s as u8) < table::NT_BASE {
                if s as u8 == la && missed.is_none() {
                    self.vals.push(V::Tok(self.k as u32));
                    self.k = (self.k + 1).min(self.toks.len() - 1);
                    continue;
                }
                return Err(self.expected(s, &stack, missed));
            }
            let a = s as usize - table::NT_BASE as usize;
            if missed.is_none() {
                let c = &chains[a * table::NT + la as usize];
                if c.empty {
                    if let Some(r) = c.enter {
                        self.enter(r)?;
                    }
                    if let Some(&m) = stack.last()
                        && m >= REDUCE
                        && KEEPS[(m - REDUCE) as usize]
                    {
                        stack.pop();
                    } else {
                        self.vals.push(V::None);
                    }
                    continue;
                }
                if c.valid {
                    if let Some(r) = c.enter {
                        self.enter(r)?;
                    }
                    stack.extend_from_slice(&c.push);
                    if c.shift {
                        self.vals.push(V::Tok(self.k as u32));
                        self.k = (self.k + 1).min(self.toks.len() - 1);
                    }
                    if let Some(p) = c.leaf {
                        let b = self.vals.len() - 1;
                        let rule = table::RULES[p as usize];
                        let v = match rule {
                            P::AtomNum | P::AtomStr | P::AtomId | P::AtomTrue | P::AtomFalse => {
                                self.atom(rule, b)
                            }
                            _ => self.reduce(rule, b)?,
                        };
                        self.vals[b] = v;
                    }
                    continue;
                }
            }
            // expand, and go on down the leftmost symbol without a round trip through the stack
            let mut s = s;
            loop {
                let a = s as usize - table::NT_BASE as usize;
                let mut p = table::TABLE[a][la as usize];
                if p == table::NONE {
                    let m = missed.get_or_insert_with(|| (expected_set(s, &stack), false, false));
                    m.1 |= a == nonterm("stmtsrest");
                    m.2 |= a == nonterm("stmts");
                    p = table::EMPTY[a];
                    if p == table::NONE {
                        return Err(self.expected(s, &stack, missed));
                    }
                }
                let rhs = table::RHS[p as usize];
                let rule = table::RULES[p as usize];
                if rhs.is_empty() {
                    if rule == P::ArgkwEmpty {
                        self.enter(rule)?;
                    }
                    // an empty last symbol of a rule that keeps its first value: that rule is done
                    if let Some(&m) = stack.last()
                        && m >= REDUCE
                        && KEEPS[(m - REDUCE) as usize]
                    {
                        stack.pop();
                    } else {
                        self.vals.push(V::None);
                    }
                    break;
                }
                if matches!(rule, P::AssigntailAssign | P::ArgkwColon) {
                    self.enter(rule)?;
                }
                stack.extend_from_slice(&expand[p as usize]);
                let first = rhs[0];
                if first >= table::NT_BASE {
                    s = first as u16;
                    continue;
                }
                if first == la && missed.is_none() {
                    self.vals.push(V::Tok(self.k as u32));
                    self.k = (self.k + 1).min(self.toks.len() - 1);
                } else {
                    stack.push(first as u16);
                }
                break;
            }
        }
        Ok(get!(self, 0, V::Stmts(s) => s))
    }

    fn expected(&self, stop: u16, stack: &[u16], missed: Option<(TermSet, bool, bool)>) -> Error {
        let (set, end, begin) = missed.unwrap_or_else(|| (expected_set(stop, stack), false, false));
        Error::Syntax(SyntaxError::Expected {
            what: if end {
                "end of statement"
            } else if begin {
                "a value"
            } else {
                what(stop)
            },
            expected: set,
            span: self.toks[self.k].span,
        })
    }

    /// Checks made as a rule is chosen, at the point the hand parser made them.
    fn enter(&mut self, p: P) -> R<()> {
        let top = self.vals.len().wrapping_sub(1);
        match p {
            P::AssigntailAssign | P::ArgkwColon => {
                let V::Expr(e) = &self.vals[top] else {
                    unreachable!("an expression before `=` or `:`")
                };
                if !matches!(e.kind, ExprKind::Name(_)) {
                    let span = e.span;
                    return err(if p == P::ArgkwColon {
                        SyntaxError::KeywordNotName { span }
                    } else {
                        SyntaxError::AssignToNonName { span }
                    });
                }
            }
            // a positional argument: the one before it, if any, sits under its `,`
            P::ArgkwEmpty => {
                let V::Expr(e) = &self.vals[top] else {
                    unreachable!("an argument")
                };
                if let (V::Tok(_), Some(V::Arg(prev))) =
                    (&self.vals[top - 1], top.checked_sub(2).map(|i| &self.vals[i]))
                    && self.la[match self.vals[top - 1] {
                        V::Tok(k) => k as usize,
                        _ => 0,
                    }] == t::COMMA
                    && prev.name.is_some()
                {
                    return err(SyntaxError::PositionalAfterKeyword { span: e.span });
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A one-token value.
    #[inline]
    fn atom(&mut self, p: P, b: usize) -> V {
        let t = self.tok(b);
        let span = t.span;
        let kind = match &t.kind {
            TokKind::Num(v, u) => ExprKind::Num(*v, *u),
            TokKind::Str(_) => ExprKind::Str(self.text(b)),
            TokKind::Id(_) => ExprKind::Name(self.text(b)),
            _ => ExprKind::Bool(p == P::AtomTrue),
        };
        V::Expr(Expr { kind, span })
    }

    /// The tree-building action of each rule, run when its right side is done; the right side's
    /// values are `self.vals[base..]`. `span` is right for a rule that starts with a token or an
    /// expression.
    fn reduce(&mut self, p: P, base: usize) -> R<V> {
        let b = base;
        let span = Span {
            lo: match &self.vals[b] {
                V::Tok(k) => self.toks[*k as usize].span.lo,
                V::Expr(e) => e.span.lo,
                _ => 0,
            },
            hi: self.last_hi(),
        };
        macro_rules! stmt {
            ($kind:expr) => {{
                let kind = $kind;
                self.stmts.push(Stmt {
                    kind,
                    span,
                    full: span,
                });
                V::Stmt
            }};
        }
        Ok(match p {
            P::ProgramSeps | P::BlockLbrace => {
                let n = match self.take(b + if p == P::ProgramSeps { 1 } else { 2 }) {
                    V::Count(n) => n as usize,
                    V::None => 0,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                };
                V::Stmts(self.stmts.split_off(self.stmts.len() - n))
            }
            P::StmtsStmt => {
                let (hi, n) = match self.take(b + 1) {
                    V::Rest(hi, n) => (Some(hi), n),
                    V::None => (None, 0),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                };
                let k = self.stmts.len() - 1 - n as usize;
                let s = &mut self.stmts[k];
                s.full.hi = hi.unwrap_or(s.span.hi);
                V::Count(n + 1)
            }
            P::StmtsrestSep => V::Rest(
                self.tok(b).span.hi,
                match self.take(b + 2) {
                    V::Count(n) => n,
                    V::None => 0,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
            ),
            P::SepsSep => V::None,
            P::SepNl | P::SepSemi => self.take(b),
            P::StmtShared => stmt!(StmtKind::Shared {
                name: self.text(b + 1),
                value: self.expr(b + 3),
            }),
            P::StmtParam => {
                let (owner, name) = get!(self, b + 1, V::Path(o, n) => (o, n));
                stmt!(StmtKind::Param {
                    owner,
                    name,
                    value: self.expr(b + 3),
                })
            }
            P::StmtFn => {
                let mut params = list!(self, b + 3, V::Params);
                params.reverse();
                stmt!(StmtKind::Fn {
                    name: self.text(b + 1),
                    params,
                    body: list!(self, b + 5, V::Stmts),
                })
            }
            P::StmtFor => {
                let mut vars = list!(self, b + 2, V::Names);
                vars.push(self.text(b + 1));
                vars.reverse();
                stmt!(StmtKind::For {
                    vars,
                    iter: self.expr(b + 4),
                    body: list!(self, b + 5, V::Stmts),
                })
            }
            P::StmtIf | P::ElsebodyIf => {
                let kind = StmtKind::If {
                    cond: self.expr(b + 1),
                    then: list!(self, b + 2, V::Stmts),
                    other: match self.take(b + 3) {
                        V::Stmts(s) => Some(s),
                        V::None => None,
                        _ => unreachable!("rule action and grammar.ll1 disagree"),
                    },
                };
                match p {
                    P::StmtIf => stmt!(kind),
                    _ => V::Stmts(vec![Stmt {
                        kind,
                        span,
                        full: span,
                    }]),
                }
            }
            P::StmtReturn => stmt!(StmtKind::Return(self.expr(b + 1))),
            P::StmtExpect => stmt!(StmtKind::Expect {
                metric: self.text(b + 1),
                args: list!(self, b + 2, V::Exprs),
                cmp: match self.take(b + 3) {
                    V::ExpCmp(c) => Some(*c),
                    V::None => None,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
                at: {
                    let mut at = list!(self, b + 4, V::Ats);
                    at.reverse();
                    at
                },
            }),
            P::ExpatAt => {
                let mut rest = list!(self, b + 4, V::Ats);
                rest.push((get!(self, b + 1, V::Segs(s) => s), self.expr(b + 3)));
                V::Ats(rest)
            }
            P::ExpmoreComma => {
                let mut rest = list!(self, b + 4, V::Ats);
                rest.push((get!(self, b + 1, V::Segs(s) => s), self.expr(b + 3)));
                V::Ats(rest)
            }
            P::AtpathId => {
                let mut segs = list!(self, b + 2, V::Segs);
                let index = list!(self, b + 1, V::Keys);
                segs.push(Seg {
                    name: self.text(b),
                    index,
                });
                segs.reverse();
                V::Segs(segs)
            }
            P::StmtSketch | P::StmtPath | P::StmtComponent => {
                let mut args = list!(self, b + 3, V::Args);
                args.reverse();
                stmt!(StmtKind::Block {
                    kind: match p {
                        P::StmtSketch => BlockKind::Sketch,
                        P::StmtPath => BlockKind::Path,
                        _ => BlockKind::Component,
                    },
                    name: self.text(b + 1),
                    args,
                    body: list!(self, b + 5, V::Stmts),
                })
            }
            // `drawing` alone: a drawing file's first line, the word as it always was
            P::StmtDrawing => match self.take(b + 1) {
                V::None => {
                    let at = self.tok(b).span;
                    stmt!(StmtKind::Expr(Expr {
                        kind: ExprKind::Name("drawing".into()),
                        span: at,
                    }))
                }
                V::Draw(d) => {
                    let (name, args, body) = *d;
                    stmt!(StmtKind::Block {
                        kind: BlockKind::Drawing,
                        name,
                        args,
                        body,
                    })
                }
                _ => unreachable!("rule action and grammar.ll1 disagree"),
            },
            P::DrawtailId => {
                let mut args = list!(self, b + 1, V::Args);
                args.reverse();
                V::Draw(Box::new((self.text(b), args, list!(self, b + 2, V::Stmts))))
            }
            P::DargsLparen => self.take(b + 1),
            P::StmtExpr => {
                let e = self.expr(b);
                stmt!(match self.take(b + 1) {
                    V::Expr(value) => {
                        let ExprKind::Name(name) = e.kind else {
                            unreachable!("checked as `=` was taken")
                        };
                        StmtKind::Assign { name, value }
                    }
                    V::None => StmtKind::Expr(e),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                })
            }
            P::AssigntailAssign
            | P::FpdefaultAssign
            | P::ArgkwColon
            | P::PowtailCaret
            | P::CompifIf
            | P::ElsepartElse
            | P::FparamrestComma
            | P::ExprsrestComma
            | P::ArgsrestComma
            | P::TuplerestComma
            | P::ListtailComma
            | P::ListrestComma => self.take(b + 1),
            P::PathId => {
                let mut segs = list!(self, b + 2, V::Segs);
                let index = list!(self, b + 1, V::Keys);
                segs.push(Seg {
                    name: self.text(b),
                    index,
                });
                segs.reverse();
                let last = segs.pop().expect("one segment");
                if segs.is_empty() || !last.index.is_empty() {
                    let at = self.tok(b - 1).span;
                    return err(SyntaxError::ParamWithoutOwner {
                        span: Span {
                            lo: at.lo,
                            hi: span.hi,
                        },
                        name: last.name,
                    });
                }
                V::Path(segs, last.name)
            }
            P::PathrestDot => {
                let mut segs = list!(self, b + 3, V::Segs);
                let index = list!(self, b + 2, V::Keys);
                segs.push(Seg {
                    name: self.text(b + 1),
                    index,
                });
                V::Segs(segs)
            }
            P::KeysLbrack => {
                let mut keys = list!(self, b + 2, V::Keys);
                keys.push(get!(self, b + 1, V::Key(k) => k));
                keys.reverse();
                V::Keys(keys)
            }
            P::KeyNum => match self.tok(b).kind {
                TokKind::Num(v, Unit::None) if v.fract() == 0.0 => V::Key(Key::Ix(v as i64)),
                _ => {
                    return err(SyntaxError::Expected {
                        what: "an instance index",
                        expected: TermSet(1 << t::NUM | 1 << t::ID),
                        span: self.tok(b).span,
                    });
                }
            },
            P::KeyId => V::Key(Key::Label(self.text(b))),
            P::KeyrestComma => {
                let mut keys = list!(self, b + 2, V::Keys);
                keys.push(get!(self, b + 1, V::Key(k) => k));
                V::Keys(keys)
            }
            P::FparamsFparam => {
                let mut ps = list!(self, b + 1, V::Params);
                ps.push(*get!(self, b, V::Param(p) => p));
                V::Params(ps)
            }
            P::FparamId => V::Param(Box::new(FnParam {
                name: self.text(b),
                ty: match self.take(b + 1) {
                    V::Name(n) => Some(n),
                    V::None => None,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
                default: match self.take(b + 2) {
                    V::Expr(e) => Some(e),
                    V::None => None,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
            })),
            P::FptypeColon => V::Name(self.text(b + 1)),
            P::ForvarsComma => {
                let mut vars = list!(self, b + 2, V::Names);
                vars.push(self.text(b + 1));
                V::Names(vars)
            }
            P::ElsebodyBlock => self.take(b),
            P::ExpargsLparen => {
                let mut e = list!(self, b + 1, V::Exprs);
                e.reverse();
                V::Exprs(e)
            }
            P::ExprsExpr | P::TupleExpr | P::ListmoreExpr => {
                let mut rest = list!(self, b + 1, V::Exprs);
                rest.push(self.expr(b));
                V::Exprs(rest)
            }
            P::ExpcmpCmpop => V::ExpCmp(Box::new((
                get!(self, b, V::Cmp(c) => c),
                self.expr(b + 1),
                match self.take(b + 2) {
                    V::Tol(t, pct) => Some((t, pct)),
                    V::None => None,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
            ))),
            P::CmpopAssign => V::Cmp(Cmp::Eq),
            P::CmpopLt => V::Cmp(Cmp::Lt),
            P::CmpopLe => V::Cmp(Cmp::Le),
            P::CmpopGt => V::Cmp(Cmp::Gt),
            P::CmpopGe => V::Cmp(Cmp::Ge),
            P::ExptolPlusminus => V::Tol(self.expr(b + 1), matches!(self.take(b + 2), V::Pct(true))),
            P::PctPercent => V::Pct(true),
            P::ArgsArg => {
                let mut rest = list!(self, b + 1, V::Args);
                rest.push(get!(self, b, V::Arg(a) => a));
                V::Args(rest)
            }
            // `at:` is a keyword argument as any other; `at` is a word only after an expect line
            P::ArgAt => V::Arg(Arg {
                name: Some(("at".into(), self.tok(b).span)),
                value: self.expr(b + 2),
            }),
            P::ArgComponent => V::Arg(Arg {
                name: Some(("component".into(), self.tok(b).span)),
                value: self.expr(b + 2),
            }),
            P::ArgExpr => {
                let e = self.expr(b);
                V::Arg(match self.take(b + 1) {
                    V::Expr(value) => {
                        let ExprKind::Name(n) = e.kind else {
                            unreachable!("checked as `:` was taken")
                        };
                        Arg {
                            name: Some((n, e.span)),
                            value,
                        }
                    }
                    V::None => Arg { name: None, value: e },
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                })
            }
            P::ExprOr => {
                let a = self.expr(b);
                match self.take(b + 1) {
                    V::IfPart(c) => {
                        let (cond, other) = *c;
                        let span = a.span.to(other.span);
                        V::Expr(Expr {
                            kind: ExprKind::If {
                                cond: Box::new(cond),
                                then: Box::new(a),
                                other: Box::new(other),
                            },
                            span,
                        })
                    }
                    V::None => V::Expr(a),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                }
            }
            P::IfpartIf => V::IfPart(Box::new((self.expr(b + 1), self.expr(b + 3)))),
            P::OrAnd | P::AndNot | P::CmpRange | P::AddMul | P::MulUnary => {
                let a = self.expr(b);
                V::Expr(fill(a, self.take(b + 1)))
            }
            P::OrrestOr
            | P::AndrestAnd
            | P::AddrestPlus
            | P::AddrestMinus
            | P::MulrestStar
            | P::MulrestSlash => {
                let op = match p {
                    P::OrrestOr => BinOp::Or,
                    P::AndrestAnd => BinOp::And,
                    P::AddrestPlus => BinOp::Add,
                    P::AddrestMinus => BinOp::Sub,
                    P::MulrestStar => BinOp::Mul,
                    _ => BinOp::Div,
                };
                let e = self.expr(b + 1);
                push(self.take(b + 2), bin(hole(), op, e))
            }
            P::CmprestRelop => {
                let op = get!(self, b, V::Op(o) => o);
                let e = self.expr(b + 1);
                push(self.take(b + 2), bin(hole(), op, e))
            }
            P::RelopEqeq => V::Op(BinOp::Eq),
            P::RelopNe => V::Op(BinOp::Ne),
            P::RelopLt => V::Op(BinOp::Lt),
            P::RelopLe => V::Op(BinOp::Le),
            P::RelopGt => V::Op(BinOp::Gt),
            P::RelopGe => V::Op(BinOp::Ge),
            P::NotNot | P::UnaryMinus => {
                let s = self.tok(b).span;
                let e = self.expr(b + 1);
                let span = s.to(e.span);
                let e = Box::new(e);
                V::Expr(Expr {
                    kind: if p == P::NotNot {
                        ExprKind::Not(e)
                    } else {
                        ExprKind::Neg(e)
                    },
                    span,
                })
            }
            P::NotCmp | P::UnaryPower => self.take(b),
            P::RangeAdd => {
                let a = self.expr(b);
                match self.take(b + 1) {
                    V::RangeTail(inclusive, hi) => {
                        let span = a.span.to(hi.span);
                        V::Expr(Expr {
                            kind: ExprKind::Range {
                                lo: Box::new(a),
                                hi: Box::new(hi),
                                inclusive,
                            },
                            span,
                        })
                    }
                    V::None => V::Expr(a),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                }
            }
            P::RangetailDotdot => V::RangeTail(false, self.expr(b + 1)),
            P::RangetailDotdoteq => V::RangeTail(true, self.expr(b + 1)),
            P::PowerPostfix => {
                let a = self.expr(b);
                match self.take(b + 1) {
                    V::Expr(e) => V::Expr(bin(a, BinOp::Pow, e)),
                    V::None => V::Expr(a),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                }
            }
            P::PostfixAtom => {
                let a = self.expr(b);
                V::Expr(fill(a, self.take(b + 1)))
            }
            P::PostrestLparen => {
                let mut args = list!(self, b + 1, V::Args);
                args.reverse();
                let hi = self.tok(b + 2).span;
                let node = Expr {
                    span: hi,
                    kind: ExprKind::Call(Box::new(hole()), args),
                };
                push(self.take(b + 3), node)
            }
            P::PostrestDot => {
                let s = self.tok(b + 1).span;
                let n = self.text(b + 1);
                let node = Expr {
                    span: s,
                    kind: ExprKind::Attr(Box::new(hole()), n),
                };
                push(self.take(b + 2), node)
            }
            P::PostrestLbrack => {
                let hi = self.tok(b + 2).span;
                let i = self.expr(b + 1);
                let node = Expr {
                    span: hi,
                    kind: ExprKind::Index(Box::new(hole()), Box::new(i)),
                };
                push(self.take(b + 3), node)
            }
            P::AtomNum | P::AtomStr | P::AtomId | P::AtomTrue | P::AtomFalse => self.atom(p, b),
            P::AtomLparen => {
                let mut items = list!(self, b + 1, V::Exprs);
                if items.len() == 1 {
                    let mut e = items.pop().expect("one");
                    e.span = span;
                    return Ok(V::Expr(e));
                }
                items.reverse();
                V::Expr(Expr {
                    kind: ExprKind::Tuple(items),
                    span,
                })
            }
            P::AtomLbrack => V::Expr(Expr {
                kind: match self.take(b + 1) {
                    V::Kind(k) => k,
                    V::None => ExprKind::List(Vec::new()),
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                },
                span,
            }),
            P::AtomLbrace => {
                let mut entries = list!(self, b + 2, V::Map);
                entries.reverse();
                V::Expr(Expr {
                    kind: ExprKind::Map(entries),
                    span,
                })
            }
            P::ListbodyExpr => {
                let item = self.expr(b);
                V::Kind(match self.take(b + 1) {
                    V::Comp(c) => {
                        let (vars, iter, cond) = *c;
                        ExprKind::Comp {
                            item: Box::new(item),
                            vars,
                            iter: Box::new(iter),
                            cond,
                        }
                    }
                    rest => {
                        let mut rest = match rest {
                            V::Exprs(r) => r,
                            _ => Vec::new(),
                        };
                        rest.push(item);
                        rest.reverse();
                        ExprKind::List(rest)
                    }
                })
            }
            P::ListtailFor => {
                let mut vars = list!(self, b + 2, V::Names);
                vars.push(self.text(b + 1));
                vars.reverse();
                let cond = match self.take(b + 5) {
                    V::Expr(c) => Some(Box::new(c)),
                    V::None => None,
                    _ => unreachable!("rule action and grammar.ll1 disagree"),
                };
                V::Comp(Box::new((vars, self.expr(b + 4), cond)))
            }
            P::MapbodyId => {
                let mut rest = list!(self, b + 4, V::Map);
                rest.push((self.text(b), self.expr(b + 2)));
                V::Map(rest)
            }
            P::MaprestComma => self.take(b + 2),
            P::AddrestEmpty
            | P::AndrestEmpty
            | P::ArgkwEmpty
            | P::ArgsEmpty
            | P::ArgsrestEmpty
            | P::AssigntailEmpty
            | P::CmprestEmpty
            | P::CompifEmpty
            | P::DargsEmpty
            | P::DrawtailEmpty
            | P::ElsepartEmpty
            | P::ExpargsEmpty
            | P::ExpcmpEmpty
            | P::ExprsEmpty
            | P::ExprsrestEmpty
            | P::ExptolEmpty
            | P::ExpatEmpty
            | P::ExpmoreEmpty
            | P::ForvarsEmpty
            | P::FparamrestEmpty
            | P::FparamsEmpty
            | P::FpdefaultEmpty
            | P::FptypeEmpty
            | P::IfpartEmpty
            | P::KeyrestEmpty
            | P::KeysEmpty
            | P::ListbodyEmpty
            | P::ListmoreEmpty
            | P::ListrestEmpty
            | P::ListtailEmpty
            | P::MapbodyEmpty
            | P::MaprestEmpty
            | P::MulrestEmpty
            | P::OrrestEmpty
            | P::PathrestEmpty
            | P::PctEmpty
            | P::PostrestEmpty
            | P::PowtailEmpty
            | P::RangetailEmpty
            | P::SepsEmpty
            | P::StmtsEmpty
            | P::StmtsrestEmpty
            | P::TupleEmpty
            | P::TuplerestEmpty => {
                unreachable!("an empty rule leaves V::None, pushed by run")
            }
        })
    }
}

/// `keeps_first` of each rule, by number.
const KEEPS: [bool; table::NP] = {
    let mut out = [false; table::NP];
    let mut p = 0;
    while p < table::NP {
        out[p] = keeps_first(table::RULES[p]);
        p += 1;
    }
    out
};

/// Rules whose value is their first symbol's when the rest of the right side came out empty.
const fn keeps_first(p: P) -> bool {
    matches!(
        p,
        P::OrAnd
            | P::AndNot
            | P::CmpRange
            | P::AddMul
            | P::MulUnary
            | P::ExprOr
            | P::RangeAdd
            | P::PowerPostfix
            | P::PostfixAtom
            | P::NotCmp
            | P::UnaryPower
            | P::SepNl
            | P::SepSemi
            | P::ElsebodyBlock
    )
}

/// What expanding each rule pushes after its first symbol: its reduce step (unless it passes its
/// one symbol's value through) and the rest of its right side, last first.
fn expansions() -> &'static [Vec<u16>] {
    static E: std::sync::OnceLock<Vec<Vec<u16>>> = std::sync::OnceLock::new();
    E.get_or_init(|| {
        (0..table::NP)
            .map(|p| {
                let rhs = table::RHS[p];
                let mut out = Vec::new();
                if !(rhs.len() == 1 && keeps_first(table::RULES[p])) {
                    out.push(REDUCE + p as u16);
                }
                out.extend(rhs.iter().skip(1).rev().map(|x| *x as u16));
                out
            })
            .collect()
    })
}

/// Expanding a nonterminal on one lookahead, then its leftmost nonterminal on the same lookahead,
/// and so on: every rule on the way is fixed by the table, so the whole descent is one push.
struct Chain {
    /// The table has a rule here that is not the empty one.
    valid: bool,
    /// The table's rule here is the empty one, with no check.
    empty: bool,
    /// The one rule on the way with a check as it is chosen.
    enter: Option<P>,
    push: Vec<u16>,
    /// The descent ends at the lookahead itself, which is taken.
    shift: bool,
    /// The last rule is that one token, reduced as it is taken.
    leaf: Option<u8>,
}

/// `chains()[nonterminal * NT + lookahead]`.
fn chains() -> &'static [Chain] {
    static C: std::sync::OnceLock<Vec<Chain>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let expand = expansions();
        let mut out = Vec::with_capacity(table::NN * table::NT);
        for a in 0..table::NN {
            for la in 0..table::NT {
                let p0 = table::TABLE[a][la];
                let mut c = Chain {
                    valid: false,
                    empty: p0 != table::NONE && table::RHS[p0 as usize].is_empty(),
                    enter: None,
                    push: Vec::new(),
                    shift: false,
                    leaf: None,
                };
                if c.empty && table::RULES[p0 as usize] == P::ArgkwEmpty {
                    c.enter = Some(P::ArgkwEmpty);
                }
                let mut x = a;
                loop {
                    let p = table::TABLE[x][la];
                    if p == table::NONE || table::RHS[p as usize].is_empty() || c.enter.is_some() {
                        if c.valid {
                            c.push.push((table::NT_BASE as usize + x) as u16);
                        }
                        break;
                    }
                    let rule = table::RULES[p as usize];
                    if matches!(rule, P::AssigntailAssign | P::ArgkwColon | P::ArgkwEmpty) {
                        if c.valid {
                            c.push.push((table::NT_BASE as usize + x) as u16);
                            break;
                        }
                        c.enter = Some(rule);
                    }
                    c.valid = true;
                    let first = table::RHS[p as usize][0];
                    if first < table::NT_BASE
                        && table::RHS[p as usize].len() == 1
                        && expand[p as usize].len() == 1
                    {
                        c.leaf = Some(p);
                        c.shift = true;
                        break;
                    }
                    c.push.extend_from_slice(&expand[p as usize]);
                    if first < table::NT_BASE {
                        c.shift = true;
                        break;
                    }
                    x = (first - table::NT_BASE) as usize;
                }
                out.push(c);
            }
        }
        out
    })
}

/// What the parser can take next with `stop` on top of the stack: FIRST of the symbols from
/// the top down, to the first that cannot be empty.
fn expected_set(stop: u16, stack: &[u16]) -> TermSet {
    let mut set = 0u128;
    for s in std::iter::once(stop).chain(stack.iter().rev().copied()) {
        if s >= REDUCE {
            continue;
        }
        if (s as u8) < table::NT_BASE {
            set |= 1 << s;
            break;
        }
        let a = s as usize - table::NT_BASE as usize;
        set |= table::FIRST[a];
        if !table::NULLABLE[a] {
            break;
        }
    }
    TermSet(set)
}

/// Where an operand goes in a chain; its span is empty so that the spans above it are those of
/// the rest, until `fill` sets their start.
fn hole() -> Expr {
    Expr {
        kind: ExprKind::Tuple(Vec::new()),
        span: Span { lo: u32::MAX, hi: 0 },
    }
}

fn left(e: &mut Expr) -> &mut Expr {
    match &mut e.kind {
        ExprKind::Bin(_, l, _) | ExprKind::Call(l, _) | ExprKind::Attr(l, _) | ExprKind::Index(l, _) => l,
        _ => unreachable!("a chain is operators and postfix ops"),
    }
}

/// `node` (an op with a hole on its left) in front of the chain after it.
fn push(rest: V, node: Expr) -> V {
    match rest {
        V::None => V::Chain(node, 1),
        V::Chain(mut t, depth) => {
            let mut h = &mut t;
            for _ in 0..depth {
                h = left(h);
            }
            *h = node;
            V::Chain(t, depth + 1)
        }
        _ => unreachable!("rule action and grammar.ll1 disagree"),
    }
}

/// The operand `a` put in its chain's hole: left-associated, spans from `a` on.
fn fill(a: Expr, rest: V) -> Expr {
    match rest {
        V::None => a,
        V::Chain(mut t, depth) => {
            let mut h = &mut t;
            for _ in 0..depth {
                h.span.lo = a.span.lo;
                h = left(h);
            }
            *h = a;
            t
        }
        _ => unreachable!("rule action and grammar.ll1 disagree"),
    }
}

fn bin(a: Expr, op: BinOp, b: Expr) -> Expr {
    let span = a.span.to(b.span);
    Expr {
        kind: ExprKind::Bin(op, Box::new(a), Box::new(b)),
        span,
    }
}

/// Words that became keywords after Forge texts were written with them as names.
pub const RESERVED: &[&str] = &["at"];

/// Where `word` is used as a name: every such token but a keyword argument's (`at:`) and an
/// expect line's `at` clause (outside its brackets).
pub fn name_uses(toks: &[Tok], word: &str) -> Vec<Span> {
    let mut out = vec![];
    // in an expect line, the first `at` at the line's own depth is the clause's keyword; a
    // line ends at a break or `;` at that depth, or a `}` closing its block
    let mut expect: Option<i32> = None;
    let mut depth = 0i32;
    for (k, t) in toks.iter().enumerate() {
        let before_colon = matches!(toks.get(k + 1).map(|x| &x.kind), Some(TokKind::Op(":")));
        match &t.kind {
            TokKind::Nl | TokKind::Op(";") if expect == Some(depth) => expect = None,
            TokKind::Op("(" | "[" | "{") => depth += 1,
            TokKind::Op(")" | "]" | "}") => {
                depth -= 1;
                if expect.is_some_and(|e| depth < e) {
                    expect = None;
                }
            }
            TokKind::Kw("expect") => expect = Some(depth),
            TokKind::Kw(w) if *w == word && *w == "at" && expect == Some(depth) => expect = None,
            TokKind::Kw(w) if *w == word && !before_colon => out.push(t.span),
            TokKind::Id(w) if w == word && !before_colon => out.push(t.span),
            _ => {}
        }
    }
    out
}

/// A reserved word used as a name, typed, with its rename fix; None if there is none or the
/// text does not lex.
pub fn reserved(src: &str) -> Option<Error> {
    let toks = lex(src).ok()?;
    RESERVED.iter().find_map(|w| {
        name_uses(&toks, w)
            .first()
            .map(|&span| Error::Syntax(crate::error::SyntaxError::ReservedWord { span, word: w }))
    })
}

pub fn parse(src: &str) -> R<Vec<Stmt>> {
    let mut la = Vec::new();
    let toks = lex_into::<false>(src, &mut Vec::new(), &mut la)?;
    if RESERVED
        .iter()
        .any(|w| toks.iter().any(|t| t.kind == TokKind::Kw(w)))
        && let Some(e) = reserved(src)
    {
        return Err(e);
    }
    let mut p = Parser {
        stmts: Vec::with_capacity(64),
        vals: Vec::with_capacity(64),
        toks,
        la,
        k: 0,
    };
    p.run()
}

/// Every statement, nested ones too, in source order.
pub fn walk<'s>(stmts: &'s [Stmt], f: &mut dyn FnMut(&'s Stmt)) {
    for s in stmts {
        f(s);
        match &s.kind {
            StmtKind::Fn { body, .. } | StmtKind::For { body, .. } | StmtKind::Block { body, .. } => {
                walk(body, f)
            }
            StmtKind::If { then, other, .. } => {
                walk(then, f);
                if let Some(o) = other {
                    walk(o, f);
                }
            }
            _ => {}
        }
    }
}
