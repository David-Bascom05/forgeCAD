//! Sheets swept along a plane's normal (`extrude_sheet`): the extrude's builder driven through a
//! sink of its own, in a crate of its own, so the solid extrude is compiled as it was.

use forge_geom::{Curve, CurveGeom, Line, Surface};
use forge_math::{Frame, Interval, Point3, tol};
use forge_ops::extrude::{Builder, ExtrudeError, Sink};
use forge_ops::profile::{Profile, Segment};
use forge_topo::{Body, EdgeId, FaceId, Name, ShellId, VertexId};

/// A sheet (a surface body, `Body::sheet`): each chain swept along the plane's normal by `distance`
/// (negative goes the other way), with no caps. A chain is segments end to end, open or closed (a
/// loop): one face per segment, named as `extrude` names a wall ("side" + its id); its rims are named
/// with "start" and "end", as the caps would be. The sheet's front, its faces' outward side, is the
/// right of each chain walking it with the plane's normal toward you: outside a counter-clockwise
/// loop, as for a solid (a loop drawn clockwise is turned). Chains must not cross themselves or each
/// other (`SelfIntersecting`, the later chain's index), nor two segments share an id.
pub fn extrude_sheet(
    plane: &Frame,
    chains: &[Vec<(Segment, u64)>],
    distance: f64,
    feature: u64,
) -> Result<Body, ExtrudeError> {
    if !(distance.abs() > tol::MIN_LENGTH && distance.abs() < f64::INFINITY) {
        return Err(ExtrudeError::TooShort);
    }
    let chains = crate::chain::sheet_chains(chains)?;
    // backwards: forwards from the flipped plane, each chain mirrored to match and turned, so it
    // runs the same way in the world and its right is the same side
    let (plane, chains) = if distance < 0.0 {
        let flipped: Vec<(Vec<(Segment, u64)>, bool)> = chains
            .iter()
            .map(|(c, closed)| {
                let m = forge_ops::profile::ProfileLoop::new(
                    c.iter().map(|(s, id)| (s.mirrored(), *id)).collect(),
                );
                (m.reversed().segments, *closed)
            })
            .collect();
        (plane.flipped(), flipped)
    } else {
        (*plane, chains)
    };
    let profile = Profile {
        plane,
        loops: Vec::new(),
    };
    let mut body = Body::new();
    body.add_shell();
    // (a sink of its own: the solid's builder is compiled as it was)
    let mut b = Builder::new(&profile, distance.abs(), Name::feature(feature), SheetSink(body));
    for (segs, closed) in &chains {
        if let [(Segment::Circle { center, radius, ccw }, id)] = segs[..] {
            b.circle_wall(center, radius, ccw, id);
        } else {
            sheet_walls(&mut b, segs, *closed);
        }
    }
    let mut body = b.body.0;
    body.mark_sheet();
    if chains.len() > 1 {
        body.rebuild_shells();
    }
    Ok(body)
}

/// A sheet's body, as a sink of its own.
struct SheetSink(Body);

impl Sink for SheetSink {
    fn add_vertex(&mut self, p: Point3, name: Name) -> VertexId {
        self.0.add_vertex(p, name)
    }
    fn add_edge(
        &mut self,
        curve: Curve,
        range: Interval,
        start: VertexId,
        end: VertexId,
        name: Name,
    ) -> EdgeId {
        self.0.add_edge(curve, range, start, end, name)
    }
    fn add_face(&mut self, surface: Surface, same_sense: bool, name: Name) -> FaceId {
        self.0.add_face(ShellId(0), surface, same_sense, name)
    }
    fn add_loop(&mut self, face: FaceId, uses: &[(EdgeId, bool)]) {
        self.0.add_loop(face, uses);
    }
}

/// A sheet's walls for a chain of lines, arcs and splines, open or closed: as `chain_walls`, with
/// no caps, and an open chain's two end uprights each used once (its boundary).
fn sheet_walls<B: Sink>(b: &mut Builder<'_, B>, segs: &[(Segment, u64)], closed: bool) {
    let this = b;
    {
        if closed {
            this.chain_walls(segs);
            return;
        }
        let n = segs.len();
        let names: Vec<Name> = segs.iter().map(|(_, id)| this.side_name(*id)).collect();
        // vertex i is where segment i starts; vertex n where the last one ends. The ends are named
        // by the one wall there and which end of the chain they are.
        let at = |i: usize| {
            if i < n {
                segs[i].0.start()
            } else {
                segs[n - 1].0.end()
            }
        };
        let joint_name = |i: usize| match i {
            0 => names[0].with_str("chain-start"),
            _ if i == n => names[n - 1].with_str("chain-end"),
            _ => Name::NONE.with_set(&[names[i - 1], names[i]]),
        };
        let mut verts: Vec<[VertexId; 2]> = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let mut pair = [VertexId(0); 2];
            for (k, level) in [0.0, this.d].into_iter().enumerate() {
                let name = Name::NONE.with_set(&[this.cap_name(k == 1), joint_name(i)]);
                pair[k] = this.body.add_vertex(this.at(at(i), level), name);
            }
            verts.push(pair);
        }
        let mut uprights: Vec<EdgeId> = Vec::with_capacity(n + 1);
        for (i, pair) in verts.iter().enumerate() {
            let line = Line::new(this.at(at(i), 0.0), this.up());
            uprights.push(this.body.add_edge(
                Curve::Line(line),
                Interval::new(0.0, this.d),
                pair[0],
                pair[1],
                joint_name(i),
            ));
        }
        for (i, (s, _)) in segs.iter().enumerate() {
            let (surface, same_sense, curve0, range, forward) = this.segment_geometry(s);
            let mut edges = [EdgeId(0); 2];
            for (k, level) in [0.0, this.d].into_iter().enumerate() {
                let (a, b) = (verts[i][k], verts[i + 1][k]);
                let (start, end) = if forward { (a, b) } else { (b, a) };
                let curve = curve0.transformed(&Frame::WORLD.with_origin(Point3::ORIGIN + this.up() * level));
                let name = Name::NONE.with_set(&[this.cap_name(k == 1), names[i]]);
                edges[k] = this.body.add_edge(curve, range, start, end, name);
            }
            let face = this.body.add_face(surface, same_sense, names[i]);
            this.body.add_loop(
                face,
                &[
                    (edges[0], forward),
                    (uprights[i + 1], true),
                    (edges[1], !forward),
                    (uprights[i], false),
                ],
            );
        }
    }
}
