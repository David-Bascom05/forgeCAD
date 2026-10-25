use forge_math::{Frame, Point2};
use forge_mesh::{Quality, tessellate, to_binary_stl};
use forge_ops::{Profile, extrude};
use forge_topo::{Body, check, genus};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: forgecad demo <out.stl> [chord-mm]");
    ExitCode::from(2)
}

/// A mounting plate: rounded outline, a slot, and two bolt holes.
fn demo_part() -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(-40.0, -25.0), 80.0, 50.0, 8.0, 0),
            Profile::rounded_rectangle(Point2::new(-15.0, -4.0), 30.0, 8.0, 3.999, 100),
            Profile::circle(Point2::new(-30.0, 15.0), 3.25, 200),
            Profile::circle(Point2::new(30.0, 15.0), 3.25, 201),
        ],
    };
    extrude(&p, 6.0, 1).expect("demo profile is valid")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "demo" {
        return usage();
    }
    let chord = match args.get(3).map(|s| s.parse::<f64>()) {
        None => Quality::default().chord,
        Some(Ok(c)) if c > 0.0 => c,
        Some(_) => return usage(),
    };
    let body = demo_part();
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
