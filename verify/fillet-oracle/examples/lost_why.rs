//! Why a pipe part is `unsure`: lost_why FC_DIR FG_DIR ID [N]
//!
//! label_pipe calls a part `unsure` when its only disagreements are points of the body before the
//! fillet (certified, from the .fc twin) that the kernel's result classifies outside. Either the
//! result lost material there, or its classification (forge-bool's `Index`) is wrong. This samples
//! N points round the picked edges, keeps those the .fc solids hold inside by more than δ, and for
//! each that `Index` puts outside the result, asks three more witnesses:
//! - `Index` on the body before the fillet (should be inside);
//! - the winding number of a fine tessellation of the result (exact solid angles);
//! - the nearest face of the result and the distance to it (on a new face, or an old one).
//!
//! A point the winding number holds inside (≈ 1) is a classification error; a point it holds
//! outside (≈ 0), far from every face, is material the result lost. The result's volume against
//! the body before plus the oracle's fill is printed too.
use forge_bool::{Index, Where as KWhere};
use forge_fillet_oracle::Field;
use forge_math::{Point3, Vec3, rng::Rng};
use forge_mesh::tessellate::{Quality, tessellate, tessellate_tolerant};
use forge_model::Feature;
use std::collections::BTreeSet;

fn unit(v: Vec3) -> Vec3 {
    v * (1.0 / v.length())
}

fn solid(line: &str) -> Field {
    let t: Vec<&str> = line.split_whitespace().collect();
    let num = |k: usize| t[k].parse::<f64>().expect("a number");
    let at = |w: &str| t.iter().position(|&x| x == w).expect(w);
    let i = at("extrude");
    let j = if t[i + 1] == "add" { i + 2 } else { i + 1 };
    let h = num(j);
    let (zi, xi, ai) = (at("z"), at("x"), at("at"));
    let z = unit(Vec3::new(num(zi + 1), num(zi + 2), num(zi + 3)));
    let x = unit(Vec3::new(num(xi + 1), num(xi + 2), num(xi + 3)));
    let o = Point3::new(num(ai + 1), num(ai + 2), num(ai + 3));
    let y = z.cross(x);
    if let Some(ci) = t.iter().position(|&w| w == "circle") {
        let c = o + x * num(ci + 1) + y * num(ci + 2);
        Field::Rod {
            o: c,
            d: z,
            h,
            radius: num(ci + 3),
        }
    } else {
        let ri = at("rect");
        let c = o + x * num(ri + 1) + y * num(ri + 2);
        Field::Block {
            o: c,
            x,
            y,
            z,
            w: num(ri + 3),
            l: num(ri + 4),
            h,
        }
    }
}

/// Generalised winding number of a closed triangle mesh at p (van Oosterom–Strackee).
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

fn tri_dist(a: Point3, b: Point3, c: Point3, p: Point3) -> f64 {
    // closest point on a triangle (Ericson)
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (p - a).length();
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (p - b).length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (p - (a + ab * (d1 / (d1 - d3)))).length();
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (p - c).length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (p - (a + ac * (d2 / (d2 - d6)))).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length();
    }
    let den = 1.0 / (va + vb + vc);
    let (v, w) = (vb * den, vc * den);
    (p - (a + ab * v + ac * w)).length()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (fc, fg, id) = (&a[1], &a[2], &a[3]);
    let n: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let fc_src = std::fs::read_to_string(format!("{fc}/{id}.fc")).expect("the .fc part");
    let lines: Vec<&str> = fc_src
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let body = Field::Union(vec![solid(lines[0]), solid(lines[1])]);
    let r: f64 = lines[2].split_whitespace().nth(1).expect("r").parse().expect("r");
    let src = std::fs::read_to_string(format!("{fg}/{id}.fg")).expect("the .fg part");
    let keep = src
        .lines()
        .position(|l| l.starts_with("round = "))
        .expect("a round");
    let src: String = src.lines().take(keep + 1).map(|l| format!("{l}\n")).collect();
    let m = forge_lang::fuzz::build(&src);
    let k = m
        .features
        .iter()
        .position(|f| matches!(f, Feature::Fillet { .. }))
        .expect("a fillet");
    let s = m.body_after(k).expect("the body before");
    let after = m.body_after(k + 1).expect("the result");
    let old: BTreeSet<_> = s.face_ids().map(|f| s.face(f).name).collect();
    let faces: Vec<_> = after.face_ids().collect();
    let mesh = [1e-3, 3e-3, 1e-2]
        .iter()
        .find_map(|&c| {
            let q = Quality {
                chord: c,
                max_angle: 0.05,
            };
            match tessellate(&after, &q) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("chord {c}: {e:?}");
                    tessellate_tolerant(&after, &q).ok()
                }
            }
        })
        .unwrap_or_default();
    // the result as STEP and the points as text, for OCCT's classifier (occt_classify.py)
    if let Ok(dir) = std::env::var("OUT") {
        let e = forge_step::export(&after, &forge_step::Options::default()).expect("a STEP file");
        std::fs::write(format!("{dir}/{id}.step"), e.text).unwrap();
    }
    let (va, vb) = (
        forge_topo::mass::mass(&after).unwrap().volume,
        forge_topo::mass::mass(&s).unwrap().volume,
    );
    println!(
        "{id}: r {r}  result {va:.4} (mesh {:.4}, open edges {})  before {vb:.4}  added {:.4}  triangles {}",
        mesh.volume(),
        mesh.open_edges().len(),
        va - vb,
        mesh.triangles.len()
    );
    let picked: Vec<_> = s
        .edge_ids()
        .filter(|&e| forge_ops::fillet::fillet_kind(&s, e) == Some(false))
        .collect();
    let mut region = forge_math::Box3::EMPTY;
    for &e in &picked {
        region = region.hull(&s.edge_box(e));
    }
    let region = region.grow(2.0 * r);
    let delta = (1e-3 * r).max(1e-4);
    let (ix, sx) = (Index::new(&after), Index::new(&s));
    let mut rng = Rng::new(99);
    let (mut inside, mut lost) = (0usize, 0usize);
    let mut extra: Vec<Point3> = a[5..]
        .chunks(3)
        .map(|c| {
            Point3::new(
                c[0].parse().unwrap(),
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
            )
        })
        .collect();
    // PTS=FILE: points as `x y z [kind]` lines (label_pipe's DUMP), judged like the given ones
    if let Ok(f) = std::env::var("PTS") {
        for l in std::fs::read_to_string(f).expect("the points").lines() {
            let w: Vec<f64> = l.split_whitespace().take(3).map(|t| t.parse().unwrap()).collect();
            extra.push(Point3::new(w[0], w[1], w[2]));
        }
    }
    let given = extra.len();
    for _ in 0..n {
        let x = Point3::new(
            rng.range(region.x.lo, region.x.hi),
            rng.range(region.y.lo, region.y.hi),
            rng.range(region.z.lo, region.z.hi),
        );
        if body.sd(x) >= -delta {
            continue;
        }
        inside += 1;
        if ix.classify(x) == KWhere::Outside {
            lost += 1;
            extra.push(x);
        }
    }
    if let Ok(dir) = std::env::var("OUT") {
        let t: String = extra
            .iter()
            .map(|x| format!("{} {} {}\n", x.x, x.y, x.z))
            .collect();
        std::fs::write(format!("{dir}/{id}.pts"), t).unwrap();
    }
    println!("sampled {n}: {inside} certified body points, {lost} that Index puts outside the result");
    for (i, x) in extra.iter().enumerate() {
        let w = if mesh.triangles.is_empty() {
            f64::NAN
        } else {
            winding(&mesh, *x)
        };
        let (mut best, mut bf) = (f64::INFINITY, 0u32);
        for (t, &f) in mesh.triangles.iter().zip(&mesh.face_of) {
            let [p, q, c] = t.map(|i| mesh.positions[i as usize]);
            let d = tri_dist(p, q, c, *x);
            if d < best {
                best = d;
                bf = f;
            }
        }
        let face = after.face(faces[bf as usize]);
        println!(
            "{} ({:.4}, {:.4}, {:.4})  sd_body {:.4}  Index before {:?}  Index after {:?}  winding {:.4}  nearest face {} ({}, {:?}) at {:.4}",
            if i < given { "given " } else { "found " },
            x.x,
            x.y,
            x.z,
            body.sd(*x),
            sx.classify(*x),
            ix.classify(*x),
            w,
            bf,
            if old.contains(&face.name) { "old" } else { "new" },
            std::mem::discriminant(&face.surface),
            best
        );
    }
}
