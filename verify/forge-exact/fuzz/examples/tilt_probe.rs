//! TRANSLATE's plane_plane probe: A a box [0,L]x[0,W]x[0,H]; B a prism under A's y=0 wall, its top
//! wall from (0, y0) to (L, y1), so the two walls are within an angle (y1-y0)/L. Kernel against the
//! reference, for each op.
//! cargo run --release -p forge-exact-fuzz --example tilt_probe -- L y0 y1
use forge_bool::Op;
use forge_exact_fuzz::pairs::{kernel_pair, pair_input};
use forge_exact_fuzz::{Reference, classify};
use forge_geom::Surface;
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::Body;

fn prism(pts: &[(f64, f64)], h: f64, feature: u64) -> Body {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                feature * 10 + i as u64,
            )
        })
        .collect();
    let profile = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 0.0)),
        loops: vec![ProfileLoop::new(segs)],
    };
    extrude(&profile, h, feature).expect("a prism extrudes")
}

fn main() {
    let a: Vec<f64> = std::env::args()
        .skip(1)
        .map(|s| s.parse().expect("a number"))
        .collect();
    let (l, y0, y1) = (a[0], a[1], a[2]);
    let (w, h, d) = (10.0, 10.0, 10.0);
    let ba = prism(&[(0.0, 0.0), (l, 0.0), (l, w), (0.0, w)], h, 1);
    let bb = prism(&[(0.0, -d), (l, -d), (l, y1), (0.0, y0)], h, 2);
    // the two walls, and what plane_plane says of them
    let wall = |b: &Body, y: f64| {
        b.face_ids()
            .filter_map(|f| match &b.face(f).surface {
                Surface::Plane(p)
                    if p.normal_vec().get().y.abs() > 0.5 && (p.frame.origin.y - y).abs() < 1.0 =>
                {
                    Some(*p)
                }
                _ => None,
            })
            .next()
            .expect("the wall")
    };
    let (p, q) = (wall(&ba, 0.0), wall(&bb, y0));
    let c = p.normal_vec().cross(q.normal_vec()).length();
    println!(
        "L {l:e} y0 {y0:e} y1 {y1:e}: |n1 x n2| {c:e}; p.origin {:?} q.origin {:?}",
        p.frame.origin, q.frame.origin
    );
    println!(
        "  plane_plane(A,B) {:?}\n  plane_plane(B,A) {:?}",
        forge_geom::intersect::plane_plane(&p, &q),
        forge_geom::intersect::plane_plane(&q, &p)
    );
    let mut r = Reference::start();
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        let ans = r.ask(&pair_input(&ba, &bb, op).expect("flat"));
        let k = kernel_pair(&ba, &bb, op);
        println!(
            "{op:?}\n  reference: {ans:?}\n  kernel: {k:?}\n  class: {:?}",
            classify(&ans, &k)
        );
    }
}
