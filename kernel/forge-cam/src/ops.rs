//! Operations: pure functions from (setup, tool, parameters) to a toolpath, every move then
//! certified against the part (G1), the shank and holder against the part and the stock (G2), and
//! rapids against both (G3). A path is shipped only when every check is `Clear`; otherwise the
//! op is refused with the check that failed, where, and a fix.
//!
//! Linking is by retract: up to the retract height (above everything the op cuts), across at it,
//! and down at the plunge feed. Rapids run at the clearance height, above the stock.
use forge_cam_geom::check::{self, Face, Move, Verdict};
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Region, Seg};
use forge_cam_geom::tool::{Part, Shape};
use forge_math::dmath;
use forge_tooling::Tool;

/// The stock: a box in the part's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stock {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Stock {
    /// The box's six faces, lowered like the part's (planes; prisms all).
    pub fn faces(&self) -> Vec<Face> {
        use forge_cam_geom::check::{Surf, TSeg, Trim};
        use forge_cam_geom::ival::Ival;
        let (a, b) = (self.min, self.max);
        let rect = |w: f64, h: f64| Trim {
            segs: vec![
                TSeg::Line {
                    a: [0.0, 0.0],
                    b: [w, 0.0],
                },
                TSeg::Line {
                    a: [w, 0.0],
                    b: [w, h],
                },
                TSeg::Line {
                    a: [w, h],
                    b: [0.0, h],
                },
                TSeg::Line {
                    a: [0.0, h],
                    b: [0.0, 0.0],
                },
            ],
        };
        let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        let mk = |o: [f64; 3], u: [f64; 3], v: [f64; 3], w: f64, h: f64, id: u32| {
            Face::new(
                Surf::Plane { o, u, v },
                Ival::new(0.0, w),
                Ival::new(0.0, h),
                Some(rect(w, h)),
                1e-9,
                id,
            )
        };
        vec![
            mk([a[0], a[1], a[2]], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], dx, dy, 1000),
            mk([a[0], a[1], b[2]], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], dx, dy, 1001),
            mk([a[0], a[1], a[2]], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], dx, dz, 1002),
            mk([a[0], b[1], a[2]], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], dx, dz, 1003),
            mk([a[0], a[1], a[2]], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], dy, dz, 1004),
            mk([b[0], a[1], a[2]], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], dy, dz, 1005),
        ]
    }
}

/// A tool as the program holds it.
#[derive(Clone, Debug)]
pub struct ToolUse {
    pub t: u32,
    pub tool: Tool,
    pub holder: Option<String>,
    pub stickout: f64,
    pub shape: Shape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Cut,
    Plunge,
    Link,
    Rapid,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Cut => "cut",
            Phase::Plunge => "plunge",
            Phase::Link => "link",
            Phase::Rapid => "rapid",
        }
    }
}

/// One move of a path, in the part's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PMove {
    Rapid {
        to: [f64; 3],
    },
    Line {
        to: [f64; 3],
        f: f64,
        phase: Phase,
    },
    /// In the xy plane about c (xy), clockwise or not.
    Arc {
        to: [f64; 3],
        c: [f64; 2],
        cw: bool,
        f: f64,
    },
    /// A canned hole cycle at each place.
    Cycle {
        kind: forge_post::CycleKind,
        holes: [f64; 2],
        z: f64,
        r: f64,
        clear: f64,
        q: f64,
        dwell: f64,
        f: f64,
    },
}

#[derive(Clone, Debug)]
pub struct OpPath {
    pub tool: usize,
    pub start: [f64; 3],
    pub moves: Vec<PMove>,
    pub rpm: f64,
    pub coolant: forge_post::Coolant,
}

/// What a check found, for the reply.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckOut {
    pub rule: &'static str,
    pub verdict: &'static str,
    pub value: f64,
    pub limit: f64,
    pub at: [f64; 3],
    pub tool_part: &'static str,
    pub other: String,
    pub phase: &'static str,
    pub move_index: usize,
}

#[derive(Clone, Debug, Default)]
pub struct CheckSummary {
    pub checks: Vec<CheckOut>,
    pub pairs: usize,
    pub cells: usize,
    pub clear: bool,
    pub least: f64,
}

/// The moves of a path as the checker's (arcs cut into pieces of at most a quarter turn, cycles
/// expanded into their motion), each with its phase.
pub fn checker_moves(p: &OpPath) -> Vec<(Move, Phase, usize)> {
    let mut out = Vec::new();
    let mut at = p.start;
    for (i, m) in p.moves.iter().enumerate() {
        match *m {
            PMove::Rapid { to } => {
                out.push((Move::Line { a: at, b: to }, Phase::Rapid, i));
                at = to;
            }
            PMove::Line { to, phase, .. } => {
                out.push((Move::Line { a: at, b: to }, phase, i));
                at = to;
            }
            PMove::Arc { to, c, cw, .. } => {
                let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
                let mut a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
                let tau = std::f64::consts::TAU;
                if cw {
                    while a1 >= a0 {
                        a1 -= tau;
                    }
                } else {
                    while a1 <= a0 {
                        a1 += tau;
                    }
                }
                let n = (((a1 - a0).abs() / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
                for k in 0..n {
                    let (u0, u1) = (
                        a0 + (a1 - a0) * k as f64 / n as f64,
                        a0 + (a1 - a0) * (k + 1) as f64 / n as f64,
                    );
                    let um = 0.5 * (u0 + u1);
                    let (sm, cm) = dmath::sin_cos(um);
                    let h = 0.5 * (u1 - u0).abs();
                    let th = dmath::sin(0.5 * h) / dmath::cos(0.5 * h);
                    // the planar arc's frame turns with the travel: x at the middle, y forward
                    let (x, y) = if cw {
                        ([cm, sm, 0.0], [sm, -cm, 0.0])
                    } else {
                        ([cm, sm, 0.0], [-sm, cm, 0.0])
                    };
                    let mv = if to[2] == at[2] {
                        Move::Arc {
                            c: [c[0], c[1], at[2]],
                            x,
                            y,
                            r,
                            t0: -th,
                            t1: th,
                        }
                    } else {
                        // helical: the height is linear in the angle; this piece's share, its
                        // ends computed in f64 and the range widened past their rounding
                        let za = at[2] + (to[2] - at[2]) * k as f64 / n as f64;
                        let zb = at[2] + (to[2] - at[2]) * (k + 1) as f64 / n as f64;
                        let w = 1e-9 * (1.0 + at[2].abs().max(to[2].abs()));
                        Move::Helix {
                            c: [c[0], c[1], 0.0],
                            x,
                            y,
                            r,
                            t0: -th,
                            t1: th,
                            z0: za.min(zb) - w,
                            z1: za.max(zb) + w,
                        }
                    };
                    out.push((mv, Phase::Cut, i));
                }
                at = to;
            }
            PMove::Cycle {
                holes, z, r, clear, ..
            } => {
                let (x, y) = (holes[0], holes[1]);
                out.push((
                    Move::Line {
                        a: at,
                        b: [x, y, clear],
                    },
                    Phase::Rapid,
                    i,
                ));
                out.push((
                    Move::Line {
                        a: [x, y, clear],
                        b: [x, y, r],
                    },
                    Phase::Rapid,
                    i,
                ));
                out.push((
                    Move::Line {
                        a: [x, y, r],
                        b: [x, y, z],
                    },
                    Phase::Cut,
                    i,
                ));
                out.push((
                    Move::Line {
                        a: [x, y, z],
                        b: [x, y, clear],
                    },
                    Phase::Rapid,
                    i,
                ));
                at = [x, y, clear];
            }
        }
    }
    out
}

/// Certifies a path: each move, each tool body, each nearby face of the part and the stock.
pub fn certify(
    p: &OpPath,
    tool: &ToolUse,
    part: &[Face],
    stock: &[Face],
    tol: f64,
    clearance: f64,
) -> CheckSummary {
    certify_staged(p, tool, part, &[(0, stock.to_vec())], tol, clearance)
}

/// `certify` with the stock as it stands at each move: `stages[k] = (first move, faces)`, in
/// increasing order of first move (an op's own completed levels clear what is above them).
pub fn certify_staged(
    p: &OpPath,
    tool: &ToolUse,
    part: &[Face],
    stages: &[(usize, Vec<Face>)],
    tol: f64,
    clearance: f64,
) -> CheckSummary {
    certify_fixtured(p, tool, part, stages, &[], 0.0, tol, clearance)
}

/// `certify_staged` with a fixture: every body of the tool (cutter included), on every piece of
/// the path, keeps the fixture clearance `fclear` from the fixture's faces (G2, corrected).
#[allow(clippy::too_many_arguments)]
pub fn certify_fixtured(
    p: &OpPath,
    tool: &ToolUse,
    part: &[Face],
    stages: &[(usize, Vec<Face>)],
    fixture: &[Face],
    fclear: f64,
    tol: f64,
    clearance: f64,
) -> CheckSummary {
    certify_with(p, tool, part, stages, fixture, fclear, tol, clearance, false)
}

/// `certify_fixtured`, and with `column` the 3-axis surfacing route for the cutter (and a shank
/// inside its column) against the part on cuts: the vertical gap of the tool's column
/// (forge_cam_geom::drop) is tried first, and only what it cannot prove goes to the
/// branch and bound. A proven gap v ≥ −tol bounds every such body's signed distance by the same
/// (drop.rs's statement), so the verdict is the check's own.
#[allow(clippy::too_many_arguments)]
pub fn certify_with(
    p: &OpPath,
    tool: &ToolUse,
    part: &[Face],
    stages: &[(usize, Vec<Face>)],
    fixture: &[Face],
    fclear: f64,
    tol: f64,
    clearance: f64,
    column: bool,
) -> CheckSummary {
    let col = if column {
        tool.shape
            .bodies
            .iter()
            .find(|b| b.part == Part::Cutter)
            .and_then(forge_cam_geom::drop::Column::of)
    } else {
        None
    };
    let mut sum = CheckSummary {
        clear: true,
        least: f64::INFINITY,
        ..Default::default()
    };
    let pad = clearance.max(0.0) + 1e-6;
    let pgrid = check::Grid::new(part, pad);
    let fgrid = check::Grid::new(fixture, fclear.max(0.0) + 1e-6);
    let sgrids: Vec<check::Grid> = stages.iter().map(|s| check::Grid::new(&s.1, pad)).collect();
    // the one point outside: the tool's top (an axis point of a body) is above every face it is
    // checked against, at every instant (G1-G3 need one point of the moved tool outside the set,
    // the faces' distances do the rest)
    let mut zmax = f64::NEG_INFINITY;
    for f in part
        .iter()
        .chain(stages.iter().flat_map(|s| s.1.iter()))
        .chain(fixture.iter())
    {
        zmax = zmax.max(f.bbox[2].hi);
    }
    let top = tool.shape.top();
    let mv = checker_moves(p);
    let cutter_r = tool
        .shape
        .bodies
        .iter()
        .filter(|b| b.part == Part::Cutter)
        .map(|b| b.rmax)
        .fold(0.0, f64::max);
    // the moves are independent: chunks of them are checked on all cores (forge_math::par),
    // each with its own scratch, and merged in move order with the sequential rules below
    // (stop at a containment failure, or after the 21st failing check, counting what was
    // counted up to there), so the summary is the same at any thread count
    const CHUNK: usize = 64;
    let nchunks = mv.len().div_ceil(CHUNK);
    let nstock = stages.iter().map(|s| s.1.len()).max().unwrap_or(0);
    let chunks: Vec<Vec<MoveOut>> = forge_math::par::map(nchunks, |c| {
        let mut cand: Vec<u32> = Vec::new();
        let mut seen = check::Seen::default();
        // each move's distance to each face's section, shared by the tool's bodies (one per
        // face set and face, reset after the move)
        let mut shared: [Vec<Option<(forge_cam_geom::ival::Ival, f64)>>; 3] = [
            vec![None; part.len()],
            vec![None; nstock],
            vec![None; fixture.len()],
        ];
        let mut touched: Vec<(usize, usize)> = Vec::new();
        let mut outs = Vec::new();
        for (m, phase, idx) in &mv[c * CHUNK..((c + 1) * CHUNK).min(mv.len())] {
            for (k, i) in touched.drain(..) {
                shared[k][i] = None;
            }
            let mut o = MoveOut::default();
            let tip_lo = m.at(forge_cam_geom::ival::Ival::new(0.0, 1.0))[2].lo;
            // proven above (a NaN is not)
            let above = forge_cam_geom::ival::Ival::pt(tip_lo)
                .add(forge_cam_geom::ival::Ival::pt(top))
                .lo
                > zmax;
            if !above {
                o.containment = Some(CheckOut {
                    rule: "containment",
                    verdict: "undecided",
                    value: tip_lo + top,
                    limit: zmax,
                    at: [f64::NAN; 3],
                    tool_part: "holder",
                    other: "the tool's top is not above the part and stock".into(),
                    phase: phase.name(),
                    move_index: *idx,
                });
                outs.push(o);
                continue;
            }
            let mut st = 0;
            while st + 1 < stages.len() && stages[st + 1].0 <= *idx {
                st += 1;
            }
            let stock: &[Face] = &stages[st].1;
            let sgrid = &sgrids[st];
            for b in &tool.shape.bodies {
                // on the surfacing route a move that has failed is not checked further (its
                // failure is reported; the rest of its faces could only add to it)
                if col.is_some() && !o.fails.is_empty() {
                    break;
                }
                let sb = check::swept_box_body(m, b);
                // the part: the cutter may touch it within tol on a cut; nothing else may touch it
                for (set, (faces, what, grid)) in [
                    (part, "part", &pgrid),
                    (stock, "stock", sgrid),
                    (fixture, "fixture", &fgrid),
                ]
                .into_iter()
                .enumerate()
                {
                    if faces.is_empty() {
                        continue;
                    }
                    if what == "stock" && !(b.part == Part::NonCutting || *phase == Phase::Rapid) {
                        // the cutter cuts the stock: that is the point
                        continue;
                    }
                    // a shank no wider than the cutter rides in the cutter's own cut: it is held
                    // to the cutter's rule (within tol), never to a clearance it cannot have; the
                    // holder and any wider shank keep the clearance
                    let narrow = b.part == Part::NonCutting && b.rmax <= cutter_r;
                    let limit = if what == "fixture" {
                        fclear
                    } else if (b.part == Part::Cutter && what == "part" || narrow) && *phase != Phase::Rapid {
                        -tol
                    } else {
                        clearance
                    };
                    grid.near(&sb, &mut cand, &mut seen);
                    for &fi in &cand {
                        if col.is_some() && !o.fails.is_empty() {
                            break;
                        }
                        let f = &faces[fi as usize];
                        if check::apart(&sb, &f.bbox, limit.max(0.0) + f.mu) {
                            continue;
                        }
                        o.pairs += 1;
                        let slot = &mut shared[set][fi as usize];
                        if slot.is_none() {
                            touched.push((set, fi as usize));
                        }
                        if let Some(col) = &col
                            && what == "part"
                            && limit <= 0.0
                            && *phase != Phase::Rapid
                            && col.holds_or_is(b)
                        {
                            let (g, n) = forge_cam_geom::drop::gap_move(col, f, m, limit, tol / 4.0, 200_000);
                            o.cells += n;
                            match g {
                                forge_cam_geom::drop::Gap::Clear { lower } => {
                                    if b.part == Part::Cutter {
                                        o.least = o.least.min(lower);
                                    }
                                    continue;
                                }
                                forge_cam_geom::drop::Gap::Over { s, p, q, .. } => {
                                    // the column's witness: this body's own signed distance there,
                                    // proven below the limit (by mu more), is a real violation
                                    let (pi, qi) = (
                                        forge_cam_geom::ival::Ival::pt(p),
                                        forge_cam_geom::ival::Ival::pt(q),
                                    );
                                    let y = f.surf.eval(pi, qi);
                                    let t = m.at(forge_cam_geom::ival::Ival::pt(s));
                                    let sd = b.sd(y[0].sub(t[0]), y[1].sub(t[1]), y[2].sub(t[2]));
                                    let d = sd.add(forge_cam_geom::ival::Ival::pt(f.mu)).hi;
                                    if f.holds(pi, qi) == 1 && d < limit {
                                        let at = [y[0].mid(), y[1].mid(), y[2].mid()];
                                        o.fails.push((
                                            CheckOut {
                                                rule: if b.part == Part::Cutter {
                                                    "gouge"
                                                } else {
                                                    "holder_collision"
                                                },
                                                verdict: "fail",
                                                value: d,
                                                limit,
                                                at,
                                                tool_part: if b.part == Part::Cutter {
                                                    "cutter"
                                                } else {
                                                    "shank/holder"
                                                },
                                                other: format!("{what} face {}", f.id),
                                                phase: phase.name(),
                                                move_index: *idx,
                                            },
                                            o.pairs,
                                            o.cells,
                                            o.least,
                                        ));
                                        continue;
                                    }
                                }
                                forge_cam_geom::drop::Gap::Unknown => {}
                            }
                        }
                        let (v, n) = check::certify_shared(b, f, m, limit, tol / 4.0, 200_000, slot);
                        o.cells += n;
                        let rule = match (b.part, *phase, what) {
                            (_, _, "fixture") => "fixture_collision",
                            (_, Phase::Rapid, _) => "rapid_clear",
                            (Part::Cutter, _, "part") => "gouge",
                            _ => "holder_collision",
                        };
                        let tool_part = if b.part == Part::Cutter {
                            "cutter"
                        } else {
                            "shank/holder"
                        };
                        let (verdict, value, at) = match v {
                            Verdict::Clear { lower } => {
                                if what == "part" && b.part == Part::Cutter {
                                    o.least = o.least.min(lower);
                                }
                                continue;
                            }
                            Verdict::Hit { d, at, .. } => ("fail", d, at),
                            Verdict::Undecided { lower, .. } => ("undecided", lower, [f64::NAN; 3]),
                        };
                        o.fails.push((
                            CheckOut {
                                rule,
                                verdict,
                                value,
                                limit,
                                at,
                                tool_part,
                                other: format!("{what} face {}", f.id),
                                phase: phase.name(),
                                move_index: *idx,
                            },
                            o.pairs,
                            o.cells,
                            o.least,
                        ));
                    }
                }
            }
            outs.push(o);
        }
        outs
    });
    for o in chunks.into_iter().flatten() {
        if let Some(c) = o.containment {
            sum.clear = false;
            sum.checks.push(c);
            return sum;
        }
        let (p0, c0, l0) = (sum.pairs, sum.cells, sum.least);
        for (c, pairs, cells, least) in o.fails {
            sum.clear = false;
            sum.checks.push(c);
            if sum.checks.len() > 20 {
                sum.pairs = p0 + pairs;
                sum.cells = c0 + cells;
                sum.least = l0.min(least);
                return sum;
            }
        }
        sum.pairs = p0 + o.pairs;
        sum.cells = c0 + o.cells;
        sum.least = l0.min(o.least);
    }
    sum
}

/// One move's checks, as the merge needs them: its counts, its least cutter distance, each
/// failing check with the counts and least as they stood when it was found.
struct MoveOut {
    pairs: usize,
    cells: usize,
    least: f64,
    fails: Vec<(CheckOut, usize, usize, f64)>,
    containment: Option<CheckOut>,
}

impl Default for MoveOut {
    fn default() -> MoveOut {
        MoveOut {
            pairs: 0,
            cells: 0,
            least: f64::INFINITY,
            fails: vec![],
            containment: None,
        }
    }
}

/// Heights an op links at.
#[derive(Clone, Copy, Debug)]
pub struct Heights {
    /// Rapids between ops and features: above the stock.
    pub clear: f64,
    /// Links within an op.
    pub retract: f64,
}

/// What an op cuts with: the tool, its slot, the heights it links at, and its feeds (cut,
/// plunge).
#[derive(Clone, Copy)]
pub struct Cut<'a> {
    pub tool: &'a ToolUse,
    pub tidx: usize,
    pub h: Heights,
    pub feed: (f64, f64),
}

/// Levels from `top` down to `bottom`, at most `step` apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub top: f64,
    pub bottom: f64,
    pub step: f64,
}

fn push_loop(out: &mut Vec<PMove>, segs: &[Seg], z: f64, f: f64) {
    for s in segs {
        match *s {
            Seg::Line { b, .. } => out.push(PMove::Line {
                to: [b.x, b.y, z],
                f,
                phase: Phase::Cut,
            }),
            Seg::Arc { b, c, ccw, .. } => out.push(PMove::Arc {
                to: [b.x, b.y, z],
                c: [c.x, c.y],
                cw: !ccw,
                f,
            }),
        }
    }
}

/// The depths from top down to bottom in steps of at most `step` (equal steps).
pub fn levels(top: f64, bottom: f64, step: f64) -> Vec<f64> {
    let n = ((top - bottom) / step - 1e-9).ceil().max(1.0) as usize;
    (1..=n)
        .map(|k| top - (top - bottom) * k as f64 / n as f64)
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpError {
    /// The region or its offset is not usable: why.
    Region(String),
    /// Nothing to cut (the offset is empty: the tool does not fit).
    ToolTooBig { tool_d: f64, why: String },
    /// No tool in the library for this: what is needed.
    NeedTool { what: String },
}

/// Contour: the region's boundary, offset by the tool radius plus `leave` (outside for the
/// part's outline: `outside` true), at each level.
pub fn contour(rg: &Region, c: Cut, sp: Span, leave: f64, outside: bool) -> Result<OpPath, OpError> {
    let Cut { tool, tidx, h, feed } = c;
    let Span { top, bottom, step } = sp;
    let r = 0.5 * tool.tool.dc + leave;
    let d = if outside { -r } else { r };
    let (loops, _) = offset(rg, d).map_err(|e| OpError::Region(format!("{e:?}")))?;
    let loops: Vec<_> = loops
        .into_iter()
        .filter(|l| !(l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b)))
        .collect();
    if loops.is_empty() {
        return Err(OpError::ToolTooBig {
            tool_d: tool.tool.dc,
            why: "the tool does not fit the contour".into(),
        });
    }
    let mut mv = Vec::new();
    let first = loops[0].segs[0].start();
    let start = [first.x, first.y, h.clear];
    for z in levels(top, bottom, step) {
        for l in &loops {
            let s = l.segs[0].start();
            mv.push(PMove::Rapid {
                to: [s.x, s.y, h.retract],
            });
            mv.push(PMove::Line {
                to: [s.x, s.y, z],
                f: feed.1,
                phase: Phase::Plunge,
            });
            push_loop(&mut mv, &l.segs, z, feed.0);
            mv.push(PMove::Line {
                to: [s.x, s.y, h.retract],
                f: feed.0,
                phase: Phase::Link,
            });
        }
    }
    mv.push(PMove::Rapid {
        to: [first.x, first.y, h.clear],
    });
    Ok(OpPath {
        tool: tidx,
        start,
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    })
}

/// Pocket: the region cleared by offset passes, each offset from the original boundary at
/// r + leave + k · stepover, innermost first, at each level.
pub fn pocket(
    rg: &Region,
    c: Cut,
    sp: Span,
    stepover: f64,
    leave: f64,
    ramp: Option<f64>,
) -> Result<OpPath, OpError> {
    let Cut { tool, tidx, h, feed } = c;
    let Span { top, bottom, step } = sp;
    let r = 0.5 * tool.tool.dc + leave;
    let mut passes = Vec::new();
    let mut k = 0;
    loop {
        let (loops, _) =
            offset(rg, r + k as f64 * stepover).map_err(|e| OpError::Region(format!("{e:?}")))?;
        if loops.is_empty() {
            break;
        }
        passes.push(loops);
        k += 1;
        if k > 10_000 {
            return Err(OpError::Region("too many passes".into()));
        }
    }
    if passes.is_empty() {
        return Err(OpError::ToolTooBig {
            tool_d: tool.tool.dc,
            why: "the tool does not fit the pocket".into(),
        });
    }
    // a helical entry about a pass's start, of radius half the tool's, where that disk lies in
    // the tool centres' region (exactly: then the helix stays where the passes go); else a
    // plunge
    let rh = 0.25 * tool.tool.dc;
    let c0 = forge_cam_geom::boolean::region_of(&passes[0]);
    passes.reverse();
    let helix_fits = |s: forge_cam_geom::region::P2| -> bool {
        let a = forge_cam_geom::region::p2(s.x + rh, s.y);
        let disk = Region {
            loops: vec![forge_cam_geom::region::Loop {
                segs: vec![Seg::Arc {
                    a,
                    b: a,
                    c: s,
                    ccw: true,
                }],
            }],
        };
        matches!(
            forge_cam_geom::boolean::boolean(&disk, &c0, forge_cam_geom::boolean::Op::Diff),
            Ok(d) if d.is_empty()
        )
    };
    let fits: Vec<Vec<bool>> = passes
        .iter()
        .map(|ls| {
            ls.iter()
                .map(|l| ramp.is_some() && helix_fits(l.segs[0].start()))
                .collect()
        })
        .collect();
    let mut mv = Vec::new();
    let s0 = passes[0][0].segs[0].start();
    let start = [s0.x, s0.y, h.clear];
    let mut above = top;
    for z in levels(top, bottom, step) {
        for (li, loops) in passes.iter().enumerate() {
            for (lj, l) in loops.iter().enumerate() {
                let s = l.segs[0].start();
                if let (true, Some(deg)) = (fits[li][lj], ramp) {
                    // down to the level above in air (it, and this disk, were cleared there),
                    // then turns of the helix at the ramp angle, then in to the pass's start
                    let p = [s.x + rh, s.y];
                    mv.push(PMove::Rapid {
                        to: [p[0], p[1], h.retract],
                    });
                    mv.push(PMove::Line {
                        to: [p[0], p[1], above],
                        f: feed.1,
                        phase: Phase::Plunge,
                    });
                    let (sa, ca) = dmath::sin_cos(deg.to_radians());
                    let per_turn = std::f64::consts::TAU * rh * sa / ca;
                    let turns = ((above - z) / per_turn - 1e-9).ceil().max(1.0) as usize;
                    let quarters = 4 * turns;
                    for k in 1..=quarters {
                        let a = std::f64::consts::FRAC_PI_2 * k as f64;
                        let (sn, cs) = dmath::sin_cos(a);
                        // the quarter points, the last exactly the start (a whole number of turns)
                        let (x, y) = if k % 4 == 0 {
                            (p[0], p[1])
                        } else {
                            (s.x + rh * cs, s.y + rh * sn)
                        };
                        let zk = above + (z - above) * k as f64 / quarters as f64;
                        mv.push(PMove::Arc {
                            to: [x, y, if k == quarters { z } else { zk }],
                            c: [s.x, s.y],
                            cw: false,
                            f: feed.1,
                        });
                    }
                    mv.push(PMove::Line {
                        to: [s.x, s.y, z],
                        f: feed.0,
                        phase: Phase::Cut,
                    });
                } else {
                    mv.push(PMove::Rapid {
                        to: [s.x, s.y, h.retract],
                    });
                    mv.push(PMove::Line {
                        to: [s.x, s.y, z],
                        f: feed.1,
                        phase: Phase::Plunge,
                    });
                }
                let point = l.segs.len() == 1 && matches!(l.segs[0], Seg::Line { a, b } if a == b);
                if !point {
                    push_loop(&mut mv, &l.segs, z, feed.0);
                }
                mv.push(PMove::Line {
                    to: [s.x, s.y, h.retract],
                    f: feed.0,
                    phase: Phase::Link,
                });
            }
        }
        above = z;
    }
    mv.push(PMove::Rapid {
        to: [s0.x, s0.y, h.clear],
    });
    Ok(OpPath {
        tool: tidx,
        start,
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    })
}

/// Facing: zig-zag passes across the stock's top region from `top` down to `z`, the tool
/// overhanging the stock by its radius each side.
/// `sp.bottom` is the faced height.
pub fn facing(st: &Stock, c: Cut, sp: Span) -> OpPath {
    let Cut { tool, tidx, h, feed } = c;
    let Span { top, bottom: z, step } = sp;
    let d = tool.tool.dc;
    let over = 0.7 * d;
    let (x0, x1) = (st.min[0] - 0.5 * d - 1.0, st.max[0] + 0.5 * d + 1.0);
    let (y0, y1) = (st.min[1], st.max[1]);
    let n = (((y1 - y0) / over).ceil() as usize).max(1);
    let mut mv = Vec::new();
    let start = [x0, y0, h.clear];
    for zl in levels(top, z, step) {
        mv.push(PMove::Rapid {
            to: [x0, y0, h.retract],
        });
        mv.push(PMove::Line {
            to: [x0, y0, zl],
            f: feed.1,
            phase: Phase::Plunge,
        });
        for i in 0..=n {
            let y = y0 + (y1 - y0) * i as f64 / n as f64;
            let (xa, xb) = if i % 2 == 0 { (x0, x1) } else { (x1, x0) };
            if i > 0 {
                mv.push(PMove::Line {
                    to: [xa, y, zl],
                    f: feed.0,
                    phase: Phase::Cut,
                });
            }
            mv.push(PMove::Line {
                to: [xb, y, zl],
                f: feed.0,
                phase: Phase::Cut,
            });
        }
        let last = if n.is_multiple_of(2) { x1 } else { x0 };
        mv.push(PMove::Line {
            to: [last, y1, h.retract],
            f: feed.0,
            phase: Phase::Link,
        });
        mv.push(PMove::Rapid {
            to: [x0, y0, h.retract],
        });
    }
    mv.push(PMove::Rapid {
        to: [x0, y0, h.clear],
    });
    OpPath {
        tool: tidx,
        start,
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    }
}

/// A canned cycle at each place (drilling straight down, along −z of the part).
/// The cycle's feed is `c.feed.0`.
pub fn holes(
    places: &[[f64; 3]],
    kind: forge_post::CycleKind,
    c: Cut,
    depth_below_mouth: f64,
    q: f64,
) -> OpPath {
    let Cut { tool, tidx, h, feed } = c;
    let f = feed.0;
    let mut mv = Vec::new();
    let start = [places[0][0], places[0][1], h.clear];
    for p in places {
        // the tip's depth: the wall's depth plus the point's length for drills
        let tip = match tool.tool.kind {
            forge_tooling::Kind::Drill => 0.5 * tool.tool.dc / (0.5 * tool.tool.angle).to_radians_tan(),
            _ => 0.0,
        };
        mv.push(PMove::Rapid {
            to: [p[0], p[1], h.clear],
        });
        mv.push(PMove::Cycle {
            kind,
            holes: [p[0], p[1]],
            z: p[2] - depth_below_mouth - tip,
            r: p[2] + 2.0,
            clear: h.clear,
            q,
            dwell: 0.0,
            f,
        });
    }
    OpPath {
        tool: tidx,
        start,
        moves: mv,
        rpm: 0.0,
        coolant: forge_post::Coolant::Flood,
    }
}

trait TanDeg {
    fn to_radians_tan(self) -> f64;
}
impl TanDeg for f64 {
    fn to_radians_tan(self) -> f64 {
        let r = self.to_radians();
        dmath::sin(r) / dmath::cos(r)
    }
}
