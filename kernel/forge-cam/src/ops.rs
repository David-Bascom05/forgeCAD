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
    /// Which of the op's paths `move_index` counts in (a hole plan posts one path per tool); 0
    /// for a single path. `certify` checks one path and leaves it 0; the caller sets it.
    pub path: usize,
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
                    path: 0,
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
                        let f = &faces[fi as usize];
                        if check::apart(&sb, &f.bbox, limit.max(0.0) + f.mu) {
                            continue;
                        }
                        o.pairs += 1;
                        let slot = &mut shared[set][fi as usize];
                        if slot.is_none() {
                            touched.push((set, fi as usize));
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
                                path: 0,
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

/// Half-width of the band a pocket's link at depth is tested in (mm).
const LINK_EPS: f64 = 1e-6;

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
/// part's outline: `outside` true), at each level, each loop through all its levels in turn.
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
    // climb with the spindle clockwise (M03): the part on the tool's right. An offset loop has
    // its region on the left: inside, the region is where the tool goes (the part on the
    // right); outside, it is the part grown, so the loops are run backwards.
    let loops: Vec<Vec<Seg>> = loops
        .iter()
        .map(|l| {
            if outside {
                l.segs.iter().rev().map(|s| s.reversed()).collect()
            } else {
                l.segs.clone()
            }
        })
        .collect();
    let mut mv = Vec::new();
    let first = loops[0][0].start();
    let start = [first.x, first.y, h.clear];
    // each loop through all its levels: down at its start from the retract height, then from
    // each level straight down at the start to the next (the column the plunge from the retract
    // height would pass through), and up once at the end
    for l in &loops {
        let s = l[0].start();
        mv.push(PMove::Rapid {
            to: [s.x, s.y, h.retract],
        });
        for z in levels(top, bottom, step) {
            mv.push(PMove::Line {
                to: [s.x, s.y, z],
                f: feed.1,
                phase: Phase::Plunge,
            });
            push_loop(&mut mv, l, z, feed.0);
        }
        mv.push(PMove::Line {
            to: [s.x, s.y, h.retract],
            f: feed.0,
            phase: Phase::Link,
        });
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
/// r + leave + k · stepover, innermost first, at each level. Within a level the tool stays down
/// from one pass to the next where the straight link lies in the tool centres' region (the
/// outermost pass's region); elsewhere it retracts and enters again (a helix where it fits).
pub fn pocket(
    rg: &Region,
    c: Cut,
    sp: Span,
    stepover: f64,
    leave: f64,
    ramp: Option<f64>,
) -> Result<OpPath, OpError> {
    pocket_in(rg, c, sp, stepover, leave, ramp, None)
}

/// `pocket`, with the stock's xy box S given: with stepover ≤ r, a pass is left out when all
/// it could cut in S is cut by the passes inside it. Passes are offsets at r + k·s; Rₖ the region pass k bounds
/// (islands as holes), the passes from k inward cover Rₖ ⊕ r (passes_cover applied to
/// rg ⊖ (k·s)), and pass k's swept band lies in Rₖ ⊕ r. Deciding from the innermost pass out,
/// pass k is left out when (Rₖ ⊕ r) ∩ S ⊆ Rₖ₊₁ ⊕ r (exact booleans), Rₖ₊₁ the next pass in:
/// the kept passes from k inward then still cover (Rₖ ⊕ r) ∩ S (induction), so the opening the
/// op claims is cleared within S, and outside S there is no stock. (A clear's region reaches
/// 2r past the stock, so its outer passes run in air.)
/// How far the tool's disk keeps from the stock's box for a side entry (mm): rapids are
/// certified 0.1 mm clear of the stock.
const SIDE_CLEAR: f64 = 0.6;
/// The last part of a side entry's way down, at the plunge feed (mm).
const SIDE_IN: f64 = 1.0;

/// The way down from the retract height to `to`, the level above the one being entered. Below
/// the first level (`cleared`) it is air: the passes are entered in the same order and the same
/// way at every level, so at the level above the tool stood at this very point at that depth (the
/// helix ends where it starts; a plunge is at the pass's start), and its body swept the column
/// above it. A link at the feed, not a plunge. At the first level the stock before the op is
/// above it: a plunge.
fn air_down(to: [f64; 3], cleared: bool, feed: (f64, f64)) -> PMove {
    if cleared {
        PMove::Line {
            to,
            f: feed.0,
            phase: Phase::Link,
        }
    } else {
        PMove::Line {
            to,
            f: feed.1,
            phase: Phase::Plunge,
        }
    }
}

pub fn pocket_in(
    rg: &Region,
    c: Cut,
    sp: Span,
    stepover: f64,
    leave: f64,
    ramp: Option<f64>,
    within: Option<&Region>,
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
    // (only where the passes cover the opening: stepover ≤ r, Pocket.passes_cover)
    if let Some(sx) = within.filter(|_| stepover <= r) {
        use forge_cam_geom::boolean::{Op, boolean, region_of};
        // passes[k] is at r + k s, outermost first here. Only passes whose loops can run
        // outside the stock are tried: a clear's region reaches 2r past it, so pass k's
        // centres can be outside S only while k s < 2r (deeper passes cut material).
        for k in (0..passes.len().saturating_sub(1)).rev() {
            if k as f64 * stepover >= 2.0 * r {
                continue;
            }
            // R_{k+1} ⊕ r
            let Some(gin) = offset(&region_of(&passes[k + 1]), -r)
                .ok()
                .map(|(g, _)| region_of(&g))
            else {
                continue;
            };
            let gin = &gin;
            passes[k].retain(|l| {
                // only an outer loop (its region alone a bounded one); its swept band is
                // (L ⊕ r) ∖ (L ⊖ r), L the region it bounds
                if !loop_ccw_approx(&l.segs) {
                    return true;
                }
                // only a loop round the whole stock box (an air ring) is tried: keeping a loop is
                // always sound, and the exact test can be slow where boundaries nearly meet
                let (lo, hi) = loop_box(&l.segs);
                let (blo, bhi) = region_box(sx);
                if !(lo[0] <= blo[0] && lo[1] <= blo[1] && hi[0] >= bhi[0] && hi[1] >= bhi[1]) {
                    return true;
                }
                let lr = region_of(std::slice::from_ref(l));
                let Ok((out, _)) = offset(&lr, -r) else {
                    return true;
                };
                let band = match offset(&lr, r) {
                    Ok((inn, _)) if !inn.is_empty() => {
                        boolean(&region_of(&out), &region_of(&inn), Op::Diff).map(|b| region_of(&b))
                    }
                    Ok(_) => Ok(region_of(&out)),
                    Err(e) => Err(e),
                };
                let left = band
                    .and_then(|b| boolean(&b, sx, Op::Inter))
                    .and_then(|i| boolean(&region_of(&i), gin, Op::Diff));
                !matches!(left, Ok(d) if d.is_empty())
            });
        }
        passes.retain(|ls| !ls.is_empty());
    }
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
    // a link at depth from one pass's start to the next's: when the straight segment lies in
    // the tool centres' region (then the tool stays in the pocket as on the passes). Tested
    // exactly, as a rectangle of half-width LINK_EPS about the segment shortened by 2 LINK_EPS
    // at each end (the ends lie on passes, in the region) less the region being empty.
    let link_ok = |a: forge_cam_geom::region::P2, b: forge_cam_geom::region::P2| -> bool {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l = (dx * dx + dy * dy).sqrt();
        if l <= 8.0 * LINK_EPS {
            return false;
        }
        let (ux, uy) = (dx / l, dy / l);
        let (nx, ny) = (-uy * LINK_EPS, ux * LINK_EPS);
        let a1 = (a.x + 2.0 * LINK_EPS * ux, a.y + 2.0 * LINK_EPS * uy);
        let b1 = (b.x - 2.0 * LINK_EPS * ux, b.y - 2.0 * LINK_EPS * uy);
        let q = [
            forge_cam_geom::region::p2(a1.0 - nx, a1.1 - ny),
            forge_cam_geom::region::p2(b1.0 - nx, b1.1 - ny),
            forge_cam_geom::region::p2(b1.0 + nx, b1.1 + ny),
            forge_cam_geom::region::p2(a1.0 + nx, a1.1 + ny),
        ];
        let band = Region {
            loops: vec![forge_cam_geom::region::Loop {
                segs: (0..4)
                    .map(|i| Seg::Line {
                        a: q[i],
                        b: q[(i + 1) % 4],
                    })
                    .collect(),
            }],
        };
        matches!(
            forge_cam_geom::boolean::boolean(&band, &c0, forge_cam_geom::boolean::Op::Diff),
            Ok(d) if d.is_empty()
        )
    };
    // a point the tool's centre may stand at: a square of half-width LINK_EPS about it inside
    // the tool centres' region (exact)
    let point_ok = |q: forge_cam_geom::region::P2| -> bool {
        let c = [
            forge_cam_geom::region::p2(q.x - LINK_EPS, q.y - LINK_EPS),
            forge_cam_geom::region::p2(q.x + LINK_EPS, q.y - LINK_EPS),
            forge_cam_geom::region::p2(q.x + LINK_EPS, q.y + LINK_EPS),
            forge_cam_geom::region::p2(q.x - LINK_EPS, q.y + LINK_EPS),
        ];
        let sq = Region {
            loops: vec![forge_cam_geom::region::Loop {
                segs: (0..4)
                    .map(|i| Seg::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    })
                    .collect(),
            }],
        };
        matches!(
            forge_cam_geom::boolean::boolean(&sq, &c0, forge_cam_geom::boolean::Op::Diff),
            Ok(d) if d.is_empty()
        )
    };
    // an entry from beside the stock (a clear's open region): the stock and the part lie within
    // the box of `within` (its loops all lines: then that box bounds them). From q, straight out
    // along x or y from the pass's start s with the tool's disk SIDE_CLEAR beyond the box, the
    // tool comes down rapid to SIDE_IN above the level and feeds in sideways to s, instead of
    // plunging into the stock. The way in: up to e, where the centre is r inside the box's
    // outline grown by r, the disk meets nothing in the box (it is beyond it); from e to s, a
    // link the tool centres' region holds (exact, as at depth). The nearest such q within 4r.
    let sbox = within.filter(|sx| {
        sx.loops
            .iter()
            .all(|l| l.segs.iter().all(|sg| matches!(sg, Seg::Line { .. })))
    });
    let side_entry = |s: forge_cam_geom::region::P2| -> Option<forge_cam_geom::region::P2> {
        let (blo, bhi) = region_box(sbox?);
        let m = r + SIDE_CLEAR;
        // q, and e a hair (1e-3) inside the box grown by r, along each of the four ways out
        let ways = [
            (
                [blo[0] - m, s.y],
                [blo[0] - r + 1e-3, s.y],
                s.x > blo[0] - r + 1e-3,
            ),
            (
                [bhi[0] + m, s.y],
                [bhi[0] + r - 1e-3, s.y],
                s.x < bhi[0] + r - 1e-3,
            ),
            (
                [s.x, blo[1] - m],
                [s.x, blo[1] - r + 1e-3],
                s.y > blo[1] - r + 1e-3,
            ),
            (
                [s.x, bhi[1] + m],
                [s.x, bhi[1] + r - 1e-3],
                s.y < bhi[1] + r - 1e-3,
            ),
        ];
        let mut best: Option<(f64, forge_cam_geom::region::P2)> = None;
        for (q, e, beyond) in ways {
            if !beyond {
                continue;
            }
            let (q, e) = (
                forge_cam_geom::region::p2(q[0], q[1]),
                forge_cam_geom::region::p2(e[0], e[1]),
            );
            let d = ((q.x - s.x) * (q.x - s.x) + (q.y - s.y) * (q.y - s.y)).sqrt();
            if d > 4.0 * r || best.is_some_and(|(bd, _)| bd <= d) {
                continue;
            }
            if point_ok(e) && link_ok(e, s) {
                best = Some((d, q));
            }
        }
        best.map(|(_, q)| q)
    };
    let fits: Vec<Vec<bool>> = passes
        .iter()
        .map(|ls| {
            ls.iter()
                .map(|l| ramp.is_some() && helix_fits(l.segs[0].start()))
                .collect()
        })
        .collect();
    // the order the passes are cut in, the same at every level, and whether each is reached
    // from the one before at depth: greedily the innermost pass that is ready (no uncut pass
    // starts inside it) and linkable at depth, else the first ready pass, entered from above
    let order = pass_order(&passes, &link_ok);
    let mut mv = Vec::new();
    let s0 = passes[order[0].0][order[0].1].segs[0].start();
    let start = [s0.x, s0.y, h.clear];
    let mut above = top;
    for z in levels(top, bottom, step) {
        // where the tool is when it stayed down after a pass (each pass ends at its start)
        let mut down: Option<forge_cam_geom::region::P2> = None;
        for &(li, lj, link) in &order {
            let l = &passes[li][lj];
            let s = l.segs[0].start();
            let linked = match down {
                Some(p) if link => {
                    if p != s {
                        mv.push(PMove::Line {
                            to: [s.x, s.y, z],
                            f: feed.0,
                            phase: Phase::Link,
                        });
                    }
                    true
                }
                Some(p) => {
                    mv.push(PMove::Line {
                        to: [p.x, p.y, h.retract],
                        f: feed.0,
                        phase: Phase::Link,
                    });
                    false
                }
                None => false,
            };
            if linked {
                // at the pass's start, at depth
            } else if let (true, Some(deg)) = (fits[li][lj], ramp) {
                // down to the level above in air (it, and this disk, were cleared there),
                // then turns of the helix at the ramp angle, then in to the pass's start
                let p = [s.x + rh, s.y];
                mv.push(PMove::Rapid {
                    to: [p[0], p[1], h.retract],
                });
                mv.push(air_down([p[0], p[1], above], above < top, feed));
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
            } else if let Some(q) = side_entry(s) {
                // beside the stock: rapid down clear of it, the last SIDE_IN at the plunge feed,
                // then in sideways
                mv.push(PMove::Rapid {
                    to: [q.x, q.y, h.retract],
                });
                if z + SIDE_IN < h.retract {
                    mv.push(PMove::Rapid {
                        to: [q.x, q.y, z + SIDE_IN],
                    });
                }
                mv.push(PMove::Line {
                    to: [q.x, q.y, z],
                    f: feed.1,
                    phase: Phase::Plunge,
                });
                if q != s {
                    mv.push(PMove::Line {
                        to: [s.x, s.y, z],
                        f: feed.0,
                        phase: Phase::Cut,
                    });
                }
            } else {
                mv.push(PMove::Rapid {
                    to: [s.x, s.y, h.retract],
                });
                if above < top {
                    mv.push(air_down([s.x, s.y, above], true, feed));
                }
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
            down = Some(s);
        }
        if let Some(p) = down {
            mv.push(PMove::Line {
                to: [p.x, p.y, h.retract],
                f: feed.0,
                phase: Phase::Link,
            });
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

/// A loop's box from its segments' ends (arcs may bulge past it): for choosing loops only.
fn loop_box(segs: &[Seg]) -> ([f64; 2], [f64; 2]) {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    let mut add = |x: f64, y: f64| {
        lo = [lo[0].min(x), lo[1].min(y)];
        hi = [hi[0].max(x), hi[1].max(y)];
    };
    for sg in segs {
        match *sg {
            Seg::Line { a, b } => {
                add(a.x, a.y);
                add(b.x, b.y);
            }
            Seg::Arc { a, b, .. } => {
                // the ends only: the box is used to choose loops to test, never as a bound
                add(a.x, a.y);
                add(b.x, b.y);
            }
        }
    }
    (lo, hi)
}

fn region_box(r: &Region) -> ([f64; 2], [f64; 2]) {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for l in &r.loops {
        let (a, b) = loop_box(&l.segs);
        lo = [lo[0].min(a[0]), lo[1].min(a[1])];
        hi = [hi[0].max(b[0]), hi[1].max(b[1])];
    }
    (lo, hi)
}

/// Whether a closed loop runs counter-clockwise (its signed area, arcs as chords): for choosing
/// which loops to test, never for a check.
fn loop_ccw_approx(segs: &[Seg]) -> bool {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for sg in segs {
        match *sg {
            Seg::Line { a, .. } => pts.push((a.x, a.y)),
            Seg::Arc { a, b, c, ccw } => {
                pts.push((a.x, a.y));
                // the arc's middle, so a half or whole turn counts
                let a0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let a1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let tau = std::f64::consts::TAU;
                let mut sw = a1 - a0;
                if ccw {
                    while sw <= 0.0 {
                        sw += tau;
                    }
                } else {
                    while sw >= 0.0 {
                        sw -= tau;
                    }
                }
                let r = ((a.x - c.x) * (a.x - c.x) + (a.y - c.y) * (a.y - c.y)).sqrt();
                for q in 1..4 {
                    let (sn, cs) = dmath::sin_cos(a0 + sw * q as f64 / 4.0);
                    pts.push((c.x + r * cs, c.y + r * sn));
                }
            }
        }
    }
    let n = pts.len();
    let mut area = 0.0;
    for i in 0..n {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % n];
        area += x0 * y1 - x1 * y0;
    }
    area > 0.0
}

/// Whether q is inside a closed loop (even–odd, arcs as 32 chords a turn): for ordering passes
/// only, never for a check.
fn inside_loop_approx(segs: &[Seg], q: forge_cam_geom::region::P2) -> bool {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for sg in segs {
        match *sg {
            Seg::Line { a, .. } => pts.push((a.x, a.y)),
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x) * (a.x - c.x) + (a.y - c.y) * (a.y - c.y)).sqrt();
                let a0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let a1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let tau = std::f64::consts::TAU;
                let mut sw = a1 - a0;
                if ccw {
                    while sw <= 0.0 {
                        sw += tau;
                    }
                } else {
                    while sw >= 0.0 {
                        sw -= tau;
                    }
                }
                let n = ((sw.abs() / tau * 32.0).ceil() as usize).max(1);
                for k in 0..n {
                    let (sn, cs) = dmath::sin_cos(a0 + sw * k as f64 / n as f64);
                    pts.push((c.x + r * cs, c.y + r * sn));
                }
            }
        }
    }
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % n];
        if (y0 > q.y) != (y1 > q.y) && q.x < x0 + (q.y - y0) * (x1 - x0) / (y1 - y0) {
            inside = !inside;
        }
    }
    inside
}

/// The order of a pocket's passes (innermost first as given): each step the innermost ready
/// pass (no uncut pass starts inside it) linkable at depth from where the last ended (nearest
/// among equals), else the first ready pass. (ring, loop, reached at depth).
fn pass_order(
    passes: &[Vec<forge_cam_geom::offset::OLoop>],
    link_ok: &dyn Fn(forge_cam_geom::region::P2, forge_cam_geom::region::P2) -> bool,
) -> Vec<(usize, usize, bool)> {
    let all: Vec<(usize, usize)> = passes
        .iter()
        .enumerate()
        .flat_map(|(i, ls)| (0..ls.len()).map(move |j| (i, j)))
        .collect();
    let start = |(i, j): (usize, usize)| passes[i][j].segs[0].start();
    // inner[k]: the passes of smaller ring index starting inside pass k
    let inner: Vec<Vec<usize>> = all
        .iter()
        .map(|&(i, j)| {
            all.iter()
                .enumerate()
                .filter(|&(_, &(i2, _))| i2 < i)
                .filter(|&(_, &o)| inside_loop_approx(&passes[i][j].segs, start(o)))
                .map(|(k, _)| k)
                .collect()
        })
        .collect();
    let mut done = vec![false; all.len()];
    let mut order = Vec::with_capacity(all.len());
    let mut at: Option<forge_cam_geom::region::P2> = None;
    while order.len() < all.len() {
        let ready = |k: usize| !done[k] && inner[k].iter().all(|&m| done[m]);
        let mut pick: Option<(usize, bool)> = None;
        if let Some(p) = at {
            let mut best: Option<(usize, usize, f64)> = None;
            for (k, &o) in all.iter().enumerate() {
                if !ready(k) {
                    continue;
                }
                let s = start(o);
                let d = (s.x - p.x) * (s.x - p.x) + (s.y - p.y) * (s.y - p.y);
                let better = match best {
                    None => true,
                    Some((_, bi, bd)) => o.0 < bi || (o.0 == bi && d < bd),
                };
                if better && (s == p || link_ok(p, s)) {
                    best = Some((k, o.0, d));
                }
            }
            pick = best.map(|(k, _, _)| (k, true));
        }
        let (k, linked) = pick.unwrap_or_else(|| ((0..all.len()).find(|&k| ready(k)).unwrap_or(0), false));
        done[k] = true;
        order.push((all[k].0, all[k].1, linked));
        at = Some(start(all[k]));
    }
    order
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
