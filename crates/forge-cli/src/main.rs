use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_mesh::{Quality, TriMesh, tessellate, to_binary_stl};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::{Body, FaceId, check, genus};
use std::process::ExitCode;
use std::time::Instant;

fn usage() -> ExitCode {
    eprintln!("usage: forgecad demo <out.stl> [chord-mm]\n       forgecad preview <out.json>");
    ExitCode::from(2)
}

/// A mounting plate: rounded outline, a slot, and two bolt holes.
fn plate() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(-40.0, -25.0), 80.0, 50.0, 8.0, 0),
            Profile::rounded_rectangle(Point2::new(-15.0, -4.0), 30.0, 8.0, 3.999, 100),
            Profile::circle(Point2::new(-30.0, 15.0), 3.25, 200),
            Profile::circle(Point2::new(30.0, 15.0), 3.25, 201),
        ],
    }
}

const Y: Axis2 = Axis2 {
    origin: Point2::ORIGIN,
    dir: Vec2::Y,
};

/// A pin: cylinder shank, hemispherical head.
fn pin() -> Profile {
    let q = dmath::FRAC_PI_2;
    Profile {
        plane: Frame::WORLD,
        loops: vec![ProfileLoop::new(vec![
            (
                Segment::Line {
                    a: Point2::ORIGIN,
                    b: Point2::new(5.0, 0.0),
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(5.0, 0.0),
                    b: Point2::new(5.0, 20.0),
                },
                1,
            ),
            (
                Segment::Arc {
                    center: Point2::new(0.0, 20.0),
                    radius: 5.0,
                    start: 0.0,
                    sweep: q,
                },
                2,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 25.0),
                    b: Point2::ORIGIN,
                },
                3,
            ),
        ])],
    }
}

fn ring() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rounded_rectangle(
            Point2::new(10.0, 0.0),
            8.0,
            12.0,
            2.0,
            0,
        )],
    }
}

fn holed_ring() -> Profile {
    Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(8.0, 0.0), 14.0, 10.0, 3.0, 0),
            Profile::circle(Point2::new(15.0, 5.0), 2.5, 20),
        ],
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("demo") if args.len() >= 3 => demo(&args),
        Some("preview") if args.len() >= 3 => preview(&args[2]),
        _ => usage(),
    }
}

fn demo(args: &[String]) -> ExitCode {
    let chord = match args.get(3).map(|s| s.parse::<f64>()) {
        None => Quality::default().chord,
        Some(Ok(c)) if c > 0.0 => c,
        Some(_) => return usage(),
    };
    let body = extrude(&plate(), 6.0, 1).expect("demo profile is valid");
    let problems = check(&body);
    let mesh = match tessellate(
        &body,
        &Quality {
            chord,
            ..Quality::default()
        },
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("tessellation failed: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "faces {}  edges {}  vertices {}  genus {}  problems {}",
        body.faces.len(),
        body.edges.len(),
        body.vertices.len(),
        genus(&body),
        problems.len()
    );
    println!(
        "triangles {}  volume {:.4} mm^3  area {:.4} mm^2  open edges {}",
        mesh.triangles.len(),
        mesh.volume(),
        mesh.area(),
        mesh.open_edges().len()
    );
    if let Err(e) = std::fs::write(&args[2], to_binary_stl(&mesh, "ForgeCAD demo")) {
        eprintln!("writing {}: {e}", args[2]);
        return ExitCode::FAILURE;
    }
    println!("wrote {}", args[2]);
    if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn f32s(v: &[f32]) -> String {
    b64(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}

/// Triangles split per face so each face carries its exact surface normals.
fn shaded(body: &Body, m: &TriMesh) -> (Vec<f32>, Vec<f32>) {
    let (mut pos, mut nrm) = (Vec::new(), Vec::new());
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        let face = body.face(FaceId(f));
        let flat = m.triangle_normal(*t).normalize().map(|u| u.get());
        for &i in t {
            let p = m.positions[i as usize];
            let n = face
                .surface
                .normal(face.surface.project(p))
                .map(|n| if face.same_sense { n.get() } else { -n.get() })
                .or(flat)
                .unwrap_or_default();
            pos.extend([p.x as f32, p.y as f32, p.z as f32]);
            nrm.extend([n.x as f32, n.y as f32, n.z as f32]);
        }
    }
    (pos, nrm)
}

fn edges(body: &Body) -> Vec<f32> {
    let mut out = Vec::new();
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let n = if matches!(edge.curve, forge_geom::Curve::Line(_)) {
            1
        } else {
            48
        };
        for i in 0..n {
            for k in [i, i + 1] {
                let t = edge.range.lo + edge.range.width() * (k as f64 / n as f64);
                let p = edge.curve.eval(t.min(edge.range.hi));
                out.extend([p.x as f32, p.y as f32, p.z as f32]);
            }
        }
    }
    out
}

fn preview(path: &str) -> ExitCode {
    type Build = Box<dyn Fn() -> Body>;
    let parts: Vec<(&str, &str, Build)> = vec![
        (
            "Mounting plate",
            "Extrude 6 mm: rounded outline, slot, two bolt holes",
            Box::new(|| extrude(&plate(), 6.0, 1).unwrap()),
        ),
        (
            "Pin",
            "Revolve 360°: cylinder shank, hemisphere head",
            Box::new(|| revolve(&pin(), Y, dmath::TAU, 1).unwrap()),
        ),
        (
            "Ring",
            "Revolve 360°: rounded rectangle, four tori",
            Box::new(|| revolve(&ring(), Y, dmath::TAU, 1).unwrap()),
        ),
        (
            "Partial ring",
            "Revolve 270°: rounded section with a hole, planar end caps",
            Box::new(|| revolve(&holed_ring(), Y, 1.5 * dmath::PI, 1).unwrap()),
        ),
    ];
    let mut json = String::from("{\"parts\":[");
    for (k, (name, how, build)) in parts.iter().enumerate() {
        let t0 = Instant::now();
        let reps = 200;
        for _ in 0..reps {
            std::hint::black_box(build());
        }
        let build_us = t0.elapsed().as_secs_f64() * 1e6 / reps as f64;
        let body = build();
        let t1 = Instant::now();
        for _ in 0..20 {
            std::hint::black_box(tessellate(&body, &Quality::default()).unwrap());
        }
        let mesh_us = t1.elapsed().as_secs_f64() * 1e6 / 20.0;
        let mesh = tessellate(&body, &Quality::default()).unwrap();
        let (pos, nrm) = shaded(&body, &mesh);
        if k > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{name}\",\"how\":\"{how}\",\"faces\":{},\"edges\":{},\"vertices\":{},\"genus\":{},\"problems\":{},\
             \"triangles\":{},\"open\":{},\"volume\":{:.3},\"build_us\":{:.1},\"mesh_us\":{:.1},\
             \"pos\":\"{}\",\"nrm\":\"{}\",\"lines\":\"{}\"}}",
            body.faces.len(),
            body.edges.len(),
            body.vertices.len(),
            genus(&body),
            check(&body).len(),
            mesh.triangles.len(),
            mesh.open_edges().len(),
            mesh.volume(),
            build_us,
            mesh_us,
            f32s(&pos),
            f32s(&nrm),
            f32s(&edges(&body)),
        ));
    }
    json.push_str("]}");
    if let Err(e) = std::fs::write(path, json) {
        eprintln!("writing {path}: {e}");
        return ExitCode::FAILURE;
    }
    println!("wrote {path}");
    ExitCode::SUCCESS
}
