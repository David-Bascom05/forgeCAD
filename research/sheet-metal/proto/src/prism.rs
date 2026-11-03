//! Option B: the wall as a flat prism, each cut a taller prism through it, forge-bool, and the
//! loops read back off the top face.
use crate::region::{Component, Piece, Region};
use forge_bool::{BoolError, Op, boolean_into, nary};
use forge_geom::spline::spline_of;
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::extrude::extrude;
use forge_ops::{ExtrudeError, Profile, ProfileLoop};
use forge_topo::{Body, FaceId};
use std::sync::Arc;

pub const T: f64 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub enum PrismError {
    Wall(ExtrudeError),
    Cut {
        index: usize,
        why: ExtrudeError,
    },
    Bool {
        index: usize,
        why: BoolError,
    },
    /// An edge of the top face that is not a line, circle or spline.
    Curve {
        face: FaceId,
    },
}

fn up(z: f64) -> Frame {
    Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z))
}

fn prism(l: &[ProfileLoop], z: f64, h: f64) -> Result<Body, ExtrudeError> {
    extrude(
        &Profile {
            plane: up(z),
            loops: l.to_vec(),
        },
        h,
        1,
    )
}

/// One cut after another.
pub fn subtract_seq(wall: &[ProfileLoop], cuts: &[ProfileLoop]) -> Result<Body, PrismError> {
    let mut body = prism(wall, 0.0, T).map_err(PrismError::Wall)?;
    for (i, c) in cuts.iter().enumerate() {
        let tool =
            prism(std::slice::from_ref(c), -1.0, T + 2.0).map_err(|why| PrismError::Cut { index: i, why })?;
        boolean_into(&mut body, &tool, Op::Subtract, i as u64 + 2)
            .map_err(|why| PrismError::Bool { index: i, why })?;
    }
    Ok(body.into_compact())
}

/// All cuts at once, one thread.
pub fn subtract_nary(wall: &[ProfileLoop], cuts: &[ProfileLoop]) -> Result<Body, PrismError> {
    let body = prism(wall, 0.0, T).map_err(PrismError::Wall)?;
    let mut tools = Vec::with_capacity(cuts.len());
    for (i, c) in cuts.iter().enumerate() {
        tools.push(
            prism(std::slice::from_ref(c), -1.0, T + 2.0).map_err(|why| PrismError::Cut { index: i, why })?,
        );
    }
    let mut refs: Vec<&Body> = vec![&body];
    refs.extend(tools.iter());
    let ops = vec![Op::Subtract; tools.len()];
    nary(&refs, &ops, 1).map_err(|why| PrismError::Bool { index: 0, why })
}

/// The top face's loops as pieces, outer first: what a sheet part would keep.
pub fn read_back(body: &Body) -> Result<Region, PrismError> {
    let mut parts = Vec::new();
    for f in body.face_ids() {
        let face = body.face(f);
        let Surface::Plane(pl) = &face.surface else {
            continue;
        };
        let z = pl.frame.z().get();
        let n = if face.same_sense { z.z } else { -z.z };
        if (pl.frame.origin.z - T).abs() > 1e-9 || n < 0.5 {
            continue;
        }
        let mut comp = Component {
            outer: Vec::new(),
            holes: Vec::new(),
        };
        for &lp in &face.loops {
            let mut ps = Vec::new();
            for c in body.lp(lp).coedges() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                // a closed edge: two pieces, as a profile's arcs stop short of a whole turn
                let cuts = if matches!(e.curve, Curve::Circle(_)) && e.range.width() >= dmath::TAU - 1e-9 {
                    vec![(e.range.lo, e.range.mid()), (e.range.mid(), e.range.hi)]
                } else {
                    vec![(e.range.lo, e.range.hi)]
                };
                let mut here: Vec<Piece> = Vec::new();
                for (lo, hi) in cuts {
                    here.push(piece(&e.curve, lo, hi, co.forward).ok_or(PrismError::Curve { face: f })?);
                }
                if !co.forward {
                    here.reverse();
                }
                ps.extend(here);
            }
            let area: f64 = ps.iter().map(Piece::area_term).sum();
            if area > 0.0 {
                comp.outer = ps;
            } else {
                comp.holes.push(ps);
            }
        }
        parts.push(comp);
    }
    Ok(Region {
        parts,
        pinches: Vec::new(),
    })
}

fn piece(c: &Curve, lo: f64, hi: f64, fw: bool) -> Option<Piece> {
    let (t0, t1) = if fw { (lo, hi) } else { (hi, lo) };
    Some(match c {
        Curve::Line(_) => {
            let q = |t: f64| {
                let p = c.eval(t);
                Point2::new(p.x, p.y)
            };
            Piece::Line { a: q(t0), b: q(t1) }
        }
        Curve::Circle(ci) => {
            // in world terms: the frame's z up or down
            let f = &ci.frame;
            let k = f.z().get().z.signum();
            let base = dmath::atan2(f.x().get().y, f.x().get().x);
            Piece::Arc {
                c: Point2::new(f.origin.x, f.origin.y),
                r: ci.radius,
                t0: base + k * t0,
                t1: base + k * t1,
            }
        }
        _ => {
            let s = spline_of(c)?;
            Piece::Spline {
                s: Arc::clone(&s.c),
                t0,
                t1,
            }
        }
    })
}
