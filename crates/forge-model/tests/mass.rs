//! Exact mass properties against closed forms: volume, area, centroid, inertia, principal axes.
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_model::{fillets, finishing};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::Body;
use forge_topo::mass::{MassProps, mass};

const PI: f64 = dmath::PI;
const Y: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

fn line(a: [f64; 2], b: [f64; 2], i: u64) -> (Segment, u64) {
    (
        Segment::Line {
            a: Point2::new(a[0], a[1]),
            b: Point2::new(b[0], b[1]),
        },
        i,
    )
}

fn profile(l: ProfileLoop) -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![l],
    }
}

fn props(b: &Body) -> MassProps {
    let m = mass(b).expect("mass properties");
    assert!(m.err[0] <= 1e-12 * m.volume, "volume error {}", m.err[0]);
    m
}

fn near(got: f64, want: f64, scale: f64, what: &str) {
    assert!((got - want).abs() <= 1e-11 * scale, "{what}: {got} vs {want}");
}

/// Revolved about y: a ball, a hemisphere, a torus and a cone.
#[test]
fn solids_of_revolution() {
    let r = 5.0;
    let half = ProfileLoop::new(vec![
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start: -PI / 2.0,
                sweep: PI,
            },
            0,
        ),
        line([0.0, r], [0.0, -r], 1),
    ]);
    let ball = props(&revolve(&profile(half.clone()), Y, dmath::TAU, 1).expect("ball"));
    let v = 4.0 / 3.0 * PI * r * r * r;
    near(ball.volume, v, v, "ball volume");
    near(ball.area, 4.0 * PI * r * r, v, "ball area");
    for (i, row) in ball.inertia.iter().enumerate() {
        for (j, &x) in row.iter().enumerate() {
            let want = if i == j { 0.4 * v * r * r } else { 0.0 };
            near(x, want, v * r * r, "ball inertia");
        }
    }
    // half a ball: its centroid 3r/8 from the centre, square to the cut
    let hemi = props(&revolve(&profile(half), Y, PI, 1).expect("hemisphere"));
    near(hemi.volume, v / 2.0, v, "hemisphere volume");
    near(hemi.area, 3.0 * PI * r * r, v, "hemisphere area");
    let g = hemi.centroid - Point3::ORIGIN;
    near(g.length(), 3.0 * r / 8.0, r, "hemisphere centroid");
    near(g.y, 0.0, r, "hemisphere centroid on the cut");

    let (big, small) = (10.0, 3.0);
    let torus = props(
        &revolve(
            &profile(Profile::circle(Point2::new(big, 0.0), small, 0)),
            Y,
            dmath::TAU,
            1,
        )
        .expect("torus"),
    );
    let v = 2.0 * PI * PI * big * small * small;
    near(torus.volume, v, v, "torus volume");
    near(torus.area, 4.0 * PI * PI * big * small, v, "torus area");
    // about the axis (y): m (R^2 + 3 r^2 / 4); across it: m (R^2 / 2 + 5 r^2 / 8)
    near(
        torus.inertia[1][1],
        v * (big * big + 0.75 * small * small),
        v * big * big,
        "torus axial",
    );
    near(
        torus.inertia[0][0],
        v * (0.5 * big * big + 0.625 * small * small),
        v * big * big,
        "torus across",
    );
    near(
        torus.principal[2],
        torus.inertia[1][1],
        v * big * big,
        "torus principal",
    );
    assert!((torus.axes[2] - Vec3::Y).length() < 1e-9, "{:?}", torus.axes);

    let tri = ProfileLoop::new(vec![
        line([0.0, 0.0], [4.0, 0.0], 0),
        line([4.0, 0.0], [0.0, 7.0], 1),
        line([0.0, 7.0], [0.0, 0.0], 2),
    ]);
    let cone = props(&revolve(&profile(tri), Y, dmath::TAU, 1).expect("cone"));
    let v = PI * 16.0 * 7.0 / 3.0;
    near(cone.volume, v, v, "cone volume");
    near(cone.area, PI * 4.0 * (4.0 + 65f64.sqrt()), v, "cone area");
    near(cone.centroid.y, 7.0 / 4.0, 7.0, "cone centroid");
    near(cone.inertia[1][1], 0.3 * v * 16.0, v * 16.0, "cone axial");
}

/// A tilted block: its inertia about the centroid, and the principal axes are its edges.
#[test]
fn tilted_block() {
    let (a, b, h) = (10.0, 20.0, 5.0);
    let x = Vec3::new(1.0, 2.0, 0.5);
    let z = Vec3::new(-0.3, 0.1, 1.0);
    let frame = Frame::from_zx(Point3::new(3.0, -2.0, 7.0), z.normalize().expect("z"), x).expect("frame");
    let body = extrude(
        &Profile {
            plane: frame,
            loops: vec![Profile::rectangle(Point2::new(1.0, -4.0), a, b, 0)],
        },
        h,
        1,
    )
    .expect("block");
    let m = props(&body);
    let v = a * b * h;
    near(m.volume, v, v, "volume");
    near(m.area, 2.0 * (a * b + b * h + h * a), v, "area");
    let c = frame.point_to_world(Point3::new(1.0 + a / 2.0, -4.0 + b / 2.0, h / 2.0));
    assert!(m.centroid.distance(c) < 1e-12 * 30.0, "{:?} vs {c:?}", m.centroid);
    // principal moments m (p^2 + q^2) / 12 over the other two sides, smallest first
    let mut want = [b * b + h * h, a * a + h * h, a * a + b * b].map(|s| v * s / 12.0);
    want.sort_by(f64::total_cmp);
    for (got, want) in m.principal.iter().zip(want) {
        near(*got, want, v * 500.0, "principal");
    }
    // the smallest about the longest side (b, the frame's y)
    assert!(m.axes[0].cross(frame.y().get()).length() < 1e-9, "{:?}", m.axes);
    assert!(m.axes[2].cross(frame.z().get()).length() < 1e-9, "{:?}", m.axes);
    let faces: f64 = m.faces.iter().map(|f| f.1).sum();
    near(faces, m.area, v, "face areas");
}

/// Shelled, chamfered and filleted parts with closed-form volumes.
#[test]
fn finished_parts_by_hand() {
    let mut n = 0;
    for (name, part, want) in finishing::parts()
        .into_iter()
        .chain(fillets::cases().into_iter().take(13))
    {
        let Some(want) = want else { continue };
        let b = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let m = mass(&b).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(m.err[0] <= 1e-9 * m.volume, "{name}: error {}", m.err[0]);
        assert!(
            (m.volume - want).abs() <= 1e-9 * want,
            "{name}: volume {} want {want}",
            m.volume
        );
        n += 1;
    }
    assert!(n >= 20, "{n}");
}
