//! Exports for the OCCT check of the uniform scale (tools/occt/scale.py): each suite part of one
//! body as STEP, and for each factor our scaled body's vertices and points on its faces' surfaces
//! (each face's own: the image of a point of the face's surface, projected onto the scaled surface
//! and evaluated there), with our exact mass properties of both.
//!
//!   cargo run --release -p forge-model --example scale_occt -- DIR
//!   tools/occt/scale.py DIR
use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Point2, Point3};
use forge_model::{BodyFeature, Feature, Step, suite};
use std::fmt::Write as _;

const CENTRE: Point3 = Point3::new(3.0, -7.0, 2.5);
const KS: [f64; 3] = [2.5, 0.375, 1.7];

fn p3(p: Point3) -> String {
    format!("[{:e},{:e},{:e}]", p.x, p.y, p.z)
}

fn main() {
    let dir = std::env::args().nth(1).expect("an output directory");
    std::fs::create_dir_all(&dir).expect("dir");
    let mut index = String::from("[");
    let mut first = true;
    for (name, part) in suite::parts() {
        let Ok(b) = part.regenerate() else { continue };
        let b = b.compact();
        let Ok(x) = forge_step::write::export(&b, &forge_step::write::Options::default()) else {
            continue;
        };
        std::fs::write(format!("{dir}/{name}.step"), &x.text).expect("write");
        let m0 = forge_topo::mass::mass(&b).expect("mass");
        for k in KS {
            let mut p = part.clone();
            p.features.push(Feature::Body(Box::new(BodyFeature::Move {
                steps: vec![Step::Scale { about: CENTRE, k }],
                copy: false,
            })));
            let s = p.regenerate().expect("scaled").compact();
            let m1 = forge_topo::mass::mass(&s).expect("mass");
            let verts: Vec<String> = s.vertices.iter().map(|v| p3(v.point)).collect();
            let mut pts = vec![];
            for (f0, f1) in b.faces.iter().zip(&s.faces) {
                let analytic = !matches!(f0.surface, Surface::Procedural(_));
                // points of the original face's surface near its edges' middles and its box's centre
                let mut seeds: Vec<Point3> = vec![];
                let fid = b.face_ids().find(|&i| b.face(i).name == f0.name).expect("face");
                seeds.push(b.face_box(fid).center());
                for &l in &f0.loops {
                    for c in b.lp(l).coedges().take(4) {
                        let e = b.edge(b.coedge(c).edge);
                        seeds.push(forge_geom::CurveGeom::eval(&e.curve, e.range.mid()));
                    }
                }
                for q in seeds {
                    let on0 = f0.surface.eval(f0.surface.project(q));
                    let img = forge_geom::similar::point(on0, CENTRE, k);
                    let uv: Point2 = f1.surface.project(img);
                    let on1 = f1.surface.eval(uv);
                    pts.push(format!("[{},{}]", p3(on1), analytic));
                }
            }
            if !first {
                index.push(',');
            }
            first = false;
            let _ = write!(
                index,
                "{{\"name\":\"{name}\",\"step\":\"{name}.step\",\"c\":{},\"k\":{k:e},\"fitted\":{:e},\
                 \"v0\":{:e},\"a0\":{:e},\"v1\":{:e},\"a1\":{:e},\"verts\":[{}],\"pts\":[{}]}}",
                p3(CENTRE),
                x.uncertainty,
                m0.volume,
                m0.area,
                m1.volume,
                m1.area,
                verts.join(","),
                pts.join(",")
            );
        }
    }
    index.push(']');
    std::fs::write(format!("{dir}/index.json"), index).expect("index");
}
