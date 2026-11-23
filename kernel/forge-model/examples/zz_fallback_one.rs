//! (scratch, not committed) fuzz seed's feature n redone as a plain boolean of the stage before it
//! and the feature's tool built alone: `zz_fallback_one <seed> <n>`
use forge_model::{Combine, Feature, Part, suite};

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let part = suite::fuzz(a[0], 200);
    let n = a[1] as usize;
    let prev = Part {
        features: part.features[..n].to_vec(),
    }
    .regenerate()
    .expect("stage before");
    let mut f = part.features[n].clone();
    let op = match &mut f {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } => {
            let o = *op;
            *op = Combine::New;
            o
        }
        other => panic!("{other:?}"),
    };
    let tool = Part { features: vec![f] }.regenerate().expect("tool");
    let bop = match op {
        Combine::Add => forge_bool::Op::Union,
        Combine::Cut => forge_bool::Op::Subtract,
        _ => forge_bool::Op::Intersect,
    };
    println!("prev check {:?}", forge_topo::check(&prev));
    println!("tool check {:?}", forge_topo::check(&tool));
    for f in prev.face_ids() {
        if let forge_geom::Surface::Torus(tr) = &prev.face(f).surface {
            let bx = prev.face_box(f);
            let hits: Vec<_> = tool
                .face_ids()
                .filter(|&g| tool.face_box(g).overlaps(&bx))
                .collect();
            println!(
                "torus face {f:?} major {} box {bx:?} tool faces overlapping {hits:?}",
                tr.major
            );
        }
    }
    if let (Some(fa), Some(fb)) = (
        std::env::var("ZZ_A").ok().and_then(|x| x.parse::<u32>().ok()),
        std::env::var("ZZ_B").ok().and_then(|x| x.parse::<u32>().ok()),
    ) {
        use forge_geom::{CurveGeom, SurfaceGeom};
        let (sa, sb) = (
            &prev.face(forge_topo::FaceId(fa)).surface,
            &tool.face(forge_topo::FaceId(fb)).surface,
        );
        let whole = match sa {
            forge_geom::Surface::Torus(t) => {
                forge_math::Box3::from_point(t.frame.origin).grow(t.major.abs() + t.minor)
            }
            _ => prev.face_box(forge_topo::FaceId(fa)),
        };
        let bb = tool.face_box(forge_topo::FaceId(fb));
        let cut = |x: forge_math::Interval, y: forge_math::Interval| {
            forge_math::Interval::new(x.lo.max(y.lo), x.hi.min(y.hi))
        };
        let region = forge_math::Box3 {
            x: cut(whole.x, bb.x),
            y: cut(whole.y, bb.y),
            z: cut(whole.z, bb.z),
        }
        .grow(forge_math::tol::LINEAR);
        println!("region {region:?}");
        match forge_bool::ssi::surfaces_in(sa, sb, region) {
            forge_geom::intersect::SurfaceSurface::Curves(cs) => {
                for (c, k) in &cs {
                    let d = c.domain();
                    println!("curve {k:?} domain {d:?}");
                    let n = 24;
                    for i in 0..n {
                        let t = d.lo + (d.hi - d.lo) * i as f64 / n as f64;
                        let p = c.eval(t);
                        println!(
                            "   t {t:.3} p {p:?} da {:.2e} db {:.2e}",
                            sa.distance(p),
                            sb.distance(p)
                        );
                    }
                }
            }
            o => println!("ssi {o:?}"),
        }
    }
    if std::env::var_os("ZZ_SURF").is_some() {
        for f in tool.face_ids() {
            println!("tool face {f:?} {:?}", tool.face(f).surface);
        }
        for f in prev.face_ids() {
            if let forge_geom::Surface::Cylinder(c) = &prev.face(f).surface
                && (c.radius - 10.575244744739232).abs() < 1e-9
            {
                println!(
                    "prev face {f:?} {:?} box {:?}",
                    prev.face(f).surface,
                    prev.face_box(f)
                );
            }
        }
    }
    let mut t = prev.clone();
    let r = forge_bool::boolean_into(&mut t, &tool, bop, 1);
    println!("boolean_into {:?}", r.as_ref().err());
    if r.is_ok() {
        let c = t.compact();
        let probs = forge_topo::check(&c);
        println!("into check: {:?}", probs);
        for pr in &probs {
            if let forge_topo::Problem::EdgeUses { edge, .. } = pr {
                let e = c.edge(*edge);
                println!(
                    "  edge {:?} curve {:?} range {:?} tol {}",
                    edge, e.curve, e.range, e.tol
                );
                for &co in &e.coedges {
                    let f = c.coedge_face(co);
                    println!(
                        "  on face {:?} {:?} surface {:?}",
                        f,
                        c.face(f).name,
                        c.face(f).surface
                    );
                }
            }
        }
    }
    let r = forge_bool::boolean(&prev, &tool, bop, 1);
    match r {
        Ok(b) => println!("boolean (unified) check: {:?}", forge_topo::check(&b)),
        Err(e) => println!("boolean err {e:?}"),
    }
    let r = forge_bool::nary(&[&prev, &tool], &[bop], 1);
    match r {
        Ok(b) => println!("nary check: {:?}", forge_topo::check(&b)),
        Err(e) => println!("nary err {e:?}"),
    }
}
