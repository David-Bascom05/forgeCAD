//! The parser: recursive descent of grammar.ll1 in one pass over the tokens, with no value stack.
//! Every text gets the table-driven parser's answer (`Parser::run`): the same tree, spans and
//! all, where it parses, and the same error where it does not. It is the recursive descent
//! `rdS`/`rdF` of research/forge-lang/lean/ForgeLang/LL1/Driver.lean, which `rd_eq_run` proves to
//! give the table machine's accepts and errors on every input, with the actions on top:
//!
//! - each nonterminal is a function that starts with `pick`: TABLE's rule for the lookahead; where
//!   TABLE has none, the miss is noted (its position, and the expected set FIRST of the
//!   nonterminal, with `fk` if it can be empty) and the nonterminal's empty rule (EMPTY) taken, or
//!   the parse stops there if it has none. Each function's `fk` is FIRST of what follows its call
//!   in the rule that calls it, with the caller's own `fk` where that rest can be empty (`after`).
//! - after a miss the descent goes on as before, never taking a token: the first terminal it
//!   meets ends the parse, with that terminal as what was wanted.
//! - a terminal that is not the lookahead, before any miss, ends the parse wanting it alone.
//! - the actions' five checks (a name before `=` or `:`, a positional argument after a keyword
//!   one, an instance index, a param's owner) are made where the table parser makes them: as the
//!   rule is picked (`enter`) or as it is done (`reduce`), so in missed mode too.
//!
//! A right-recursive rule's tail (`stmtsrest`, `orrest`, `argsrest`, ..) is a loop: its recursive
//! call is the rule's last symbol, so its `fk` is the loop's own. The spans are the actions': a
//! rule's span runs from its first token or expression to the last token taken (`last_hi`); an
//! operator or postfix chain's nodes run from the chain's first operand to the end of their own
//! right part.
//!
//! The core keeps to plain enums, loops and tables (no trait objects, no unsafe), so that it can
//! be translated and proven equal to `rdS` directly.

use super::table::{self, P, t};
use super::{
    Arg, BinOp, BlockKind, Cmp, Error, Expr, ExprKind, FnParam, Key, Seg, Span, Stmt, StmtKind, SyntaxError,
    TermSet, Tok, TokKind, Unit,
};

/// A parse's answer so far, or the error it stopped with.
type F<T> = Result<T, Box<Error>>;

/// A nonterminal's number, by name.
const fn nt(name: &str) -> usize {
    let mut k = 0;
    while k < table::NN {
        let (a, b) = (table::NONTERMS[k].as_bytes(), name.as_bytes());
        if a.len() == b.len() {
            let mut i = 0;
            while i < a.len() && a[i] == b[i] {
                i += 1;
            }
            if i == a.len() {
                return k;
            }
        }
        k += 1;
    }
    panic!("not a nonterminal of grammar.ll1")
}

const PROGRAM: usize = nt("program");
const BLOCK: usize = nt("block");
const STMTS: usize = nt("stmts");
const STMTSREST: usize = nt("stmtsrest");
const SEPS: usize = nt("seps");
const SEP: usize = nt("sep");
const STMT: usize = nt("stmt");
const ASSIGNTAIL: usize = nt("assigntail");
const DRAWTAIL: usize = nt("drawtail");
const DARGS: usize = nt("dargs");
const HARNTAIL: usize = nt("harntail");
const PATH: usize = nt("path");
const PATHREST: usize = nt("pathrest");
const KEYS: usize = nt("keys");
const KEY: usize = nt("key");
const KEYREST: usize = nt("keyrest");
const FPARAMS: usize = nt("fparams");
const FPARAMREST: usize = nt("fparamrest");
const FPARAM: usize = nt("fparam");
const FPTYPE: usize = nt("fptype");
const FPDEFAULT: usize = nt("fpdefault");
const FORVARS: usize = nt("forvars");
const ELSEPART: usize = nt("elsepart");
const ELSEBODY: usize = nt("elsebody");
const EXPARGS: usize = nt("expargs");
const EXPRS: usize = nt("exprs");
const EXPRSREST: usize = nt("exprsrest");
const EXPCMP: usize = nt("expcmp");
const CMPOP: usize = nt("cmpop");
const EXPTOL: usize = nt("exptol");
const PCT: usize = nt("pct");
const EXPAT: usize = nt("expat");
const EXPMORE: usize = nt("expmore");
const ATPATH: usize = nt("atpath");
const ARGS: usize = nt("args");
const ARGSREST: usize = nt("argsrest");
const ARG: usize = nt("arg");
const ARGKW: usize = nt("argkw");
const EXPR: usize = nt("expr");
const IFPART: usize = nt("ifpart");
const OR: usize = nt("or");
const ORREST: usize = nt("orrest");
const AND: usize = nt("and");
const ANDREST: usize = nt("andrest");
const NOT: usize = nt("not");
const CMP: usize = nt("cmp");
const CMPREST: usize = nt("cmprest");
const RELOP: usize = nt("relop");
const RANGE: usize = nt("range");
const RANGETAIL: usize = nt("rangetail");
const ADD: usize = nt("add");
const ADDREST: usize = nt("addrest");
const MUL: usize = nt("mul");
const MULREST: usize = nt("mulrest");
const UNARY: usize = nt("unary");
const POWER: usize = nt("power");
const POWTAIL: usize = nt("powtail");
const POSTFIX: usize = nt("postfix");
const POSTREST: usize = nt("postrest");
const ATOM: usize = nt("atom");
const TUPLE: usize = nt("tuple");
const TUPLEREST: usize = nt("tuplerest");
const LISTBODY: usize = nt("listbody");
const LISTTAIL: usize = nt("listtail");
const LISTMORE: usize = nt("listmore");
const LISTREST: usize = nt("listrest");
const COMPIF: usize = nt("compif");
const MAPBODY: usize = nt("mapbody");
const MAPREST: usize = nt("maprest");

/// The longest right side of a rule.
const MAX_RHS: usize = 8;

/// For each rule and each place in its right side, what follows that place in the rule: its
/// FIRST, and whether it can be empty (Lean's `firstSeq` and `nullSeq` of the rest). A const,
/// not a static: each call site's place is a constant, which the compiler folds.
#[allow(clippy::large_const_arrays)]
const AFTER: [[(u64, bool); MAX_RHS]; table::NP] = {
    assert!(table::NT <= 64);
    let mut out = [[(0u64, true); MAX_RHS]; table::NP];
    let mut p = 0;
    while p < table::NP {
        // the rule numbers are the order of P
        assert!(table::RULES[p] as usize == p);
        let rhs = table::RHS[p];
        assert!(rhs.len() <= MAX_RHS);
        let (mut first, mut null) = (0u64, true);
        let mut i = rhs.len();
        while i > 0 {
            i -= 1;
            out[p][i] = (first, null);
            let s = rhs[i];
            if s < table::NT_BASE {
                first = 1 << s;
                null = false;
            } else {
                let a = (s - table::NT_BASE) as usize;
                first = table::FIRST[a] as u64 | if table::NULLABLE[a] { first } else { 0 };
                null = null && table::NULLABLE[a];
            }
        }
        p += 1;
    }
    out
};

/// `fk` of the symbol at place `i` of rule `p`, called with `fk`.
#[inline]
const fn after(p: P, i: usize, fk: u64) -> u64 {
    let (first, null) = AFTER[p as usize][i];
    if null { first | fk } else { first }
}

/// TABLE as rules, a row of 64 for each nonterminal (every terminal's number is below 64): no
/// bounds to check on a lookahead.
const PICK: [[Option<P>; 64]; table::NN] = {
    let mut out = [[None; 64]; table::NN];
    let mut a = 0;
    while a < table::NN {
        let mut x = 0;
        while x < table::NT {
            let p = table::TABLE[a][x];
            if p != table::NONE {
                out[a][x] = Some(table::RULES[p as usize]);
            }
            x += 1;
        }
        a += 1;
    }
    out
};

/// The lookaheads on which each of the operator tails after a postfix chain takes its empty rule
/// from TABLE: on these, an operand with nothing after it is the whole `or`, no tail missing.
const TAILS_END: u64 = {
    let tails = [POWTAIL, MULREST, ADDREST, RANGETAIL, CMPREST, ANDREST, ORREST];
    let mut set = 0u64;
    let mut x = 0;
    while x < table::NT {
        let mut all = true;
        let mut k = 0;
        while k < tails.len() {
            let a = tails[k];
            let p = table::TABLE[a][x];
            if p == table::NONE || p != table::EMPTY[a] {
                all = false;
            }
            k += 1;
        }
        if all {
            set |= 1 << x;
        }
        x += 1;
    }
    set
};

/// The tree of the tokens (`la`, each one's terminal), or the error the table parser gives.
pub(super) fn parse(toks: Vec<Tok>, la: Vec<u8>) -> Result<Vec<Stmt>, Error> {
    let mut p = Fast {
        la: la[0],
        toks,
        las: la,
        k: 0,
        miss: None,
        deep: 0,
    };
    match p.program() {
        Ok(s) => Ok(s),
        // (a reserved word used as a name is said first, as the table parser says it)
        Err(e) => Err(super::reserved_tok(&p.toks).unwrap_or(*e)),
    }
}

/// After a miss: the expected set, and whether `stmtsrest` (a statement could have ended) and
/// `stmts` (one could have begun) were missed.
#[derive(Clone, Copy)]
struct Miss {
    set: u64,
    end: bool,
    begin: bool,
}

pub(super) struct Fast {
    toks: Vec<Tok>,
    /// Each token's terminal.
    las: Vec<u8>,
    k: usize,
    /// The lookahead: `las[k]`.
    la: u8,
    miss: Option<Miss>,
    /// How deep the tree being built goes where the parse is now: a level for each nested
    /// expression, block and prefix operator, and for each link of a chain (`a + b + ..` nests to
    /// the left). Evaluation and drop recurse that deep, so past `MAX_DEPTH` a file is refused.
    deep: u32,
}

/// Deeper than any program written by hand or made by a tool, shallow enough for evaluation on a
/// thread's 2 MB stack.
pub(super) const MAX_DEPTH: u32 = 400;

impl Fast {
    /// One level deeper, or the refusal past `MAX_DEPTH`.
    #[inline]
    fn deeper(&mut self) -> F<()> {
        self.deep += 1;
        if self.deep > MAX_DEPTH {
            return Err(Box::new(Error::Syntax(SyntaxError::TooDeep {
                span: self.span(),
            })));
        }
        Ok(())
    }

    #[inline]
    fn span(&self) -> Span {
        self.toks[self.k].span
    }
    #[inline]
    fn last_hi(&self) -> u32 {
        self.toks[self.k.saturating_sub(1)].span.hi
    }

    /// The error of a parse that stops at symbol `stop` (a terminal, or a nonterminal with no rule
    /// for the lookahead and no empty one), as `Parser::expected` words it.
    #[cold]
    fn stop(&self, stop: u16) -> Box<Error> {
        let m = self.miss.unwrap_or(Miss {
            // a nonterminal past the terminals' 64 has no bit: the words come from `what`
            set: 1u64.checked_shl(u32::from(stop)).unwrap_or(0),
            end: false,
            begin: false,
        });
        Box::new(Error::Syntax(SyntaxError::Expected {
            what: if m.end {
                "end of statement"
            } else if m.begin {
                "a value"
            } else {
                super::what(stop)
            },
            expected: TermSet(m.set as u128),
            span: self.span(),
        }))
    }

    /// Nonterminal `a`'s rule for the lookahead (Lean's `pick`).
    #[inline]
    fn pick(&mut self, a: usize, fk: u64) -> F<P> {
        match PICK[a][(self.la & 63) as usize] {
            Some(p) => Ok(p),
            None => self.missed(a, fk),
        }
    }

    #[cold]
    fn missed(&mut self, a: usize, fk: u64) -> F<P> {
        let m = self.miss.get_or_insert(Miss {
            set: table::FIRST[a] as u64 | if table::NULLABLE[a] { fk } else { 0 },
            end: false,
            begin: false,
        });
        m.end |= a == STMTSREST;
        m.begin |= a == STMTS;
        let p = table::EMPTY[a];
        if p == table::NONE {
            return Err(self.stop((table::NT_BASE as usize + a) as u16));
        }
        Ok(table::RULES[p as usize])
    }

    /// Takes terminal `a` (Lean's `shiftT`; the table parser never steps past EOF).
    #[inline]
    fn shift(&mut self, a: u8) -> F<Span> {
        if self.miss.is_none() && self.la == a {
            let s = self.span();
            self.k = (self.k + 1).min(self.toks.len() - 1);
            self.la = self.las[self.k];
            return Ok(s);
        }
        Err(self.stop(a as u16))
    }

    /// `shift` of the lookahead itself, after TABLE picked a rule that starts with it.
    #[inline]
    fn take(&mut self) -> F<Span> {
        if self.miss.is_none() {
            let s = self.span();
            self.k = (self.k + 1).min(self.toks.len() - 1);
            self.la = self.las[self.k];
            return Ok(s);
        }
        Err(self.stop(self.la as u16))
    }

    /// An ID's text, taken out of its token.
    #[inline]
    fn id(&mut self) -> F<(String, Span)> {
        let k = self.k;
        let span = self.shift(t::ID)?;
        let s = match &mut self.toks[k].kind {
            TokKind::Id(s) => std::mem::take(s),
            _ => String::new(),
        };
        Ok((s, span))
    }

    // ---- statements ------------------------------------------------------------------------

    fn program(&mut self) -> F<Vec<Stmt>> {
        self.pick(PROGRAM, 0)?;
        self.seps(after(P::ProgramSeps, 0, 0))?;
        let s = self.stmts(after(P::ProgramSeps, 1, 0))?;
        self.shift(t::EOF)?;
        Ok(s)
    }

    fn block(&mut self, fk: u64) -> F<Vec<Stmt>> {
        self.pick(BLOCK, fk)?;
        self.shift(t::LBRACE)?;
        self.deeper()?;
        self.seps(after(P::BlockLbrace, 1, fk))?;
        let s = self.stmts(after(P::BlockLbrace, 2, fk))?;
        self.shift(t::RBRACE)?;
        self.deep -= 1;
        Ok(s)
    }

    /// stmts: statements, each one's `full` running on over the first separator after it.
    fn stmts(&mut self, fk: u64) -> F<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            let p = self.pick(STMTS, fk)?;
            if p == P::StmtsEmpty {
                return Ok(out);
            }
            let mut s = self.stmt(after(P::StmtsStmt, 0, fk))?;
            let r = self.pick(STMTSREST, after(P::StmtsStmt, 1, fk))?;
            if r == P::StmtsrestEmpty {
                out.push(s);
                return Ok(out);
            }
            s.full.hi = self.sep(after(r, 0, fk))?.hi;
            self.seps(after(r, 1, fk))?;
            out.push(s);
        }
    }

    #[inline]
    fn seps(&mut self, fk: u64) -> F<()> {
        while self.pick(SEPS, fk)? == P::SepsSep {
            self.sep(after(P::SepsSep, 0, fk))?;
        }
        Ok(())
    }

    #[inline]
    fn sep(&mut self, fk: u64) -> F<Span> {
        self.pick(SEP, fk)?;
        self.take()
    }

    fn stmt(&mut self, fk: u64) -> F<Stmt> {
        let lo = self.span().lo;
        let p = self.pick(STMT, fk)?;
        let kind = match p {
            P::StmtShared => {
                self.shift(t::SHARED)?;
                let (name, _) = self.id()?;
                self.shift(t::ASSIGN)?;
                StmtKind::Shared {
                    name,
                    value: self.expr(after(P::StmtShared, 3, fk))?,
                }
            }
            P::StmtParam => {
                self.shift(t::PARAM)?;
                let (owner, name) = self.path(after(P::StmtParam, 1, fk), lo)?;
                self.shift(t::ASSIGN)?;
                StmtKind::Param {
                    owner,
                    name,
                    value: self.expr(after(P::StmtParam, 3, fk))?,
                }
            }
            P::StmtFn => {
                self.shift(t::FN)?;
                let (name, _) = self.id()?;
                self.shift(t::LPAREN)?;
                let params = self.fparams(after(P::StmtFn, 3, fk))?;
                self.shift(t::RPAREN)?;
                StmtKind::Fn {
                    name,
                    params,
                    body: self.block(after(P::StmtFn, 5, fk))?.into(),
                }
            }
            P::StmtFor => {
                self.shift(t::FOR)?;
                let mut vars = vec![self.id()?.0];
                self.forvars(after(P::StmtFor, 2, fk), &mut vars)?;
                self.shift(t::IN)?;
                let iter = self.expr(after(P::StmtFor, 4, fk))?;
                StmtKind::For {
                    vars,
                    iter,
                    body: self.block(after(P::StmtFor, 5, fk))?,
                }
            }
            P::StmtIf => {
                self.shift(t::IF)?;
                let cond = self.expr(after(P::StmtIf, 1, fk))?;
                let then = self.block(after(P::StmtIf, 2, fk))?;
                let other = self.elsepart(after(P::StmtIf, 3, fk))?;
                StmtKind::If { cond, then, other }
            }
            P::StmtReturn => {
                self.shift(t::RETURN)?;
                StmtKind::Return(self.expr(after(P::StmtReturn, 1, fk))?)
            }
            P::StmtExpect => {
                self.shift(t::EXPECT)?;
                let (metric, _) = self.id()?;
                let args = self.expargs(after(P::StmtExpect, 2, fk))?;
                let cmp = self.expcmp(after(P::StmtExpect, 3, fk))?;
                let at = self.expat(after(P::StmtExpect, 4, fk))?;
                StmtKind::Expect {
                    metric,
                    args,
                    cmp,
                    at,
                }
            }
            P::StmtSketch | P::StmtPath | P::StmtComponent => {
                let (kind, kw) = match p {
                    P::StmtSketch => (BlockKind::Sketch, t::SKETCH),
                    P::StmtPath => (BlockKind::Path, t::PATH),
                    _ => (BlockKind::Component, t::COMPONENT),
                };
                self.shift(kw)?;
                let (name, _) = self.id()?;
                self.shift(t::LPAREN)?;
                let args = self.args(after(P::StmtSketch, 3, fk))?;
                self.shift(t::RPAREN)?;
                StmtKind::Block {
                    kind,
                    name,
                    args,
                    body: self.block(after(P::StmtSketch, 5, fk))?,
                }
            }
            P::StmtDrawing => {
                let at = self.shift(t::DRAWING)?;
                let fk = after(P::StmtDrawing, 1, fk);
                let q = self.pick(DRAWTAIL, fk)?;
                if q == P::DrawtailId {
                    let (name, args, body) = self.named_block(q, fk)?;
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
            P::StmtHarness => {
                let at = self.shift(t::HARNESS)?;
                let fk = after(P::StmtHarness, 1, fk);
                let q = self.pick(HARNTAIL, fk)?;
                if q == P::HarntailId {
                    let (name, args, body) = self.named_block(q, fk)?;
                    StmtKind::Block {
                        kind: BlockKind::Harness,
                        name,
                        args,
                        body,
                    }
                } else {
                    self.shift(t::LPAREN)?;
                    let args = self.args(after(q, 1, fk))?;
                    self.shift(t::RPAREN)?;
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
                let e = self.expr(after(P::StmtExpr, 0, fk))?;
                let fk = after(P::StmtExpr, 1, fk);
                if self.pick(ASSIGNTAIL, fk)? == P::AssigntailAssign {
                    // (checked as the rule is picked: `enter`)
                    let ExprKind::Name(name) = e.kind else {
                        return Err(Box::new(Error::Syntax(SyntaxError::AssignToNonName {
                            span: e.span,
                        })));
                    };
                    self.shift(t::ASSIGN)?;
                    StmtKind::Assign {
                        name,
                        value: self.expr(after(P::AssigntailAssign, 1, fk))?,
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
        Ok(Stmt {
            kind,
            span,
            full: span,
        })
    }

    /// drawtail and harntail's `ID dargs block`, rule `q` picked.
    fn named_block(&mut self, q: P, fk: u64) -> F<(String, Vec<Arg>, Vec<Stmt>)> {
        let (name, _) = self.id()?;
        let dk = after(q, 1, fk);
        let args = if self.pick(DARGS, dk)? == P::DargsLparen {
            self.shift(t::LPAREN)?;
            let a = self.args(after(P::DargsLparen, 1, dk))?;
            self.shift(t::RPAREN)?;
            a
        } else {
            Vec::new()
        };
        Ok((name, args, self.block(after(q, 2, fk))?))
    }

    /// elsepart: `else` and a block, or `else if ..`.
    fn elsepart(&mut self, fk: u64) -> F<Option<Vec<Stmt>>> {
        if self.pick(ELSEPART, fk)? == P::ElsepartEmpty {
            return Ok(None);
        }
        self.shift(t::ELSE)?;
        let fk = after(P::ElsepartElse, 1, fk);
        let p = self.pick(ELSEBODY, fk)?;
        if p == P::ElsebodyBlock {
            return Ok(Some(self.block(after(P::ElsebodyBlock, 0, fk))?));
        }
        let lo = self.shift(t::IF)?.lo;
        let cond = self.expr(after(P::ElsebodyIf, 1, fk))?;
        let then = self.block(after(P::ElsebodyIf, 2, fk))?;
        let other = self.elsepart(after(P::ElsebodyIf, 3, fk))?;
        let span = Span {
            lo,
            hi: self.last_hi(),
        };
        Ok(Some(vec![Stmt {
            kind: StmtKind::If { cond, then, other },
            span,
            full: span,
        }]))
    }

    /// forvars: `, ID` each, onto `vars`.
    fn forvars(&mut self, fk: u64, vars: &mut Vec<String>) -> F<()> {
        while self.pick(FORVARS, fk)? == P::ForvarsComma {
            self.shift(t::COMMA)?;
            vars.push(self.id()?.0);
        }
        Ok(())
    }

    fn expargs(&mut self, fk: u64) -> F<Vec<Expr>> {
        let mut args = Vec::new();
        if self.pick(EXPARGS, fk)? == P::ExpargsEmpty {
            return Ok(args);
        }
        self.shift(t::LPAREN)?;
        let fk1 = after(P::ExpargsLparen, 1, fk);
        while self.pick(EXPRS, fk1)? == P::ExprsExpr {
            args.push(self.expr(after(P::ExprsExpr, 0, fk1))?);
            if self.pick(EXPRSREST, fk1)? == P::ExprsrestEmpty {
                break;
            }
            self.shift(t::COMMA)?;
        }
        self.shift(t::RPAREN)?;
        Ok(args)
    }

    #[allow(clippy::type_complexity)]
    fn expcmp(&mut self, fk: u64) -> F<Option<(Cmp, Expr, Option<(Expr, bool)>)>> {
        let p = self.pick(EXPCMP, fk)?;
        if p == P::ExpcmpEmpty {
            return Ok(None);
        }
        let c = match self.pick(CMPOP, after(P::ExpcmpCmpop, 0, fk))? {
            P::CmpopAssign => (Cmp::Eq, t::ASSIGN),
            P::CmpopLt => (Cmp::Lt, t::LT),
            P::CmpopLe => (Cmp::Le, t::LE),
            P::CmpopGt => (Cmp::Gt, t::GT),
            _ => (Cmp::Ge, t::GE),
        };
        self.shift(c.1)?;
        let v = self.expr(after(P::ExpcmpCmpop, 1, fk))?;
        let tk = after(P::ExpcmpCmpop, 2, fk);
        let tol = if self.pick(EXPTOL, tk)? == P::ExptolPlusminus {
            self.shift(t::PLUSMINUS)?;
            let tv = self.unary(after(P::ExptolPlusminus, 1, tk))?;
            let pct = self.pick(PCT, after(P::ExptolPlusminus, 2, tk))? == P::PctPercent;
            if pct {
                self.shift(t::PERCENT)?;
            }
            Some((tv, pct))
        } else {
            None
        };
        Ok(Some((c.0, v, tol)))
    }

    fn expat(&mut self, fk: u64) -> F<Vec<(Vec<Seg>, Expr)>> {
        let mut at = Vec::new();
        if self.pick(EXPAT, fk)? == P::ExpatEmpty {
            return Ok(at);
        }
        self.shift(t::AT)?;
        let mut p = P::ExpatAt;
        loop {
            let segs = self.atpath(after(p, 1, fk))?;
            self.shift(t::ASSIGN)?;
            at.push((segs, self.expr(after(p, 3, fk))?));
            p = self.pick(EXPMORE, after(p, 4, fk))?;
            if p == P::ExpmoreEmpty {
                return Ok(at);
            }
            self.shift(t::COMMA)?;
        }
    }

    /// atpath: `ID keys pathrest`, a path's segments in order.
    fn atpath(&mut self, fk: u64) -> F<Vec<Seg>> {
        let p = self.pick(ATPATH, fk)?;
        self.segs(p, fk)
    }

    /// `ID keys pathrest` of rule `p` (path's or atpath's): the segments in order.
    fn segs(&mut self, p: P, fk: u64) -> F<Vec<Seg>> {
        let (name, _) = self.id()?;
        let index = self.keys(after(p, 1, fk))?;
        let mut segs = vec![Seg { name, index }];
        let rk = after(p, 2, fk);
        while self.pick(PATHREST, rk)? == P::PathrestDot {
            self.shift(t::DOT)?;
            let (name, _) = self.id()?;
            let index = self.keys(after(P::PathrestDot, 2, rk))?;
            segs.push(Seg { name, index });
        }
        Ok(segs)
    }

    /// A param's path: its owner's segments and its name; `lo`, where the statement starts.
    fn path(&mut self, fk: u64, lo: u32) -> F<(Vec<Seg>, String)> {
        let p = self.pick(PATH, fk)?;
        let mut segs = self.segs(p, fk)?;
        // (checked as the rule is done: `reduce`)
        let last = segs.pop().expect("one segment");
        if segs.is_empty() || !last.index.is_empty() {
            return Err(Box::new(Error::Syntax(SyntaxError::ParamWithoutOwner {
                span: Span {
                    lo,
                    hi: self.last_hi(),
                },
                name: last.name,
            })));
        }
        Ok((segs, last.name))
    }

    fn keys(&mut self, fk: u64) -> F<Vec<Key>> {
        let mut keys = Vec::new();
        if self.pick(KEYS, fk)? == P::KeysEmpty {
            return Ok(keys);
        }
        self.shift(t::LBRACK)?;
        let mut kk = after(P::KeysLbrack, 1, fk);
        let rk = after(P::KeysLbrack, 2, fk);
        loop {
            keys.push(self.key(kk)?);
            if self.pick(KEYREST, rk)? == P::KeyrestEmpty {
                break;
            }
            self.shift(t::COMMA)?;
            kk = after(P::KeyrestComma, 1, rk);
        }
        self.shift(t::RBRACK)?;
        Ok(keys)
    }

    fn key(&mut self, fk: u64) -> F<Key> {
        if self.pick(KEY, fk)? == P::KeyId {
            return Ok(Key::Label(self.id()?.0));
        }
        let k = self.k;
        let span = self.shift(t::NUM)?;
        // (checked as the rule is done: `reduce`)
        match self.toks[k].kind {
            TokKind::Num(v, Unit::None) if v.fract() == 0.0 => Ok(Key::Ix(v as i64)),
            _ => Err(Box::new(Error::Syntax(SyntaxError::Expected {
                what: "an instance index",
                expected: TermSet(1 << t::NUM | 1 << t::ID),
                span,
            }))),
        }
    }

    fn fparams(&mut self, fk: u64) -> F<Vec<FnParam>> {
        let mut ps = Vec::new();
        while self.pick(FPARAMS, fk)? == P::FparamsFparam {
            let pk = after(P::FparamsFparam, 0, fk);
            self.pick(FPARAM, pk)?;
            let (name, _) = self.id()?;
            let ty = if self.pick(FPTYPE, after(P::FparamId, 1, pk))? == P::FptypeColon {
                self.shift(t::COLON)?;
                Some(self.id()?.0)
            } else {
                None
            };
            let dk = after(P::FparamId, 2, pk);
            let default = if self.pick(FPDEFAULT, dk)? == P::FpdefaultAssign {
                self.shift(t::ASSIGN)?;
                Some(self.expr(after(P::FpdefaultAssign, 1, dk))?)
            } else {
                None
            };
            ps.push(FnParam { name, ty, default });
            if self.pick(FPARAMREST, after(P::FparamsFparam, 1, fk))? == P::FparamrestEmpty {
                break;
            }
            self.shift(t::COMMA)?;
        }
        Ok(ps)
    }

    /// args: each argument, and a `,` after each but maybe the last.
    fn args(&mut self, fk: u64) -> F<Vec<Arg>> {
        let mut args: Vec<Arg> = Vec::new();
        while self.pick(ARGS, fk)? == P::ArgsArg {
            let ak = after(P::ArgsArg, 0, fk);
            let p = self.pick(ARG, ak)?;
            let arg = match p {
                P::ArgAt | P::ArgComponent => {
                    let (w, kw) = if p == P::ArgAt {
                        ("at", t::AT)
                    } else {
                        ("component", t::COMPONENT)
                    };
                    let s = self.shift(kw)?;
                    self.shift(t::COLON)?;
                    Arg {
                        name: Some((w.into(), s)),
                        value: self.expr(after(P::ArgAt, 2, ak))?,
                    }
                }
                _ => {
                    let e = self.expr(after(P::ArgExpr, 0, ak))?;
                    let kk = after(P::ArgExpr, 1, ak);
                    if self.pick(ARGKW, kk)? == P::ArgkwColon {
                        // (checked as the rule is picked: `enter`)
                        let ExprKind::Name(n) = e.kind else {
                            return Err(Box::new(Error::Syntax(SyntaxError::KeywordNotName {
                                span: e.span,
                            })));
                        };
                        self.shift(t::COLON)?;
                        Arg {
                            name: Some((n, e.span)),
                            value: self.expr(after(P::ArgkwColon, 1, kk))?,
                        }
                    } else {
                        // a positional argument after a keyword one (`enter` of the empty rule)
                        if args.last().is_some_and(|a| a.name.is_some()) {
                            return Err(Box::new(Error::Syntax(SyntaxError::PositionalAfterKeyword {
                                span: e.span,
                            })));
                        }
                        Arg { name: None, value: e }
                    }
                }
            };
            args.push(arg);
            if self.pick(ARGSREST, after(P::ArgsArg, 1, fk))? == P::ArgsrestEmpty {
                break;
            }
            self.take()?;
        }
        Ok(args)
    }

    // ---- expressions ------------------------------------------------------------------------

    pub(super) fn expr(&mut self, fk: u64) -> F<Expr> {
        self.deeper()?;
        let e = self.expr_in(fk)?;
        self.deep -= 1;
        Ok(e)
    }

    fn expr_in(&mut self, fk: u64) -> F<Expr> {
        self.pick(EXPR, fk)?;
        let a = self.or(after(P::ExprOr, 0, fk))?;
        let fk = after(P::ExprOr, 1, fk);
        if self.pick(IFPART, fk)? == P::IfpartEmpty {
            return Ok(a);
        }
        self.shift(t::IF)?;
        let cond = self.or(after(P::IfpartIf, 1, fk))?;
        self.shift(t::ELSE)?;
        let other = self.expr(after(P::IfpartIf, 3, fk))?;
        let span = a.span.to(other.span);
        Ok(Expr {
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

    /// `or`, each level below it its operand and then its tail. On a lookahead that starts an
    /// atom, every level from `or` down to `postfix` takes its one rule, no miss possible: the
    /// postfix chain is each level's first operand, and each level's tail follows in turn from
    /// the innermost out; where none of the tails is there and none is missed (`TAILS_END`), the
    /// chain is the whole `or`.
    fn or(&mut self, fk: u64) -> F<Expr> {
        if PICK[POSTFIX][(self.la & 63) as usize].is_none() {
            return self.or_by_levels(fk);
        }
        let k_and = after(P::OrAnd, 0, fk);
        let k_not = after(P::AndNot, 0, k_and);
        let k_cmp = after(P::NotCmp, 0, k_not);
        let k_range = after(P::CmpRange, 0, k_cmp);
        let k_add = after(P::RangeAdd, 0, k_range);
        let k_mul = after(P::AddMul, 0, k_add);
        let k_unary = after(P::MulUnary, 0, k_mul);
        let k_power = after(P::UnaryPower, 0, k_unary);
        let a = self.postfix_picked(after(P::PowerPostfix, 0, k_power))?;
        if self.miss.is_none() && TAILS_END >> (self.la & 63) & 1 == 1 {
            return Ok(a);
        }
        let a = self.powtail(after(P::PowerPostfix, 1, k_power), a)?;
        let a = self.mulrest(after(P::MulUnary, 1, k_mul), a)?;
        let a = self.addrest(after(P::AddMul, 1, k_add), a)?;
        let a = self.rangetail(after(P::RangeAdd, 1, k_range), a)?;
        let a = self.cmprest(after(P::CmpRange, 1, k_cmp), a)?;
        let a = self.andrest(after(P::AndNot, 1, k_and), a)?;
        self.orrest(after(P::OrAnd, 1, fk), a)
    }

    fn or_by_levels(&mut self, fk: u64) -> F<Expr> {
        self.pick(OR, fk)?;
        let a = self.and(after(P::OrAnd, 0, fk))?;
        self.orrest(after(P::OrAnd, 1, fk), a)
    }

    #[inline]
    fn orrest(&mut self, fk: u64, mut a: Expr) -> F<Expr> {
        let d = self.deep;
        while self.pick(ORREST, fk)? == P::OrrestOr {
            self.take()?;
            self.deeper()?;
            let b = self.and(after(P::OrrestOr, 1, fk))?;
            a = Self::chain(a, BinOp::Or, b);
        }
        self.deep = d;
        Ok(a)
    }

    fn and(&mut self, fk: u64) -> F<Expr> {
        self.pick(AND, fk)?;
        let a = self.not(after(P::AndNot, 0, fk))?;
        self.andrest(after(P::AndNot, 1, fk), a)
    }

    #[inline]
    fn andrest(&mut self, fk: u64, mut a: Expr) -> F<Expr> {
        let d = self.deep;
        while self.pick(ANDREST, fk)? == P::AndrestAnd {
            self.take()?;
            self.deeper()?;
            let b = self.not(after(P::AndrestAnd, 1, fk))?;
            a = Self::chain(a, BinOp::And, b);
        }
        self.deep = d;
        Ok(a)
    }

    fn not(&mut self, fk: u64) -> F<Expr> {
        let p = self.pick(NOT, fk)?;
        if p == P::NotCmp {
            self.pick(CMP, after(P::NotCmp, 0, fk))?;
            let fk = after(P::NotCmp, 0, fk);
            let a = self.range(after(P::CmpRange, 0, fk))?;
            return self.cmprest(after(P::CmpRange, 1, fk), a);
        }
        let s = self.take()?;
        self.deeper()?;
        let e = self.not(after(P::NotNot, 1, fk))?;
        self.deep -= 1;
        let span = s.to(e.span);
        Ok(Expr {
            kind: ExprKind::Not(Box::new(e)),
            span,
        })
    }

    #[inline]
    fn cmprest(&mut self, fk: u64, mut a: Expr) -> F<Expr> {
        let d = self.deep;
        while self.pick(CMPREST, fk)? == P::CmprestRelop {
            self.deeper()?;
            let op = match self.pick(RELOP, after(P::CmprestRelop, 0, fk))? {
                P::RelopEqeq => BinOp::Eq,
                P::RelopNe => BinOp::Ne,
                P::RelopLt => BinOp::Lt,
                P::RelopLe => BinOp::Le,
                P::RelopGt => BinOp::Gt,
                _ => BinOp::Ge,
            };
            self.take()?;
            let b = self.range(after(P::CmprestRelop, 1, fk))?;
            a = Self::chain(a, op, b);
        }
        self.deep = d;
        Ok(a)
    }

    fn range(&mut self, fk: u64) -> F<Expr> {
        self.pick(RANGE, fk)?;
        let a = self.add(after(P::RangeAdd, 0, fk))?;
        self.rangetail(after(P::RangeAdd, 1, fk), a)
    }

    #[inline]
    fn rangetail(&mut self, fk: u64, a: Expr) -> F<Expr> {
        let p = self.pick(RANGETAIL, fk)?;
        let inclusive = match p {
            P::RangetailDotdot => false,
            P::RangetailDotdoteq => true,
            _ => return Ok(a),
        };
        self.take()?;
        let hi = self.add(after(P::RangetailDotdot, 1, fk))?;
        let span = a.span.to(hi.span);
        Ok(Expr {
            kind: ExprKind::Range {
                lo: Box::new(a),
                hi: Box::new(hi),
                inclusive,
            },
            span,
        })
    }

    fn add(&mut self, fk: u64) -> F<Expr> {
        self.pick(ADD, fk)?;
        let a = self.mul(after(P::AddMul, 0, fk))?;
        self.addrest(after(P::AddMul, 1, fk), a)
    }

    #[inline]
    fn addrest(&mut self, fk: u64, mut a: Expr) -> F<Expr> {
        let d = self.deep;
        loop {
            let p = self.pick(ADDREST, fk)?;
            let op = match p {
                P::AddrestPlus => BinOp::Add,
                P::AddrestMinus => BinOp::Sub,
                _ => {
                    self.deep = d;
                    return Ok(a);
                }
            };
            self.take()?;
            self.deeper()?;
            let b = self.mul(after(P::AddrestPlus, 1, fk))?;
            a = Self::chain(a, op, b);
        }
    }

    fn mul(&mut self, fk: u64) -> F<Expr> {
        self.pick(MUL, fk)?;
        let a = self.unary(after(P::MulUnary, 0, fk))?;
        self.mulrest(after(P::MulUnary, 1, fk), a)
    }

    #[inline]
    fn mulrest(&mut self, fk: u64, mut a: Expr) -> F<Expr> {
        let d = self.deep;
        loop {
            let p = self.pick(MULREST, fk)?;
            let op = match p {
                P::MulrestStar => BinOp::Mul,
                P::MulrestSlash => BinOp::Div,
                _ => {
                    self.deep = d;
                    return Ok(a);
                }
            };
            self.take()?;
            self.deeper()?;
            let b = self.unary(after(P::MulrestStar, 1, fk))?;
            a = Self::chain(a, op, b);
        }
    }

    fn unary(&mut self, fk: u64) -> F<Expr> {
        let p = self.pick(UNARY, fk)?;
        if p == P::UnaryPower {
            let fk = after(P::UnaryPower, 0, fk);
            self.pick(POWER, fk)?;
            let a = self.postfix(after(P::PowerPostfix, 0, fk))?;
            return self.powtail(after(P::PowerPostfix, 1, fk), a);
        }
        let s = self.take()?;
        self.deeper()?;
        let e = self.unary(after(P::UnaryMinus, 1, fk))?;
        self.deep -= 1;
        let span = s.to(e.span);
        Ok(Expr {
            kind: ExprKind::Neg(Box::new(e)),
            span,
        })
    }

    /// powtail: `^` and the power, right-associated.
    #[inline]
    fn powtail(&mut self, fk: u64, a: Expr) -> F<Expr> {
        if self.pick(POWTAIL, fk)? == P::PowtailEmpty {
            return Ok(a);
        }
        self.take()?;
        self.deeper()?;
        let e = self.unary(after(P::PowtailCaret, 1, fk))?;
        self.deep -= 1;
        let span = a.span.to(e.span);
        Ok(Expr {
            kind: ExprKind::Bin(BinOp::Pow, Box::new(a), Box::new(e)),
            span,
        })
    }

    fn postfix(&mut self, fk: u64) -> F<Expr> {
        self.pick(POSTFIX, fk)?;
        self.postfix_picked(fk)
    }

    /// `postfix` once its one rule is picked: where TABLE has it for the lookahead, picking it
    /// does nothing else (`or` checks that before it calls this).
    fn postfix_picked(&mut self, fk: u64) -> F<Expr> {
        let mut a = self.atom(after(P::PostfixAtom, 0, fk))?;
        let fk = after(P::PostfixAtom, 1, fk);
        let lo = a.span.lo;
        let d = self.deep;
        loop {
            let q = self.pick(POSTREST, fk)?;
            if q != P::PostrestLparen && q != P::PostrestDot && q != P::PostrestLbrack {
                self.deep = d;
                return Ok(a);
            }
            self.deeper()?;
            let (kind, hi) = match q {
                P::PostrestLparen => {
                    self.take()?;
                    let args = self.args(after(P::PostrestLparen, 1, fk))?;
                    let hi = self.shift(t::RPAREN)?.hi;
                    (ExprKind::Call(Box::new(a), args), hi)
                }
                P::PostrestDot => {
                    self.take()?;
                    let (n, s) = self.id()?;
                    (ExprKind::Attr(Box::new(a), n), s.hi)
                }
                P::PostrestLbrack => {
                    self.take()?;
                    let i = self.expr(after(P::PostrestLbrack, 1, fk))?;
                    let hi = self.shift(t::RBRACK)?.hi;
                    (ExprKind::Index(Box::new(a), Box::new(i)), hi)
                }
                _ => return Ok(a),
            };
            a = Expr {
                kind,
                span: Span { lo, hi },
            };
        }
    }

    fn atom(&mut self, fk: u64) -> F<Expr> {
        let span = self.span();
        let p = self.pick(ATOM, fk)?;
        let (x, k) = (self.k, self.k);
        let kind = match p {
            P::AtomNum => {
                self.take()?;
                match self.toks[x].kind {
                    TokKind::Num(v, u) => ExprKind::Num(v, u),
                    _ => ExprKind::Num(0.0, Unit::None),
                }
            }
            P::AtomStr => {
                self.take()?;
                match &mut self.toks[k].kind {
                    TokKind::Str(s) => ExprKind::Str(std::mem::take(s)),
                    _ => ExprKind::Str(String::new()),
                }
            }
            P::AtomId => {
                self.take()?;
                match &mut self.toks[k].kind {
                    TokKind::Id(s) => ExprKind::Name(std::mem::take(s)),
                    _ => ExprKind::Name(String::new()),
                }
            }
            P::AtomTrue => {
                self.take()?;
                ExprKind::Bool(true)
            }
            P::AtomFalse => {
                self.take()?;
                ExprKind::Bool(false)
            }
            P::AtomLparen => return self.paren(fk),
            P::AtomLbrack => return self.list(fk),
            _ => return self.map(fk),
        };
        Ok(Expr { kind, span })
    }

    /// `( tuple )`: one item is that item, its span the brackets'; else a tuple.
    fn paren(&mut self, fk: u64) -> F<Expr> {
        let lo = self.shift(t::LPAREN)?.lo;
        let fk1 = after(P::AtomLparen, 1, fk);
        let mut items = Vec::new();
        while self.pick(TUPLE, fk1)? == P::TupleExpr {
            items.push(self.expr(after(P::TupleExpr, 0, fk1))?);
            if self.pick(TUPLEREST, fk1)? == P::TuplerestEmpty {
                break;
            }
            self.take()?;
        }
        let hi = self.shift(t::RPAREN)?.hi;
        let span = Span { lo, hi };
        if items.len() == 1 {
            let mut e = items.pop().expect("one");
            e.span = span;
            return Ok(e);
        }
        Ok(Expr {
            kind: ExprKind::Tuple(items),
            span,
        })
    }

    /// `[ listbody ]`: a list, or a comprehension.
    fn list(&mut self, fk: u64) -> F<Expr> {
        let lo = self.shift(t::LBRACK)?.lo;
        let bk = after(P::AtomLbrack, 1, fk);
        let kind = if self.pick(LISTBODY, bk)? == P::ListbodyEmpty {
            ExprKind::List(Vec::new())
        } else {
            let item = self.expr(after(P::ListbodyExpr, 0, bk))?;
            let tk = after(P::ListbodyExpr, 1, bk);
            match self.pick(LISTTAIL, tk)? {
                P::ListtailFor => {
                    self.shift(t::FOR)?;
                    let mut vars = vec![self.id()?.0];
                    self.forvars(after(P::ListtailFor, 2, tk), &mut vars)?;
                    self.shift(t::IN)?;
                    let iter = self.or(after(P::ListtailFor, 4, tk))?;
                    let ck = after(P::ListtailFor, 5, tk);
                    let cond = if self.pick(COMPIF, ck)? == P::CompifIf {
                        self.shift(t::IF)?;
                        Some(Box::new(self.or(after(P::CompifIf, 1, ck))?))
                    } else {
                        None
                    };
                    ExprKind::Comp {
                        item: Box::new(item),
                        vars,
                        iter: Box::new(iter),
                        cond,
                    }
                }
                P::ListtailComma => {
                    self.shift(t::COMMA)?;
                    let mut items = vec![item];
                    let mk = after(P::ListtailComma, 1, tk);
                    while self.pick(LISTMORE, mk)? == P::ListmoreExpr {
                        items.push(self.expr(after(P::ListmoreExpr, 0, mk))?);
                        if self.pick(LISTREST, mk)? == P::ListrestEmpty {
                            break;
                        }
                        self.shift(t::COMMA)?;
                    }
                    ExprKind::List(items)
                }
                _ => ExprKind::List(vec![item]),
            }
        };
        let hi = self.shift(t::RBRACK)?.hi;
        Ok(Expr {
            kind,
            span: Span { lo, hi },
        })
    }

    /// `{ seps mapbody }`: labelled values.
    fn map(&mut self, fk: u64) -> F<Expr> {
        let lo = self.shift(t::LBRACE)?.lo;
        self.seps(after(P::AtomLbrace, 1, fk))?;
        let mk = after(P::AtomLbrace, 2, fk);
        let mut entries = Vec::new();
        while self.pick(MAPBODY, mk)? == P::MapbodyId {
            let (name, _) = self.id()?;
            self.shift(t::COLON)?;
            entries.push((name, self.expr(after(P::MapbodyId, 2, mk))?));
            self.seps(after(P::MapbodyId, 3, mk))?;
            if self.pick(MAPREST, mk)? == P::MaprestEmpty {
                break;
            }
            self.shift(t::COMMA)?;
            self.seps(after(P::MaprestComma, 1, mk))?;
        }
        let hi = self.shift(t::RBRACE)?.hi;
        Ok(Expr {
            kind: ExprKind::Map(entries),
            span: Span { lo, hi },
        })
    }
}
