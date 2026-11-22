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
mod sort;
mod uf;

pub use index::Index;
use uf::Uf;

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
    /// Per input face, its body.
    pub face_body: Vec<BodyId>,
    /// Per input face, its carrier class (two faces one surface).
    pub face_class: Vec<u32>,
    /// A cell's covector: its row of the side table, one sign per class. No two rows equal. Every
    /// facet's two sides should be here too (the witnesses name them); one that is not is added by
    /// the core for its own use.
    pub cells: Vec<Vec<Sgn>>,
    /// Decided: the facet lies inside the face. In face arena order.
    pub pieces: Vec<(CellId, FaceId)>,
    /// Decided: the side cell is inside the body (the witness answers that are "inside").
    pub inside: Vec<(CellId, BodyId)>,
    /// Decided: the cells of dimension 1 round the pieces, without repeats.
    pub edges: Vec<CellId>,
    /// Decided: the cells of dimension 0 round the pieces, without repeats.
    pub verts: Vec<CellId>,
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

/// The core's answer. There is no retry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok(Body),
    Empty,
    Refused(Refusal),
}

const NONE: u32 = u32::MAX;

/// Past this many zeros a cell's neighbours are found by scanning instead of enumerating.
const ENUM_ZEROS: usize = 12;

/// `Cov.zeroIdx`: the first zero, or the length if there is none.
pub fn zero_idx(x: &[Sgn]) -> usize {
    let mut i = 0;
    while i < x.len() {
        if x[i] == Sgn::Zero {
            return i;
        }
        i += 1;
    }
    x.len()
}

/// `Cov.isFacet`: exactly one zero.
pub fn is_facet(x: &[Sgn]) -> bool {
    let mut n = 0;
    let mut i = 0;
    while i < x.len() {
        if x[i] == Sgn::Zero {
            n += 1;
        }
        i += 1;
    }
    n == 1
}

/// `Cov.le`: `x` is a face of `y`.
pub fn le(x: &[Sgn], y: &[Sgn]) -> bool {
    if x.len() != y.len() {
        return false;
    }
    let mut i = 0;
    while i < x.len() {
        if x[i] != Sgn::Zero && x[i] != y[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// `X.set i s` (unchanged past the end, as `List.set`).
fn with_set(x: &[Sgn], i: usize, s: Sgn) -> Vec<Sgn> {
    let mut v = x.to_vec();
    if i < v.len() {
        v[i] = s;
    }
    v
}

/// The zeros of a covector.
fn zeros(x: &[Sgn]) -> Vec<usize> {
    let mut z = Vec::new();
    let mut i = 0;
    while i < x.len() {
        if x[i] == Sgn::Zero {
            z.push(i);
        }
        i += 1;
    }
    z
}

/// The cells the core works over: the ledger's, and any side cell it lacked.
struct Cells<'a> {
    l: &'a Ledger,
    index: Index,
    extra: Vec<Vec<Sgn>>,
}

impl Cells<'_> {
    fn cov(&self, id: u32) -> &[Sgn] {
        let n = self.l.cells.len();
        if (id as usize) < n {
            &self.l.cells[id as usize]
        } else {
            &self.extra[id as usize - n]
        }
    }

    fn len(&self) -> usize {
        self.l.cells.len() + self.extra.len()
    }

    fn find(&self, key: &[Sgn]) -> u32 {
        let f = self.index.find(&self.l.cells, key);
        if f != NONE {
            return f;
        }
        let mut i = 0;
        while i < self.extra.len() {
            if self.extra[i].as_slice() == key {
                return (self.l.cells.len() + i) as u32;
            }
            i += 1;
        }
        NONE
    }

    /// The id of `key`, added if new.
    fn intern(&mut self, key: Vec<Sgn>) -> u32 {
        let f = self.find(&key);
        if f != NONE {
            return f;
        }
        self.extra.push(key);
        (self.len() - 1) as u32
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
fn split_ok(l: &Ledger, cells: &Cells) -> bool {
    let mut triples: Vec<(u32, u32, u32)> = Vec::new();
    let mut k = 0;
    while k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        if (c.0 as usize) >= cells.len() {
            return false;
        }
        let x = cells.cov(c.0);
        if !is_facet(x) {
            return false;
        }
        if zero_idx(x) != cls_of(l, f) as usize {
            return false;
        }
        let b = body_of(l, f);
        if b >= l.nb {
            return false;
        }
        triples.push((c.0, b, f.0));
        k += 1;
    }
    sort::sort_triples(&mut triples);
    let mut k = 1;
    while k < triples.len() {
        let (c0, b0, f0) = triples[k - 1];
        let (c1, b1, f1) = triples[k];
        if c0 == c1 && b0 == b1 && f0 != f1 {
            return false;
        }
        k += 1;
    }
    true
}

fn split(l: &Ledger, cells: &mut Cells) -> Split {
    let n = cells.len();
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
        let x = cells.cov(facets[i]).to_vec();
        let z = zero_idx(&x);
        plus.push(cells.intern(with_set(&x, z, Sgn::Pos)));
        minus.push(cells.intern(with_set(&x, z, Sgn::Neg)));
        i += 1;
    }
    let mut tope_pos = vec![NONE; cells.len()];
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
    let mut facet_pos = vec![NONE; cells.len()];
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
    let mut cells = Cells {
        l,
        index: Index::new(&l.cells),
        extra: Vec::new(),
    };
    let mut out = Vec::new();
    if !split_ok(l, &cells) {
        return out;
    }
    let s = split(l, &mut cells);
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
    out
}

/// `memb i T` for every body and tope: `memb[i][t]`.
fn membership(l: &Ledger, cells: &Cells, s: &Split) -> Vec<Vec<bool>> {
    let nb = l.nb as usize;
    // the witness answers, per body, by cell
    let mut inside = vec![vec![false; cells.len()]; nb];
    let mut k = 0;
    while k < l.inside.len() {
        let (c, b) = l.inside[k];
        if (b.0 as usize) < nb && (c.0 as usize) < cells.len() {
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
    match rule {
        Rule::Union => {
            let mut i = 0;
            while i < m.len() {
                if m[i] {
                    return true;
                }
                i += 1;
            }
            false
        }
        Rule::Inter => {
            let mut i = 0;
            while i < m.len() {
                if !m[i] {
                    return false;
                }
                i += 1;
            }
            // (the Lean's rules read `m i` for i past nb as false)
            !m.is_empty()
        }
        Rule::Diff => {
            if m.is_empty() || !m[0] {
                return false;
            }
            let mut i = 1;
            while i < m.len() {
                if m[i] {
                    return false;
                }
                i += 1;
            }
            true
        }
        Rule::Table(t) => {
            let mut row: usize = 0;
            let mut i = 0;
            while i < m.len() {
                if m[i] {
                    if i >= usize::BITS as usize {
                        return false;
                    }
                    row |= 1usize << i;
                }
                i += 1;
            }
            row < t.len() && t[row]
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
fn around(cells: &Cells, a: &Assembled, x: u32) -> Vec<u32> {
    let xc = cells.cov(x);
    let z = zeros(xc);
    let mut out = Vec::new();
    if z.len() > ENUM_ZEROS {
        let mut p = 0;
        while p < a.result.len() {
            if le(xc, cells.cov(a.result[p])) {
                out.push(p as u32);
            }
            p += 1;
        }
        return out;
    }
    let mut buf = xc.to_vec();
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
                    buf[z[j]] = Sgn::Zero;
                } else {
                    buf[z[j]] = if (m >> bit) & 1 == 1 { Sgn::Pos } else { Sgn::Neg };
                    bit += 1;
                }
                j += 1;
            }
            let id = cells.find(&buf);
            if id != NONE && a.rpos[id as usize] != NONE {
                out.push(a.rpos[id as usize]);
            }
            m += 1;
        }
        keep += 1;
    }
    sort::sort_u32(&mut out);
    out
}

/// The cells `e` with `v ≤ e ≤ φ` other than `φ` itself: `φ` with a non-empty set of `v`'s other
/// zeros zeroed (`z` is `v`'s zeros). Only those the ledger has are returned, by id.
fn between(cells: &Cells, z: &[usize], phi: u32) -> Vec<u32> {
    let pc = cells.cov(phi);
    let zp = zero_idx(pc);
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
    let mut buf = pc.to_vec();
    let mut m: u32 = 1;
    while m < combos {
        let mut j = 0;
        while j < k {
            buf[others[j]] = if (m >> j) & 1 == 1 {
                Sgn::Zero
            } else {
                pc[others[j]]
            };
            j += 1;
        }
        let id = cells.find(&buf);
        if id != NONE {
            out.push(id);
        }
        m += 1;
    }
    out
}

/// The edge cells `e` (of `is_edge`) with `v ≤ e ≤ φ`.
fn edges_between(cells: &Cells, is_edge: &[bool], v: u32, z: &[usize], phi: u32) -> Vec<u32> {
    let mut out = Vec::new();
    if z.len() > ENUM_ZEROS {
        let vc = cells.cov(v);
        let pc = cells.cov(phi);
        let mut e = 0;
        while e < is_edge.len() {
            if is_edge[e] && le(vc, cells.cov(e as u32)) && le(cells.cov(e as u32), pc) {
                out.push(e as u32);
            }
            e += 1;
        }
        return out;
    }
    let bs = between(cells, z, phi);
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
    out
}

/// **The core**: one pass, no retries (`ForgeExact.DecideOnce.core`).
pub fn core(l: &Ledger, rule: &Rule) -> Outcome {
    let mut cells = Cells {
        l,
        index: Index::new(&l.cells),
        extra: Vec::new(),
    };
    // 1–2. split and imprint
    if !split_ok(l, &cells) {
        return Outcome::Refused(Refusal::SplitCrossed);
    }
    let s = split(l, &mut cells);
    let cells = cells;
    // 3. classify: flood plus witness
    let memb = membership(l, &cells, &s);
    let nb = l.nb as usize;
    let mut k = 0;
    while k < s.facets.len() {
        let p = s.tope_pos[s.plus[k] as usize] as usize;
        let m = s.tope_pos[s.minus[k] as usize] as usize;
        let mut i = 0;
        while i < nb {
            if s.piece_of[k][i] && memb[i][p] == memb[i][m] {
                return Outcome::Refused(Refusal::InconsistentClassification);
            }
            i += 1;
        }
        k += 1;
    }
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
    // 5. assemble
    let mut a = Assembled {
        result: Vec::new(),
        orient: Vec::new(),
        rpos: vec![NONE; cells.len()],
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
    // 6. the touch rule: edges, then vertices
    let mut is_edge = vec![false; cells.len()];
    let mut around_e = Vec::new();
    let mut k = 0;
    while k < l.edges.len() {
        let e = l.edges[k].0;
        if (e as usize) < is_edge.len() {
            is_edge[e as usize] = true;
        }
        k += 1;
    }
    let mut k = 0;
    while k < l.edges.len() {
        let e = l.edges[k].0;
        let ar = around(&cells, &a, e);
        if ar.len() > 2 {
            return Outcome::Refused(Refusal::ZeroThickness(CellId(e)));
        }
        around_e.push(ar);
        k += 1;
    }
    let mut around_v = Vec::new();
    let mut k = 0;
    while k < l.verts.len() {
        let v = l.verts[k].0;
        let ar = around(&cells, &a, v);
        if bad_vert(&cells, &a, &is_edge, v, &ar) {
            return Outcome::Refused(Refusal::ZeroThickness(CellId(v)));
        }
        around_v.push(ar);
        k += 1;
    }
    // 7. empty
    if a.result.is_empty() {
        return Outcome::Empty;
    }
    // 8. unify
    Outcome::Ok(unify(l, &cells, &s, &a, &is_edge, &around_e, &around_v))
}

/// `badVert`: the result facets round `v` (`ar`) in more than one class of `fanRel v`.
fn bad_vert(cells: &Cells, a: &Assembled, is_edge: &[bool], v: u32, ar: &[u32]) -> bool {
    if ar.is_empty() {
        return false;
    }
    let z = zeros(cells.cov(v));
    let mut uf = Uf::new(ar.len());
    // the first fan member seen through each edge cell: (edge, local index)
    let mut seen: Vec<(u32, u32)> = Vec::new();
    let mut j = 0;
    while j < ar.len() {
        let phi = a.result[ar[j] as usize];
        let es = edges_between(cells, is_edge, v, &z, phi);
        let mut q = 0;
        while q < es.len() {
            let mut found = NONE;
            let mut r = 0;
            while r < seen.len() {
                if seen[r].0 == es[q] {
                    found = seen[r].1;
                    break;
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
    let mut j = 1;
    while j < ar.len() {
        if uf.find(j) != uf.find(0) {
            return true;
        }
        j += 1;
    }
    false
}

/// `dedup (around x).map key`: the faces round a cell, as face numbers, in the order their facets
/// come in the result.
fn faces_at(face_of: &[u32], ar: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut j = 0;
    while j < ar.len() {
        let f = face_of[ar[j] as usize];
        let mut dup = false;
        let mut q = 0;
        while q < out.len() {
            if out[q] == f {
                dup = true;
                break;
            }
            q += 1;
        }
        if !dup {
            out.push(f);
        }
        j += 1;
    }
    out
}

/// `sameSet` on two face lists.
fn same_set(a: &[u32], b: &[u32]) -> bool {
    let mut i = 0;
    while i < a.len() {
        if !b.contains(&a[i]) {
            return false;
        }
        i += 1;
    }
    let mut i = 0;
    while i < b.len() {
        if !a.contains(&b[i]) {
            return false;
        }
        i += 1;
    }
    true
}

/// **Unify** (`Ledger.unify`): reads the assembled facets by reference and writes a fresh body.
fn unify(
    l: &Ledger,
    cells: &Cells,
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
                if zero_idx(cells.cov(a.result[c])) == zero_idx(cells.cov(a.result[d]))
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
                plane: zero_idx(cells.cov(h)) as u32,
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
    let mut ec_pos = vec![NONE; cells.len()];
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
            let z = zeros(cells.cov(v));
            let mut round: Vec<u32> = Vec::new();
            let ar = &around_v[k];
            let mut j = 0;
            while j < ar.len() {
                let es = edges_between(cells, is_edge, v, &z, a.result[ar[j] as usize]);
                let mut q = 0;
                while q < es.len() {
                    let ep = ec_pos[es[q] as usize];
                    if ep != NONE && !round.contains(&ep) {
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
