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

/// The key a skinned wall (sweep, coil, loft, thread groove) carries for profile segment
/// `segment`: a whole circle is skinned as two halves (`half` 0 and 1), any other segment as one
/// (`half` 0).
pub fn skin_key(segment: u64, half: u64) -> Name {
    Name::NONE.with(segment).with(half)
}

/// A skinned wall from the segment piece `key` (`skin_key`) over stretch `k`.
pub fn skin_side_of(root: Name, key: Name, k: u64) -> Name {
    root.with_str("side").with_name(key).with(k)
}

/// A skinned wall: from `segment`'s `half` over stretch `k` (a sweep's path piece, counting a
/// straight run-on at the start; a loft's span between profiles k and k + 1; a coil's or a
/// thread's one turn-run, 0).
pub fn skin_side(root: Name, segment: u64, half: u64, k: u64) -> Name {
    skin_side_of(root, skin_key(segment, half), k)
}

/// A shell's (or a solid offset's) face moved off the part's face `face`: inside a shell, its
/// inner skin; outward, its outer one.
pub fn offset(root: Name, face: Name) -> Name {
    face.with_name(root).with_str("offset")
}

/// A shell's rim: the wall joining the skins round loop `k` of open face `face`.
pub fn rim(root: Name, face: Name, k: u64) -> Name {
    face.with_name(root).with_str("rim").with(k)
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
    fn circle(id: u64, r: f64, plane: Frame) -> Profile {
        Profile {
            plane,
            loops: vec![ProfileLoop {
                segments: vec![(
                    Segment::Circle {
                        center: Point2::new(0.0, 0.0),
                        radius: r,
                        ccw: true,
                    },
                    id,
                )],
            }],
        }
    }

    #[test]
    fn a_sweep_names_its_walls_per_segment_half_and_path_piece() {
        use crate::{PathSeg, sweep};
        use forge_geom::Circle;
        use forge_math::{Point3, Vec3};
        let p = circle(3, 2.0, Frame::WORLD);
        let bend = Circle {
            frame: Frame::from_axes(Point3::new(20.0, 0.0, 10.0), -Vec3::X, Vec3::Z, Vec3::Y).expect("frame"),
            radius: 20.0,
        };
        let path = [
            PathSeg::Line {
                a: Point3::ORIGIN,
                b: Point3::new(0.0, 0.0, 10.0),
            },
            PathSeg::Arc {
                circle: bend,
                start: 0.0,
                sweep: std::f64::consts::FRAC_PI_2,
            },
        ];
        let b = sweep(&p, &path, 4).expect("sweeps");
        let root = Name::feature(4);
        assert!(b.face_by_name(start(root)).is_some());
        assert!(b.face_by_name(end(root)).is_some());
        for k in 0..2 {
            for half in 0..2 {
                assert!(
                    b.face_by_name(skin_side(root, 3, half, k)).is_some(),
                    "piece {k} half {half}"
                );
            }
        }
    }

    #[test]
    fn a_coil_and_a_loft_name_theirs_alike() {
        use crate::sweep::{Handed, Winding};
        use crate::{LoftGuides, LoftKind, coil, loft};
        let w = Winding {
            pitch: 5.0,
            turns: 2.0,
            taper: 0.0,
            handed: Handed::Right,
        };
        let axis = Axis2 {
            origin: Point2::new(0.0, 0.0),
            dir: Vec2::new(0.0, 1.0),
        };
        let ring = Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop {
                segments: vec![(
                    Segment::Circle {
                        center: Point2::new(10.0, 0.0),
                        radius: 1.0,
                        ccw: true,
                    },
                    6,
                )],
            }],
        };
        let b = coil(&ring, axis, &w, 5).expect("winds");
        let root = Name::feature(5);
        for half in 0..2 {
            assert!(b.face_by_name(skin_side(root, 6, half, 0)).is_some());
        }
        let up = Frame::WORLD.with_origin(forge_math::Point3::new(0.0, 0.0, 10.0));
        let mut top = square([5, 6, 7, 8]);
        top.plane = up;
        let b = loft(
            &[square([1, 2, 3, 4]), top],
            LoftKind::Ruled,
            &LoftGuides::None,
            8,
        )
        .expect("lofts");
        let root = Name::feature(8);
        assert!(b.face_by_name(start(root)).is_some() && b.face_by_name(end(root)).is_some());
        for s in [1, 2, 3, 4] {
            assert!(b.face_by_name(skin_side(root, s, 0, 0)).is_some(), "segment {s}");
        }
    }
    #[test]
    fn a_shell_names_its_inner_skin_and_rims_from_the_faces_they_came_from() {
        let ids = [1, 2, 3, 4];
        let b = extrude(&square(ids), 5.0, 7).expect("extrudes");
        let part = Name::feature(7);
        let open = end(part);
        let s = crate::shell(&b, 1.0, false, &[open], 9).expect("shells");
        let root = Name::feature(9);
        assert!(s.face_by_name(offset(root, start(part))).is_some());
        for id in ids {
            assert!(
                s.face_by_name(offset(root, side(part, id))).is_some(),
                "segment {id}"
            );
        }
        assert!(s.face_by_name(rim(root, open, 0)).is_some());
    }
}
