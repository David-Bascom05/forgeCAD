//! A face may run past the domain its procedural surface states, onto the part carried on beyond it
//! (a mitre takes a loft's wall there): its fit must cover it, or the file's edges lie off the fit.
use forge_agent::model::Model;
use forge_geom::{Curve, Line, SurfaceGeom};
use forge_math::{Interval, Point2, Point3};
use forge_topo::{Body, Name};

#[test]
fn a_faces_fit_box_covers_it_past_the_stated_domain() {
    let f = |z: f64| format!("profile z 0 0 1 x 1 0 0 at 0 0 {z} rect -10 -5 20 10 ");
    let src = format!("loft smooth {}{}{}\n", f(0.0), f(20.0), f(45.0));
    let m = Model::new(&src);
    assert!(m.error.is_none(), "{:?}", m.error.map(|e| e.msg));
    let loft = m.body().expect("a body");
    let wall = loft
        .face_ids()
        .find(|&g| matches!(loft.face(g).surface, forge_geom::Surface::Procedural(_)))
        .expect("a loft wall");
    let surface = loft.face(wall).surface.clone();
    let d = surface.domain();
    assert!(d.v.period.is_none());

    // a patch of the wall from below the domain's start to its middle
    let span = d.v.hi - d.v.lo;
    let (v0, v1) = (d.v.lo - 0.1 * span, d.v.lo + 0.5 * span);
    let (u0, u1) = (
        d.u.lo + 0.25 * (d.u.hi - d.u.lo),
        d.u.lo + 0.75 * (d.u.hi - d.u.lo),
    );
    let corners = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)].map(|(u, v)| surface.eval(Point2::new(u, v)));
    let mut b = Body::new();
    let shell = b.add_shell();
    let face = b.add_face(shell, surface.clone(), true, Name::feature(1));
    let names = |k: u64| Name::feature(100 + k);
    let vs: Vec<_> = corners
        .iter()
        .enumerate()
        .map(|(k, &p): (usize, &Point3)| b.add_vertex(p, names(k as u64)))
        .collect();
    let uses: Vec<_> = (0..4)
        .map(|k| {
            let (p, q) = (corners[k], corners[(k + 1) % 4]);
            let line = Line::new(p, (q - p).normalize().expect("apart"));
            let range = Interval::new(0.0, p.distance(q));
            let e = b.add_edge(
                Curve::Line(line),
                range,
                vs[k],
                vs[(k + 1) % 4],
                names(10 + k as u64),
            );
            (e, true)
        })
        .collect();
    b.add_loop(face, &uses);

    let got = forge_step::write::face_box(&b, face);
    assert!(
        got.v.lo <= v0 + 1e-6,
        "the fit stops at {} short of {v0}",
        got.v.lo
    );
    assert!(
        got.v.hi >= v1 - 1e-6 && got.v.hi <= d.v.hi,
        "{} {}",
        got.v.hi,
        d.v.hi
    );
}
