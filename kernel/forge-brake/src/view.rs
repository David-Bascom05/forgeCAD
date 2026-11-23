//! A step drawn at one instant, for a viewer: the tools' sections placed in the press, and the
//! part's pieces seen along the bed (their outlines through the motion, projected on x, z). For
//! the eye only: the checks never read this.
use crate::geo::Seg;
use crate::model::Model;
use crate::sim::{Phase, Step, StepCtx};
use crate::tooling::Tooling;
use forge_math::Point2;

/// One closed outline in the press section (x, z), mm.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    /// `punch`, `die`, `ram`, `bed`, `finger`, or `part`.
    pub what: &'static str,
    /// The tool's name as the checks give it (`punch:ID/body`..), or the piece's index.
    pub name: String,
    pub points: Vec<[f64; 2]>,
}

/// The picture at one instant of a step, and where in its motion that is.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub phase: Phase,
    /// The bend's turn, and the punch tip's height.
    pub phi: f64,
    pub tip: f64,
    pub outlines: Vec<Outline>,
}

fn sample(s: &Seg, n: usize) -> Vec<Point2> {
    match s {
        Seg::Line { a, b } => vec![*a, *b],
        Seg::Arc { .. } => (0..=n).map(|i| s.point(i as f64 / n as f64)).collect(),
    }
}

/// Step `st` (after the bends `done`) at `t` in [0, 1]: the first fifth the punch coming down,
/// then the bend to its overbent turn, the last fifth the punch going back up.
pub fn frame(m: &Model, tl: &Tooling, done: &[bool], st: &Step, t: f64) -> Frame {
    let ctx = StepCtx::new(m, tl, done, st.bend, st.setup, st.overbend);
    let t = t.clamp(0.0, 1.0);
    let (phase, phi, tip) = if t < 0.2 {
        let h = ctx.z_tdc + (ctx.ztip(0.0) - ctx.z_tdc) * (t / 0.2);
        (Phase::Approach, 0.0, h)
    } else if t <= 0.8 {
        // up to the overbend and sprung back over the last tenth of the bend
        let f = (t - 0.2) / 0.6;
        let phi = if f < 0.9 {
            ctx.phimax * f / 0.9
        } else {
            ctx.phimax + (ctx.theta - ctx.phimax) * (f - 0.9) / 0.1
        };
        (Phase::Bend, phi, ctx.ztip(phi))
    } else {
        let f = (t - 0.8) / 0.2;
        let h0 = ctx.ztip(ctx.theta);
        (Phase::Return, ctx.theta, h0 + (ctx.z_tdc - h0) * f)
    };
    let mut outlines = Vec::new();
    let shift = |r: &crate::geo::Region, dz: f64, what: &'static str, name: String| Outline {
        what,
        name,
        points: r
            .loops
            .iter()
            .flatten()
            .flat_map(|s| sample(s, 12))
            .map(|p| [p.x, p.y + dz])
            .collect(),
    };
    outlines.push(shift(
        &ctx.punch.tip_section(),
        tip,
        "punch",
        format!("punch:{}/tip", ctx.punch.id),
    ));
    outlines.push(shift(
        &ctx.punch.body_section(),
        tip,
        "punch",
        format!("punch:{}/body", ctx.punch.id),
    ));
    let pr = &tl.press;
    let rect = |x0: f64, z0: f64, x1: f64, z1: f64| vec![[x0, z0], [x1, z0], [x1, z1], [x0, z1]];
    let ph = ctx.punch.height;
    outlines.push(Outline {
        what: "ram",
        name: "ram".into(),
        points: rect(-pr.ram.front, tip + ph, pr.ram.back, tip + ph + 80.0),
    });
    outlines.push(shift(
        &ctx.die.section(),
        0.0,
        "die",
        format!("die:{}", ctx.die.id),
    ));
    let dh = ctx.die.height;
    outlines.push(Outline {
        what: "bed",
        name: "bed".into(),
        points: rect(-pr.bed.front, -dh - 80.0, pr.bed.back, -dh),
    });
    if let Some(g) = &st.gauge {
        let bg = &pr.backgauge;
        let gap = if phase == Phase::Approach {
            2.0 * tl.clearance
        } else {
            bg.retract.max(2.0 * tl.clearance)
        };
        outlines.push(Outline {
            what: "finger",
            name: "backgauge:finger1".into(),
            points: rect(
                g.x + gap,
                g.r,
                g.x + gap + bg.finger.depth,
                g.r + bg.finger.height,
            ),
        });
    }
    // the part: each piece's boundary on both skins, at this turn, seen along the bed
    let phase_phi = if phase == Phase::Approach { 0.0 } else { phi };
    for (i, pc) in ctx.pieces.iter().enumerate() {
        for w in [0.0, -ctx.t] {
            let pts: Vec<[f64; 2]> = pc
                .region()
                .bound
                .iter()
                .flat_map(|s| sample(s, 8))
                .map(|p| {
                    let q = ctx.side_point(pc, p.x, p.y, w, phase_phi);
                    let r = ctx.rel(q, pc.side, phase_phi);
                    [r.x, r.z + ctx.ztip(phase_phi)]
                })
                .collect();
            outlines.push(Outline {
                what: "part",
                name: format!("{i}"),
                points: pts,
            });
        }
    }
    Frame {
        phase,
        phi,
        tip,
        outlines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, library, plan};
    use forge_math::{Frame as F3, dmath};
    use forge_ops::{ProfileLoop, Segment};
    use forge_sheet::{BendRule, FeatureId, Gauge, Sheet};

    #[test]
    fn an_ell_is_drawn_through_its_bend() {
        let pts = [(0.0, 0.0), (120.0, 0.0), (120.0, 60.0), (0.0, 60.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let g = Gauge {
            thickness: 2.0,
            radius: 2.0,
            rule: BendRule::K(0.4),
        };
        let (mut s, base) = Sheet::face(FeatureId(1), g, F3::WORLD, &[lp]).unwrap();
        s.flange(base, 1, dmath::FRAC_PI_2, 30.0).unwrap();
        let m = Model::new(&s).unwrap();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        let st = &p.steps[0];
        let a = frame(&m, &tl, &[false], st, 0.0);
        let b = frame(&m, &tl, &[false], st, 0.7);
        assert_eq!(a.phase, Phase::Approach);
        assert_eq!(b.phase, Phase::Bend);
        assert!(a.tip > b.tip && b.phi > 0.0);
        // flat on the die at first: every part point between the die's top and t above it
        for o in a.outlines.iter().filter(|o| o.what == "part") {
            for p in &o.points {
                assert!(p[1] > -1e-9 && p[1] < 2.0 + 1e-9, "{p:?}");
            }
        }
    }
}
