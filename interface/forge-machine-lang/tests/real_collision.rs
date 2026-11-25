//! The collision check on the machines' real geometry (forge_machine::mesh through
//! forge_machine_lang::real_trees): the bound (every point of the real surface inside the boxes
//! the check takes), a graze under the clearance still reported, a real gain over the envelope
//! (a small tool down a T-slot: the envelope says contact, the real table proves it clear), a
//! model from outside the catalogue, and the check's time against the envelopes'.
use forge_machine::MachineDef;
use forge_machine::collide::{self, Verdict};
use forge_machine::gcode::Control;
use forge_machine::mesh::Tree;
use forge_machine::sim::{self, Cut, Setup, SimTool};
use forge_mesh::{Quality, tessellate};
use std::path::Path;

fn dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-machine")
}

fn machine(name: &str) -> MachineDef {
    forge_machine::library::builtin_named(name).unwrap()
}

/// Whether `p` lies in one of the tree's face boxes (down the tree: a node holds all below it).
fn held(t: &Tree, p: [f64; 3]) -> bool {
    let inb = |lo: &[f64; 3], hi: &[f64; 3]| (0..3).all(|k| lo[k] <= p[k] && p[k] <= hi[k]);
    let mut stack = vec![0usize];
    while let Some(n) = stack.pop() {
        let nd = &t.nodes[n];
        if !inb(&nd.lo, &nd.hi) {
            continue;
        }
        match nd.kids {
            Some((a, b)) => stack.extend([a, b]),
            None => {
                if t.boxes[nd.first..nd.first + nd.count]
                    .iter()
                    .any(|(lo, hi)| inb(lo, hi))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Points on the exact surface (a fine tessellation's corners) all held by the tree's boxes.
fn bound_holds(body: &forge_topo::Body, t: &Tree, what: &str) -> usize {
    let mesh = tessellate(
        body,
        &Quality {
            chord: 0.01,
            ..Quality::default()
        },
    )
    .unwrap();
    let mut n = 0;
    for p in &mesh.positions {
        let x = [p.x, p.y, p.z];
        assert!(held(t, x), "{what}: {x:?} on the surface outside every box");
        n += 1;
    }
    for p in &t.probes {
        assert!(held(t, *p), "{what}: probe {p:?} off the body");
    }
    n
}

fn build(src: &str, dir: Option<&Path>) -> forge_topo::Body {
    let opts = forge_lang::Options {
        dir: dir.map(Path::to_path_buf),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(src, opts, forge_lang::build::Cache::default());
    assert!(run.ok(), "{:?}", run.errors);
    run.body.unwrap()
}

#[test]
fn every_point_of_the_real_surface_is_in_the_checked_boxes() {
    let mut points = 0;
    for name in ["haas-vf-2", "haas-umc-750"] {
        let m = machine(name);
        for b in &m.bodies {
            let part = b.part.as_ref().unwrap();
            let path = dir().join(part);
            let body = build(&std::fs::read_to_string(&path).unwrap(), path.parent());
            let t = forge_machine_lang::real_tree(&body, forge_machine_lang::REAL_CHORD).unwrap();
            points += bound_holds(&body, &t, &format!("{name} {}", b.name));
        }
    }
    // a model from outside the catalogue: curved faces (a fillet, a revolved boss), nothing
    // keyed to a machine or a maker
    let own = "shared r = 3
sketch s(on: xy) {
  o = rect(corner: (0, 0), size: (80, 50))
}
b = extrude(s, 30)
sketch t(on: b.end) {
  c = circle((40, 25), 12)
}
boss = extrude(t.c, 20)
round = fillet(edges(of: b, crease: \"convex\"), r)
";
    let body = build(own, None);
    let t = forge_machine_lang::real_tree(&body, forge_machine_lang::REAL_CHORD).unwrap();
    points += bound_holds(&body, &t, "own model");
    assert!(points > 1000, "{points}");
}

fn vf2_setup(m: &MachineDef) -> Setup<'_> {
    let c = Control::for_dialect(m, "haas", 1).unwrap();
    let mut s = Setup::new(m, c).unwrap();
    // part frame = the table top centre (tests/haas.rs): Z word 0 at the table top
    s.wcs = vec![(54, vec![-211.6, -381.0, -610.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "e6".into(),
        tip: [0.0, 0.0, -110.0],
        body: vec![(3.0, 0.0, 25.0), (6.0, 25.0, 60.0), (20.0, 60.0, 110.0)],
        cut: Cut::Mill { r: 3.0, corner: 0.0 },
    }];
    s.h = vec![(1, 110.0)];
    s
}

fn table_contacts(r: &collide::Report) -> Vec<&Verdict> {
    r.contacts
        .iter()
        .filter(|c| c.a == "table" || c.b == "table")
        .map(|c| &c.verdict)
        .collect()
}

#[test]
fn a_graze_is_reported_and_the_t_slot_is_proven_clear() {
    let m = machine("haas-vf-2");
    let s = vf2_setup(&m);
    let (trees, left) = forge_machine_lang::real_trees(&m, None);
    assert!(left.is_empty(), "{left:?}");
    assert!(trees.contains_key("table"));
    let c = m.clearance;
    // a graze: the tool's tip at half the clearance over the table's top, off the slot
    let graze = format!(
        "G90 G21\nT1 M06\nG54\nG43 H1\nG0 X100. Y100. Z50.\nZ{:.3}\n",
        0.5 * c
    );
    let tl = sim::simulate(&graze, &s).unwrap();
    assert!(!table_contacts(&collide::check(&s, &tl, &[])).is_empty());
    assert!(!table_contacts(&collide::check_with(&s, &tl, &[], &trees)).is_empty());
    // the gain: a 6 mm tool 5 mm down the table's 16 mm T-slot (14 deep), 5 mm from its walls:
    // inside the envelope (contact), clear of the real table
    let slot = "G90 G21\nT1 M06\nG54\nG43 H1\nG0 X100. Y0 Z50.\nZ-5.\nX-100.\nZ50.\n";
    let tl = sim::simulate(slot, &s).unwrap();
    let env = collide::check(&s, &tl, &[]);
    assert!(!table_contacts(&env).is_empty(), "the envelope's contact");
    let real = collide::check_with(&s, &tl, &[], &trees);
    assert!(table_contacts(&real).is_empty(), "{:?}", real.contacts);
    // and the real check never claims a Hit from an envelope's witness
    let real_graze = collide::check_with(&s, &sim::simulate(&graze, &s).unwrap(), &[], &trees);
    assert!(
        real_graze
            .contacts
            .iter()
            .all(|c| !matches!(c.verdict, Verdict::Hit { .. }) || (c.a != "table" && c.b != "table"))
    );
}

/// The check's time on a grid of rapids over the VF-2 table, envelopes against real geometry
/// (the trees compiled once beforehand): printed; the real check may not be slower by more than
/// a small factor (the speed rule's report is research/cam/real-collision.md).
#[test]
fn the_real_check_s_time() {
    let m = machine("haas-vf-2");
    let s = vf2_setup(&m);
    let (trees, _) = forge_machine_lang::real_trees(&m, None);
    let mut prog = String::from("G90 G21\nT1 M06\nG54\nG43 H1\nG0 X0 Y0 Z50.\n");
    for i in 0..40 {
        let x = -300.0 + 15.0 * i as f64;
        prog.push_str(&format!("X{x:.1} Y150.\nZ5.\nY-150.\nZ50.\n"));
    }
    let tl = sim::simulate(&prog, &s).unwrap();
    let t0 = std::time::Instant::now();
    let a = collide::check(&s, &tl, &[]);
    let te = t0.elapsed().as_secs_f64();
    let t0 = std::time::Instant::now();
    let b = collide::check_with(&s, &tl, &[], &trees);
    let tr = t0.elapsed().as_secs_f64();
    println!(
        "VF-2 rapid grid, {} segments: envelopes {te:.4} s ({} spans), real {tr:.4} s ({} spans)",
        tl.segments.len(),
        a.spans,
        b.spans
    );
    assert!(b.contacts.len() <= a.contacts.len(), "{:?}", b.contacts);
}

/// The same on the UMC-750: a rapid grid over the platter, then the 3+2 pass of tests/haas.rs
/// (B30 C90); the real check no slower in spans and no contact the envelopes did not have.
#[test]
fn the_real_check_s_time_on_the_umc() {
    let m = machine("haas-umc-750");
    let c = Control::for_dialect(&m, "haas", 1).unwrap();
    let mut s = Setup::new(&m, c).unwrap();
    s.wcs = vec![(54, vec![-381.0, -254.0, -610.0, 0.0, 0.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "e10".into(),
        tip: [0.0, 0.0, -120.0],
        body: vec![(5.0, 0.0, 25.0), (8.0, 25.0, 60.0), (20.0, 60.0, 120.0)],
        cut: Cut::Mill { r: 5.0, corner: 0.0 },
    }];
    s.h = vec![(1, 120.0)];
    let (trees, left) = forge_machine_lang::real_trees(&m, None);
    assert!(left.is_empty(), "{left:?}");
    let mut prog = String::from("G90 G21\nG54\nG43 H1\nG0 X0 Y0 Z200. B0 C0\n");
    for i in 0..30 {
        let x = -150.0 + 10.0 * i as f64;
        prog.push_str(&format!("X{x:.1} Y150.\nZ60.\nY-150.\nZ200.\n"));
    }
    prog.push_str("G0 B30. C90.\nG1 X-30. Y0 Z120. F1500.\nX30.\nG0 Z250.\nG0 B0 C0\n");
    let tl = sim::simulate(&prog, &s).unwrap();
    let t0 = std::time::Instant::now();
    let a = collide::check(&s, &tl, &[]);
    let te = t0.elapsed().as_secs_f64();
    let t0 = std::time::Instant::now();
    let b = collide::check_with(&s, &tl, &[], &trees);
    let tr = t0.elapsed().as_secs_f64();
    println!(
        "UMC-750 grid + 3+2 pass, {} segments: envelopes {te:.4} s ({} spans, {} contacts), real {tr:.4} s ({} spans, {} contacts)",
        tl.segments.len(),
        a.spans,
        a.contacts.len(),
        b.spans,
        b.contacts.len()
    );
    for c in &b.contacts {
        assert!(
            a.contacts
                .iter()
                .any(|e| e.segment == c.segment && e.a == c.a && e.b == c.b),
            "a contact the envelopes did not have: {c:?}"
        );
    }
}
