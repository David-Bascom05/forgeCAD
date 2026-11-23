//! The A8 ledger (research/decide-once/face-arrangement.md §1): cells are the components of the
//! region minus the near faces, with explicit incidence. Every id is a `u32` index; nothing is a
//! covector. The Lean is `IncLedger` (verify/forge-exact/ForgeExact/DecideOnce/IncCore.lean); the
//! conditions a builder meets against the plane ledger are `Refines` (Refines.lean).
//!
//! Types only, plus `ledger_wf` (`LedgerWF` v2). Written for Aeneas, like the v1 core: while
//! loops, no closures, no iterators, no maps.

use crate::{BodyId, FaceId, Sgn};

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

/// A face of the result (`OutFaceI`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFace {
    pub facets: Vec<FacetId>,
    pub plane: u32,
    pub orient: Sgn,
    /// The input face its first facet lies in (the first in arena order): its name.
    pub origin: FaceId,
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

/// A Csr is well formed over `n_ids` ids: `start` starts at 0, never decreases, ends at
/// `ids.len()`, and every id is below `n_ids`.
pub fn csr_ok(c: &Csr, n_ids: u32) -> bool {
    let mut ok = !c.start.is_empty() && c.start[0] == 0;
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

/// `LedgerWF` v2: the Csrs well formed, a facet at most twice per edge row, no repeat in a
/// vert's edges, each sector's facet in range and its two edges in its vert's row, ids and counts
/// in range, a facet's two sides distinct rooms, and every facet a piece of some face.
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
            ok = (sc.facet as usize) < nf
                && count_in(&l.vert_edges.ids, lo, hi, sc.a) == 1
                && count_in(&l.vert_edges.ids, lo, hi, sc.b) == 1;
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
        ok = (c.0 as usize) < nf && (g.0 as usize) < l.face_body.len();
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
