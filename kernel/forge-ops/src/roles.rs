//! The names a feature's role faces carry, from its root name (`Name::feature(id)`): an extrude's
//! caps and walls, a revolve's walls. The builders name faces through these, so a reference by
//! role (a language's `boss.end`, `arm.side(l1)`) finds the face by its name, never by a hash
//! read off a built part. A face a later boolean splits keeps the whole's name in its pieces
//! (`Name::is_piece_of`); one the finish joins is listed in `Body::joins.aliases`.

use forge_topo::Name;

/// The cap an extrude (straight or tapered) starts from, at its sketch.
pub fn start(root: Name) -> Name {
    root.with_str("start")
}

/// The cap at its far end.
pub fn end(root: Name) -> Name {
    root.with_str("end")
}

/// The wall profile segment `segment` makes.
pub fn side(root: Name, segment: u64) -> Name {
    root.with_str("side").with(segment)
}

/// A revolve's wall from `segment` over its `span`th part of the turn (a whole turn is built as
/// two halves, and the second is an alias of the first).
pub fn revolve_side(root: Name, segment: u64, span: u64) -> Name {
    side(root, segment).with(span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
    use forge_math::{Frame, Point2, Vec2};

    fn square(ids: [u64; 4]) -> Profile {
        let p = [(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)].map(|(x, y)| Point2::new(x, y));
        Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop {
                segments: (0..4)
                    .map(|k| {
                        (
                            Segment::Line {
                                a: p[k],
                                b: p[(k + 1) % 4],
                            },
                            ids[k],
                        )
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn an_extrude_names_its_caps_and_walls_by_role() {
        let ids = [11, 22, 33, 44];
        let b = extrude(&square(ids), 5.0, 7).expect("extrudes");
        let root = Name::feature(7);
        for n in [start(root), end(root)]
            .into_iter()
            .chain(ids.map(|s| side(root, s)))
        {
            assert!(b.face_by_name(n).is_some(), "{n:?} names a face");
        }
        assert_eq!(b.face_ids().count(), 6);
        // the end cap is the one at the far side
        let top = b.face_by_name(end(root)).expect("end");
        assert!(b.face_box(top).z.lo > 4.99);
    }

    #[test]
    fn a_tapered_extrude_names_them_alike() {
        let ids = [1, 2, 3, 4];
        let bodies = crate::tapered(&square(ids), 4.0, 0.05, 3).expect("tapers");
        let [b] = &bodies[..] else {
            panic!("one solid, a small taper")
        };
        let root = Name::feature(3);
        assert!(b.face_by_name(start(root)).is_some());
        assert!(b.face_by_name(end(root)).is_some());
        for s in ids {
            assert!(b.face_by_name(side(root, s)).is_some());
        }
    }

    #[test]
    fn a_revolve_names_its_walls_per_segment_and_span() {
        let ids = [5, 6, 7, 8];
        let axis = Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        };
        let b = revolve(&square(ids), axis, std::f64::consts::FRAC_PI_2, 9).expect("revolves");
        let root = Name::feature(9);
        // the wall on the axis makes no face; the other three do
        for s in [5, 6, 7] {
            let n = revolve_side(root, s, 0);
            assert!(b.face_by_name(n).is_some(), "segment {s}");
        }
    }
}
