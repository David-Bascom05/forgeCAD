//! Section views: a solid cut by the prism swept from a cutting line along the view direction
//! (one segment a full section, more an offset or half section), and the faces the cut made,
//! for hatching.

use crate::view::View;
use forge_bool::{Op, boolean};
use forge_geom::{CurveGeom, Surface};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::{Body, FaceId};

/// The cut: the cutting line (in `view`'s plane coordinates) and the side of it that is taken
/// away (the side the section is seen from).
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub view: View,
    pub line: Vec<Point2>,
    /// A unit direction in the view plane: the side taken away.
    pub away: Vec2,
}

/// The region taken away, as a polygon in the view plane: the cutting line run on far past both
/// ends, closed round the side taken away. `reach` is beyond anything drawn.
fn region(cut: &Cut, reach: f64) -> Vec<Point2> {
    let n = cut.line.len();
    let d0 = (cut.line[0] - cut.line[1]).normalize().unwrap_or(Vec2::X);
    let d1 = (cut.line[n - 1] - cut.line[n - 2]).normalize().unwrap_or(Vec2::X);
    let mut out = vec![cut.line[0] + d0 * reach];
    out.extend(cut.line.iter().copied());
    out.push(cut.line[n - 1] + d1 * reach);
    out.push(cut.line[n - 1] + d1 * reach + cut.away * (3.0 * reach));
    out.push(cut.line[0] + d0 * reach + cut.away * (3.0 * reach));
    // counter-clockwise, as a profile's outer loop
    let area: f64 = (0..out.len())
        .map(|k| out[k].to_vec().cross(out[(k + 1) % out.len()].to_vec()))
        .sum();
    if area < 0.0 {
        out.reverse();
    }
    out
}

/// `body` (placed by `pose`) with the cut's side taken away; the error is the boolean's, in words.
pub fn cut_body(body: &Body, pose: &Frame, cut: &Cut, reach: f64, feature: u64) -> Result<Body, String> {
    let v = &cut.view;
    let plane_w = Frame::from_axes(
        Point3::ORIGIN + v.toward.get() * -reach,
        v.right.get(),
        v.up.get(),
        v.toward.get(),
    )
    .ok_or("the view's axes are not a frame")?;
    let plane = pose.inverse().compose(&plane_w);
    let poly = region(cut, reach);
    let segs: Vec<(Segment, u64)> = (0..poly.len())
        .map(|k| {
            (
                Segment::Line {
                    a: poly[k],
                    b: poly[(k + 1) % poly.len()],
                },
                k as u64,
            )
        })
        .collect();
    let prism = extrude(
        &Profile {
            plane,
            loops: vec![ProfileLoop::new(segs)],
        },
        2.0 * reach,
        feature,
    )
    .map_err(|e| format!("the cutting prism: {e:?}"))?;
    boolean(body, &prism, Op::Subtract, feature ^ 0x5ec7).map_err(|e| format!("{e:?}"))
}

/// The faces of `body` (placed by `pose`) lying on the cutting planes: what is hatched.
pub fn cut_faces(body: &Body, pose: &Frame, cut: &Cut) -> Vec<FaceId> {
    let v = &cut.view;
    let planes: Vec<(Vec3, f64)> = cut
        .line
        .windows(2)
        .filter_map(|w| {
            let n2 = (w[1] - w[0]).perp().normalize()?;
            let n = v.unproject_vec(n2);
            let at = Point3::ORIGIN + v.unproject_vec(w[0].to_vec());
            Some((n, n.dot(at.to_vec())))
        })
        .collect();
    body.face_ids()
        .filter(|&f| match &body.face(f).surface {
            Surface::Plane(p) => {
                let n = pose.vec_to_world(p.frame.z().get());
                let o = pose.point_to_world(p.frame.origin);
                planes
                    .iter()
                    .any(|(m, d)| n.cross(*m).length() < 1e-9 && (m.dot(o.to_vec()) - d).abs() < 1e-6)
            }
            _ => false,
        })
        .collect()
}

/// A face's loops as polygons on the view, within `chord`.
pub fn face_loops(body: &Body, pose: &Frame, f: FaceId, view: &View, chord: f64) -> Vec<Vec<Point2>> {
    body.face(f)
        .loops
        .iter()
        .map(|&l| {
            let mut pts = Vec::new();
            for c in body.lp(l).coedges() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                let r = e.range;
                let n = match &e.curve {
                    forge_geom::Curve::Line(_) => 1,
                    _ => {
                        let len = e.curve.eval(r.lo).distance(e.curve.eval(r.hi)).max(1e-9);
                        let span = e.curve.bbox(r).diagonal().max(len);
                        ((span / chord).sqrt().ceil() as usize).clamp(8, 256)
                    }
                };
                for k in 0..n {
                    let t = k as f64 / n as f64;
                    let t = if co.forward {
                        r.lo + t * r.width()
                    } else {
                        r.hi - t * r.width()
                    };
                    pts.push(view.project(pose.point_to_world(e.curve.eval(t))));
                }
            }
            pts
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_ops::Profile;

    #[test]
    fn a_block_cut_in_half() {
        let plane = Frame::WORLD;
        let loops = vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)];
        let b = extrude(&Profile { plane, loops }, 5.0, 1).unwrap();
        // front view, cut along x = 4, the right side taken away
        let view = View::named("front").unwrap();
        let cut = Cut {
            view,
            line: vec![Point2::new(4.0, -10.0), Point2::new(4.0, 20.0)],
            away: Vec2::X,
        };
        let c = cut_body(&b, &Frame::WORLD, &cut, 100.0, 7).unwrap();
        let m = forge_topo::mass::mass(&c).unwrap();
        assert!((m.volume - 4.0 * 20.0 * 5.0).abs() < 1e-6, "{}", m.volume);
        let faces = cut_faces(&c, &Frame::WORLD, &cut);
        assert_eq!(faces.len(), 1);
        let right = View::named("right").unwrap();
        let loops = face_loops(&c, &Frame::WORLD, faces[0], &right, 0.01);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
    }
}
