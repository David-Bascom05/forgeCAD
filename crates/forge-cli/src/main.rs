mod serve;

use forge_agent::cmd::{Args, COMMANDS, Session};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_mesh::{Quality, TriMesh, tessellate, to_binary_stl};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::{Body, FaceId, check, genus};
use std::process::ExitCode;
use std::time::Instant;

fn usage() -> ExitCode {
    eprintln!(
        "usage: forgecad build|query|measure|render|check|diff FILE [ARGS] [--json] [--at FEAT] [--labels SEL] [--px N] [-o OUT]\n       \
         forgecad serve | mcp [--root DIR]   (JSON-RPC / MCP on stdio, .fc files under DIR, default .)\n       \
         forgecad demo <out.stl> [chord-mm] | preview <out.json>\nformat and selectors: AGENTS.md"
    );
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
        Some(m @ ("serve" | "mcp")) => match &args[2..] {
            [] => serve::run(m == "mcp", "."),
            [flag, root] if flag == "--root" => serve::run(m == "mcp", root),
            _ => usage(),
        },
        Some(c) if COMMANDS.contains(&c) && args.len() >= 3 => command(c, &args[2..]),
        _ => usage(),
    }
}

fn command(cmd: &str, argv: &[String]) -> ExitCode {
    let mut a = Args {
        file: argv[0].clone(),
        ..Args::default()
    };
    let (mut json, mut out) = (false, None);
    let mut it = argv[1..].iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--json" => json = true,
            "--at" => a.at = it.next().cloned(),
            "--labels" => a.labels = it.next().cloned(),
            "--px" => a.px = it.next().and_then(|p| p.parse().ok()),
            "-o" => out = it.next().cloned(),
            w if w.starts_with("--") => {
                eprintln!("unknown option {w} (--json --at --labels --px -o)");
                return ExitCode::from(2);
            }
            _ => a.words.push(w.clone()),
        }
    }
    let r = Session::default().run(cmd, &a);
    let mut text = if json { r.json.to_string() } else { r.text };
    if let Some(png) = &r.png {
        let path = out.unwrap_or_else(|| format!("{}.png", a.file.trim_end_matches(".fc")));
        if let Err(e) = std::fs::write(&path, png) {
            eprintln!("writing {path}: {e}");
            return ExitCode::FAILURE;
        }
        if !json {
            text = format!("wrote {path} {text}");
        }
    }
    println!("{text}");
    if r.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
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

pub(crate) fn b64(bytes: &[u8]) -> String {
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

/// Coordinates as 16-bit steps across `bbox` (lo xyz, hi xyz), little endian.
fn q16(v: &[f32], bbox: &[f32; 6]) -> String {
    let bytes: Vec<u8> = v
        .iter()
        .enumerate()
        .flat_map(|(i, &x)| {
            let (lo, hi) = (bbox[i % 3], bbox[3 + i % 3]);
            let t = if hi > lo { (x - lo) / (hi - lo) } else { 0.0 };
            ((t.clamp(0.0, 1.0) * 65535.0).round() as u16).to_le_bytes()
        })
        .collect();
    b64(&bytes)
}

/// Unit vectors as signed bytes.
fn q8(v: &[f32]) -> String {
    b64(&v
        .iter()
        .map(|&x| (x.clamp(-1.0, 1.0) * 127.0).round() as i8 as u8)
        .collect::<Vec<u8>>())
}

fn bbox(sets: &[&[f32]]) -> [f32; 6] {
    let mut b = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for v in sets {
        for (i, &x) in v.iter().enumerate() {
            b[i % 3] = b[i % 3].min(x);
            b[3 + i % 3] = b[3 + i % 3].max(x);
        }
    }
    b
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
            12
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

/// A part for the preview page: the solid, how long building it took, and
/// whether its history regenerates.
struct Shown {
    name: String,
    how: String,
    body: Body,
    build_us: f64,
    features: usize,
    status: String,
}

fn simple(name: &str, how: &str, build: impl Fn() -> Body) -> Shown {
    let reps = 200;
    let t0 = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(build());
    }
    let build_us = t0.elapsed().as_secs_f64() * 1e6 / reps as f64;
    Shown {
        name: name.into(),
        how: how.into(),
        body: build(),
        build_us,
        features: 1,
        status: "ok".into(),
    }
}

/// Regenerates a history from scratch (best of three). If it fails, shows the
/// solid up to the last feature that worked and says where it stops.
fn history(name: &str, how: &str, part: &forge_model::Part) -> Shown {
    let n = part.features.len();
    let prefix = |k: usize| forge_model::Part {
        features: part.features[..k].to_vec(),
    };
    let (k, status) = match part.regenerate() {
        Ok(_) => (n, "ok".to_string()),
        Err(e) => {
            let (mut lo, mut hi) = (1, n);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if prefix(mid).regenerate().is_ok() {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            (lo - 1, format!("stops at feature {}: {e:?}", lo - 1))
        }
    };
    let shown = prefix(k.max(1));
    let mut best = f64::INFINITY;
    let mut body = None;
    for _ in 0..3 {
        let t = Instant::now();
        let b = shown.regenerate().expect("prefix regenerates");
        best = best.min(t.elapsed().as_secs_f64() * 1e6);
        body = Some(b);
    }
    Shown {
        name: name.into(),
        how: how.into(),
        body: body.expect("ran"),
        build_us: best,
        features: k,
        status,
    }
}

fn preview(path: &str) -> ExitCode {
    let mut parts: Vec<Shown> = vec![
        simple(
            "Mounting plate",
            "Extrude 6 mm: rounded outline, slot, two bolt holes",
            || extrude(&plate(), 6.0, 1).unwrap(),
        ),
        simple("Pin", "Revolve 360°: cylinder shank, hemisphere head", || {
            revolve(&pin(), Y, dmath::TAU, 1).unwrap()
        }),
        simple("Ring", "Revolve 360°: rounded rectangle, four tori", || {
            revolve(&ring(), Y, dmath::TAU, 1).unwrap()
        }),
        simple(
            "Partial ring",
            "Revolve 270°: rounded section with a hole, planar end caps",
            || revolve(&holed_ring(), Y, 1.5 * dmath::PI, 1).unwrap(),
        ),
    ];
    let blurb = |n: &str| match n {
        "plate" => "Torture plate: holes, pockets, bosses on one plane, rotated slots, dimples, counterbores",
        "shaft" => "Revolved stepped shaft: grooves, cross holes, flats, keyways, radial pin holes",
        "housing" => "Hollow box: inside bosses, screw holes, ribs, wall windows",
        "coplanar" => "Features that share planes and faces on purpose",
        "patterns" => "Arrays of holes, bosses and rotated slots that overlap",
        "mixed" => "Domes, spherical cuts, revolved rings and cones with extrudes",
        _ => "",
    };
    for (name, part) in forge_model::suite::parts() {
        parts.push(history(&format!("Suite: {name}"), blurb(name), &part));
    }
    for seed in [1u64, 12, 15, 16, 2, 19] {
        parts.push(history(
            &format!("Fuzz seed {seed}"),
            "Random history: blocks, rods, rounded slots, balls, rings, tilted planes",
            &forge_model::suite::fuzz(seed, 80),
        ));
    }
    let mut json = String::from("{\"parts\":[");
    for (k, p) in parts.iter().enumerate() {
        let big = p.body.faces.len() > 50;
        let q = if big {
            Quality {
                chord: 0.25,
                max_angle: 0.7,
            }
        } else {
            Quality::default()
        };
        let body = p.body.compact();
        let mut status = p.status.replace('"', "'");
        let (mesh, mesh_us) = match tessellate(&body, &q) {
            Ok(m) => {
                let reps = if big { 3 } else { 20 };
                let t1 = Instant::now();
                for _ in 0..reps {
                    std::hint::black_box(tessellate(&body, &q).ok());
                }
                (m, t1.elapsed().as_secs_f64() * 1e6 / reps as f64)
            }
            Err(e) => {
                status = format!("{status}; mesh failed: {e:?}").replace('"', "'");
                (forge_mesh::TriMesh::default(), 0.0)
            }
        };
        let (pos, nrm) = shaded(&body, &mesh);
        let lines = edges(&body);
        let bb = bbox(&[&pos, &lines]);
        if k > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{}\",\"how\":\"{}\",\"status\":\"{}\",\"features\":{},\"faces\":{},\"edges\":{},\"vertices\":{},\"genus\":{},\"problems\":{},\
             \"triangles\":{},\"open\":{},\"volume\":{:.3},\"build_us\":{:.1},\"mesh_us\":{:.1},\
             \"bbox\":{:?},\"pos\":\"{}\",\"nrm\":\"{}\",\"lines\":\"{}\"}}",
            p.name,
            p.how,
            status,
            p.features,
            body.faces.len(),
            body.edges.len(),
            body.vertices.len(),
            genus(&body),
            check(&body).len(),
            mesh.triangles.len(),
            mesh.open_edges().len(),
            mesh.volume(),
            p.build_us,
            mesh_us,
            bb,
            q16(&pos, &bb),
            q8(&nrm),
            q16(&lines, &bb),
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
