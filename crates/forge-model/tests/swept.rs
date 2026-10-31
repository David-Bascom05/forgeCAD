//! Sweeps, coils and lofts mesh closed and enclose their exact volumes, alone and in parts.
use forge_geom::Circle;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::suite;
use forge_ops::{Axis2, Handed, LoftKind, PathSeg, Profile, Winding, coil, loft, sweep};
use forge_topo::Body;

fn volume(b: &Body) -> f64 {
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let m = tessellate(
        b,
        &Quality {
            chord: 0.002,
            ..Quality::default()
        },
    )
    .expect("meshes");
    assert!(m.open_edges().is_empty(), "closed mesh");
    m.volume()
}

fn close(got: f64, want: f64) {
    assert!((got - want).abs() <= 2e-3 * want, "{got} vs {want}");
}

/// A helical sweep of a section in the axis's plane encloses its area times the path its
/// centroid travels (Pappus: the rise moves material along, it adds none).
#[test]
fn coil_volume_is_area_times_centroid_path() {
    let plane = Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("square axes");
    let up = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::new(0.0, 1.0),
    };
    let wire = Profile {
        plane,
        loops: vec![Profile::circle(Point2::new(15.0, 0.0), 1.5, 0)],
    };
    let turns = 2.5;
    let w = |pitch: f64, handed: Handed| Winding {
        pitch,
        turns,
        taper: 0.0,
        handed,
    };
    for handed in [Handed::Right, Handed::Left] {
        let v = volume(&coil(&wire, up, &w(6.0, handed), 1).expect("coil"));
        close(v, dmath::PI * 1.5 * 1.5 * 15.0 * dmath::TAU * turns);
    }
    let bar = Profile {
        plane,
        loops: vec![Profile::rectangle(Point2::new(8.0, -1.0), 3.0, 2.0, 0)],
    };
    let v = volume(&coil(&bar, up, &w(4.0, Handed::Right), 1).expect("coil"));
    close(v, 6.0 * 9.5 * dmath::TAU * turns);
}

/// Along a path through the profile's centroid, a sweep encloses the area times the length.
#[test]
fn sweep_volume_is_area_times_length() {
    let (r, bend) = (2.0, 8.0);
    let c = Point3::new(bend, 0.0, 10.0);
    let path = [
        PathSeg::Line {
            a: Point3::ORIGIN,
            b: Point3::new(0.0, 0.0, 10.0),
        },
        PathSeg::Arc {
            circle: Circle {
                frame: Frame::from_zx(c, UnitVec3::Y, -Vec3::X).expect("square axes"),
                radius: bend,
            },
            start: 0.0,
            sweep: dmath::FRAC_PI_2,
        },
        PathSeg::Line {
            a: Point3::new(bend, 0.0, 10.0 + bend),
            b: Point3::new(bend + 10.0, 0.0, 10.0 + bend),
        },
    ];
    let pipe = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    let v = volume(&sweep(&pipe, &path, 1).expect("sweep"));
    close(v, dmath::PI * r * r * (20.0 + bend * dmath::FRAC_PI_2));
}

/// Between similar sections a loft is a frustum: h (A0 + A1 + sqrt(A0 A1)) / 3, oblique or not.
#[test]
fn loft_volume_is_a_frustum() {
    let at = |o: Point3| Frame::WORLD.with_origin(o);
    let square = |o: Point3, s: f64| Profile {
        plane: at(o),
        loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
    };
    let frustum = |h: f64, a0: f64, a1: f64| h * (a0 + a1 + (a0 * a1).sqrt()) / 3.0;
    let v = volume(
        &loft(
            &[
                square(Point3::ORIGIN, 4.0),
                square(Point3::new(0.0, 0.0, 10.0), 2.0),
            ],
            LoftKind::Ruled,
            &forge_ops::LoftGuides::None,
            1,
        )
        .expect("loft"),
    );
    close(v, frustum(10.0, 64.0, 16.0));
    let round = |o: Point3, r: f64| Profile {
        plane: at(o),
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    let v = volume(
        &loft(
            &[
                round(Point3::ORIGIN, 4.0),
                round(Point3::new(3.0, 0.0, 10.0), 2.0),
            ],
            LoftKind::Ruled,
            &forge_ops::LoftGuides::None,
            1,
        )
        .expect("loft"),
    );
    close(v, frustum(10.0, dmath::PI * 16.0, dmath::PI * 4.0));
    // a smooth loft through coaxial circles whose radii grow evenly: the spline through
    // evenly growing points is the straight line, so a cone frustum again
    let v = volume(
        &loft(
            &[
                round(Point3::ORIGIN, 4.0),
                round(Point3::new(0.0, 0.0, 4.0), 3.2),
                round(Point3::new(0.0, 0.0, 10.0), 2.0),
            ],
            LoftKind::Smooth {
                start: None,
                end: None,
            },
            &forge_ops::LoftGuides::None,
            1,
        )
        .expect("loft"),
    );
    close(v, frustum(10.0, dmath::PI * 16.0, dmath::PI * 4.0));
}

#[test]
fn suite_parts_mesh_closed() {
    for name in ["sweep", "coil", "loft"] {
        let b = suite::part(name)
            .expect("a suite part")
            .regenerate()
            .expect("regenerates");
        let m = tessellate(&b, &Quality::default()).expect("meshes");
        assert!(m.open_edges().is_empty(), "{name}");
        assert!(m.volume() > 0.0, "{name}");
    }
}
