//! indexwhy FG ID [x y z]..: forge-bool's `Index` against a mesh winding number at points of a
//! pipe part's fillet result (the .fg cut after its `round`). PTS=FILE adds `x y z` lines;
//! RAND=N adds N random points in the result's box; OUT=DIR writes DIR/ID.step and DIR/ID.pts.
use forge_bool::{Index, Where};
use forge_math::{Point3, rng::Rng};
use forge_mesh::tessellate::{Quality, tessellate, tessellate_tolerant};
use forge_model::Feature;

fn winding(m: &forge_mesh::mesh::TriMesh, p: Point3) -> f64 {
    let mut w = 0.0;
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize] - p);
        let (la, lb, lc) = (a.length(), b.length(), c.length());
        let num = a.dot(b.cross(c));
        let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
        w += 2.0 * forge_math::dmath::atan2(num, den);
    }
    w / (4.0 * std::f64::consts::PI)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (fg, id) = (&a[1], &a[2]);
    let src = std::fs::read_to_string(format!("{fg}/{id}.fg")).expect("the .fg part");
    let full = std::env::var("FULL").is_ok();
    let keep = src
        .lines()
        .position(|l| l.starts_with("round = "))
        .expect("a round");
    let src: String = if full {
        src
    } else {
        src.lines().take(keep + 1).map(|l| format!("{l}\n")).collect()
    };
    let m = forge_lang::fuzz::build(&src);
    let k = m
        .features
        .iter()
        .position(|f| matches!(f, Feature::Fillet { .. }))
        .expect("a fillet");
    let after = if full {
        m.body_after(m.features.len()).expect("the result")
    } else {
        m.body_after(k + 1).expect("the result")
    };
    let mesh = [1e-3, 3e-3, 1e-2, 3e-2]
        .iter()
        .find_map(|&c| {
            let q = Quality {
                chord: c,
                max_angle: 0.05,
            };
            tessellate(&after, &q)
                .or_else(|_| tessellate_tolerant(&after, &q))
                .ok()
        })
        .unwrap_or_default();
    let mut pts: Vec<Point3> = a[3..]
        .chunks(3)
        .map(|c| {
            Point3::new(
                c[0].parse().unwrap(),
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
            )
        })
        .collect();
    if let Ok(f) = std::env::var("PTS") {
        for l in std::fs::read_to_string(f).expect("the points").lines() {
            let w: Vec<f64> = l.split_whitespace().take(3).map(|t| t.parse().unwrap()).collect();
            pts.push(Point3::new(w[0], w[1], w[2]));
        }
    }
    let ix = Index::new(&after);
    let given = pts.len();
    if let Ok(n) = std::env::var("RAND") {
        let n: usize = n.parse().unwrap();
        let b = ix.bbox;
        let mut rng = Rng::new(5);
        for _ in 0..n {
            pts.push(Point3::new(
                rng.range(b.x.lo, b.x.hi),
                rng.range(b.y.lo, b.y.hi),
                rng.range(b.z.lo, b.z.hi),
            ));
        }
    }
    if let Ok(dir) = std::env::var("OUT") {
        let e = forge_step::export(&after, &forge_step::Options::default()).expect("a STEP file");
        std::fs::write(format!("{dir}/{id}.step"), e.text).unwrap();
        let t: String = pts
            .iter()
            .map(|x| format!("{} {} {} {:?}\n", x.x, x.y, x.z, ix.classify(*x)))
            .collect();
        std::fs::write(format!("{dir}/{id}.pts"), t).unwrap();
    }
    println!(
        "{id}: faces {} triangles {} open {}",
        after.faces.len(),
        mesh.triangles.len(),
        mesh.open_edges().len()
    );
    if let Ok(d) = std::env::var("WALK") {
        let d: Vec<f64> = d.split(',').map(|t| t.parse().unwrap()).collect();
        let dir = forge_math::Vec3::new(d[0], d[1], d[2]);
        let dir = dir * (1.0 / dir.length());
        let mut last = -1.0;
        for k in 0..=600 {
            let t = k as f64 * 0.02;
            let w = winding(&mesh, pts[0] + dir * t).round();
            if w != last {
                println!("walk t {t:.2} winding {w}");
                last = w;
            }
        }
        if let Ok(r) = std::env::var("BISECT") {
            let r: Vec<f64> = r.split(',').map(|t| t.parse().unwrap()).collect();
            let (mut lo, mut hi) = (r[0], r[1]);
            let w0 = winding(&mesh, pts[0] + dir * lo).round();
            for _ in 0..40 {
                let m = 0.5 * (lo + hi);
                if winding(&mesh, pts[0] + dir * m).round() == w0 {
                    lo = m
                } else {
                    hi = m
                }
            }
            let x = pts[0] + dir * lo;
            let faces: Vec<_> = after.face_ids().collect();
            let mut best = (f64::INFINITY, 0u32);
            for (t, &f) in mesh.triangles.iter().zip(&mesh.face_of) {
                let c = t.map(|i| mesh.positions[i as usize]);
                let g = forge_math::Point3::new(
                    (c[0].x + c[1].x + c[2].x) / 3.0,
                    (c[0].y + c[1].y + c[2].y) / 3.0,
                    (c[0].z + c[1].z + c[2].z) / 3.0,
                );
                let d = g.distance(x);
                if d < best.0 {
                    best = (d, f);
                }
            }
            println!(
                "crossing at t {lo:.6} {x:?} nearest mesh face f{} ({:.4})",
                faces[best.1 as usize].index(),
                best.0
            );
        }
    }
    let mut bad = 0;
    for (i, x) in pts.iter().enumerate() {
        let w = winding(&mesh, *x);
        let c = ix.classify(*x);
        let wrong = match c {
            Where::Inside => w < 0.5,
            Where::Outside => w > 0.5,
            Where::On => false,
        };
        if wrong {
            bad += 1;
        }
        if i < given || wrong {
            println!(
                "{} ({:.6}, {:.6}, {:.6}) Index {:?} winding {:.4}{}",
                if i < given { "given" } else { "rand " },
                x.x,
                x.y,
                x.z,
                c,
                w,
                if wrong { "  WRONG" } else { "" }
            );
        }
    }
    println!("{id}: {bad} of {} disagree", pts.len());
}
