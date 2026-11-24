//! Probing ops: a touch probe as a tool, and the paths that measure a surface, a bore, a boss,
//! a web, a pocket or a corner with it.
//!
//! A probe is a `ToolUse` with `probe: true` (forge-cam certifies every move of it as
//! non-cutting, its stylus ball touching only inside a `PMove::Probe`'s contact window). Its
//! shape: the stylus ball (the `Cutter` body), the stem from where it meets the ball up to the
//! mounting face, the probe body's rings above that, all about the tool axis from the ball's
//! lowest point (the tool tip).
//!
//! A probed point is a point on the surface `w` with the outward normal `n` (out of the
//! material, toward the probe). The ball meets it with its centre at w + r n, so the tip (the
//! ball's lowest point) is at w + r n − r ẑ: that is the `PMove::Probe`'s nominal contact.
use forge_cam::ops::{OpPath, PMove, Phase, ToolUse};
use forge_cam_geom::tool::{Body, PSeg, Part, Shape};
use forge_tooling::{Kind, Tool};

/// A touch probe's geometry (mm): ball diameter, stem length (mounting face to ball centre) and
/// diameter, the probe body's rings (r, from, to) above the mounting face.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeGeom {
    pub ball: f64,
    pub length: f64,
    pub stem: f64,
    pub body: Vec<(f64, f64, f64)>,
}

/// One probed point: the nominal surface point (part frame) and the probe's travel direction
/// (unit, into the surface: −n).
#[derive(Clone, Debug, PartialEq)]
pub struct ProbePoint {
    pub at: [f64; 3],
    pub dir: [f64; 3],
}

/// What a probing op measures, with fixed-name fields for the reply.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeOut {
    /// surface, bore, boss, web, pocket, corner, or inspect.
    pub kind: String,
    pub points: Vec<ProbePoint>,
    /// The quantity measured: "z", "centre and diameter", "centre and width", "corner x y", or
    /// "deviation along the normal".
    pub measures: &'static str,
    /// The work offset the measurement is for (54..59), if any. Writing it from the probe's
    /// result needs the control's own probing cycle or variables: the reply says whether the
    /// posted program does (`sets_wcs`).
    pub wcs: Option<u32>,
    pub sets_wcs: bool,
    /// Inspection: the nominal value and the tolerance (±) it is checked against.
    pub nominal: Option<f64>,
    pub tol: Option<f64>,
    /// The probe moves' overtravel and contact window entry (mm).
    pub over: f64,
    pub entry: f64,
}

/// The probe's shape, tip at the origin, axis +z.
pub fn shape(g: &ProbeGeom) -> Result<Shape, String> {
    let r = 0.5 * g.ball;
    let rs = 0.5 * g.stem;
    if r.is_nan() || r <= 0.0 {
        return Err("ball: the stylus ball's diameter, > 0".into());
    }
    if rs.is_nan() || rs <= 0.0 || rs >= r {
        return Err("stem: the stylus stem's diameter, > 0 and less than the ball's".into());
    }
    if g.length.is_nan() || g.length <= r {
        return Err(
            "length: the stylus length (mounting face to ball centre), more than the ball's radius".into(),
        );
    }
    let bad = |_| "the probe's shape is not a solid of revolution".to_string();
    let ball = Body::new(
        vec![
            PSeg::Arc {
                a: [0.0, 0.0],
                b: [r, r],
                c: [0.0, r],
                r,
                ccw: true,
            },
            PSeg::Arc {
                a: [r, r],
                b: [0.0, 2.0 * r],
                c: [0.0, r],
                r,
                ccw: true,
            },
        ],
        Part::Cutter,
    )
    .map_err(bad)?;
    // the stem from where it meets the ball up to the mounting face
    let z0 = r + (r * r - rs * rs).sqrt();
    let face = r + g.length;
    let stem = Body::new(
        vec![
            PSeg::Line {
                a: [0.0, z0],
                b: [rs, z0],
            },
            PSeg::Line {
                a: [rs, z0],
                b: [rs, face],
            },
            PSeg::Line {
                a: [rs, face],
                b: [0.0, face],
            },
        ],
        Part::NonCutting,
    )
    .map_err(bad)?;
    let mut bodies = vec![ball, stem];
    if g.body.is_empty() {
        return Err("body: the probe body's rings (r, from, to) above the mounting face; its envelope is needed to check it".into());
    }
    for &(br, a, b) in &g.body {
        if br.is_nan() || br <= 0.0 || a.is_nan() || b.is_nan() || a >= b || a < 0.0 {
            return Err("body: rings (r > 0, 0 <= from < to) above the mounting face".into());
        }
        bodies.push(
            Body::new(
                vec![
                    PSeg::Line {
                        a: [0.0, face + a],
                        b: [br, face + a],
                    },
                    PSeg::Line {
                        a: [br, face + a],
                        b: [br, face + b],
                    },
                    PSeg::Line {
                        a: [br, face + b],
                        b: [0.0, face + b],
                    },
                ],
                Part::NonCutting,
            )
            .map_err(bad)?,
        );
    }
    Ok(Shape { bodies })
}

/// The probe as a tool in the spindle (T `t`): `probe: true`, its stand-in cutting record says it
/// cuts nothing (no flutes, no feeds).
pub fn tool_use(id: &str, t: u32, g: &ProbeGeom, note: String) -> Result<ToolUse, String> {
    let shape = shape(g)?;
    let top = shape.top();
    Ok(ToolUse {
        t,
        tool: Tool {
            id: id.to_string(),
            kind: Kind::Ball,
            dc: g.ball,
            re: 0.5 * g.ball,
            angle: 0.0,
            tip_d: 0.0,
            tp: 0.0,
            nof: 0,
            lcf: 0.5 * g.ball + g.length,
            dcon: g.stem,
            oal: top,
            hand: String::new(),
            material: String::new(),
            vendor: String::new(),
            product_id: String::new(),
            source: "a touch probe (forge-cam-lang probe()): cuts nothing".into(),
            taken: String::new(),
            note,
            presets: Vec::new(),
        },
        holder: None,
        stickout: top,
        shape,
        probe: true,
    })
}

fn add(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] + k * b[0], a[1] + k * b[1], a[2] + k * b[2]]
}

/// The tip's nominal contact for a surface point `w` with outward normal `n`.
pub fn contact(w: [f64; 3], n: [f64; 3], r: f64) -> [f64; 3] {
    [w[0] + r * n[0], w[1] + r * n[1], w[2] + r * n[2] - r]
}

/// What a probing op asks for.
#[derive(Clone, Debug)]
pub struct Plan {
    pub kind: String,
    /// surface: the point the ball's tip touches; bore/boss/web/pocket: the feature's centre at
    /// the height the ball's centre probes at; corner: the corner's point at that height.
    pub at: [f64; 3],
    /// bore/boss: diameter; web/pocket: width.
    pub size: f64,
    /// web/pocket: the axis across it (0 x, 1 y).
    pub axis: usize,
    /// corner: its free side's signs (±1, ±1).
    pub corner: [f64; 2],
    /// How far before contact each probe move starts (mm).
    pub approach: f64,
    pub over: f64,
    pub entry: f64,
    pub feed: f64,
    pub link_feed: f64,
    /// The setup's clear height (rapids) and the top the probe starts down from.
    pub clear: f64,
    pub top: f64,
}

/// The probed points (surface point, outward normal) of a plan.
/// (surface point, outward normal).
type Probed = Vec<([f64; 3], [f64; 3])>;

fn points(p: &Plan) -> Result<Probed, String> {
    let a = p.at;
    let h = 0.5 * p.size;
    let ax = |k: usize, s: f64| {
        let mut v = [0.0; 3];
        v[k] = s;
        v
    };
    Ok(match p.kind.as_str() {
        "surface" => vec![(a, [0.0, 0.0, 1.0])],
        "bore" | "pocket" | "boss" | "web" => {
            if h.is_nan() || h <= 0.0 {
                return Err(format!(
                    "{}: size, the {} (mm), > 0",
                    p.kind,
                    if matches!(p.kind.as_str(), "bore" | "boss") {
                        "diameter"
                    } else {
                        "width"
                    }
                ));
            }
            let axes: Vec<usize> = if matches!(p.kind.as_str(), "bore" | "boss") {
                vec![0, 1]
            } else {
                vec![p.axis]
            };
            let inside = matches!(p.kind.as_str(), "bore" | "pocket");
            let mut v = vec![];
            for k in axes {
                for s in [1.0, -1.0] {
                    let w = add(a, ax(k, s), h);
                    // a bore's wall faces its centre; a boss's faces out
                    let n = if inside { ax(k, -s) } else { ax(k, s) };
                    v.push((w, n));
                }
            }
            v
        }
        "corner" => {
            let [sx, sy] = p.corner;
            if sx.abs() != 1.0 || sy.abs() != 1.0 {
                return Err("corner: side, the corner's free side as (±1, ±1)".into());
            }
            // the two faces meeting at the corner, probed a little in from it, from the free side
            let inset = p.approach.max(1.0);
            vec![
                ([a[0], a[1] - sy * inset, a[2]], [sx, 0.0, 0.0]),
                ([a[0] - sx * inset, a[1], a[2]], [0.0, sy, 0.0]),
            ]
        }
        k => {
            return Err(format!(
                "kind: `{k}` is not one of surface, bore, boss, web, pocket, corner"
            ));
        }
    })
}

/// The probing path: for each point, from above at the clear height down (a protected feed
/// move) to `approach` off the surface, the probe move, back to the approach point; up between
/// points that need it, and at the end.
pub fn path(p: &Plan, tool: usize, ball: f64) -> Result<(OpPath, Vec<ProbePoint>), String> {
    let r = 0.5 * ball;
    if [p.over, p.entry, p.approach, p.feed]
        .iter()
        .any(|x| x.is_nan() || *x <= 0.0)
        || p.approach <= p.entry
    {
        return Err("probing: over > 0, entry > 0, approach > entry and feed > 0".into());
    }
    let pts = points(p)?;
    let mut moves = vec![];
    let mut probes = vec![];
    // a bore or pocket is probed from its centre without leaving it; anything else from outside
    let from_centre = matches!(p.kind.as_str(), "bore" | "pocket");
    let start = [p.at[0], p.at[1], p.clear];
    let mut at = start;
    let up = |at: [f64; 3]| [at[0], at[1], p.clear];
    if from_centre {
        let c = contact(p.at, [0.0, 0.0, 1.0], r);
        let c = [p.at[0], p.at[1], c[2]];
        moves.push(PMove::Line {
            to: [p.at[0], p.at[1], p.top],
            f: p.link_feed,
            phase: Phase::Link,
        });
        moves.push(PMove::Line {
            to: c,
            f: p.link_feed,
            phase: Phase::Link,
        });
        at = c;
    }
    for (w, n) in pts {
        let to = contact(w, n, r);
        let dir = [-n[0], -n[1], -n[2]];
        let a = add(to, n, p.approach);
        if !from_centre {
            // over the approach point at the clear height, then down to it
            if at != up(a) {
                if at[2] != p.clear {
                    moves.push(PMove::Rapid { to: up(at) });
                }
                moves.push(PMove::Rapid { to: up(a) });
            }
            moves.push(PMove::Line {
                to: a,
                f: p.link_feed,
                phase: Phase::Link,
            });
        } else {
            moves.push(PMove::Line {
                to: a,
                f: p.link_feed,
                phase: Phase::Link,
            });
        }
        // it backs off along the normal to the approach point
        moves.push(PMove::Probe {
            to,
            f: p.feed,
            over: p.over,
            entry: p.entry,
            back: true,
        });
        at = a;
        probes.push(ProbePoint { at: w, dir });
    }
    if from_centre {
        let c = [p.at[0], p.at[1], at[2]];
        moves.push(PMove::Line {
            to: c,
            f: p.link_feed,
            phase: Phase::Link,
        });
        at = c;
    }
    moves.push(PMove::Rapid { to: up(at) });
    Ok((
        OpPath {
            tool,
            start,
            moves,
            rpm: 0.0,
            coolant: forge_post::Coolant::Off,
        },
        probes,
    ))
}

/// One point probed along its own normal `n` (unit, out of the material): `inspect`.
pub fn path_along(
    p: &Plan,
    tool: usize,
    ball: f64,
    n: [f64; 3],
) -> Result<(OpPath, Vec<ProbePoint>), String> {
    if [p.over, p.entry, p.approach, p.feed]
        .iter()
        .any(|x| x.is_nan() || *x <= 0.0)
        || p.approach <= p.entry
    {
        return Err("probing: over > 0, entry > 0, approach > entry and feed > 0".into());
    }
    let to = contact(p.at, n, 0.5 * ball);
    let a = add(to, n, p.approach);
    let start = [a[0], a[1], p.clear];
    Ok((
        OpPath {
            tool,
            start,
            moves: vec![
                PMove::Line {
                    to: a,
                    f: p.link_feed,
                    phase: Phase::Link,
                },
                PMove::Probe {
                    to,
                    f: p.feed,
                    over: p.over,
                    entry: p.entry,
                    back: true,
                },
                PMove::Rapid { to: start },
            ],
            rpm: 0.0,
            coolant: forge_post::Coolant::Off,
        },
        vec![ProbePoint {
            at: p.at,
            dir: [-n[0], -n[1], -n[2]],
        }],
    ))
}
