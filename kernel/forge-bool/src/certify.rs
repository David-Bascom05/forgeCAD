//! The certificate a boolean hands to the proven checker (verify/forge-certify), behind the
//! `certify` feature: off, nothing here is compiled. Per boolean: both operands as they came in,
//! the op, the regions the plan split each operand's faces into (its `Spec`s, as loops of points:
//! a vertex is the same point wherever it appears) and the result, or none if the boolean refused.
//! Nothing the kernel decides reads any of it.

use crate::boolean::{Op, Plan, VKey};
use forge_math::Point3;
use forge_topo::{Body, FaceId};
use std::cell::RefCell;

/// A region of an operand's face: `side` 0 the target, 1 the tool.
#[derive(Clone, Debug)]
pub struct Piece {
    pub side: u8,
    pub face: FaceId,
    pub loops: Vec<Vec<Point3>>,
}

/// One boolean.
#[derive(Clone, Debug)]
pub struct Record {
    pub a: Body,
    pub b: Body,
    pub op: Op,
    pub pieces: Vec<Piece>,
    pub result: Option<Body>,
}

thread_local! {
    static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OPEN: RefCell<Option<Record>> = const { RefCell::new(None) };
    static DONE: RefCell<Vec<Record>> = const { RefCell::new(Vec::new()) };
    static SKIPPED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// Records this thread's booleans from now (`true`) or stops (`false`, the default): with the
/// feature compiled in but recording off, a boolean costs one flag test.
pub fn record(on: bool) {
    ON.with(|c| c.set(on));
}

/// The booleans this thread recorded since the last call.
pub fn take() -> Vec<Record> {
    DONE.with(|d| std::mem::take(&mut *d.borrow_mut()))
}

/// The booleans this thread did since the last call by a path that gives no certificate (named).
pub fn take_skipped() -> Vec<&'static str> {
    SKIPPED.with(|d| std::mem::take(&mut *d.borrow_mut()))
}

pub(crate) fn skipped(path: &'static str) {
    if ON.with(|c| c.get()) {
        SKIPPED.with(|d| d.borrow_mut().push(path));
    }
}

pub(crate) fn begin(a: &Body, b: &Body, op: Op) {
    if !ON.with(|c| c.get()) {
        return;
    }
    OPEN.with(|o| {
        *o.borrow_mut() = Some(Record {
            a: a.clone(),
            b: b.clone(),
            op,
            pieces: Vec::new(),
            result: None,
        })
    });
}

fn point(a: &Body, b: &Body, plan: &Plan, v: VKey) -> Point3 {
    let v = plan.alias.iter().find(|(f, _)| *f == v).map_or(v, |&(_, t)| t);
    match v {
        VKey::Old(0, id) => a.vertex(id).point,
        VKey::Old(_, id) => b.vertex(id).point,
        VKey::New(i) => plan.points[i as usize],
    }
}

/// The plan's regions, read before `apply` changes the target.
pub(crate) fn pieces(a: &Body, b: &Body, plan: &Plan) {
    let mut out = Vec::with_capacity(plan.specs.len());
    for s in &plan.specs {
        let mut loops: Vec<Vec<Point3>> = s
            .loops
            .iter()
            .map(|lp| {
                lp.iter()
                    .map(|&(p, fwd)| {
                        let pc = &plan.pieces[p];
                        point(a, b, plan, if fwd { pc.start } else { pc.end })
                    })
                    .collect()
            })
            .collect();
        // a face extended in place keeps its own loops (but those it gives up)
        if s.extend && s.side == 0 {
            for &l in a.face(s.from).loops.iter() {
                if s.remove.contains(&l) {
                    continue;
                }
                loops.push(
                    a.lp(l)
                        .coedges()
                        .map(|c| {
                            let ce = a.coedge(c);
                            let e = a.edge(ce.edge);
                            a.vertex(if ce.forward { e.start } else { e.end }).point
                        })
                        .collect(),
                );
            }
        }
        out.push(Piece {
            side: s.side as u8,
            face: s.from,
            loops,
        });
    }
    OPEN.with(|o| {
        if let Some(r) = o.borrow_mut().as_mut() {
            r.pieces = out;
        }
    });
}

pub(crate) fn finish(result: Option<&Body>) {
    if let Some(mut r) = OPEN.with(|o| o.borrow_mut().take()) {
        r.result = result.map(|b| b.compact());
        DONE.with(|d| d.borrow_mut().push(r));
    }
}
