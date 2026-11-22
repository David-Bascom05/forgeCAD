//! A recursive-descent parser of grammar.ll1 for texts that parse: the same tree, spans and all,
//! as the table-driven parser (`Parser::run`) builds, in one pass over the tokens with no value
//! stack. It answers None at the first token the grammar or an action check (a name before `=`
//! or `:`, a positional argument after a keyword one, an instance index, a param's owner) would
//! refuse, and `parse` then runs the table parser, which says exactly what is wrong.
//!
//! Each function is the rule of the same name; where the grammar lists a rule's tail (`orrest`,
//! `argsrest`, ..) the loop here is that rule, its empty choice the loop's end. The spans are
//! the actions': a rule's span runs from its first token or expression to the last token taken
//! (`last_hi`); an operator or postfix chain's nodes run from the chain's first operand to the
//! end of their own right part.

use super::{
    Arg, BinOp, BlockKind, Cmp, Expr, ExprKind, FnParam, Key, Seg, Span, Stmt, StmtKind, Tok, TokKind, Unit,
};

type F<T> = Option<T>;

pub(super) struct Fast {
    toks: Vec<Tok>,
    k: usize,
}

/// The tree of `toks`, or None where the table parser must say what is wrong.
pub(super) fn parse(toks: Vec<Tok>) -> F<Vec<Stmt>> {
    let mut p = Fast { toks, k: 0 };
    p.seps();
    let s = p.stmts()?;
    matches!(p.kind(), TokKind::Eof).then_some(s)
}

impl Fast {
    #[inline]
    fn kind(&self) -> &TokKind {
        &self.toks[self.k].kind
    }
    #[inline]
    fn op(&self, o: &str) -> bool {
        matches!(self.kind(), TokKind::Op(x) if *x == o)
    }
    #[inline]
    fn kw(&self, w: &str) -> bool {
        matches!(self.kind(), TokKind::Kw(x) if *x == w)
    }
    #[inline]
    fn span(&self) -> Span {
        self.toks[self.k].span
    }
    /// Takes the token, as the table parser does (it never steps past EOF).
    #[inline]
    fn bump(&mut self) -> Span {
        let s = self.toks[self.k].span;
        self.k = (self.k + 1).min(self.toks.len() - 1);
        s
    }
    #[inline]
    fn last_hi(&self) -> u32 {
        self.toks[self.k.saturating_sub(1)].span.hi
    }
    fn expect_op(&mut self, o: &str) -> F<Span> {
        self.op(o).then(|| self.bump())
    }
    fn expect_kw(&mut self, w: &str) -> F<Span> {
        self.kw(w).then(|| self.bump())
    }
    /// An ID's text, taken out of its token.
    fn id(&mut self) -> F<(String, Span)> {
        let span = self.span();
        match &mut self.toks[self.k].kind {
            TokKind::Id(s) => {
                let s = std::mem::take(s);
                self.bump();
                Some((s, span))
            }
            _ => None,
        }
    }
    fn is_sep(&self) -> bool {
        matches!(self.kind(), TokKind::Nl) || self.op(";")
    }
    fn seps(&mut self) {
        while self.is_sep() {
            self.bump();
        }
    }

    // ---- statements ------------------------------------------------------------------------

    /// stmts: statements, each one's `full` running on over the separator after it; ends where
    /// no separator follows one, or at `}` or EOF.
    fn stmts(&mut self) -> F<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            if matches!(self.kind(), TokKind::Eof) || self.op("}") {
                return Some(out);
            }
            let mut s = self.stmt()?;
            if self.is_sep() {
                s.full.hi = self.bump().hi;
                self.seps();
                out.push(s);
            } else {
                out.push(s);
                return Some(out);
            }
        }
    }

    fn block(&mut self) -> F<Vec<Stmt>> {
        self.expect_op("{")?;
        self.seps();
        let s = self.stmts()?;
        self.expect_op("}")?;
        Some(s)
    }

    fn stmt(&mut self) -> F<Stmt> {
        let lo = self.span().lo;
        let kind = match self.kind() {
            TokKind::Kw("shared") => {
                self.bump();
                let (name, _) = self.id()?;
                self.expect_op("=")?;
                StmtKind::Shared {
                    name,
                    value: self.expr()?,
                }
            }
            TokKind::Kw("param") => {
                self.bump();
                let (owner, name) = self.path()?;
                self.expect_op("=")?;
                StmtKind::Param {
                    owner,
                    name,
                    value: self.expr()?,
                }
            }
            TokKind::Kw("fn") => {
                self.bump();
                let (name, _) = self.id()?;
                self.expect_op("(")?;
                let params = self.fparams()?;
                self.expect_op(")")?;
                StmtKind::Fn {
                    name,
                    params,
                    body: self.block()?,
                }
            }
            TokKind::Kw("for") => {
                self.bump();
                let vars = self.vars()?;
                self.expect_kw("in")?;
                StmtKind::For {
                    vars,
                    iter: self.expr()?,
                    body: self.block()?,
                }
            }
            TokKind::Kw("if") => {
                self.bump();
                self.if_rest()?
            }
            TokKind::Kw("return") => {
                self.bump();
                StmtKind::Return(self.expr()?)
            }
            TokKind::Kw("expect") => {
                self.bump();
                self.expect_rest()?
            }
            TokKind::Kw(w @ ("sketch" | "path" | "component")) => {
                let kind = match *w {
                    "sketch" => BlockKind::Sketch,
                    "path" => BlockKind::Path,
                    _ => BlockKind::Component,
                };
                self.bump();
                let (name, _) = self.id()?;
                self.expect_op("(")?;
                let args = self.args()?;
                self.expect_op(")")?;
                StmtKind::Block {
                    kind,
                    name,
                    args,
                    body: self.block()?,
                }
            }
            TokKind::Kw("drawing") => {
                let at = self.bump();
                if matches!(self.kind(), TokKind::Id(_)) {
                    let (name, args, body) = self.named_block()?;
                    StmtKind::Block {
                        kind: BlockKind::Drawing,
                        name,
                        args,
                        body,
                    }
                } else {
                    StmtKind::Expr(Expr {
                        kind: ExprKind::Name("drawing".into()),
                        span: at,
                    })
                }
            }
            TokKind::Kw("harness") => {
                let at = self.bump();
                if matches!(self.kind(), TokKind::Id(_)) {
                    let (name, args, body) = self.named_block()?;
                    StmtKind::Block {
                        kind: BlockKind::Harness,
                        name,
                        args,
                        body,
                    }
                } else {
                    self.expect_op("(")?;
                    let args = self.args()?;
                    self.expect_op(")")?;
                    StmtKind::Expr(Expr {
                        kind: ExprKind::Call(
                            Box::new(Expr {
                                kind: ExprKind::Name("harness".into()),
                                span: at,
                            }),
                            args,
                        ),
                        span: at,
                    })
                }
            }
            _ => {
                let e = self.expr()?;
                if self.op("=") {
                    let ExprKind::Name(name) = e.kind else {
                        return None;
                    };
                    self.bump();
                    StmtKind::Assign {
                        name,
                        value: self.expr()?,
                    }
                } else {
                    StmtKind::Expr(e)
                }
            }
        };
        let span = Span {
            lo,
            hi: self.last_hi(),
        };
        Some(Stmt {
            kind,
            span,
            full: span,
        })
    }

    /// `if`'s rest, its `if` taken: the condition, the block and an `else` part.
    fn if_rest(&mut self) -> F<StmtKind> {
        let cond = self.expr()?;
        let then = self.block()?;
        let other = if self.kw("else") {
            self.bump();
            if self.kw("if") {
                let lo = self.bump().lo;
                let kind = self.if_rest()?;
                let span = Span {
                    lo,
                    hi: self.last_hi(),
                };
                Some(vec![Stmt {
                    kind,
                    span,
                    full: span,
                }])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Some(StmtKind::If { cond, then, other })
    }

    /// drawtail and harntail's `ID dargs block`.
    fn named_block(&mut self) -> F<(String, Vec<Arg>, Vec<Stmt>)> {
        let (name, _) = self.id()?;
        let args = if self.op("(") {
            self.bump();
            let a = self.args()?;
            self.expect_op(")")?;
            a
        } else {
            Vec::new()
        };
        Some((name, args, self.block()?))
    }

    /// `ID (',' ID)*`: a loop's names.
    fn vars(&mut self) -> F<Vec<String>> {
        let mut vars = vec![self.id()?.0];
        while self.op(",") {
            self.bump();
            vars.push(self.id()?.0);
        }
        Some(vars)
    }

    fn expect_rest(&mut self) -> F<StmtKind> {
        let (metric, _) = self.id()?;
        let mut args = Vec::new();
        if self.op("(") {
            self.bump();
            while !self.op(")") {
                args.push(self.expr()?);
                if !self.op(",") {
                    break;
                }
                self.bump();
            }
            self.expect_op(")")?;
        }
        let cmp = match self.kind() {
            TokKind::Op(o @ ("=" | "<" | "<=" | ">" | ">=")) => {
                let c = match *o {
                    "=" => Cmp::Eq,
                    "<" => Cmp::Lt,
                    "<=" => Cmp::Le,
                    ">" => Cmp::Gt,
                    _ => Cmp::Ge,
                };
                self.bump();
                let v = self.expr()?;
                let tol = if self.op("+-") {
                    self.bump();
                    let t = self.unary()?;
                    let pct = self.op("%");
                    if pct {
                        self.bump();
                    }
                    Some((t, pct))
                } else {
                    None
                };
                Some((c, v, tol))
            }
            _ => None,
        };
        let mut at = Vec::new();
        if self.kw("at") {
            self.bump();
            loop {
                let segs = self.segs()?;
                self.expect_op("=")?;
                at.push((segs, self.expr()?));
                if !self.op(",") {
                    break;
                }
                self.bump();
            }
        }
        Some(StmtKind::Expect {
            metric,
            args,
            cmp,
            at,
        })
    }

    /// `ID keys ('.' ID keys)*`: a path's segments, in order.
    fn segs(&mut self) -> F<Vec<Seg>> {
        let mut segs = Vec::new();
        loop {
            let (name, _) = self.id()?;
            let index = self.keys()?;
            segs.push(Seg { name, index });
            if !self.op(".") {
                return Some(segs);
            }
            self.bump();
        }
    }

    /// A param's path: its owner's segments and its name (an owner, and no index on the name).
    fn path(&mut self) -> F<(Vec<Seg>, String)> {
        let mut segs = self.segs()?;
        let last = segs.pop()?;
        if segs.is_empty() || !last.index.is_empty() {
            return None;
        }
        Some((segs, last.name))
    }

    fn keys(&mut self) -> F<Vec<Key>> {
        let mut keys = Vec::new();
        if !self.op("[") {
            return Some(keys);
        }
        self.bump();
        loop {
            keys.push(match &self.toks[self.k].kind {
                TokKind::Num(v, Unit::None) if v.fract() == 0.0 => {
                    let k = Key::Ix(*v as i64);
                    self.bump();
                    k
                }
                TokKind::Id(_) => Key::Label(self.id()?.0),
                _ => return None,
            });
            if !self.op(",") {
                break;
            }
            self.bump();
        }
        self.expect_op("]")?;
        Some(keys)
    }

    fn fparams(&mut self) -> F<Vec<FnParam>> {
        let mut ps = Vec::new();
        while matches!(self.kind(), TokKind::Id(_)) {
            let (name, _) = self.id()?;
            let ty = if self.op(":") {
                self.bump();
                Some(self.id()?.0)
            } else {
                None
            };
            let default = if self.op("=") {
                self.bump();
                Some(self.expr()?)
            } else {
                None
            };
            ps.push(FnParam { name, ty, default });
            if !self.op(",") {
                break;
            }
            self.bump();
        }
        Some(ps)
    }

    /// args: each argument, and a `,` after each but maybe the last.
    fn args(&mut self) -> F<Vec<Arg>> {
        let mut args: Vec<Arg> = Vec::new();
        loop {
            if self.op(")") {
                return Some(args);
            }
            let arg = match self.kind() {
                TokKind::Kw(w @ ("at" | "component")) => {
                    let w: &'static str = w;
                    let s = self.bump();
                    self.expect_op(":")?;
                    Arg {
                        name: Some((w.into(), s)),
                        value: self.expr()?,
                    }
                }
                _ => {
                    let e = self.expr()?;
                    if self.op(":") {
                        let ExprKind::Name(n) = e.kind else {
                            return None;
                        };
                        self.bump();
                        Arg {
                            name: Some((n, e.span)),
                            value: self.expr()?,
                        }
                    } else {
                        // a positional argument after a keyword one: the table parser's error
                        if args.last().is_some_and(|a| a.name.is_some()) {
                            return None;
                        }
                        Arg { name: None, value: e }
                    }
                }
            };
            args.push(arg);
            if !self.op(",") {
                return Some(args);
            }
            self.bump();
        }
    }

    // ---- expressions ------------------------------------------------------------------------

    pub(super) fn expr(&mut self) -> F<Expr> {
        let a = self.or()?;
        if !self.kw("if") {
            return Some(a);
        }
        self.bump();
        let cond = self.or()?;
        self.expect_kw("else")?;
        let other = self.expr()?;
        let span = a.span.to(other.span);
        Some(Expr {
            kind: ExprKind::If {
                cond: Box::new(cond),
                then: Box::new(a),
                other: Box::new(other),
            },
            span,
        })
    }

    /// A left-associated node: `a op b`, from the chain's first operand to `b`'s end.
    fn chain(a: Expr, op: BinOp, b: Expr) -> Expr {
        let span = Span {
            lo: a.span.lo,
            hi: b.span.hi,
        };
        Expr {
            kind: ExprKind::Bin(op, Box::new(a), Box::new(b)),
            span,
        }
    }

    fn or(&mut self) -> F<Expr> {
        let mut a = self.and()?;
        while self.kw("or") {
            self.bump();
            let b = self.and()?;
            a = Self::chain(a, BinOp::Or, b);
        }
        Some(a)
    }

    fn and(&mut self) -> F<Expr> {
        let mut a = self.not()?;
        while self.kw("and") {
            self.bump();
            let b = self.not()?;
            a = Self::chain(a, BinOp::And, b);
        }
        Some(a)
    }

    fn not(&mut self) -> F<Expr> {
        if self.kw("not") {
            let s = self.bump();
            let e = self.not()?;
            let span = s.to(e.span);
            return Some(Expr {
                kind: ExprKind::Not(Box::new(e)),
                span,
            });
        }
        self.cmp()
    }

    fn cmp(&mut self) -> F<Expr> {
        let mut a = self.range()?;
        loop {
            let op = match self.kind() {
                TokKind::Op("==") => BinOp::Eq,
                TokKind::Op("!=") => BinOp::Ne,
                TokKind::Op("<") => BinOp::Lt,
                TokKind::Op("<=") => BinOp::Le,
                TokKind::Op(">") => BinOp::Gt,
                TokKind::Op(">=") => BinOp::Ge,
                _ => return Some(a),
            };
            self.bump();
            let b = self.range()?;
            a = Self::chain(a, op, b);
        }
    }

    fn range(&mut self) -> F<Expr> {
        let a = self.add()?;
        let inclusive = match self.kind() {
            TokKind::Op("..") => false,
            TokKind::Op("..=") => true,
            _ => return Some(a),
        };
        self.bump();
        let hi = self.add()?;
        let span = a.span.to(hi.span);
        Some(Expr {
            kind: ExprKind::Range {
                lo: Box::new(a),
                hi: Box::new(hi),
                inclusive,
            },
            span,
        })
    }

    fn add(&mut self) -> F<Expr> {
        let mut a = self.mul()?;
        loop {
            let op = match self.kind() {
                TokKind::Op("+") => BinOp::Add,
                TokKind::Op("-") => BinOp::Sub,
                _ => return Some(a),
            };
            self.bump();
            let b = self.mul()?;
            a = Self::chain(a, op, b);
        }
    }

    fn mul(&mut self) -> F<Expr> {
        let mut a = self.unary()?;
        loop {
            let op = match self.kind() {
                TokKind::Op("*") => BinOp::Mul,
                TokKind::Op("/") => BinOp::Div,
                _ => return Some(a),
            };
            self.bump();
            let b = self.unary()?;
            a = Self::chain(a, op, b);
        }
    }

    fn unary(&mut self) -> F<Expr> {
        if self.op("-") {
            let s = self.bump();
            let e = self.unary()?;
            let span = s.to(e.span);
            return Some(Expr {
                kind: ExprKind::Neg(Box::new(e)),
                span,
            });
        }
        let a = self.postfix()?;
        if !self.op("^") {
            return Some(a);
        }
        self.bump();
        let e = self.unary()?;
        let span = a.span.to(e.span);
        Some(Expr {
            kind: ExprKind::Bin(BinOp::Pow, Box::new(a), Box::new(e)),
            span,
        })
    }

    fn postfix(&mut self) -> F<Expr> {
        let mut a = self.atom()?;
        let lo = a.span.lo;
        loop {
            let (kind, hi) = match self.kind() {
                TokKind::Op("(") => {
                    self.bump();
                    let args = self.args()?;
                    let hi = self.expect_op(")")?.hi;
                    (ExprKind::Call(Box::new(a), args), hi)
                }
                TokKind::Op(".") => {
                    self.bump();
                    let (n, s) = self.id()?;
                    (ExprKind::Attr(Box::new(a), n), s.hi)
                }
                TokKind::Op("[") => {
                    self.bump();
                    let i = self.expr()?;
                    let hi = self.expect_op("]")?.hi;
                    (ExprKind::Index(Box::new(a), Box::new(i)), hi)
                }
                _ => return Some(a),
            };
            a = Expr {
                kind,
                span: Span { lo, hi },
            };
        }
    }

    fn atom(&mut self) -> F<Expr> {
        let span = self.span();
        let kind = match &mut self.toks[self.k].kind {
            TokKind::Num(v, u) => ExprKind::Num(*v, *u),
            TokKind::Str(s) => ExprKind::Str(std::mem::take(s)),
            TokKind::Id(s) => ExprKind::Name(std::mem::take(s)),
            TokKind::Kw("true") => ExprKind::Bool(true),
            TokKind::Kw("false") => ExprKind::Bool(false),
            TokKind::Op("(") => return self.paren(),
            TokKind::Op("[") => return self.list(),
            TokKind::Op("{") => return self.map(),
            _ => return None,
        };
        self.bump();
        Some(Expr { kind, span })
    }

    /// `( tuple )`: one item is that item, its span the brackets'; else a tuple.
    fn paren(&mut self) -> F<Expr> {
        let lo = self.bump().lo;
        let mut items = Vec::new();
        while !self.op(")") {
            items.push(self.expr()?);
            if !self.op(",") {
                break;
            }
            self.bump();
        }
        let hi = self.expect_op(")")?.hi;
        let span = Span { lo, hi };
        if items.len() == 1 {
            let mut e = items.pop().expect("one");
            e.span = span;
            return Some(e);
        }
        Some(Expr {
            kind: ExprKind::Tuple(items),
            span,
        })
    }

    /// `[ listbody ]`: a list, or a comprehension.
    fn list(&mut self) -> F<Expr> {
        let lo = self.bump().lo;
        let kind = if self.op("]") {
            ExprKind::List(Vec::new())
        } else {
            let item = self.expr()?;
            if self.kw("for") {
                self.bump();
                let vars = self.vars()?;
                self.expect_kw("in")?;
                let iter = self.or()?;
                let cond = if self.kw("if") {
                    self.bump();
                    Some(Box::new(self.or()?))
                } else {
                    None
                };
                ExprKind::Comp {
                    item: Box::new(item),
                    vars,
                    iter: Box::new(iter),
                    cond,
                }
            } else {
                let mut items = vec![item];
                if self.op(",") {
                    self.bump();
                    while !self.op("]") {
                        items.push(self.expr()?);
                        if !self.op(",") {
                            break;
                        }
                        self.bump();
                    }
                }
                ExprKind::List(items)
            }
        };
        let hi = self.expect_op("]")?.hi;
        Some(Expr {
            kind,
            span: Span { lo, hi },
        })
    }

    /// `{ seps mapbody }`: labelled values.
    fn map(&mut self) -> F<Expr> {
        let lo = self.bump().lo;
        self.seps();
        let mut entries = Vec::new();
        while matches!(self.kind(), TokKind::Id(_)) {
            let (name, _) = self.id()?;
            self.expect_op(":")?;
            entries.push((name, self.expr()?));
            self.seps();
            if !self.op(",") {
                break;
            }
            self.bump();
            self.seps();
        }
        let hi = self.expect_op("}")?.hi;
        Some(Expr {
            kind: ExprKind::Map(entries),
            span: Span { lo, hi },
        })
    }
}
