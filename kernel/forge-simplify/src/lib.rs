//! Recognize removable geometric details on a finished solid. The result is a set of
//! named faces for the direct-edit healer, independent of the source's modeling history.
use forge_geom::{Cylinder, Surface};
use forge_math::tol;
use forge_topo::{Body, EdgeId, FaceId, LoopId, Name};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Bore {
    pub diameter: f64,
    /// Every face along the passage, including pieces on either side of an imported seam.
    pub walls: Vec<Name>,
    /// The supporting faces on either end. These are retrimmed by a fill.
    pub ends: [Name; 2],
}

fn wall(body: &Body, id: FaceId) -> Option<&Cylinder> {
    let face = body.face(id);
    match &face.surface {
        Surface::Cylinder(cylinder) if !face.same_sense => Some(cylinder),
        _ => None,
    }
}

fn coaxial(a: &Cylinder, b: &Cylinder) -> bool {
    a.frame.z().cross(b.frame.z()).length() <= tol::ANGULAR
        && a.frame.z().cross(b.frame.origin - a.frame.origin).length() <= tol::LINEAR
        && (a.radius - b.radius).abs() <= tol::LINEAR
}

fn edge_faces(body: &Body, edge: EdgeId) -> impl Iterator<Item = FaceId> + '_ {
    body.edge(edge).coedges.iter().map(|&c| body.coedge_face(c))
}

fn rim(body: &Body, edge: EdgeId, face: FaceId, cylinder: &Cylinder) -> Option<LoopId> {
    let support = body.face(face);
    let Surface::Plane(plane) = &support.surface else {
        return None;
    };
    if plane.frame.z().cross(cylinder.frame.z()).length() > tol::ANGULAR {
        return None;
    }
    support
        .loops
        .iter()
        .skip(1)
        .copied()
        .find(|&l| body.lp(l).coedges().any(|c| body.coedge(c).edge == edge))
}

/// Find bores with exactly two complete, distinct openings in planar faces.
/// Counterbores, blind holes and intersecting passages are left for more specific recognizers.
pub fn through_bores(body: &Body) -> Vec<Bore> {
    let mut visited = BTreeSet::new();
    let mut bores = Vec::new();
    for id in body.face_ids() {
        let Some(cylinder) = wall(body, id) else { continue };
        if !visited.insert(id) {
            continue;
        }
        let mut group = BTreeSet::from([id]);
        let mut todo = vec![id];
        let mut rims = BTreeSet::new();
        let mut complete = true;
        while let Some(current) = todo.pop() {
            for edge in body
                .face(current)
                .loops
                .iter()
                .flat_map(|&l| body.lp(l).coedges().map(|c| body.coedge(c).edge))
            {
                let mut adjacent = edge_faces(body, edge);
                let Some(other) = adjacent.find(|&f| f != current) else {
                    // A periodic seam can be used twice by the same face.
                    complete &= body.edge(edge).coedges.len() == 2;
                    continue;
                };
                if wall(body, other).is_some_and(|next| coaxial(cylinder, next)) {
                    if group.insert(other) {
                        visited.insert(other);
                        todo.push(other);
                    }
                } else if let Some(l) = rim(body, edge, other, cylinder) {
                    rims.insert((other, l));
                } else {
                    complete = false;
                }
            }
        }
        if !complete || rims.len() != 2 {
            continue;
        }
        let mut ends = rims.iter();
        let (a, _) = *ends.next().expect("two bore rims");
        let (b, _) = *ends.next().expect("two bore rims");
        if a == b
            || !rims.iter().all(|&(face, l)| {
                body.lp(l)
                    .coedges()
                    .all(|c| edge_faces(body, body.coedge(c).edge).any(|f| f != face && group.contains(&f)))
            })
        {
            continue;
        }
        let mut walls: Vec<_> = group.iter().map(|&f| body.face(f).name).collect();
        walls.sort_unstable();
        bores.push(Bore {
            diameter: 2.0 * cylinder.radius,
            walls,
            ends: [body.face(a).name, body.face(b).name],
        });
    }
    bores.sort_by(|a, b| a.walls.cmp(&b.walls));
    bores
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanarDetail {
    pub pocket: bool,
    /// Diagonal of the detail's box in its support frame.
    pub diagonal: f64,
    pub faces: Vec<Name>,
    pub support: Name,
}

/// Find straight-edged planar depressions and protrusions whose whole boundary is one
/// inner loop of a retained planar face. A through cut or a feature touching the outside
/// of the support has additional boundary edges and is left alone.
pub fn planar_details(body: &Body) -> Vec<PlanarDetail> {
    let mut details = Vec::new();
    for id in body.face_ids() {
        let support = body.face(id);
        let Surface::Plane(plane) = &support.surface else {
            continue;
        };
        for &inner in support.loops.iter().skip(1) {
            let Some((faces, bounds)) = bounded_planar_faces(body, id, inner, &plane.frame) else {
                continue;
            };
            if bounds.z.lo < -tol::LINEAR && bounds.z.hi > tol::LINEAR {
                continue;
            }
            let below = bounds.z.lo < -tol::LINEAR;
            let above = bounds.z.hi > tol::LINEAR;
            if !below && !above {
                continue;
            }
            let pocket = below == support.same_sense;
            let size = forge_math::Vec3::new(bounds.x.width(), bounds.y.width(), bounds.z.width());
            details.push(PlanarDetail {
                pocket,
                diagonal: size.length(),
                faces,
                support: support.name,
            });
        }
    }
    details.sort_by(|a, b| a.faces.cmp(&b.faces));
    details
}

fn bounded_planar_faces(
    body: &Body,
    support: FaceId,
    inner: LoopId,
    frame: &forge_math::Frame,
) -> Option<(Vec<Name>, forge_math::interval::Box3)> {
    use forge_geom::Curve;
    use forge_math::interval::Box3;
    let rim: BTreeSet<_> = body.lp(inner).coedges().map(|c| body.coedge(c).edge).collect();
    let mut unseen = rim.clone();
    let mut faces = BTreeSet::new();
    let mut todo: Vec<_> = rim
        .iter()
        .filter_map(|&edge| edge_faces(body, edge).find(|&face| face != support))
        .collect();
    let mut bounds = Box3::EMPTY;
    while let Some(id) = todo.pop() {
        if !faces.insert(id) {
            continue;
        }
        let face = body.face(id);
        if !matches!(face.surface, Surface::Plane(_)) {
            return None;
        }
        for &loop_id in &face.loops {
            for c in body.lp(loop_id).coedges() {
                let edge = body.coedge(c).edge;
                let e = body.edge(edge);
                if !matches!(e.curve, Curve::Line(_)) {
                    return None;
                }
                bounds.include(frame.point_to_local(body.vertex(e.start).point));
                let next = edge_faces(body, edge).find(|&other| other != id)?;
                if next == support {
                    if !rim.contains(&edge) {
                        return None;
                    }
                    unseen.remove(&edge);
                } else if !faces.contains(&next) {
                    todo.push(next);
                }
            }
        }
    }
    if !unseen.is_empty() || bounds.is_empty() {
        return None;
    }
    let mut names: Vec<_> = faces.iter().map(|&f| body.face(f).name).collect();
    names.sort_unstable();
    Some((names, bounds))
}
