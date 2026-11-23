//! The kernel's splines against their specification (research/spline-spec, the Lean in
//! research/spline-loops/lean/SplineSpec). Every curve here is built by the kernel, then handed
//! with the data it was built from to the Lean oracle (`lake exe splineoracle`,
//! SplineSpec/Oracle.lean), which computes the specification's curve exactly and answers `ok` when
//! every Bezier point the kernel holds is within the curve's scale over 2^30 of the specification's
//! (so the curves are, everywhere).
//!
//! - control splines (`Spline::from_nurbs`): random degree 1..5, clamped or not, inner knots of
//!   multiplicity up to the degree, weights, in the plane and in space; the closed `cv` form;
//! - through-splines (`Spline::through`): random points, natural and slope ends, periodic;
//! - the splines of the repository's own tests and parts, restated.
//!
//! As parse_diff does: the oracle's answers are kept as a digest (tests/spline_diff.digest), so
//! without Lean the test checks that the kernel still builds exactly the curves Lean accepted;
//! `FG_LEAN=1` runs the oracle, line by line, and rewrites the digest.

use forge_geom::spline::{PlaneSpline, Spline, uniform_knots};
use forge_math::rng::Rng;
use forge_math::{Point2, Point3, Vec2, Vec3};
use std::fmt::Write as _;

const DIGEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/spline_diff.digest");
const LEAN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/spline-loops/lean");

/// An f64 exactly, as `m:e` for m 2^e.
fn ex(x: f64) -> String {
    assert!(x.is_finite());
    if x == 0.0 {
        return "0:0".into();
    }
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { -1i64 } else { 1 };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let frac = (bits & ((1u64 << 52) - 1)) as i64;
    let (mut m, mut e) = if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1i64 << 52), exp - 1075)
    };
    while m & 1 == 0 {
        m >>= 1;
        e += 1;
    }
    format!("{}:{}", sign * m, e)
}

fn push_pieces(line: &mut String, s: &Spline, with_count: bool) {
    let (breaks, pts) = s.homogeneous();
    line.push_str(" |");
    if with_count {
        write!(line, " {}", breaks.len() - 1).unwrap();
    }
    for b in breaks {
        write!(line, " {}", ex(*b)).unwrap();
    }
    for h in pts {
        for x in h {
            write!(line, " {}", ex(*x)).unwrap();
        }
    }
}

/// A control spline's line, or None where the kernel refuses the data.
fn control_line(p: usize, knots: &[f64], w: Option<&[f64]>, poles: &[Point3], d: usize) -> Option<String> {
    let s = Spline::from_nurbs(p, knots, poles, w)?;
    let mut line = format!("C {p} {} {d}", poles.len());
    for k in knots {
        write!(line, " {}", ex(*k)).unwrap();
    }
    for i in 0..poles.len() {
        write!(line, " {}", ex(w.map_or(1.0, |w| w[i]))).unwrap();
    }
    for q in poles {
        for x in &[q.x, q.y, q.z][..d] {
            write!(line, " {}", ex(*x)).unwrap();
        }
    }
    push_pieces(&mut line, &s, true);
    Some(line)
}

/// A through-spline's line: points (the first repeated last when periodic), end slopes.
fn through_line(pts: &[Point3], from: Option<Vec3>, to: Option<Vec3>, d: usize) -> Option<String> {
    let s = Spline::through(pts, from, to)?;
    let n = pts.len();
    let periodic = n > 2 && pts[0].distance(pts[n - 1]) <= forge_math::tol::LINEAR;
    let mut line = format!("T {d} {} ", n - 1);
    let end = |v: Option<Vec3>| match v {
        None => "natural".to_string(),
        Some(v) => {
            let mut s = "slope".to_string();
            for x in &[v.x, v.y, v.z][..d] {
                write!(s, " {}", ex(*x)).unwrap();
            }
            s
        }
    };
    if periodic {
        line.push_str("periodic");
    } else {
        write!(line, "open {} {}", end(from), end(to)).unwrap();
    }
    for q in pts {
        for x in &[q.x, q.y, q.z][..d] {
            write!(line, " {}", ex(*x)).unwrap();
        }
    }
    push_pieces(&mut line, &s, false);
    Some(line)
}

fn random_knots(r: &mut Rng, p: usize, n: usize) -> Vec<f64> {
    // inner knots of multiplicity up to p, ends clamped or not
    let clamped = r.bool();
    let mut k: Vec<f64> = Vec::with_capacity(n + p + 1);
    let mut x = r.range(-5.0, 5.0);
    if clamped {
        k.extend(std::iter::repeat_n(x, p + 1));
    }
    while k.len() < n + p + 1 {
        x += r.range(0.05, 3.0);
        let mult = 1 + r.below(p);
        for _ in 0..mult {
            if k.len() < n + p + 1 {
                k.push(x);
            }
        }
    }
    if clamped {
        // the last p + 1 equal
        let last = k[n];
        for v in &mut k[n..] {
            *v = last;
        }
        // an inner knot may now repeat more than p times: let the kernel refuse it then
    }
    k
}

fn random_point(r: &mut Rng, d: usize, s: f64) -> Point3 {
    Point3::new(
        r.range(-s, s),
        r.range(-s, s),
        if d == 3 { r.range(-s, s) } else { 0.0 },
    )
}

fn corpus() -> Vec<String> {
    let mut out = Vec::new();
    let mut r = Rng::new(0x5eed_5911);
    // control splines
    for _ in 0..400 {
        let p = 1 + r.below(5);
        let n = p + 1 + r.below(8);
        let d = if r.bool() { 2 } else { 3 };
        let scale = [1.0, 50.0, 1000.0][r.below(3)];
        let poles: Vec<Point3> = (0..n).map(|_| random_point(&mut r, d, scale)).collect();
        let knots = if r.below(4) == 0 {
            uniform_knots(p, n)
        } else {
            random_knots(&mut r, p, n)
        };
        let w: Option<Vec<f64>> = r.bool().then(|| (0..n).map(|_| r.range(0.2, 5.0)).collect());
        if let Some(l) = control_line(p, &knots, w.as_deref(), &poles, d) {
            out.push(l);
        }
    }
    // the closed cv form, as the sketch makes it
    for _ in 0..60 {
        let p = 1 + r.below(4);
        let m = p + 2 + r.below(6);
        let q: Vec<Point3> = (0..m).map(|_| random_point(&mut r, 2, 40.0)).collect();
        let poles: Vec<Point3> = (0..m + p).map(|i| q[i % m]).collect();
        let knots: Vec<f64> = (0..m + 2 * p + 1).map(|i| i as f64).collect();
        out.extend(control_line(p, &knots, None, &poles, 2));
    }
    // through-splines
    for _ in 0..300 {
        let m = 1 + r.below(12);
        let d = if r.below(3) == 0 { 3 } else { 2 };
        let scale = [1.0, 30.0, 500.0][r.below(3)];
        let mut pts: Vec<Point3> = (0..=m).map(|_| random_point(&mut r, d, scale)).collect();
        let periodic = m >= 2 && r.below(3) == 0;
        let (mut from, mut to) = (None, None);
        if periodic {
            let first = pts[0];
            pts.push(first);
        } else {
            let v = |r: &mut Rng| {
                let p = random_point(r, d, 3.0);
                Vec3::new(p.x, p.y, p.z)
            };
            if r.bool() {
                from = Some(v(&mut r));
            }
            if r.bool() {
                to = Some(v(&mut r));
            }
        }
        out.extend(through_line(&pts, from, to, d));
    }
    // the repository's own splines, restated: tests/kernel_misc.rs, kernel_build_failures.rs and
    // parts/sheet/spline.fg (each as its sketch makes it)
    let p2 = |v: &[(f64, f64)]| v.iter().map(|&(x, y)| Point2::new(x, y)).collect::<Vec<_>>();
    let mut planar: Vec<PlaneSpline> = Vec::new();
    planar.extend(PlaneSpline::through(
        p2(&[(0.0, 0.0), (10.0, 8.0), (20.0, 8.0), (30.0, 0.0)]),
        None,
        None,
    ));
    planar.extend(PlaneSpline::control(
        2,
        p2(&[(0.0, 0.0), (5.0, 8.0), (10.0, 0.0)]),
        None,
        None,
    ));
    planar.extend(PlaneSpline::control(
        1,
        p2(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0), (0.0, 0.0)]),
        Some(vec![0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 4.0]),
        None,
    ));
    planar.extend(PlaneSpline::control(
        2,
        p2(&[(0.0, 0.0), (5.0, 8.0), (10.0, 0.0)]),
        None,
        Some(vec![1.0, 3.0, 1.0]),
    ));
    planar.extend(PlaneSpline::control(
        2,
        p2(&[(0.0, 0.0), (3.0, 6.0), (7.0, 6.0), (10.0, 0.0)]),
        Some(vec![0.0, 0.0, 0.0, 1.0, 3.0, 3.0, 3.0]),
        Some(vec![1.0, 2.0, 0.5, 1.0]),
    ));
    planar.extend(PlaneSpline::through(
        p2(&[(0.0, 0.0), (5.0, 4.0), (10.0, 0.0)]),
        Some(Vec2::new(1.0, 2.0)),
        Some(Vec2::new(1.0, -2.0)),
    ));
    planar.extend(PlaneSpline::through(
        p2(&[(0.0, 0.0), (5.0, 4.0), (10.0, 0.0)]),
        Some(Vec2::new(3.0, 0.0)),
        None,
    ));
    // the sheet part's window: x0 8, x1 22, y0 6, y1 24, bulge 3, closed
    planar.extend(PlaneSpline::through(
        p2(&[(8.0, 6.0), (19.0, 9.0), (22.0, 24.0), (11.0, 21.0), (8.0, 6.0)]),
        None,
        None,
    ));
    let arc = |i: usize| {
        // the 60-point closed loop of kernel_build_failures.rs: two lobes, restated as the
        // curve its points sample (a circle of radius 12 about (38, 0) rolled on one of 12)
        let a = std::f64::consts::TAU * i as f64 / 60.0;
        let r = 12.0 + 4.0 * forge_math::dmath::cos(3.0 * a);
        (
            38.0 + r * forge_math::dmath::cos(a),
            r * forge_math::dmath::sin(a),
        )
    };
    let mut lobe: Vec<(f64, f64)> = (0..60).map(arc).collect();
    lobe.push(lobe[0]);
    planar.extend(PlaneSpline::through(p2(&lobe), None, None));
    let poly = p2(&[
        (55.0, 10.0),
        (47.19638711935487, 10.614871027096626),
        (39.69266270539641, 12.435854959638824),
        (32.777190679215906, 15.392972406318552),
        (26.715728752538098, 19.37258300203048),
        (21.74121550789819, 24.22175254337273),
        (18.04481869954853, 29.754130164317125),
        (15.768588783870783, 35.7571096954839),
        (15.0, 42.0),
    ]);
    planar.extend(PlaneSpline::control(3, poly, None, None));
    for c in planar {
        let p3 = |p: &Point2| Point3::new(p.x, p.y, 0.0);
        let v3 = |v: Vec2| Vec3::new(v.x, v.y, 0.0);
        match &c.form {
            forge_geom::spline::Form::Control {
                degree,
                pts,
                knots,
                weights,
            } => {
                let k = knots.clone().unwrap_or_else(|| uniform_knots(*degree, pts.len()));
                let poles: Vec<Point3> = pts.iter().map(p3).collect();
                out.extend(control_line(*degree, &k, weights.as_deref(), &poles, 2));
            }
            forge_geom::spline::Form::Through { pts, from, to } => {
                let q: Vec<Point3> = pts.iter().map(p3).collect();
                out.extend(through_line(&q, from.map(v3), to.map(v3), 2));
            }
            forge_geom::spline::Form::Ellipse { .. } => {}
        }
    }
    out
}

/// FNV-1a of every (input, answer), in order.
fn digest(answers: &[(String, String)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (w, a) in answers {
        for b in w.bytes().chain(*b"\t").chain(a.bytes()).chain(*b"\n") {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

#[test]
fn splines_match_the_spec() {
    let lines = corpus();
    assert!(lines.len() > 600, "{} cases", lines.len());
    let answers: Vec<(String, String)> = lines.iter().map(|l| (l.clone(), "ok".to_string())).collect();
    let line = format!("splines {} {:016x}", answers.len(), digest(&answers));
    if std::env::var_os("FG_LEAN").is_none() {
        let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
        assert!(
            have.lines().any(|l| l == line),
            "no Lean answers for these curves ({line}); run the oracle: (cd research/spline-loops/lean && lake build splineoracle) then FG_LEAN=1 cargo test -p forge-geom --test spline_diff"
        );
        return;
    }
    let path = format!("{LEAN_DIR}/.lake/build/bin/splineoracle");
    let mut child = std::process::Command::new(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{path}: {e}; lake build splineoracle first"));
    let mut input = String::new();
    for l in &lines {
        input.push_str(l);
        input.push('\n');
    }
    let mut stdin = child.stdin.take().expect("stdin");
    let feed = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write to the oracle");
    });
    let out = child.wait_with_output().expect("the oracle runs");
    feed.join().expect("fed");
    let out = String::from_utf8(out.stdout).expect("utf8");
    let lean: Vec<&str> = out.lines().collect();
    assert_eq!(lean.len(), lines.len(), "one answer per curve");
    let worst = lean
        .iter()
        .filter_map(|a| a.split('\t').nth(1))
        .filter_map(|b| b.parse::<f64>().ok())
        .fold(0.0f64, f64::max);
    eprintln!(
        "{} curves; the largest distance from the spec, as Lean bounds it: {worst:e}",
        lines.len()
    );
    let bad: Vec<String> = lines
        .iter()
        .zip(&lean)
        .filter(|(_, a)| a.split('\t').next() != Some("ok"))
        .take(5)
        .map(|(w, a)| format!("  {}..\n    Lean {a}", &w[..w.len().min(160)]))
        .collect();
    assert!(
        bad.is_empty(),
        "the kernel and the spec disagree:\n{}",
        bad.join("\n")
    );
    std::fs::write(DIGEST_PATH, format!("{line}\n")).expect("the digest");
}
