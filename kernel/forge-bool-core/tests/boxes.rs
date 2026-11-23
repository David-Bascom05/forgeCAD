//! The box ledgers of verify/forge-exact/ForgeExact/DecideOnce/Boxes.lean and every `#guard` of
//! Rungs.lean, against the Rust core: the six retry rungs' flat instances and the corner, edge and
//! face touches. The Lean evaluates the same ledgers at build time; the answers must agree.

use forge_bool_core::{
    BodyId, CellId, FaceId, Ledger, MetricLedger, Outcome, Refusal, Rule, Sgn, core, witnesses,
};

type D = (i64, i64, i64);

#[derive(Clone, Copy)]
struct BoxN {
    lo: (i64, i64, i64),
    hi: (i64, i64, i64),
}

fn bx(lo: (i64, i64, i64), hi: (i64, i64, i64)) -> BoxN {
    BoxN { lo, hi }
}

fn ax(b: &BoxN, a: usize) -> (i64, i64) {
    match a {
        0 => (b.lo.0, b.hi.0),
        1 => (b.lo.1, b.hi.1),
        _ => (b.lo.2, b.hi.2),
    }
}

fn get(c: D, a: usize) -> i64 {
    match a {
        0 => c.0,
        1 => c.1,
        _ => c.2,
    }
}

fn ax_signs(k: i64, a: i64) -> Vec<Sgn> {
    (0..=k)
        .map(|j| {
            if a < 2 * j {
                Sgn::Neg
            } else if a == 2 * j {
                Sgn::Zero
            } else {
                Sgn::Pos
            }
        })
        .collect()
}

fn cov(k: i64, c: D) -> Vec<Sgn> {
    let mut v = ax_signs(k, c.0);
    v.extend(ax_signs(k, c.1));
    v.extend(ax_signs(k, c.2));
    v
}

fn odds(lo: i64, hi: i64) -> Vec<i64> {
    (lo..hi).map(|t| 2 * t + 1).collect()
}

fn place(a: usize, x: i64, u: i64, v: i64) -> D {
    match a {
        0 => (x, u, v),
        1 => (u, x, v),
        _ => (u, v, x),
    }
}

fn face_cells(b: &BoxN, side: usize) -> Vec<D> {
    let a = side / 2;
    let k = if side.is_multiple_of(2) {
        ax(b, a).0
    } else {
        ax(b, a).1
    };
    let o1 = (a + 1) % 3;
    let o2 = (a + 2) % 3;
    let (us, vs) = if o1 < o2 {
        (odds(ax(b, o1).0, ax(b, o1).1), odds(ax(b, o2).0, ax(b, o2).1))
    } else {
        (odds(ax(b, o2).0, ax(b, o2).1), odds(ax(b, o1).0, ax(b, o1).1))
    };
    let mut out = Vec::new();
    for &u in &us {
        for &v in &vs {
            out.push(place(a, 2 * k, u, v));
        }
    }
    out
}

fn holds(b: &BoxN, c: D) -> bool {
    (0..3).all(|a| 2 * ax(b, a).0 < get(c, a) && get(c, a) < 2 * ax(b, a).1)
}

fn even_ax(c: D) -> usize {
    if c.0 % 2 == 0 {
        0
    } else if c.1 % 2 == 0 {
        1
    } else {
        2
    }
}

fn bump(a: usize, d: i64, c: D) -> D {
    match a {
        0 => (c.0 + d, c.1, c.2),
        1 => (c.0, c.1 + d, c.2),
        _ => (c.0, c.1, c.2 + d),
    }
}

fn dedup<T: PartialEq + Clone>(xs: Vec<T>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for x in xs {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

/// `boxLedger K boxes`, with the cells interned (one id per covector) as the bridge must.
fn box_ledger(k: i64, boxes: &[BoxN]) -> Ledger {
    let mut ps = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        for side in 0..6 {
            for c in face_cells(b, side) {
                ps.push((c, 6 * i + side));
            }
        }
    }
    let cs = dedup(ps.iter().map(|p| p.0).collect());
    let sides = dedup(
        cs.iter()
            .flat_map(|&c| [bump(even_ax(c), 1, c), bump(even_ax(c), -1, c)])
            .collect(),
    );
    let es = dedup(
        cs.iter()
            .flat_map(|&c| {
                let a = even_ax(c);
                let (o1, o2) = ((a + 1) % 3, (a + 2) % 3);
                [bump(o1, 1, c), bump(o1, -1, c), bump(o2, 1, c), bump(o2, -1, c)]
            })
            .collect(),
    );
    let vs = dedup(
        cs.iter()
            .flat_map(|&c| {
                let a = even_ax(c);
                let (o1, o2) = ((a + 1) % 3, (a + 2) % 3);
                [
                    bump(o2, 1, bump(o1, 1, c)),
                    bump(o2, -1, bump(o1, 1, c)),
                    bump(o2, 1, bump(o1, -1, c)),
                    bump(o2, -1, bump(o1, -1, c)),
                ]
            })
            .collect(),
    );
    let mut cells: Vec<Vec<Sgn>> = Vec::new();
    let id = |c: D, cells: &mut Vec<Vec<Sgn>>| -> CellId {
        let v = cov(k, c);
        if let Some(i) = cells.iter().position(|x| *x == v) {
            return CellId(i as u32);
        }
        cells.push(v);
        CellId(cells.len() as u32 - 1)
    };
    let pieces = ps
        .iter()
        .map(|&(c, f)| (id(c, &mut cells), FaceId(f as u32)))
        .collect();
    let mut inside = Vec::new();
    for &t in &sides {
        for (i, b) in boxes.iter().enumerate() {
            if holds(b, t) {
                inside.push((id(t, &mut cells), BodyId(i as u32)));
            }
        }
        id(t, &mut cells);
    }
    let edges = es.iter().map(|&c| id(c, &mut cells)).collect();
    let verts = vs.iter().map(|&c| id(c, &mut cells)).collect();
    let nf = 6 * boxes.len();
    Ledger {
        nb: boxes.len() as u32,
        face_body: (0..nf).map(|f| BodyId((f / 6) as u32)).collect(),
        face_class: (0..nf)
            .map(|f| {
                let b = &boxes[f / 6];
                let side = f % 6;
                let a = side / 2;
                (a as i64 * (k + 1)
                    + if side.is_multiple_of(2) {
                        ax(b, a).0
                    } else {
                        ax(b, a).1
                    }) as u32
            })
            .collect(),
        cells,
        pieces,
        inside,
        edges,
        verts,
    }
}

/// `Outcome.summary`: faces, facets, edges, vertices.
#[derive(Debug, PartialEq)]
enum Summary {
    Ok(usize, usize, usize, usize),
    Empty,
    Split,
    Inconsistent,
    Zero(Vec<Sgn>),
}

fn summary(l: &Ledger, o: Outcome) -> Summary {
    match o {
        Outcome::Ok(b) => Summary::Ok(
            b.faces.len(),
            b.faces.iter().map(|f| f.facets.len()).sum(),
            b.edges.len(),
            b.verts.len(),
        ),
        Outcome::Empty => Summary::Empty,
        Outcome::Refused(Refusal::SplitCrossed) => Summary::Split,
        Outcome::Refused(Refusal::InconsistentClassification) => Summary::Inconsistent,
        Outcome::Refused(Refusal::ZeroThickness(c)) => Summary::Zero(l.cells[c.0 as usize].clone()),
    }
}

fn run(k: i64, boxes: &[BoxN], rule: Rule) -> Summary {
    let l = box_ledger(k, boxes);
    summary(&l, core(&l, &rule, &MetricLedger::default()))
}

const T: (i64, i64, i64) = (0, 0, 0);

#[test]
fn no_run_on_flush_and_gap() {
    assert_eq!(
        run(2, &[bx(T, (2, 1, 1)), bx((1, 0, 1), (2, 1, 2))], Rule::Union),
        Summary::Ok(8, 14, 18, 12)
    );
    assert_eq!(
        run(3, &[bx(T, (3, 1, 1)), bx((1, 0, 1), (2, 1, 2))], Rule::Union),
        Summary::Ok(10, 18, 24, 16)
    );
}

#[test]
fn chords_notch() {
    assert_eq!(
        run(2, &[bx(T, (2, 1, 2)), bx((1, 0, 0), (2, 1, 1))], Rule::Diff),
        Summary::Ok(8, 14, 18, 12)
    );
}

#[test]
fn absorb_hair_roots() {
    assert_eq!(
        run(2, &[bx(T, (2, 1, 1)), bx((0, 0, 1), (1, 1, 2))], Rule::Union),
        Summary::Ok(8, 14, 18, 12)
    );
}

#[test]
fn fences_blade_on_hub() {
    assert_eq!(
        run(3, &[bx(T, (3, 3, 1)), bx((1, 1, 1), (2, 2, 2))], Rule::Union),
        Summary::Ok(11, 34, 24, 16)
    );
}

#[test]
fn careful_through_hole() {
    assert_eq!(
        run(3, &[bx(T, (3, 3, 1)), bx((1, 1, 0), (2, 2, 1))], Rule::Diff),
        Summary::Ok(10, 32, 24, 16)
    );
}

#[test]
fn nary_one_call() {
    let boxes = [
        bx(T, (1, 1, 1)),
        bx((1, 0, 0), (2, 1, 1)),
        bx((1, 0, 1), (2, 1, 2)),
    ];
    assert_eq!(run(2, &boxes, Rule::Union), Summary::Ok(8, 14, 18, 12));
    // the same as a history table: in any of the three
    let table = (0..8).map(|r| r != 0).collect();
    assert_eq!(run(2, &boxes, Rule::Table(table)), Summary::Ok(8, 14, 18, 12));
}

#[test]
fn nary_edge_touch_refused() {
    let boxes = [
        bx(T, (1, 1, 1)),
        bx((1, 1, 0), (2, 2, 1)),
        bx((0, 1, 1), (1, 2, 2)),
    ];
    assert!(matches!(run(2, &boxes, Rule::Union), Summary::Zero(_)));
}

#[test]
fn corner_edge_and_face_touch() {
    assert_eq!(
        run(2, &[bx(T, (1, 1, 1)), bx((1, 1, 1), (2, 2, 2))], Rule::Union),
        Summary::Zero(cov(2, (2, 2, 2)))
    );
    assert_eq!(
        run(2, &[bx(T, (1, 1, 1)), bx((1, 1, 0), (2, 2, 1))], Rule::Union),
        Summary::Zero(cov(2, (2, 2, 1)))
    );
    assert_eq!(
        run(2, &[bx(T, (1, 1, 1)), bx((1, 0, 0), (2, 1, 1))], Rule::Union),
        Summary::Ok(6, 10, 12, 8)
    );
}

#[test]
fn empty_and_identity() {
    // a box less itself is empty; with itself, the box
    let b = bx(T, (1, 1, 1));
    assert_eq!(run(1, &[b, b], Rule::Diff), Summary::Empty);
    assert_eq!(run(1, &[b, b], Rule::Union), Summary::Ok(6, 6, 12, 8));
    assert_eq!(run(1, &[b, b], Rule::Inter), Summary::Ok(6, 6, 12, 8));
}

/// The witnesses the core asks for are the inside answers it reads: answering only those gives
/// the same outcome as answering every side cell.
#[test]
fn witnesses_suffice() {
    let boxes = [bx(T, (3, 3, 1)), bx((1, 1, 0), (2, 2, 1))];
    let full = box_ledger(3, &boxes);
    let mut l = full.clone();
    let ws = witnesses(&l);
    assert!(!ws.is_empty());
    l.inside.retain(|p| ws.contains(p));
    assert_eq!(
        core(&full, &Rule::Diff, &MetricLedger::default()),
        core(&l, &Rule::Diff, &MetricLedger::default())
    );
}

/// Every Allen pattern of two boxes on a 4-step grid, under the three ops: never a split or
/// classification refusal (theorem (b)'s no_retry on consistent ledgers).
#[test]
fn box_pairs_never_retry() {
    let iv = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3), (1, 4), (0, 4)];
    let a = bx((1, 1, 1), (3, 3, 3));
    let mut n = 0;
    for &(x0, x1) in &iv {
        for &(y0, y1) in &iv {
            for &(z0, z1) in &iv {
                let b = bx((x0, y0, z0), (x1, y1, z1));
                for rule in [Rule::Union, Rule::Inter, Rule::Diff] {
                    let s = run(4, &[a, b], rule);
                    assert!(!matches!(s, Summary::Split | Summary::Inconsistent), "{s:?}");
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 512 * 3);
}

/// The metric ledger (stage 4b) on Rungs.lean's two features, with the Lean's answers.
fn run_m(k: i64, boxes: &[BoxN], rule: Rule, thin: D) -> Summary {
    let l = box_ledger(k, boxes);
    let c = cov(k, thin);
    let id = CellId(
        l.cells
            .iter()
            .position(|x| *x == c)
            .expect("the thin cell is a side cell") as u32,
    );
    let m = MetricLedger {
        thin: vec![id],
        enclosed: vec![id],
    };
    summary(&l, core(&l, &rule, &m))
}

#[test]
fn collapse_fills_an_enclosed_void_and_refuses_a_fin() {
    let block = [bx(T, (3, 3, 3)), bx((1, 1, 1), (2, 2, 2))];
    assert_eq!(run_m(3, &block, Rule::Diff, (3, 3, 3)), Summary::Ok(6, 54, 12, 8));
    assert_eq!(run(3, &block, Rule::Diff), Summary::Ok(12, 60, 24, 16));
    let fin = [bx(T, (3, 1, 1)), bx((1, 0, 1), (2, 1, 2))];
    assert_eq!(
        run_m(3, &fin, Rule::Union, (3, 1, 3)),
        Summary::Zero(cov(3, (3, 1, 3)))
    );
}

/// The v2 ledger's well-formedness check on a hand-made ledger: two boxes' worth is not needed,
/// one facet between two rooms, one edge, one vert, is enough to exercise each clause.
#[test]
fn v2_ledger_wf() {
    use forge_bool_core::v2::*;
    let good = Ledger {
        nb: 1,
        face_body: vec![BodyId(0)],
        face_class: vec![0],
        n_rooms: 2,
        facet_class: vec![0],
        facet_rooms: vec![(0, 1)],
        pieces: vec![(FacetId(0), FaceId(0))],
        inside: vec![(RoomId(1), BodyId(0))],
        edge_facets: Csr {
            start: vec![0, 2],
            ids: vec![0, 0],
        },
        vert_edges: Csr {
            start: vec![0, 1],
            ids: vec![0],
        },
        vert_sectors: SecCsr {
            start: vec![0, 1],
            sec: vec![Sector { facet: 0, a: 0, b: 0 }],
        },
    };
    assert!(ledger_wf(&good));
    let mut l = good.clone();
    l.facet_rooms = vec![(1, 1)];
    assert!(!ledger_wf(&l), "a facet's sides are distinct rooms");
    let mut l = good.clone();
    l.edge_facets.ids = vec![0, 0, 0];
    l.edge_facets.start = vec![0, 3];
    assert!(!ledger_wf(&l), "a facet at most twice round an edge");
    let mut l = good.clone();
    l.pieces.clear();
    assert!(!ledger_wf(&l), "every facet a piece");
    let mut l = good;
    l.vert_sectors.sec[0].b = 5;
    assert!(!ledger_wf(&l), "a sector's edges at its vert");
}
