//! The A8 ledger (research/decide-once/face-arrangement.md §1): cells are the components of the
//! region minus the near faces, with explicit incidence. Every id is a `u32` index; nothing is a
//! covector. The Lean is `IncLedger` (verify/forge-exact/ForgeExact/DecideOnce/IncCore.lean); the
//! conditions a builder meets against the plane ledger are `Refines` (Refines.lean).
//!
//! Types only, plus `ledger_wf` (`LedgerWF` v2). Written for Aeneas, like the v1 core: while
//! loops, no closures, no iterators, no maps.

use crate::union_find::Uf;
use crate::{BodyId, FaceId, Rule, Sgn, apply_rule, faces_at, same_set};

/// A facet: a component of a near face's relative interior minus its cut set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FacetId(pub u32);

/// A room: a radial class of facet sides (`plus` / `minus` in the Lean).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomId(pub u32);

/// An edge: a component of the union of the cut sets minus the verts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub u32);

/// A vert: where cut segments end or meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertId(pub u32);

/// Compressed rows: row `k` is `ids[start[k] .. start[k + 1]]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Csr {
    pub start: Vec<u32>,
    pub ids: Vec<u32>,
}

/// A sector at a vert: a facet's angular part there, between two of the vert's edges
/// (`Sector` in the Lean: its facet and its bounds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sector {
    pub facet: u32,
    pub a: u32,
    pub b: u32,
}

/// Per vert, its sectors: row `k` is `sec[start[k] .. start[k + 1]]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecCsr {
    pub start: Vec<u32>,
    pub sec: Vec<Sector>,
}

/// The v2 ledger (`IncLedger`). Facets are `0 .. facet_class.len()`, edges the rows of
/// `edge_facets`, verts the rows of `vert_edges` (and of `vert_sectors`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    /// Bodies `0 .. nb`.
    pub nb: u32,
    /// Per input face, its body.
    pub face_body: Vec<BodyId>,
    /// Per input face, its carrier class.
    pub face_class: Vec<u32>,
    /// Rooms `0 .. n_rooms`.
    pub n_rooms: u32,
    /// Per facet, its carrier class (`carrier`).
    pub facet_class: Vec<u32>,
    /// Per facet: (room on its class normal's + side, room on its − side) (`plus`, `minus`).
    pub facet_rooms: Vec<(u32, u32)>,
    /// Decided: the facet lies in the face. Every facet is in one or more.
    pub pieces: Vec<(FacetId, FaceId)>,
    /// Decided: the room is inside the body (the witness answers that are "inside").
    pub inside: Vec<(RoomId, BodyId)>,
    /// Per edge, the facets whose closure holds it, a facet once per wing (so at most twice).
    pub edge_facets: Csr,
    /// Per vert, the edges whose closure holds it, without repeats.
    pub vert_edges: Csr,
    /// Per vert, its sectors.
    pub vert_sectors: SecCsr,
}

/// Stage 4b's tables, per room.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetricLedger {
    /// Rooms thinner than LINEAR.
    pub thin: Vec<RoomId>,
    /// Rooms every boundary facet of which is a piece (none reaches a wall).
    pub enclosed: Vec<RoomId>,
}

/// A face of the result (`OutFaceI`): its facets, carrier and sense. Naming is the write's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFace {
    pub facets: Vec<FacetId>,
    pub plane: u32,
    pub orient: Sgn,
}

/// An edge of the result (`OutEdgeI`): a run of edges between one pair of faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutEdge {
    pub cells: Vec<EdgeId>,
    /// The two faces, each by its first facet.
    pub faces: Vec<FacetId>,
}

/// The result body (`BodyI`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub faces: Vec<OutFace>,
    pub edges: Vec<OutEdge>,
    pub verts: Vec<VertId>,
}

/// Why the v2 core refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    SplitCrossed,
    InconsistentClassification,
    /// A non-manifold edge of the result.
    ZeroThicknessEdge(EdgeId),
    /// A non-manifold vert of the result.
    ZeroThicknessVert(VertId),
    /// A thin room the metric stage cannot collapse.
    ZeroThicknessRoom(RoomId),
}

/// The v2 core's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok(Body),
    Empty,
    Refused(Refusal),
}

/// `start` is not empty and its first entry is 0. (Its own fn, so the translation's test has a
/// declared `Bool` type.)
fn starts_at_zero(start: &[u32]) -> bool {
    if start.is_empty() { false } else { start[0] == 0 }
}

/// A Csr is well formed over `n_ids` ids: `start` starts at 0, never decreases, ends at
/// `ids.len()`, and every id is below `n_ids`.
pub fn csr_ok(c: &Csr, n_ids: u32) -> bool {
    let mut ok = starts_at_zero(&c.start);
    let mut k = 1;
    while ok && k < c.start.len() {
        ok = c.start[k - 1] <= c.start[k];
        k += 1;
    }
    ok = ok && c.start[c.start.len() - 1] as usize == c.ids.len();
    let mut i = 0;
    while ok && i < c.ids.len() {
        ok = c.ids[i] < n_ids;
        i += 1;
    }
    ok
}

/// The number of rows a `start` list gives.
fn rows(start: &[u32]) -> usize {
    if start.is_empty() { 0 } else { start.len() - 1 }
}

/// How often `x` is in `ids[lo .. hi]`.
fn count_in(ids: &[u32], lo: usize, hi: usize, x: u32) -> usize {
    let mut n = 0;
    let mut i = lo;
    while i < hi {
        if ids[i] == x {
            n += 1;
        }
        i += 1;
    }
    n
}

/// How many of `secs[lo..hi]` are `x` (facet and both edges alike).
fn count_sec(secs: &[Sector], lo: usize, hi: usize, x: Sector) -> usize {
    let mut n = 0;
    let mut i = lo;
    while i < hi {
        let y = secs[i];
        if y.facet == x.facet && y.a == x.a && y.b == x.b {
            n += 1;
        }
        i += 1;
    }
    n
}

/// `LedgerWF` v2: the Csrs well formed; a facet at most twice per edge row; no repeat in a
/// vert's edges or in its sectors; face ids and sectors below `u32::MAX`, and the facet-body
/// table's size within `usize`; each sector's facet in range, its two edges in its vert's row and
/// the facet round both; ids and counts in range; a facet's two sides distinct rooms; and every
/// facet a piece of some face of its own class.
pub fn ledger_wf(l: &Ledger) -> bool {
    let nf = l.facet_class.len();
    let ne = rows(&l.edge_facets.start);
    let nv = rows(&l.vert_edges.start);
    let mut ok = l.nb > 0
        && l.face_body.len() == l.face_class.len()
        && l.facet_rooms.len() == nf
        && (nf as u64) < u32::MAX as u64
        && (ne as u64) < u32::MAX as u64
        && (nv as u64) < u32::MAX as u64
        && (l.face_body.len() as u64) < u32::MAX as u64
        && (nf as u128) * (l.nb as u128) <= usize::MAX as u128
        && (l.vert_sectors.sec.len() as u64) < u32::MAX as u64
        && l.n_rooms < u32::MAX
        && csr_ok(&l.edge_facets, nf as u32)
        && csr_ok(&l.vert_edges, ne as u32)
        && l.vert_sectors.start.len() == l.vert_edges.start.len();
    // the sector rows' bounds
    if ok {
        let s = &l.vert_sectors.start;
        ok = s[0] == 0 && s[s.len() - 1] as usize == l.vert_sectors.sec.len();
        let mut k = 1;
        while ok && k < s.len() {
            ok = s[k - 1] <= s[k];
            k += 1;
        }
    }
    // a facet at most twice round an edge
    let mut e = 0;
    while ok && e < ne {
        let (lo, hi) = (
            l.edge_facets.start[e] as usize,
            l.edge_facets.start[e + 1] as usize,
        );
        let mut i = lo;
        while ok && i < hi {
            ok = count_in(&l.edge_facets.ids, lo, hi, l.edge_facets.ids[i]) <= 2;
            i += 1;
        }
        e += 1;
    }
    // no repeated edge at a vert; each sector's facet in range, its edges in the vert's row
    let mut v = 0;
    while ok && v < nv {
        let (lo, hi) = (l.vert_edges.start[v] as usize, l.vert_edges.start[v + 1] as usize);
        let mut i = lo;
        while ok && i < hi {
            ok = count_in(&l.vert_edges.ids, lo, hi, l.vert_edges.ids[i]) == 1;
            i += 1;
        }
        let (slo, shi) = (
            l.vert_sectors.start[v] as usize,
            l.vert_sectors.start[v + 1] as usize,
        );
        let mut j = slo;
        while ok && j < shi {
            let sc = l.vert_sectors.sec[j];
            ok = count_sec(&l.vert_sectors.sec, slo, shi, sc) == 1
                && (sc.facet as usize) < nf
                && count_in(&l.vert_edges.ids, lo, hi, sc.a) == 1
                && count_in(&l.vert_edges.ids, lo, hi, sc.b) == 1;
            // the sector's facet is round both its edges
            if ok {
                let (alo, ahi) = row(&l.edge_facets, sc.a as usize);
                let (blo, bhi) = row(&l.edge_facets, sc.b as usize);
                ok = count_in(&l.edge_facets.ids, alo, ahi, sc.facet) >= 1
                    && count_in(&l.edge_facets.ids, blo, bhi, sc.facet) >= 1;
            }
            j += 1;
        }
        v += 1;
    }
    // a facet's two sides are distinct rooms in range
    let mut f = 0;
    while ok && f < nf {
        let (p, m) = l.facet_rooms[f];
        ok = p != m && p < l.n_rooms && m < l.n_rooms;
        f += 1;
    }
    // pieces and witnesses in range; every facet a piece of some face
    let mut is_piece = vec![false; nf];
    let mut k = 0;
    while ok && k < l.pieces.len() {
        let (c, g) = l.pieces[k];
        ok = (c.0 as usize) < nf
            && (g.0 as usize) < l.face_body.len()
            && l.face_class[g.0 as usize] == l.facet_class[c.0 as usize];
        if ok {
            is_piece[c.0 as usize] = true;
        }
        k += 1;
    }
    let mut k = 0;
    while ok && k < l.inside.len() {
        let (r, b) = l.inside[k];
        ok = r.0 < l.n_rooms && b.0 < l.nb;
        k += 1;
    }
    let mut f = 0;
    while ok && f < nf {
        ok = is_piece[f];
        f += 1;
    }
    ok
}

const NONE: u32 = u32::MAX;

/// A Csr row's bounds.
fn row(c: &Csr, k: usize) -> (usize, usize) {
    (c.start[k] as usize, c.start[k + 1] as usize)
}

/// `IncLedger.bodyOf` (`getD f 0`).
fn body_of(l: &Ledger, f: FaceId) -> u32 {
    if (f.0 as usize) < l.face_body.len() {
        l.face_body[f.0 as usize].0
    } else {
        0
    }
}

/// `IncLedger.clsOf` (`getD f 0`).
fn cls_of(l: &Ledger, f: FaceId) -> u32 {
    if (f.0 as usize) < l.face_class.len() {
        l.face_class[f.0 as usize]
    } else {
        0
    }
}

/// `splitOk`: every piece a listed facet, of its face's carrier, in a body that exists, with two
/// distinct rooms; no facet inside two faces of one body.
fn split_ok(l: &Ledger) -> bool {
    let nf = l.facet_class.len();
    let nb = l.nb as usize;
    // per (facet, body): the face it is a piece of, first seen
    let mut first = vec![NONE; nf * nb];
    let mut ok = l.facet_rooms.len() == nf;
    let mut k = 0;
    while ok && k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        let b = body_of(l, f);
        ok = (c.0 as usize) < nf
            && l.facet_class[c.0 as usize] == cls_of(l, f)
            && b < l.nb
            && l.facet_rooms[c.0 as usize].0 != l.facet_rooms[c.0 as usize].1;
        if ok {
            let at = c.0 as usize * nb + b as usize;
            if first[at] == NONE {
                first[at] = f.0;
            } else {
                ok = first[at] == f.0;
            }
        }
        k += 1;
    }
    ok
}

/// Stages 1–3's tables: per facet and body `isPieceOf`, the rooms in first-seen order (`rooms`),
/// and per room its position there.
struct Split {
    piece_of: Vec<bool>,
    rooms: Vec<u32>,
    room_pos: Vec<u32>,
}

fn split(l: &Ledger) -> Split {
    let nf = l.facet_class.len();
    let nb = l.nb as usize;
    let mut piece_of = vec![false; nf * nb];
    let mut k = 0;
    while k < l.pieces.len() {
        let (c, f) = l.pieces[k];
        let b = body_of(l, f) as usize;
        if b < nb {
            piece_of[c.0 as usize * nb + b] = true;
        }
        k += 1;
    }
    let mut room_pos = vec![NONE; l.n_rooms as usize];
    let mut rooms = Vec::new();
    let mut f = 0;
    while f < nf {
        let (p, m) = l.facet_rooms[f];
        if room_pos[p as usize] == NONE {
            room_pos[p as usize] = rooms.len() as u32;
            rooms.push(p);
        }
        if room_pos[m as usize] == NONE {
            room_pos[m as usize] = rooms.len() as u32;
            rooms.push(m);
        }
        f += 1;
    }
    Split {
        piece_of,
        rooms,
        room_pos,
    }
}

/// Body `i`'s flood over the rooms (positions): joined across every facet not a piece of `i`.
/// `find T` is the Lean's `head (adjOf (crossings i)) rooms T`.
fn flood(l: &Ledger, s: &Split, i: usize) -> Uf {
    let nb = l.nb as usize;
    let mut uf = Uf::new(s.rooms.len());
    let mut f = 0;
    while f < l.facet_class.len() {
        if !s.piece_of[f * nb + i] {
            let (p, m) = l.facet_rooms[f];
            uf.union(s.room_pos[p as usize] as usize, s.room_pos[m as usize] as usize);
        }
        f += 1;
    }
    uf
}

/// Each flood class's witness, per body: the (room, body) pairs the bridge decides by a ray (its
/// first member in room order), in body order, then room order.
pub fn witnesses(l: &Ledger) -> Vec<(RoomId, BodyId)> {
    let mut out = Vec::new();
    if split_ok(l) {
        let s = split(l);
        let mut i = 0;
        while i < l.nb as usize {
            let mut uf = flood(l, &s, i);
            let mut t = 0;
            while t < s.rooms.len() {
                if uf.find(t) == t {
                    out.push((RoomId(s.rooms[t]), BodyId(i as u32)));
                }
                t += 1;
            }
            i += 1;
        }
    }
    out
}

/// `memb i T` per body and room position: the witness answer of its flood head.
fn membership(l: &Ledger, s: &Split) -> Vec<Vec<bool>> {
    let nb = l.nb as usize;
    let nr = l.n_rooms as usize;
    let mut inside = vec![false; nr * nb];
    let mut k = 0;
    while k < l.inside.len() {
        let (r, b) = l.inside[k];
        if (b.0 as usize) < nb && (r.0 as usize) < nr {
            inside[r.0 as usize * nb + b.0 as usize] = true;
        }
        k += 1;
    }
    let mut memb = Vec::new();
    let mut i = 0;
    while i < nb {
        let mut uf = flood(l, s, i);
        let mut row = Vec::new();
        let mut t = 0;
        while t < s.rooms.len() {
            let h = uf.find(t);
            row.push(inside[s.rooms[h] as usize * nb + i]);
            t += 1;
        }
        memb.push(row);
        i += 1;
    }
    memb
}

/// The work of stages 4–7 over the keep table: the result facets in facet order, each one's
/// position there (or NONE) and sense.
struct Assembled {
    result: Vec<u32>,
    rpos: Vec<u32>,
    orient: Vec<Sgn>,
}

/// **The A8 core with the metric ledger** (`coreMI`): split check, classify check, keep per
/// room, the collapse of the thin rooms (`collapse`; the identity when `m.thin` is empty), then
/// the touch rule, the empty test and unify. One pass, no retries.
pub fn core(l: &Ledger, rule: &Rule, m: &MetricLedger) -> Outcome {
    if !split_ok(l) {
        Outcome::Refused(Refusal::SplitCrossed)
    } else {
        let s = split(l);
        let memb = membership(l, &s);
        let nb = l.nb as usize;
        let nf = l.facet_class.len();
        // 3. classify: a piece of body i has i's membership differ across it
        let mut ok = true;
        let mut f = 0;
        while ok && f < nf {
            let (p, m) = l.facet_rooms[f];
            let (pp, mp) = (s.room_pos[p as usize] as usize, s.room_pos[m as usize] as usize);
            let mut i = 0;
            while ok && i < nb {
                ok = !s.piece_of[f * nb + i] || memb[i][pp] != memb[i][mp];
                i += 1;
            }
            f += 1;
        }
        if !ok {
            Outcome::Refused(Refusal::InconsistentClassification)
        } else {
            // 4. keep, per room position
            let mut keep = Vec::with_capacity(s.rooms.len());
            let mut t = 0;
            while t < s.rooms.len() {
                let mut mv = Vec::with_capacity(nb);
                let mut i = 0;
                while i < nb {
                    mv.push(memb[i][t]);
                    i += 1;
                }
                keep.push(apply_rule(rule, &mv));
                t += 1;
            }
            // 4b. collapse the sub-LINEAR features
            let bad = collapse(l, &s, &mut keep, m);
            if bad != NONE {
                Outcome::Refused(Refusal::ZeroThicknessRoom(RoomId(bad)))
            } else {
                core_k(l, &s, &keep)
            }
        }
    }
}

/// Stage 4b (`IncLedger.collapse`): the thin rooms fall into features (classes under "the two
/// sides of a facet", `comp`); a feature's rim is the rooms that are not thin beside it (`rim`,
/// in room order). If every feature's rim is kept alike (`uniform`) and every room of it is
/// enclosed (`collapsible`), each thin room takes its rim's first room's keep, its own if the rim
/// is empty (`fill`), in `keep`; and NONE is returned. Else the first thin room, in room order,
/// whose feature is not so (its id).
fn collapse(l: &Ledger, s: &Split, keep: &mut [bool], mt: &MetricLedger) -> u32 {
    let nt = s.rooms.len();
    let nr = l.n_rooms as usize;
    let mut is_thin = vec![false; nt];
    let mut is_enc = vec![false; nt];
    let mut any = false;
    let mut k = 0;
    while k < mt.thin.len() {
        let r = mt.thin[k].0 as usize;
        if r < nr && s.room_pos[r] != NONE {
            is_thin[s.room_pos[r] as usize] = true;
            any = true;
        }
        k += 1;
    }
    let mut bad = NONE;
    if any {
        let mut k = 0;
        while k < mt.enclosed.len() {
            let r = mt.enclosed[k].0 as usize;
            if r < nr && s.room_pos[r] != NONE {
                is_enc[s.room_pos[r] as usize] = true;
            }
            k += 1;
        }
        let nf = l.facet_class.len();
        // the features: thin rooms joined across a facet
        let mut uf = Uf::new(nt);
        let mut f = 0;
        while f < nf {
            let (p, m) = l.facet_rooms[f];
            let (pp, mp) = (s.room_pos[p as usize] as usize, s.room_pos[m as usize] as usize);
            if is_thin[pp] && is_thin[mp] {
                uf.union(pp, mp);
            }
            f += 1;
        }
        // each feature's rim (by root), and whether every room of it is enclosed
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
        let mut f = 0;
        while f < nf {
            let (p, m) = l.facet_rooms[f];
            let (pp, mp) = (s.room_pos[p as usize] as usize, s.room_pos[m as usize] as usize);
            if is_thin[pp] && !is_thin[mp] {
                let r = uf.find(pp);
                rim[r].push(mp as u32);
            }
            if is_thin[mp] && !is_thin[pp] {
                let r = uf.find(mp);
                rim[r].push(pp as u32);
            }
            f += 1;
        }
        let mut t = 0;
        while t < nt {
            if rim[t].len() > 1 {
                crate::sort::sort_u32(&mut rim[t]);
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
        // the first thin room whose feature cannot be collapsed
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
                    bad = s.rooms[t];
                }
            }
            t += 1;
        }
        // fill: every thin room takes its rim's first keep (its own if none)
        if bad == NONE {
            let mut fill = vec![false; nt];
            let mut t = 0;
            while t < nt {
                if is_thin[t] {
                    let r = uf.find(t);
                    fill[t] = if rim[r].is_empty() {
                        keep[t]
                    } else {
                        keep[rim[r][0] as usize]
                    };
                }
                t += 1;
            }
            let mut t = 0;
            while t < nt {
                if is_thin[t] {
                    keep[t] = fill[t];
                }
                t += 1;
            }
        }
    }
    bad
}

/// 5–7 on a keep table (per room position).
fn core_k(l: &Ledger, s: &Split, keep: &[bool]) -> Outcome {
    let nf = l.facet_class.len();
    // 5. assemble
    let mut a = Assembled {
        result: Vec::new(),
        rpos: vec![NONE; nf],
        orient: Vec::new(),
    };
    let mut f = 0;
    while f < nf {
        let (p, m) = l.facet_rooms[f];
        let kp = keep[s.room_pos[p as usize] as usize];
        let km = keep[s.room_pos[m as usize] as usize];
        if kp != km {
            a.rpos[f] = a.result.len() as u32;
            a.result.push(f as u32);
            a.orient.push(if km { Sgn::Pos } else { Sgn::Neg });
        }
        f += 1;
    }
    // 6. the touch rule: edges (more than two result wings), then verts (sectors in more than
    // one fan class); the first bad cell refused
    let ne = rows(&l.edge_facets.start);
    let nv = rows(&l.vert_edges.start);
    let mut bad = Outcome::Empty;
    let mut found = false;
    let mut e = 0;
    while !found && e < ne {
        if around_e(l, &a, e).len() > 2 {
            bad = Outcome::Refused(Refusal::ZeroThicknessEdge(EdgeId(e as u32)));
            found = true;
        }
        e += 1;
    }
    let mut v = 0;
    while !found && v < nv {
        if bad_vert(l, &a, v) {
            bad = Outcome::Refused(Refusal::ZeroThicknessVert(VertId(v as u32)));
            found = true;
        }
        v += 1;
    }
    if found {
        bad
    } else if a.result.is_empty() {
        // 7. empty
        Outcome::Empty
    } else {
        Outcome::Ok(unify(l, &a))
    }
}

/// `aroundE`: the result wings round edge `e`, as result positions, in row order.
fn around_e(l: &Ledger, a: &Assembled, e: usize) -> Vec<u32> {
    let (lo, hi) = row(&l.edge_facets, e);
    let mut out = Vec::new();
    let mut k = lo;
    while k < hi {
        let p = a.rpos[l.edge_facets.ids[k] as usize];
        if p != NONE {
            out.push(p);
        }
        k += 1;
    }
    out
}

/// `aroundV`: the result sectors round vert `v`, as indices into `vert_sectors.sec`.
fn around_v(l: &Ledger, a: &Assembled, v: usize) -> Vec<usize> {
    let (lo, hi) = (
        l.vert_sectors.start[v] as usize,
        l.vert_sectors.start[v + 1] as usize,
    );
    let mut out = Vec::new();
    let mut k = lo;
    while k < hi {
        if a.rpos[l.vert_sectors.sec[k].facet as usize] != NONE {
            out.push(k);
        }
        k += 1;
    }
    out
}

/// `fanRel`: two sectors sharing a bounding edge.
fn fan_rel(s: Sector, t: Sector) -> bool {
    s.a == t.a || s.a == t.b || s.b == t.a || s.b == t.b
}

/// The fan classes of a vert's result sectors (positions in `ar`), joined by `fanRel`.
fn fans(l: &Ledger, ar: &[usize]) -> Uf {
    let mut uf = Uf::new(ar.len());
    let mut p = 0;
    while p < ar.len() {
        let mut q = p + 1;
        while q < ar.len() {
            if fan_rel(l.vert_sectors.sec[ar[p]], l.vert_sectors.sec[ar[q]]) {
                uf.union(p, q);
            }
            q += 1;
        }
        p += 1;
    }
    uf
}

/// `badVert`: the result sectors round `v` in more than one fan class.
fn bad_vert(l: &Ledger, a: &Assembled, v: usize) -> bool {
    let ar = around_v(l, a, v);
    let mut uf = fans(l, &ar);
    let mut bad = false;
    let mut p = 1;
    while !bad && p < ar.len() {
        bad = uf.find(p) != uf.find(0);
        p += 1;
    }
    bad
}

/// Stage 7, unify: result facets of one carrier and sense sharing an edge are one face (the class's
/// first in result order is its head); an edge run joins edges with one pair of faces
/// through verts that are not result verts.
fn unify(l: &Ledger, a: &Assembled) -> Body {
    let nr = a.result.len();
    let ne = rows(&l.edge_facets.start);
    let nv = rows(&l.vert_edges.start);
    // joinRel, through each edge's result wings
    let mut join = Uf::new(nr);
    let mut e = 0;
    while e < ne {
        let ar = around_e(l, a, e);
        let mut p = 0;
        while p < ar.len() {
            let mut q = p + 1;
            while q < ar.len() {
                let (c, d) = (ar[p] as usize, ar[q] as usize);
                if l.facet_class[a.result[c] as usize] == l.facet_class[a.result[d] as usize]
                    && a.orient[c] == a.orient[d]
                {
                    join.union(c, d);
                }
                q += 1;
            }
            p += 1;
        }
        e += 1;
    }
    // per result position, its face number (heads numbered in result order)
    let mut face_of = vec![NONE; nr];
    let mut faces: Vec<OutFace> = Vec::new();
    let mut head_of = vec![NONE; nr];
    let mut p = 0;
    while p < nr {
        let r = join.find(p);
        head_of[p] = a.result[r];
        if r == p {
            face_of[p] = faces.len() as u32;
            faces.push(OutFace {
                facets: Vec::new(),
                plane: l.facet_class[a.result[p] as usize],
                orient: a.orient[p],
            });
        } else {
            face_of[p] = face_of[r];
        }
        p += 1;
    }
    let mut p = 0;
    while p < nr {
        faces[face_of[p] as usize].facets.push(FacetId(a.result[p]));
        p += 1;
    }
    // the faces round each edge and vert, by head facet (`facesAt`)
    let mut at_e: Vec<Vec<u32>> = Vec::with_capacity(ne);
    let mut e = 0;
    while e < ne {
        let ar = around_e(l, a, e);
        at_e.push(faces_at(&head_of, &ar));
        e += 1;
    }
    let mut is_out_v = vec![false; nv];
    let mut verts = Vec::new();
    let mut v = 0;
    while v < nv {
        let ar = around_v(l, a, v);
        // (the sectors' facets as result positions, then their faces, v1's `faces_at`)
        let mut ps = Vec::with_capacity(ar.len());
        let mut k = 0;
        while k < ar.len() {
            ps.push(a.rpos[l.vert_sectors.sec[ar[k]].facet as usize]);
            k += 1;
        }
        if faces_at(&head_of, &ps).len() >= 3 {
            is_out_v[v] = true;
            verts.push(VertId(v as u32));
        }
        v += 1;
    }
    // edges between two faces, in edge order, joined by erel through verts that are not result verts
    let mut ecs: Vec<u32> = Vec::new();
    let mut ec_pos = vec![NONE; ne];
    let mut e = 0;
    while e < ne {
        if at_e[e].len() == 2 {
            ec_pos[e] = ecs.len() as u32;
            ecs.push(e as u32);
        }
        e += 1;
    }
    let mut erel = Uf::new(ecs.len());
    let mut v = 0;
    while v < nv {
        if !is_out_v[v] {
            let (lo, hi) = row(&l.vert_edges, v);
            let mut i = lo;
            while i < hi {
                let e1 = l.vert_edges.ids[i] as usize;
                let mut j = i + 1;
                while j < hi {
                    let e2 = l.vert_edges.ids[j] as usize;
                    if ec_pos[e1] != NONE && ec_pos[e2] != NONE && same_set(&at_e[e1], &at_e[e2]) {
                        erel.union(ec_pos[e1] as usize, ec_pos[e2] as usize);
                    }
                    j += 1;
                }
                i += 1;
            }
        }
        v += 1;
    }
    let mut edges: Vec<OutEdge> = Vec::new();
    let mut edge_of = vec![NONE; ecs.len()];
    let mut p = 0;
    while p < ecs.len() {
        let r = erel.find(p);
        if r == p {
            edge_of[p] = edges.len() as u32;
            let fs = &at_e[ecs[p] as usize];
            let mut ff = Vec::with_capacity(fs.len());
            let mut k = 0;
            while k < fs.len() {
                ff.push(FacetId(fs[k]));
                k += 1;
            }
            edges.push(OutEdge {
                cells: Vec::new(),
                faces: ff,
            });
        } else {
            edge_of[p] = edge_of[r];
        }
        edges[edge_of[p] as usize].cells.push(EdgeId(ecs[p]));
        p += 1;
    }
    Body { faces, edges, verts }
}
