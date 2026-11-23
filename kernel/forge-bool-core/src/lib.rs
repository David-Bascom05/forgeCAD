//! The decide-once flat-face boolean's combinatorial core (research/kernel-audit/decide-once-spec.md
//! §3 and §4), the Rust of `ForgeExact.DecideOnce.core` (verify/forge-exact/ForgeExact/DecideOnce/
//! Core.lean).
//!
//! It reads a [`Ledger`] (every geometric fact as a decided sign, a bool or an id, decided once by
//! the numeric bridge before it runs) and a [`Rule`], and writes a fresh [`Body`] or refuses. It
//! takes no point, plane or number: there is no `f64` anywhere in this crate.
//!
//! Written for Aeneas: no floats, no thread-locals, no `Cell`/`RefCell`, no global state, no
//! closures, no trait objects, no `unsafe`, no parallelism. Loops are `while` over index ranges.
//! Where the Lean takes a closure by rounds (`cls`), this uses a union–find whose root is the
//! least member in list order, which is the Lean's `head` (`Closure.lean`, `head_eq`); where the
//! Lean filters a list by `Cov.le`, this enumerates the few covectors that can satisfy it and
//! looks them up in a sorted index, then restores list order. The sets are the same; see each
//! function's comment for the equality the refinement proof needs.
//!
//! Cells are keyed by covector: the bridge must give one id per covector (`Ledger::cells` holds no
//! repeats), so the refinement map `id ↦ cells[id]` is injective.

#![forbid(unsafe_code)]

mod index;
pub mod row;
mod sort;
mod union_find;
pub mod v2;

pub use index::Index;
pub use row::Row;
use row::{row_eq, row_get, row_le, row_set, row_zero_mask};
use union_find::Uf;

/// A side of a carrier class: below it, on it, above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sgn {
    Neg,
    Zero,
    Pos,
}

/// A cell: the index of its covector in [`Ledger::cells`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId(pub u32);

/// An input face, by its arena index (body 0's faces first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(pub u32);

/// An operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyId(pub u32);

/// The ledger, built once by the numeric bridge, read-only to the core. Every field is a table of
/// decisions each made once (`Ledger.lean`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    /// The number of bodies (operands), `0 .. nb`.
    pub nb: u32,
    /// The number of carrier classes: every covector's length (at most [`row::MAX_WIDTH`] for the
    /// core to run).
    pub width: u32,
    /// Per input face, its body.
    pub face_body: Vec<BodyId>,
    /// Per input face, its carrier class (two faces one surface).
    pub face_class: Vec<u32>,
    /// A cell's covector: its row of the side table, one sign per class. No two rows equal. Every
    /// facet's two sides should be here too (the witnesses name them); one that is not is added by
    /// the core for its own use.
    pub cells: Vec<Row>,
    /// Decided: the facet lies inside the face. In face arena order.
    pub pieces: Vec<(CellId, FaceId)>,
    /// Decided: the side cell is inside the body (the witness answers that are "inside").
    pub inside: Vec<(CellId, BodyId)>,
    /// Decided: the cells of dimension 1 round the pieces, without repeats.
    pub edges: Vec<CellId>,
    /// Decided: the cells of dimension 0 round the pieces, without repeats.
    pub verts: Vec<CellId>,
}

/// §6.1 step 3's two tables, decided once per side cell by the bridge (stage 4b, `Metric.lean`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetricLedger {
    /// Side cells thinner than LINEAR (inradius under LINEAR/2).
    pub thin: Vec<CellId>,
    /// Side cells every facet of which is a piece.
    pub enclosed: Vec<CellId>,
}

/// An op as a pointwise rule on the membership vector: `m(i)` is "in body `i`".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// In some body.
    Union,
    /// In every body.
    Inter,
    /// In body 0 and in no other.
    Diff,
    /// Row `Σ m(i)·2^i` of the table (a whole history over `nb` bodies); past its end, out.
    Table(Vec<bool>),
}

/// A face of the result, written fresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFace {
    /// Its facets, in input arena order.
    pub facets: Vec<CellId>,
    /// Its carrier class.
    pub plane: u32,
    /// Outward: `Pos` if along the class's positive side.
    pub orient: Sgn,
    /// The input face its first facet lies in (the first in arena order): its name.
    pub origin: FaceId,
}

/// An edge of the result: a straight run of edge cells between one pair of faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutEdge {
    /// Its edge cells, in the ledger's edge order.
    pub cells: Vec<CellId>,
    /// The two faces it joins, each by its first facet, in the order their facets come in the
    /// result.
    pub faces: Vec<CellId>,
}

/// The result body, written fresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub faces: Vec<OutFace>,
    pub edges: Vec<OutEdge>,
    /// The vertex cells where three or more faces meet, in the ledger's vertex order.
    pub verts: Vec<CellId>,
}

/// Why the core refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    SplitCrossed,
    InconsistentClassification,
    /// A non-manifold edge or vertex of the result, at that cell.
    ZeroThickness(CellId),
}

/// Why the core does not run on a ledger (a counted, decided rule: the caller takes the other
/// path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// More classes than a row holds.
    Wide,
}

/// The core's answer. There is no retry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok(Body),
    Empty,
    Refused(Refusal),
    Skip(SkipReason),
}

const NONE: u32 = u32::MAX;

/// Past this many zeros a cell's neighbours are found by scanning instead of enumerating.
const ENUM_ZEROS: usize = 12;

/// `Cov.zeroIdx`: the first zero of a covector of width `w`, or `w` if there is none.
pub fn zero_idx(x: Row, w: usize) -> usize {
    let m = row_zero_mask(x, w);
    let mut i = 0;
    while i < w && (m >> i) & 1 == 0 {
        i += 1;
    }
    i
}

/// `Cov.isFacet`: exactly one zero.
pub fn is_facet(x: Row, w: usize) -> bool {
    let m = row_zero_mask(x, w);
    let mut n: usize = 0;
    let mut i = 0;
    while i < w {
        if (m >> i) & 1 == 1 {
            n += 1;
        }
        i += 1;
    }
    n == 1
}

/// `Cov.le`: `x` is a face of `y` (rows of one width).
pub fn le(x: Row, y: Row) -> bool {
    row_le(x, y)
}

/// `X.set i s` (unchanged past the width, as `List.set`).
fn with_set(x: Row, w: usize, i: usize, s: Sgn) -> Row {
    if i < w { row_set(x, i, s) } else { x }
}

/// The zeros of a covector of width `w`.
fn zeros(x: Row, w: usize) -> Vec<usize> {
    let m = row_zero_mask(x, w);
    let mut z = Vec::new();
    let mut i = 0;
    while i < w {
        if (m >> i) & 1 == 1 {
            z.push(i);
        }
        i += 1;
    }
    z
}

/// The ledger's width.
fn wd(l: &Ledger) -> usize {
    l.width as usize
}

/// The cells the core adds to the ledger's: any side cell it lacked. Owned, and passed beside
/// the ledger (no struct holds a borrow, for Aeneas).
struct Ext {
    index: Index,
    extra: Vec<Row>,
}

fn new_ext(l: &Ledger) -> Ext {
    Ext {
        index: Index::new(&l.cells),
        extra: Vec::new(),
    }
}

/// A cell's covector.
fn cov(l: &Ledger, ext: &Ext, id: u32) -> Row {
    let n = l.cells.len();
    if (id as usize) < n {
        l.cells[id as usize]
    } else {
        ext.extra[id as usize - n]
    }
}

fn ncells(l: &Ledger, ext: &Ext) -> usize {
    l.cells.len() + ext.extra.len()
}

/// The id of a covector, or NONE.
fn find(l: &Ledger, ext: &Ext, key: Row) -> u32 {
    let mut f = ext.index.find_h(&l.cells, key);
    let mut i = 0;
    while f == NONE && i < ext.extra.len() {
        if row_eq(ext.extra[i], key) {
            f = (l.cells.len() + i) as u32;
        }
        i += 1;
    }
    f
}

/// The id of `key`, added if new.
fn intern(l: &Ledger, ext: &mut Ext, key: Row) -> u32 {
    let f = find(l, ext, key);
    if f != NONE {
        f
    } else {
        ext.extra.push(key);
        (ncells(l, ext) - 1) as u32
    }
}

/// `Ledger.bodyOf` (`getD f 0`).
fn body_of(l: &Ledger, f: FaceId) -> u32 {
    if (f.0 as usize) < l.face_body.len() {
        l.face_body[f.0 as usize].0
    } else {
        0
    }
}

/// `Ledger.clsOf` (`getD f 0`).
fn cls_of(l: &Ledger, f: FaceId) -> u32 {
    if (f.0 as usize) < l.face_class.len() {
        l.face_class[f.0 as usize]
    } else {
        0
    }
}

/// Stages 1–3's tables: the facets, their sides, the topes and each body's flood.
struct Split {
    /// `facets`: the facets of the pieces, first occurrences in order.
    facets: Vec<u32>,
    /// per facet, its plus and minus side cells
    plus: Vec<u32>,
    minus: Vec<u32>,
    /// `topes`, in order
    topes: Vec<u32>,
    /// per cell id, its position in `topes` (or NONE)
    tope_pos: Vec<u32>,
    /// per cell id, the first input face it is a piece of (or NONE): `origin`
    first_face: Vec<u32>,
    /// per facet and body: `isPieceOf`
    piece_of: Vec<Vec<bool>>,
}

/// `splitOk`: every piece a facet of its face's class, in a body that exists, and no facet
/// inside two faces of one body.
fn split_ok(l: &Ledger, ext: &Ext) -> bool {
    let mut ok = true;
    let mut triples: Vec<(u32, u32, u32)> = Vec::new();
    let mut k = 0;
    while ok && k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        if (c.0 as usize) >= ncells(l, ext) {
            ok = false;
        } else {
            let x = cov(l, ext, c.0);
            let b = body_of(l, f);
            ok = is_facet(x, wd(l)) && zero_idx(x, wd(l)) == cls_of(l, f) as usize && b < l.nb;
            triples.push((c.0, b, f.0));
        }
        k += 1;
    }
    if ok {
        sort::sort_triples(&mut triples);
    }
    let mut k = 1;
    while ok && k < triples.len() {
        let (c0, b0, f0) = triples[k - 1];
        let (c1, b1, f1) = triples[k];
        ok = !(c0 == c1 && b0 == b1 && f0 != f1);
        k += 1;
    }
    ok
}

fn split(l: &Ledger, ext: &mut Ext) -> Split {
    let n = ncells(l, ext);
    let mut first_face = vec![NONE; n];
    let mut facets = Vec::new();
    let mut k = 0;
    while k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        if first_face[c.0 as usize] == NONE {
            first_face[c.0 as usize] = f.0;
            facets.push(c.0);
        }
        k += 1;
    }
    let mut plus = Vec::new();
    let mut minus = Vec::new();
    let mut i = 0;
    while i < facets.len() {
        let x = cov(l, ext, facets[i]);
        let z = zero_idx(x, wd(l));
        plus.push(intern(l, ext, with_set(x, wd(l), z, Sgn::Pos)));
        minus.push(intern(l, ext, with_set(x, wd(l), z, Sgn::Neg)));
        i += 1;
    }
    let mut tope_pos = vec![NONE; ncells(l, ext)];
    let mut topes = Vec::new();
    let mut i = 0;
    while i < facets.len() {
        let p = plus[i];
        if tope_pos[p as usize] == NONE {
            tope_pos[p as usize] = topes.len() as u32;
            topes.push(p);
        }
        let m = minus[i];
        if tope_pos[m as usize] == NONE {
            tope_pos[m as usize] = topes.len() as u32;
            topes.push(m);
        }
        i += 1;
    }
    let mut facet_pos = vec![NONE; ncells(l, ext)];
    let mut i = 0;
    while i < facets.len() {
        facet_pos[facets[i] as usize] = i as u32;
        i += 1;
    }
    let mut piece_of = vec![vec![false; l.nb as usize]; facets.len()];
    let mut k = 0;
    while k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        let b = body_of(l, f) as usize;
        let p = facet_pos[c.0 as usize] as usize;
        if b < l.nb as usize {
            piece_of[p][b] = true;
        }
        k += 1;
    }
    Split {
        facets,
        plus,
        minus,
        topes,
        tope_pos,
        first_face,
        piece_of,
    }
}

/// Body `i`'s flood over the topes: the union–find of `crossings i`, root the least position, so
/// `find T` is the Lean's `head (adjOf (crossings i)) topes T`.
fn flood(s: &Split, i: usize) -> Uf {
    let mut uf = Uf::new(s.topes.len());
    let mut k = 0;
    while k < s.facets.len() {
        if !s.piece_of[k][i] {
            let a = s.tope_pos[s.plus[k] as usize] as usize;
            let b = s.tope_pos[s.minus[k] as usize] as usize;
            uf.union(a, b);
        }
        k += 1;
    }
    uf
}

/// Each flood class's witness, per body: the (side cell, body) pairs the bridge must decide by a
/// ray before the core runs (`Ledger::inside` is read only at these). In body order, then tope
/// order. The ledger's `inside` is not read.
pub fn witnesses(l: &Ledger) -> Vec<(CellId, BodyId)> {
    let mut ext = new_ext(l);
    let mut out = Vec::new();
    // (none past a row's width: the core skips such a ledger)
    if l.width <= row::MAX_WIDTH && split_ok(l, &ext) {
        let s = split(l, &mut ext);
        let mut i = 0;
        while i < l.nb as usize {
            let mut uf = flood(&s, i);
            let mut t = 0;
            while t < s.topes.len() {
                if uf.find(t) == t {
                    out.push((CellId(s.topes[t]), BodyId(i as u32)));
                }
                t += 1;
            }
            i += 1;
        }
    }
    out
}

/// `memb i T` for every body and tope: `memb[i][t]`.
fn membership(l: &Ledger, ext: &Ext, s: &Split) -> Vec<Vec<bool>> {
    let nb = l.nb as usize;
    // the witness answers, per body, by cell
    let mut inside = vec![vec![false; ncells(l, ext)]; nb];
    let mut k = 0;
    while k < l.inside.len() {
        let (c, b) = l.inside[k];
        if (b.0 as usize) < nb && (c.0 as usize) < ncells(l, ext) {
            inside[b.0 as usize][c.0 as usize] = true;
        }
        k += 1;
    }
    let mut memb = Vec::new();
    let mut i = 0;
    while i < nb {
        let mut uf = flood(s, i);
        let mut row = Vec::new();
        let mut t = 0;
        while t < s.topes.len() {
            let r = uf.find(t);
            row.push(inside[i][s.topes[r] as usize]);
            t += 1;
        }
        memb.push(row);
        i += 1;
    }
    memb
}

/// The rule at one membership vector.
fn apply_rule(rule: &Rule, m: &[bool]) -> bool {
    // the numbers of bodies in and out (the Lean's rules read `m i` past nb as false)
    let mut ins = 0;
    let mut i = 0;
    while i < m.len() {
        if m[i] {
            ins += 1;
        }
        i += 1;
    }
    match rule {
        Rule::Union => ins > 0,
        Rule::Inter => !m.is_empty() && ins == m.len(),
        Rule::Diff => !m.is_empty() && m[0] && ins == 1,
        Rule::Table(t) => {
            let mut row: usize = 0;
            let mut fits = true;
            let mut i = 0;
            while i < m.len() {
                if m[i] {
                    if i >= usize::BITS as usize {
                        fits = false;
                    } else {
                        row |= 1usize << i;
                    }
                }
                i += 1;
            }
            fits && row < t.len() && t[row]
        }
    }
}

/// The work of stages 4–8 over one split.
struct Assembled {
    /// `result`: the result facets, in facet order
    result: Vec<u32>,
    /// per result facet, `orient`
    orient: Vec<Sgn>,
    /// per cell id, its position in `result` (or NONE)
    rpos: Vec<u32>,
}

/// `around x`: the positions in `result` of the result facets `φ` with `x ≤ φ`, in result order.
/// Such a `φ` is `x` with one of its zeros kept and the others given a sign, so for few zeros these
/// are enumerated and looked up; otherwise the result is scanned (the Lean's filter).
fn around(l: &Ledger, ext: &Ext, a: &Assembled, x: u32) -> Vec<u32> {
    let xc = cov(l, ext, x);
    let z = zeros(xc, wd(l));
    let mut out = Vec::new();
    if z.len() > ENUM_ZEROS {
        let mut p = 0;
        while p < a.result.len() {
            if le(xc, cov(l, ext, a.result[p])) {
                out.push(p as u32);
            }
            p += 1;
        }
    } else {
        around_enum(l, ext, a, xc, &z, &mut out);
    }
    out
}

/// `around`'s enumeration: `x` (`xc`, zeros `z`) with one zero kept and the others signed.
fn around_enum(l: &Ledger, ext: &Ext, a: &Assembled, xc: Row, z: &[usize], out: &mut Vec<u32>) {
    let mut buf = xc;
    let k = z.len();
    let mut keep = 0;
    while keep < k {
        let combos: u32 = 1u32 << (k - 1);
        let mut m: u32 = 0;
        while m < combos {
            let mut bit = 0;
            let mut j = 0;
            while j < k {
                if j == keep {
                    buf = row_set(buf, z[j], Sgn::Zero);
                } else {
                    buf = row_set(buf, z[j], if (m >> bit) & 1 == 1 { Sgn::Pos } else { Sgn::Neg });
                    bit += 1;
                }
                j += 1;
            }
            let id = find(l, ext, buf);
            if id != NONE && a.rpos[id as usize] != NONE {
                out.push(a.rpos[id as usize]);
            }
            m += 1;
        }
        keep += 1;
    }
    sort::sort_u32(out);
}

/// The cells `e` with `v ≤ e ≤ φ` other than `φ` itself: `φ` with a non-empty set of `v`'s other
/// zeros zeroed (`z` is `v`'s zeros). Only those the ledger has are returned, by id.
fn between(l: &Ledger, ext: &Ext, z: &[usize], phi: u32) -> Vec<u32> {
    let pc = cov(l, ext, phi);
    let zp = zero_idx(pc, wd(l));
    let mut others = Vec::new();
    let mut j = 0;
    while j < z.len() {
        if z[j] != zp {
            others.push(z[j]);
        }
        j += 1;
    }
    let mut out = Vec::new();
    let k = others.len();
    let combos: u32 = 1u32 << k;
    let mut buf = pc;
    let mut m: u32 = 1;
    while m < combos {
        let mut j = 0;
        while j < k {
            buf = row_set(
                buf,
                others[j],
                if (m >> j) & 1 == 1 {
                    Sgn::Zero
                } else {
                    row_get(pc, others[j])
                },
            );
            j += 1;
        }
        let id = find(l, ext, buf);
        if id != NONE {
            out.push(id);
        }
        m += 1;
    }
    out
}

/// The edge cells `e` (of `is_edge`) with `v ≤ e ≤ φ`.
fn edges_between(l: &Ledger, ext: &Ext, is_edge: &[bool], v: u32, z: &[usize], phi: u32) -> Vec<u32> {
    let mut out = Vec::new();
    if z.len() > ENUM_ZEROS {
        let vc = cov(l, ext, v);
        let pc = cov(l, ext, phi);
        let mut e = 0;
        while e < is_edge.len() {
            if is_edge[e] && le(vc, cov(l, ext, e as u32)) && le(cov(l, ext, e as u32), pc) {
                out.push(e as u32);
            }
            e += 1;
        }
    } else {
        let bs = between(l, ext, z, phi);
        let mut i = 0;
        while i < bs.len() {
            if is_edge[bs[i] as usize] {
                out.push(bs[i]);
            }
            i += 1;
        }
        // (`φ` itself, with nothing zeroed, is an edge cell only if the ledger says so)
        if is_edge[phi as usize] {
            out.push(phi);
        }
    }
    out
}

/// **The core**: one pass, no retries (`ForgeExact.DecideOnce.coreM`: `core` with the metric
/// stage, which is the identity when `m.thin` is empty).
pub fn core(l: &Ledger, rule: &Rule, m: &MetricLedger) -> Outcome {
    if l.width > row::MAX_WIDTH {
        // (checked before any row is read)
        Outcome::Skip(SkipReason::Wide)
    } else {
        let mut ext = new_ext(l);
        // 1–2. split and imprint
        if !split_ok(l, &ext) {
            Outcome::Refused(Refusal::SplitCrossed)
        } else {
            let s = split(l, &mut ext);
            classify_on(l, &ext, &s, rule, m)
        }
    }
}

/// Stage 3's check, then 4–8.
fn classify_on(l: &Ledger, ext: &Ext, s: &Split, rule: &Rule, mt: &MetricLedger) -> Outcome {
    // 3. classify: flood plus witness
    let memb = membership(l, ext, s);
    let nb = l.nb as usize;
    let mut ok = true;
    let mut k = 0;
    while ok && k < s.facets.len() {
        let p = s.tope_pos[s.plus[k] as usize] as usize;
        let m = s.tope_pos[s.minus[k] as usize] as usize;
        let mut i = 0;
        while ok && i < nb {
            ok = !(s.piece_of[k][i] && memb[i][p] == memb[i][m]);
            i += 1;
        }
        k += 1;
    }
    if !ok {
        Outcome::Refused(Refusal::InconsistentClassification)
    } else {
        keep_on(l, ext, s, rule, &memb, mt)
    }
}

/// Stages 4–8.
fn keep_on(l: &Ledger, ext: &Ext, s: &Split, rule: &Rule, memb: &[Vec<bool>], mt: &MetricLedger) -> Outcome {
    let nb = l.nb as usize;
    // 4. keep, tabled once per tope
    let mut keep = Vec::new();
    let mut t = 0;
    while t < s.topes.len() {
        let mut m = Vec::new();
        let mut i = 0;
        while i < nb {
            m.push(memb[i][t]);
            i += 1;
        }
        keep.push(apply_rule(rule, &m));
        t += 1;
    }
    // 4b. collapse the sub-LINEAR features
    let bad = collapse(l, ext, s, &mut keep, mt);
    if bad != NONE {
        Outcome::Refused(Refusal::ZeroThickness(CellId(bad)))
    } else {
        core_k(l, ext, s, &keep)
    }
}

/// Stage 4b (`Ledger.collapse`): the thin side cells fall into features (classes under "the two
/// sides of a facet"); a feature's rim is the cells that are not thin beside it. If every
/// feature's rim is kept alike and every cell of it is enclosed, each thin cell takes its rim's
/// first cell's keep (its own if the rim is empty), in `keep`; and NONE is returned. Else the
/// first thin cell, in tope order, whose feature is not so (its id).
fn collapse(l: &Ledger, ext: &Ext, s: &Split, keep: &mut [bool], mt: &MetricLedger) -> u32 {
    let nt = s.topes.len();
    let n = ncells(l, ext);
    let mut is_thin = vec![false; nt];
    let mut is_enc = vec![false; nt];
    let mut any = false;
    let mut k = 0;
    while k < mt.thin.len() {
        let c = mt.thin[k].0 as usize;
        if c < n && s.tope_pos[c] != NONE {
            is_thin[s.tope_pos[c] as usize] = true;
            any = true;
        }
        k += 1;
    }
    let mut bad = NONE;
    if any {
        let mut k = 0;
        while k < mt.enclosed.len() {
            let c = mt.enclosed[k].0 as usize;
            if c < n && s.tope_pos[c] != NONE {
                is_enc[s.tope_pos[c] as usize] = true;
            }
            k += 1;
        }
        // the features: thin cells joined across a facet, rooted at their first in tope order
        let mut uf = Uf::new(nt);
        let mut k = 0;
        while k < s.facets.len() {
            let p = s.tope_pos[s.plus[k] as usize] as usize;
            let q = s.tope_pos[s.minus[k] as usize] as usize;
            if is_thin[p] && is_thin[q] {
                uf.union(p, q);
            }
            k += 1;
        }
        // each feature's rim (by root), and whether every cell of it is enclosed
        let mut rim: Vec<Vec<u32>> = vec![Vec::new(); nt];
        let mut all_enc = vec![true; nt];
        let mut t = 0;
        while t < nt {
            if is_thin[t] && !is_enc[t] {
                let r = uf.find(t);
                all_enc[r] = false;
            }
            t += 1;
        }
        let mut k = 0;
        while k < s.facets.len() {
            let p = s.tope_pos[s.plus[k] as usize] as usize;
            let q = s.tope_pos[s.minus[k] as usize] as usize;
            if is_thin[p] && !is_thin[q] {
                let r = uf.find(p);
                rim[r].push(q as u32);
            }
            if is_thin[q] && !is_thin[p] {
                let r = uf.find(q);
                rim[r].push(p as u32);
            }
            k += 1;
        }
        let mut t = 0;
        while t < nt {
            if rim[t].len() > 1 {
                sort::sort_u32(&mut rim[t]);
                let mut d: Vec<u32> = Vec::new();
                let mut j = 0;
                while j < rim[t].len() {
                    if d.is_empty() || d[d.len() - 1] != rim[t][j] {
                        d.push(rim[t][j]);
                    }
                    j += 1;
                }
                rim[t] = d;
            }
            t += 1;
        }
        // the first thin cell whose feature cannot be collapsed
        let mut t = 0;
        while bad == NONE && t < nt {
            if is_thin[t] {
                let r = uf.find(t);
                let rm = &rim[r];
                let first = if rm.is_empty() {
                    keep[t]
                } else {
                    keep[rm[0] as usize]
                };
                let mut uniform = true;
                let mut j = 0;
                while uniform && j < rm.len() {
                    uniform = keep[rm[j] as usize] == first;
                    j += 1;
                }
                if !(uniform && all_enc[r]) {
                    bad = s.topes[t];
                }
            }
            t += 1;
        }
        if bad == NONE {
            let old = keep.to_vec();
            let mut t = 0;
            while t < nt {
                if is_thin[t] {
                    let r = uf.find(t);
                    if !rim[r].is_empty() {
                        keep[t] = old[rim[r][0] as usize];
                    }
                }
                t += 1;
            }
        }
    }
    bad
}

/// Stages 5–8 on a keep table (`Ledger.coreK`).
fn core_k(l: &Ledger, ext: &Ext, s: &Split, keep: &[bool]) -> Outcome {
    // 5. assemble
    let mut a = Assembled {
        result: Vec::new(),
        orient: Vec::new(),
        rpos: vec![NONE; ncells(l, ext)],
    };
    let mut k = 0;
    while k < s.facets.len() {
        let kp = keep[s.tope_pos[s.plus[k] as usize] as usize];
        let km = keep[s.tope_pos[s.minus[k] as usize] as usize];
        if kp != km {
            a.rpos[s.facets[k] as usize] = a.result.len() as u32;
            a.result.push(s.facets[k]);
            a.orient.push(if km { Sgn::Pos } else { Sgn::Neg });
        }
        k += 1;
    }
    // 6. the touch rule: edges, then vertices; the first bad cell refused
    let mut is_edge = vec![false; ncells(l, ext)];
    let mut k = 0;
    while k < l.edges.len() {
        let e = l.edges[k].0;
        if (e as usize) < is_edge.len() {
            is_edge[e as usize] = true;
        }
        k += 1;
    }
    let mut bad = NONE;
    let mut around_e = Vec::new();
    let mut k = 0;
    while bad == NONE && k < l.edges.len() {
        let e = l.edges[k].0;
        let ar = around(l, ext, &a, e);
        if ar.len() > 2 {
            bad = e;
        }
        around_e.push(ar);
        k += 1;
    }
    let mut around_v = Vec::new();
    let mut k = 0;
    while bad == NONE && k < l.verts.len() {
        let v = l.verts[k].0;
        let ar = around(l, ext, &a, v);
        if bad_vert(l, ext, &a, &is_edge, v, &ar) {
            bad = v;
        }
        around_v.push(ar);
        k += 1;
    }
    if bad != NONE {
        Outcome::Refused(Refusal::ZeroThickness(CellId(bad)))
    } else if a.result.is_empty() {
        // 7. empty
        Outcome::Empty
    } else {
        // 8. unify
        Outcome::Ok(unify(l, ext, s, &a, &is_edge, &around_e, &around_v))
    }
}

/// `badVert`: the result facets round `v` (`ar`) in more than one class of `fanRel v`.
fn bad_vert(l: &Ledger, ext: &Ext, a: &Assembled, is_edge: &[bool], v: u32, ar: &[u32]) -> bool {
    let z = zeros(cov(l, ext, v), wd(l));
    let mut uf = Uf::new(ar.len());
    // the first fan member seen through each edge cell: (edge, local index)
    let mut seen: Vec<(u32, u32)> = Vec::new();
    let mut j = 0;
    while j < ar.len() {
        let phi = a.result[ar[j] as usize];
        let es = edges_between(l, ext, is_edge, v, &z, phi);
        let mut q = 0;
        while q < es.len() {
            let mut found = NONE;
            let mut r = 0;
            while found == NONE && r < seen.len() {
                if seen[r].0 == es[q] {
                    found = seen[r].1;
                }
                r += 1;
            }
            if found == NONE {
                seen.push((es[q], j as u32));
            } else {
                uf.union(found as usize, j);
            }
            q += 1;
        }
        j += 1;
    }
    let mut bad = false;
    let mut j = 1;
    while !bad && j < ar.len() {
        bad = uf.find(j) != uf.find(0);
        j += 1;
    }
    bad
}

/// `u32` in a list.
fn has(xs: &[u32], x: u32) -> bool {
    let mut found = false;
    let mut i = 0;
    while !found && i < xs.len() {
        found = xs[i] == x;
        i += 1;
    }
    found
}

/// `dedup (around x).map key`: the faces round a cell, as face numbers, in the order their facets
/// come in the result.
fn faces_at(face_of: &[u32], ar: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut j = 0;
    while j < ar.len() {
        let f = face_of[ar[j] as usize];
        if !has(&out, f) {
            out.push(f);
        }
        j += 1;
    }
    out
}

/// `sameSet` on two face lists.
fn same_set(a: &[u32], b: &[u32]) -> bool {
    let mut same = true;
    let mut i = 0;
    while same && i < a.len() {
        same = has(b, a[i]);
        i += 1;
    }
    let mut i = 0;
    while same && i < b.len() {
        same = has(a, b[i]);
        i += 1;
    }
    same
}

/// **Unify** (`Ledger.unify`): reads the assembled facets by reference and writes a fresh body.
fn unify(
    l: &Ledger,
    ext: &Ext,
    s: &Split,
    a: &Assembled,
    is_edge: &[bool],
    around_e: &[Vec<u32>],
    around_v: &[Vec<u32>],
) -> Body {
    // faces: the classes of `joinRel` over the result, rooted at their first facet
    let nr = a.result.len();
    let mut join = Uf::new(nr);
    let mut k = 0;
    while k < around_e.len() {
        let ar = &around_e[k];
        let mut p = 0;
        while p < ar.len() {
            let mut q = p + 1;
            while q < ar.len() {
                let (c, d) = (ar[p] as usize, ar[q] as usize);
                if zero_idx(cov(l, ext, a.result[c]), wd(l)) == zero_idx(cov(l, ext, a.result[d]), wd(l))
                    && a.orient[c] == a.orient[d]
                {
                    join.union(c, d);
                }
                q += 1;
            }
            p += 1;
        }
        k += 1;
    }
    // per result position, its face number (heads numbered in result order)
    let mut face_of = vec![NONE; nr];
    let mut faces: Vec<OutFace> = Vec::new();
    let mut heads: Vec<u32> = Vec::new();
    let mut p = 0;
    while p < nr {
        let r = join.find(p);
        if r == p {
            face_of[p] = faces.len() as u32;
            let h = a.result[p];
            let ff = s.first_face[h as usize];
            faces.push(OutFace {
                facets: Vec::new(),
                plane: zero_idx(cov(l, ext, h), wd(l)) as u32,
                orient: a.orient[p],
                origin: FaceId(if ff == NONE { 0 } else { ff }),
            });
            heads.push(h);
        } else {
            face_of[p] = face_of[r];
        }
        faces[face_of[p] as usize].facets.push(CellId(a.result[p]));
        p += 1;
    }
    // the faces round each edge and vertex cell
    let mut at_e = Vec::new();
    let mut k = 0;
    while k < around_e.len() {
        at_e.push(faces_at(&face_of, &around_e[k]));
        k += 1;
    }
    let mut at_v = Vec::new();
    let mut k = 0;
    while k < around_v.len() {
        at_v.push(faces_at(&face_of, &around_v[k]));
        k += 1;
    }
    // vertices: three or more faces
    let mut verts = Vec::new();
    let mut is_out_v = vec![false; l.verts.len()];
    let mut k = 0;
    while k < l.verts.len() {
        if at_v[k].len() >= 3 {
            verts.push(l.verts[k]);
            is_out_v[k] = true;
        }
        k += 1;
    }
    // edge cells between two faces, numbered in edge order
    let mut ecs: Vec<usize> = Vec::new();
    let mut ec_pos = vec![NONE; ncells(l, ext)];
    let mut k = 0;
    while k < l.edges.len() {
        if at_e[k].len() == 2 {
            ec_pos[l.edges[k].0 as usize] = ecs.len() as u32;
            ecs.push(k);
        }
        k += 1;
    }
    // joined through vertex cells that are not vertices, when they join one pair of faces
    let mut erel = Uf::new(ecs.len());
    let mut k = 0;
    while k < l.verts.len() {
        if !is_out_v[k] {
            let v = l.verts[k].0;
            let z = zeros(cov(l, ext, v), wd(l));
            let mut round: Vec<u32> = Vec::new();
            let ar = &around_v[k];
            let mut j = 0;
            while j < ar.len() {
                let es = edges_between(l, ext, is_edge, v, &z, a.result[ar[j] as usize]);
                let mut q = 0;
                while q < es.len() {
                    let ep = ec_pos[es[q] as usize];
                    if ep != NONE && !has(&round, ep) {
                        round.push(ep);
                    }
                    q += 1;
                }
                j += 1;
            }
            let mut p = 0;
            while p < round.len() {
                let mut q = p + 1;
                while q < round.len() {
                    let (e1, e2) = (round[p] as usize, round[q] as usize);
                    if same_set(&at_e[ecs[e1]], &at_e[ecs[e2]]) {
                        erel.union(e1, e2);
                    }
                    q += 1;
                }
                p += 1;
            }
        }
        k += 1;
    }
    let mut edges: Vec<OutEdge> = Vec::new();
    let mut edge_of = vec![NONE; ecs.len()];
    let mut p = 0;
    while p < ecs.len() {
        let r = erel.find(p);
        if r == p {
            edge_of[p] = edges.len() as u32;
            let fs = &at_e[ecs[p]];
            let mut fcs = Vec::new();
            let mut q = 0;
            while q < fs.len() {
                fcs.push(CellId(heads[fs[q] as usize]));
                q += 1;
            }
            edges.push(OutEdge {
                cells: Vec::new(),
                faces: fcs,
            });
        } else {
            edge_of[p] = edge_of[r];
        }
        edges[edge_of[p] as usize].cells.push(l.edges[ecs[p]]);
        p += 1;
    }
    Body { faces, edges, verts }
}
