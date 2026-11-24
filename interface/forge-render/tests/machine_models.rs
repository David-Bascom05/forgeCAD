//! A model the user attaches to a machine body (forge_machine_builder::models): copied into the
//! user's library (never the project), placed on the body's envelope with its units read from its
//! size, drawn in the body's place, checked against the envelope, and the envelope fitted to it.
//! `FORGE_UI_SHOTS=DIR` keeps the pictures.

use forge_machine::def::Prim;
use forge_render::gui::Headless;
use forge_render::shell::machine::{MachineUi, Tab};
use std::path::{Path, PathBuf};

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

/// An ASCII STL of a box from `lo` to `hi` (12 triangles, outward).
fn box_stl(lo: [f64; 3], hi: [f64; 3]) -> String {
    let c = |k: usize| -> [f64; 3] { std::array::from_fn(|a| if k >> a & 1 == 1 { hi[a] } else { lo[a] }) };
    let faces = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let mut s = String::from("solid box\n");
    for f in faces {
        for t in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
            s.push_str("facet normal 0 0 0\nouter loop\n");
            for v in t {
                let p = c(v);
                s.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            s.push_str("endloop\nendfacet\n");
        }
    }
    s.push_str("endsolid box\n");
    s
}

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-machine-models-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_users_model_is_kept_in_their_library_placed_drawn_and_checked() {
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let root = scratch();
    let project = root.join("project");
    let user = root.join("config/forgecad/cam/machines");
    std::fs::create_dir_all(&project).unwrap();
    // the maker's file, wherever the user saved it: in inches, a little bigger than the envelope
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&downloads).unwrap();

    let mut m = MachineUi::new(Some(project.clone()));
    m.user = Some(user.clone());
    m.reload_catalog();
    m.pick_record("haas-vf-2").unwrap();
    m.tab = Tab::Bodies;
    let d = m.def.clone().unwrap();
    let b = d
        .bodies
        .iter()
        .position(|b| b.name == "table")
        .expect("a body named table");
    m.pick = b;
    let Some(Prim::Box { min, max }) = d.bodies[b].prims.first().cloned() else {
        panic!("the table's envelope is a box")
    };
    // 5 % longer than the envelope in x, in inches
    let size: [f64; 3] = std::array::from_fn(|k| (max[k] - min[k]) / 25.4);
    let stl = downloads.join("VF2 table.stl");
    std::fs::write(&stl, box_stl([0.0; 3], [size[0] * 1.05, size[1], size[2]])).unwrap();

    let msg = m
        .attach_model(Some("table".into()), stl.to_str().unwrap())
        .unwrap();
    assert!(msg.contains("inches"), "{msg}");
    let kept = user.join("models/haas-vf-2");
    assert!(kept.join("VF2_table.stl").is_file(), "{msg}");
    assert!(kept.join("models.toml").is_file());
    let walk = |p: &Path| -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![p.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                if e.path().is_dir() {
                    stack.push(e.path());
                } else {
                    out.push(e.path());
                }
            }
        }
        out
    };
    assert!(walk(&project).is_empty(), "nothing is written into the project");

    // placed: centred over the envelope, its bottom on the envelope's bottom, in mm
    let md = m.models[0].clone();
    assert_eq!(md.scale, 25.4);
    let t = m.model_tris(&md).unwrap();
    let (lo, hi) = forge_machine_builder::models::bounds(t.pos.iter().map(|p| md.place(*p))).unwrap();
    assert!((lo[2] - min[2]).abs() < 1e-6, "{lo:?} {min:?}");
    let mid = |a: f64, b: f64| (a + b) / 2.0;
    assert!((mid(lo[0], hi[0]) - mid(min[0], max[0])).abs() < 1e-6);
    // 2.5 % past each end in x: it sticks out, and says by how much
    let o = m.model_checks[0].clone().unwrap();
    assert!(o.out > 0);
    let past = (max[0] - min[0]) * 0.025;
    assert!((o.worst - past).abs() < 0.01, "{} vs {past}", o.worst);

    // in the view in the body's place, with its envelope see-through over it
    h.gui.shell.machine = Some(m);
    h.step();
    shot(&mut h, "models-01-attached");

    // the envelope fitted to the model: the model is inside it now
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let i = 0;
        let mut d2 = m.def.clone().unwrap();
        let (lo, hi) = forge_machine_builder::models::bounds(
            forge_render::machine_model::placed_points(&m.model_tris(&m.models[i]).unwrap(), &m.models[i])
                .into_iter(),
        )
        .unwrap();
        d2.bodies[b].prims = vec![Prim::Box {
            min: lo.map(|x| x - 0.05),
            max: hi.map(|x| x + 0.05),
        }];
        m.def = Some(d2);
        m.edited(true);
        m.check_models();
        let o = m.model_checks[0].clone().unwrap();
        assert_eq!(o.out, 0, "{o:?}");
    }
    shot(&mut h, "models-02-envelope-fitted");

    // read back as a fresh workspace would; removing it removes its copy
    let mut m2 = MachineUi::new(Some(project.clone()));
    m2.user = Some(user.clone());
    m2.reload_catalog();
    m2.pick_record("haas-vf-2").unwrap();
    assert_eq!(m2.models, vec![md]);
    m2.models.clear();
    m2.models_changed();
    assert!(!kept.join("VF2_table.stl").exists());

    // a Parasolid file says what to send instead, and nothing is kept
    let xt = downloads.join("vf2.x_t");
    std::fs::write(&xt, "**PART1;\n").unwrap();
    let e = m2
        .attach_model(Some("table".into()), xt.to_str().unwrap())
        .unwrap_err();
    assert!(e.contains("STEP"), "{e}");
    assert!(m2.models.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}
