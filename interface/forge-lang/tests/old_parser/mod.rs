//! The hand-written parser grammar.ll1 replaced (forge-lang at 21d7fd16), kept as the oracle the
//! table-driven parser is checked against. Only `expected` in its errors is new: always empty.

#![allow(dead_code, clippy::all)]

use forge_lang::error::{Error, SyntaxError};
use forge_lang::syntax::*;

pub const OPS: [&str; 26] = [
    "..=", "+-", "..", "==", "!=", "<=", ">=", "+", "-", "*", "/", "^", "%", "(", ")", "[", "]", "{", "}",
    ",", ":", ".", "=", "<", ">", ";",
];

pub fn lex(src: &str) -> Result<Vec<Tok>, Error> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut depth: i32 = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            if depth == 0 {
                out.push(Tok {
                    kind: TokKind::Nl,
                    span: Span::new(i, i + 1),
                });
            }
            i += 1;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            i += 1;
            continue;
        }
        if c == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
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
            let kind = match KEYWORDS.iter().find(|k| **k == w) {
                Some(k) => TokKind::Kw(k),
                None => TokKind::Id(w.to_string()),
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
            out.push(Tok {
                kind: TokKind::Str(src[s + 1..i - 1].to_string()),
                span: Span::new(s, i),
            });
            continue;
        }
        match OPS.iter().find(|o| src[i..].starts_with(**o)) {
            Some(op) => {
                match *op {
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
    out.push(Tok {
        kind: TokKind::Eof,
        span: Span::new(b.len(), b.len()),
    });
    Ok(out)
}

pub struct Parser<'a> {
    t: &'a [Tok],
    k: usize,
}

type R<T> = Result<T, Error>;

fn err<T>(e: SyntaxError) -> R<T> {
    Err(Error::Syntax(e))
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Tok {
        &self.t[self.k.min(self.t.len() - 1)]
    }
    fn bump(&mut self) -> Tok {
        let t = self.peek().clone();
        if self.k < self.t.len() - 1 {
            self.k += 1;
        }
        t
    }
    fn is_op(&self, o: &str) -> bool {
        matches!(self.peek().kind, TokKind::Op(x) if x == o)
    }
    fn is_kw(&self, w: &str) -> bool {
        matches!(self.peek().kind, TokKind::Kw(x) if x == w)
    }
    fn eat_op(&mut self, o: &str) -> bool {
        if self.is_op(o) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn want_op(&mut self, o: &'static str) -> R<Span> {
        if self.is_op(o) {
            Ok(self.bump().span)
        } else {
            err(SyntaxError::Expected {
                expected: TermSet(0),
                what: o,
                span: self.peek().span,
            })
        }
    }
    fn want_kw(&mut self, w: &'static str) -> R<Span> {
        if self.is_kw(w) {
            Ok(self.bump().span)
        } else {
            err(SyntaxError::Expected {
                expected: TermSet(0),
                what: w,
                span: self.peek().span,
            })
        }
    }
    fn ident(&mut self) -> R<(String, Span)> {
        match self.peek().kind.clone() {
            TokKind::Id(s) => Ok((s, self.bump().span)),
            _ => err(SyntaxError::Expected {
                expected: TermSet(0),
                what: "a name",
                span: self.peek().span,
            }),
        }
    }
    fn last_hi(&self) -> u32 {
        self.t[self.k.saturating_sub(1)].span.hi
    }
    fn seps(&mut self) {
        while matches!(self.peek().kind, TokKind::Nl) || self.is_op(";") {
            self.bump();
        }
    }

    pub fn program(&mut self) -> R<Vec<Stmt>> {
        self.seps();
        let mut out = Vec::new();
        while !matches!(self.peek().kind, TokKind::Eof) {
            out.push(self.stmt()?);
        }
        Ok(out)
    }

    fn end_stmt(&mut self) -> R<u32> {
        match self.peek().kind {
            TokKind::Nl | TokKind::Op(";") => {
                let hi = self.bump().span.hi;
                self.seps();
                Ok(hi)
            }
            TokKind::Eof | TokKind::Op("}") => Ok(self.last_hi()),
            _ => err(SyntaxError::Expected {
                expected: TermSet(0),
                what: "end of statement",
                span: self.peek().span,
            }),
        }
    }

    fn block(&mut self) -> R<Vec<Stmt>> {
        self.want_op("{")?;
        self.seps();
        let mut out = Vec::new();
        while !self.is_op("}") {
            if matches!(self.peek().kind, TokKind::Eof) {
                return err(SyntaxError::Expected {
                    expected: TermSet(0),
                    what: "}",
                    span: self.peek().span,
                });
            }
            out.push(self.stmt()?);
        }
        self.want_op("}")?;
        Ok(out)
    }

    fn stmt(&mut self) -> R<Stmt> {
        let lo = self.peek().span.lo;
        let kind = self.stmt_kind()?;
        let hi = self.last_hi();
        let full_hi = self.end_stmt()?;
        Ok(Stmt {
            kind,
            span: Span { lo, hi },
            full: Span { lo, hi: full_hi },
        })
    }

    fn index_list(&mut self) -> R<Vec<Key>> {
        let mut out = Vec::new();
        if self.eat_op("[") {
            loop {
                match self.peek().kind.clone() {
                    TokKind::Num(v, Unit::None) if v.fract() == 0.0 => {
                        self.bump();
                        out.push(Key::Ix(v as i64));
                    }
                    TokKind::Id(l) => {
                        self.bump();
                        out.push(Key::Label(l));
                    }
                    _ => {
                        return err(SyntaxError::Expected {
                            expected: TermSet(0),
                            what: "an instance index",
                            span: self.peek().span,
                        });
                    }
                }
                if !self.eat_op(",") {
                    break;
                }
            }
            self.want_op("]")?;
        }
        Ok(out)
    }

    fn stmt_kind(&mut self) -> R<StmtKind> {
        match self.peek().kind.clone() {
            TokKind::Kw("shared") => {
                self.bump();
                let (name, _) = self.ident()?;
                self.want_op("=")?;
                Ok(StmtKind::Shared {
                    name,
                    value: self.expr()?,
                })
            }
            TokKind::Kw("param") => {
                let at = self.bump().span;
                let mut segs = Vec::new();
                loop {
                    let (name, _) = self.ident()?;
                    let index = self.index_list()?;
                    segs.push(Seg { name, index });
                    if !self.eat_op(".") {
                        break;
                    }
                }
                let last = segs.pop().expect("one segment");
                if segs.is_empty() || !last.index.is_empty() {
                    return err(SyntaxError::ParamWithoutOwner {
                        span: Span::new(at.lo as usize, self.last_hi() as usize),
                        name: last.name,
                    });
                }
                self.want_op("=")?;
                Ok(StmtKind::Param {
                    owner: segs,
                    name: last.name,
                    value: self.expr()?,
                })
            }
            TokKind::Kw("fn") => {
                self.bump();
                let (name, _) = self.ident()?;
                self.want_op("(")?;
                let mut params = Vec::new();
                while !self.is_op(")") {
                    let (pn, _) = self.ident()?;
                    let ty = if self.eat_op(":") {
                        Some(self.ident()?.0)
                    } else {
                        None
                    };
                    let default = if self.eat_op("=") {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    params.push(FnParam {
                        name: pn,
                        ty,
                        default,
                    });
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.want_op(")")?;
                let body = self.block()?;
                Ok(StmtKind::Fn { name, params, body })
            }
            TokKind::Kw("for") => {
                self.bump();
                let mut vars = vec![self.ident()?.0];
                while self.eat_op(",") {
                    vars.push(self.ident()?.0);
                }
                self.want_kw("in")?;
                let iter = self.expr()?;
                let body = self.block()?;
                Ok(StmtKind::For { vars, iter, body })
            }
            TokKind::Kw("if") => self.if_stmt(),
            TokKind::Kw("return") => {
                self.bump();
                Ok(StmtKind::Return(self.expr()?))
            }
            TokKind::Kw("expect") => {
                self.bump();
                let (metric, _) = self.ident()?;
                let mut args = Vec::new();
                if self.eat_op("(") {
                    while !self.is_op(")") {
                        args.push(self.expr()?);
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.want_op(")")?;
                }
                let cmp = match self.peek().kind {
                    TokKind::Op("=") => Some(Cmp::Eq),
                    TokKind::Op("<") => Some(Cmp::Lt),
                    TokKind::Op("<=") => Some(Cmp::Le),
                    TokKind::Op(">") => Some(Cmp::Gt),
                    TokKind::Op(">=") => Some(Cmp::Ge),
                    _ => None,
                };
                let cmp = match cmp {
                    Some(c) => {
                        self.bump();
                        let v = self.expr()?;
                        let tol = if self.eat_op("+-") {
                            let t = self.unary()?;
                            let pct = self.eat_op("%");
                            Some((t, pct))
                        } else {
                            None
                        };
                        Some((c, v, tol))
                    }
                    None => None,
                };
                let mut at = Vec::new();
                if self.is_kw("at") {
                    self.bump();
                    loop {
                        let mut segs = Vec::new();
                        loop {
                            let (name, _) = self.ident()?;
                            let index = self.index_list()?;
                            segs.push(Seg { name, index });
                            if !self.eat_op(".") {
                                break;
                            }
                        }
                        self.want_op("=")?;
                        at.push((segs, self.expr()?));
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                }
                Ok(StmtKind::Expect {
                    metric,
                    args,
                    cmp,
                    at,
                })
            }
            TokKind::Kw(w @ ("sketch" | "path")) => {
                self.bump();
                let (name, _) = self.ident()?;
                self.want_op("(")?;
                let args = self.args()?;
                let body = self.block()?;
                Ok(StmtKind::Block {
                    kind: if w == "sketch" {
                        BlockKind::Sketch
                    } else {
                        BlockKind::Path
                    },
                    name,
                    args,
                    body,
                })
            }
            _ => {
                let e = self.expr()?;
                if self.eat_op("=") {
                    let ExprKind::Name(name) = e.kind else {
                        return err(SyntaxError::AssignToNonName { span: e.span });
                    };
                    Ok(StmtKind::Assign {
                        name,
                        value: self.expr()?,
                    })
                } else {
                    Ok(StmtKind::Expr(e))
                }
            }
        }
    }

    fn if_stmt(&mut self) -> R<StmtKind> {
        self.want_kw("if")?;
        let cond = self.expr()?;
        let then = self.block()?;
        let other = if self.is_kw("else") {
            self.bump();
            if self.is_kw("if") {
                let lo = self.peek().span.lo;
                let k = self.if_stmt()?;
                let sp = Span {
                    lo,
                    hi: self.last_hi(),
                };
                Some(vec![Stmt {
                    kind: k,
                    span: sp,
                    full: sp,
                }])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(StmtKind::If { cond, then, other })
    }

    /// Arguments up to and including `)`, the `(` already taken.
    fn args(&mut self) -> R<Vec<Arg>> {
        let mut out = Vec::new();
        let mut kw = false;
        while !self.is_op(")") {
            if self.is_kw("at") {
                let span = self.bump().span;
                self.want_op(":")?;
                kw = true;
                out.push(Arg {
                    name: Some(("at".into(), span)),
                    value: self.expr()?,
                });
                if !self.eat_op(",") {
                    break;
                }
                continue;
            }
            let e = self.expr()?;
            if self.eat_op(":") {
                let ExprKind::Name(n) = e.kind else {
                    return err(SyntaxError::KeywordNotName { span: e.span });
                };
                kw = true;
                out.push(Arg {
                    name: Some((n, e.span)),
                    value: self.expr()?,
                });
            } else {
                if kw {
                    return err(SyntaxError::PositionalAfterKeyword { span: e.span });
                }
                out.push(Arg { name: None, value: e });
            }
            if !self.eat_op(",") {
                break;
            }
        }
        self.want_op(")")?;
        Ok(out)
    }

    pub fn expr(&mut self) -> R<Expr> {
        let a = self.or()?;
        if self.is_kw("if") {
            self.bump();
            let c = self.or()?;
            self.want_kw("else")?;
            let b = self.expr()?;
            let span = a.span.to(b.span);
            return Ok(Expr {
                kind: ExprKind::If {
                    cond: Box::new(c),
                    then: Box::new(a),
                    other: Box::new(b),
                },
                span,
            });
        }
        Ok(a)
    }

    fn bin(a: Expr, op: BinOp, b: Expr) -> Expr {
        let span = a.span.to(b.span);
        Expr {
            kind: ExprKind::Bin(op, Box::new(a), Box::new(b)),
            span,
        }
    }

    fn or(&mut self) -> R<Expr> {
        let mut a = self.and()?;
        while self.is_kw("or") {
            self.bump();
            let b = self.and()?;
            a = Self::bin(a, BinOp::Or, b);
        }
        Ok(a)
    }

    fn and(&mut self) -> R<Expr> {
        let mut a = self.not()?;
        while self.is_kw("and") {
            self.bump();
            let b = self.not()?;
            a = Self::bin(a, BinOp::And, b);
        }
        Ok(a)
    }

    fn not(&mut self) -> R<Expr> {
        if self.is_kw("not") {
            let s = self.bump().span;
            let e = self.not()?;
            let span = s.to(e.span);
            return Ok(Expr {
                kind: ExprKind::Not(Box::new(e)),
                span,
            });
        }
        self.cmp()
    }

    fn cmp(&mut self) -> R<Expr> {
        let mut a = self.range()?;
        loop {
            let op = match self.peek().kind {
                TokKind::Op("==") => BinOp::Eq,
                TokKind::Op("!=") => BinOp::Ne,
                TokKind::Op("<") => BinOp::Lt,
                TokKind::Op("<=") => BinOp::Le,
                TokKind::Op(">") => BinOp::Gt,
                TokKind::Op(">=") => BinOp::Ge,
                _ => return Ok(a),
            };
            self.bump();
            let b = self.range()?;
            a = Self::bin(a, op, b);
        }
    }

    fn range(&mut self) -> R<Expr> {
        let a = self.add()?;
        let inclusive = match self.peek().kind {
            TokKind::Op("..") => false,
            TokKind::Op("..=") => true,
            _ => return Ok(a),
        };
        self.bump();
        let b = self.add()?;
        let span = a.span.to(b.span);
        Ok(Expr {
            kind: ExprKind::Range {
                lo: Box::new(a),
                hi: Box::new(b),
                inclusive,
            },
            span,
        })
    }

    fn add(&mut self) -> R<Expr> {
        let mut a = self.mul()?;
        loop {
            let op = match self.peek().kind {
                TokKind::Op("+") => BinOp::Add,
                TokKind::Op("-") => BinOp::Sub,
                _ => return Ok(a),
            };
            self.bump();
            let b = self.mul()?;
            a = Self::bin(a, op, b);
        }
    }

    fn mul(&mut self) -> R<Expr> {
        let mut a = self.unary()?;
        loop {
            let op = match self.peek().kind {
                TokKind::Op("*") => BinOp::Mul,
                TokKind::Op("/") => BinOp::Div,
                _ => return Ok(a),
            };
            self.bump();
            let b = self.unary()?;
            a = Self::bin(a, op, b);
        }
    }

    fn unary(&mut self) -> R<Expr> {
        if self.is_op("-") {
            let s = self.bump().span;
            let e = self.unary()?;
            let span = s.to(e.span);
            return Ok(Expr {
                kind: ExprKind::Neg(Box::new(e)),
                span,
            });
        }
        self.power()
    }

    fn power(&mut self) -> R<Expr> {
        let a = self.postfix()?;
        if self.eat_op("^") {
            let b = self.unary()?;
            return Ok(Self::bin(a, BinOp::Pow, b));
        }
        Ok(a)
    }

    fn postfix(&mut self) -> R<Expr> {
        let mut e = self.atom()?;
        loop {
            if self.eat_op("(") {
                let args = self.args()?;
                let span = Span {
                    lo: e.span.lo,
                    hi: self.last_hi(),
                };
                e = Expr {
                    kind: ExprKind::Call(Box::new(e), args),
                    span,
                };
            } else if self.eat_op(".") {
                let (n, s) = self.ident()?;
                let span = e.span.to(s);
                e = Expr {
                    kind: ExprKind::Attr(Box::new(e), n),
                    span,
                };
            } else if self.eat_op("[") {
                let i = self.expr()?;
                let hi = self.want_op("]")?;
                let span = e.span.to(hi);
                e = Expr {
                    kind: ExprKind::Index(Box::new(e), Box::new(i)),
                    span,
                };
            } else {
                return Ok(e);
            }
        }
    }

    fn atom(&mut self) -> R<Expr> {
        let t = self.peek().clone();
        let kind = match t.kind {
            TokKind::Num(v, u) => {
                self.bump();
                ExprKind::Num(v, u)
            }
            TokKind::Str(s) => {
                self.bump();
                ExprKind::Str(s)
            }
            TokKind::Kw("true") => {
                self.bump();
                ExprKind::Bool(true)
            }
            TokKind::Kw("false") => {
                self.bump();
                ExprKind::Bool(false)
            }
            TokKind::Id(n) => {
                self.bump();
                ExprKind::Name(n)
            }
            TokKind::Op("(") => {
                self.bump();
                let mut items = Vec::new();
                while !self.is_op(")") {
                    items.push(self.expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
                let hi = self.want_op(")")?;
                if items.len() == 1 {
                    let mut e = items.pop().expect("one");
                    e.span = t.span.to(hi);
                    return Ok(e);
                }
                return Ok(Expr {
                    kind: ExprKind::Tuple(items),
                    span: t.span.to(hi),
                });
            }
            TokKind::Op("{") => {
                self.bump();
                let mut entries = Vec::new();
                self.seps();
                while !self.is_op("}") {
                    let (k, _) = self.ident()?;
                    self.want_op(":")?;
                    entries.push((k, self.expr()?));
                    self.seps();
                    if !self.eat_op(",") {
                        break;
                    }
                    self.seps();
                }
                let hi = self.want_op("}")?;
                return Ok(Expr {
                    kind: ExprKind::Map(entries),
                    span: t.span.to(hi),
                });
            }
            TokKind::Op("[") => {
                self.bump();
                let mut items = Vec::new();
                while !self.is_op("]") {
                    items.push(self.expr()?);
                    if self.is_kw("for") {
                        self.bump();
                        let mut vars = vec![self.ident()?.0];
                        while self.eat_op(",") {
                            vars.push(self.ident()?.0);
                        }
                        self.want_kw("in")?;
                        let iter = self.or()?;
                        let cond = if self.is_kw("if") {
                            self.bump();
                            Some(Box::new(self.or()?))
                        } else {
                            None
                        };
                        let hi = self.want_op("]")?;
                        return Ok(Expr {
                            kind: ExprKind::Comp {
                                item: Box::new(items.pop().expect("one")),
                                vars,
                                iter: Box::new(iter),
                                cond,
                            },
                            span: t.span.to(hi),
                        });
                    }
                    if !self.eat_op(",") {
                        break;
                    }
                }
                let hi = self.want_op("]")?;
                return Ok(Expr {
                    kind: ExprKind::List(items),
                    span: t.span.to(hi),
                });
            }
            _ => {
                return err(SyntaxError::Expected {
                    expected: TermSet(0),
                    what: "a value",
                    span: t.span,
                });
            }
        };
        Ok(Expr { kind, span: t.span })
    }
}

pub fn parse(src: &str) -> R<Vec<Stmt>> {
    let toks = lex(src)?;
    if let Some(e) = forge_lang::syntax::reserved(src) {
        return Err(e);
    }
    Parser { t: &toks, k: 0 }.program()
}
