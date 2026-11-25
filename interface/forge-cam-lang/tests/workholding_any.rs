//! Workholding of any geometry is checked against its own faces (David: "make sure we do not
//! over fit to one Haas vise; collision should use any vise geometry including custom imports"):
//! - two real vises of different sizes, each against its maker's model;
//! - a fixture of the user's own that no catalogue has, imported as a STEP and as a `.fg`
//!   through `workholding("NAME", at:)`, compiled while running and cached by its content;
//! - soft jaws, whose pocket is the part's faces, and which report what still stands in;
//! - and no code path names a particular model.
mod common;
use std::path::{Path, PathBuf};

fn setup(fixture: &str) -> String {
    format!(
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: 0.01, fixture: {fixture})\nskim = facing(op1, tool: t1, to: 19.95, step: 1)\nprog = program(op1, post: \"fanuc\")\n"
    )
}

#[test]
fn two_real_vises_each_against_its_own_model() {
    let dir = common::dir();
    let mut faces = vec![];
    for (name, jaw_top_over_table) in [("haas-vise-05-0404", 117.1), ("haas-vise-05-0405", 88.52)] {
        let cam = forge_cam_lang::run(
            &setup(&format!("vise(\"{name}\", across: \"x\", clamp: 15)")),
            &dir,
        );
        assert!(cam.ok, "{name}: {:?}", cam.errors);
        let f = cam.setup_view(0).unwrap().fixture.unwrap();
        assert!(f.enveloped.is_empty(), "{name}: {:?}", f.enveloped);
        // every vise body is its model's faces; only the job's parallels are boxes
        assert!(f.pieces.iter().all(|p| p.0.starts_with("parallel")), "{name}");
        let p = f.placed.as_ref().unwrap();
        assert!(
            p.bodies
                .iter()
                .filter(|b| !b.name.starts_with("parallel"))
                .all(|b| b.exact.is_some())
        );
        // each stands on the table at its own height under the jaws' top
        assert!(
            (f.jaw_top - f.table_z.unwrap() - jaw_top_over_table).abs() < 0.05,
            "{name}: {} over the table",
            f.jaw_top - f.table_z.unwrap()
        );
        faces.push(f.faces().len());
    }
    // two different bodies, not one model reused
    assert_ne!(faces[0], faces[1], "{faces:?}");
}

/// A project of its own: the flange, and a fixture plate no catalogue has, as `.fg` and as the
/// STEP that `.fg` exports to (a user's import).
fn project(tag: &str) -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forge-cam-hold-{tag}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("cam/workholding")).unwrap();
    std::fs::copy(common::dir().join("flange.fg"), d.join("flange.fg")).unwrap();
    // a plate 400 square, 20 thick, its top at z = 0, a bolt hole through it
    let plate = "shared L = 400\nshared W = 400\nshared T = 20\nparam bolt.d = 12\nparam top.hx = 150\n\nsketch plan(on: xy) {\n  outline = rect(corner: (-L / 2, -W / 2), size: (L, W))\n}\nplate = extrude(plan, T)\nsketch top(on: plate.end) {\n  h = pt(hx, hx)\n}\nbolt = hole(at: top.h, d: d, depth: thru)\n";
    std::fs::write(d.join("plate.fg"), plate).unwrap();
    let (run, _) = forge_lang::run(plate, forge_lang::Options::default(), Default::default());
    assert!(run.ok(), "{:?}", run.errors);
    let step = forge_step::export(run.body.as_ref().unwrap(), &forge_step::Options::default()).unwrap();
    std::fs::write(d.join("my-plate.step"), step.text).unwrap();
    std::fs::write(
        d.join("cam/workholding/mine.toml"),
        "[[workholding]]\nname = \"my-plate\"\nkind = \"custom\"\nvendor = \"me\"\nproduct = \"a plate of my own\"\nmodel_file = \"my-plate.step\"\n\n[[workholding]]\nname = \"my-plate-fg\"\nkind = \"custom\"\nvendor = \"me\"\nproduct = \"the same plate, as its part\"\nmodel_file = \"plate.fg\"\n",
    )
    .unwrap();
    d
}

#[test]
fn a_fixture_of_the_users_own_is_checked_against_its_faces() {
    let d = project("own");
    let mut counts = vec![];
    for name in ["my-plate", "my-plate-fg"] {
        // the plate under the stock (the stock's underside is at z = 0 in the part frame)
        let cam = forge_cam_lang::run(&setup(&format!("workholding(\"{name}\", at: (0, 0, -20))")), &d);
        assert!(cam.ok, "{name}: {:?} {:?}", cam.errors, cam.warnings);
        let f = cam.setup_view(0).unwrap().fixture.unwrap();
        assert!(f.enveloped.is_empty(), "{name}: {:?}", f.enveloped);
        assert!(f.pieces.is_empty(), "{name}: boxes {:?}", f.pieces);
        let n = f.faces().len();
        // a plate with a hole: more than a box's six faces, the hole's wall among them
        assert!(n > 6, "{name}: {n}");
        assert!(
            (f.table_z.unwrap() + 20.0).abs() < 0.05,
            "{name}: {:?}",
            f.table_z
        );
        counts.push(n);
    }
    // the STEP and the .fg it came from are the same plate
    assert_eq!(counts[0], counts[1]);
    // compiled once per content: the cache beside the document, read on the next run
    let cache: Vec<_> = std::fs::read_dir(d.join("cam/workholding/.compiled"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!cache.is_empty());
    let again = forge_cam_lang::run(&setup("workholding(\"my-plate\", at: (0, 0, -20))"), &d);
    assert!(again.ok);
    assert_eq!(
        again.setup_view(0).unwrap().fixture.unwrap().faces().len(),
        counts[0]
    );
    // a plate that reaches up into the cut is a collision found against its real faces
    let hit = forge_cam_lang::run(&setup("workholding(\"my-plate\", at: (0, 0, 5))"), &d);
    assert!(!hit.ok, "a plate across the cut certified: {:?}", hit.warnings);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn soft_jaws_report_what_stands_in_for_a_model_vise() {
    let dir = common::dir();
    let src = std::fs::read_to_string(dir.join("bracket_cam.fg")).unwrap();
    let line = "jaws = soft_jaw(vise(\"generic-vise-150\", across: \"y\")";
    assert!(src.contains(line));
    let doc = |vise: &str| src.replace(line, &format!("jaws = soft_jaw(vise(\"{vise}\", across: \"y\")"));
    // on a stand-in (no model) the jaws and the vise under them are its own geometry: nothing
    // stands in for anything
    let cam = forge_cam_lang::run(&doc("standin-vise-150"), &dir);
    assert!(cam.ok, "{:?}", cam.errors);
    for k in [1, 2] {
        let f = cam.setup_view(k).unwrap().fixture.unwrap();
        assert!(f.enveloped.is_empty(), "setup {k}: {:?}", f.enveloped);
    }
    // under a vise with a model the soft-jaw path still stands its sizes' boxes: each such body
    // is named in `enveloped`, and the document is told
    let cam = forge_cam_lang::run(&doc("haas-vise-05-0404"), &dir);
    assert!(cam.ok, "{:?}", cam.errors);
    for k in [1, 2] {
        let f = cam.setup_view(k).unwrap().fixture.unwrap();
        assert!(
            !f.enveloped.is_empty() && f.enveloped.iter().all(|e| e.contains("haas-05-0404")),
            "setup {k}: {:?}",
            f.enveloped
        );
    }
    assert!(
        cam.warnings
            .iter()
            .any(|w| w.contains("soft jaws on a model vise")),
        "{:?}",
        cam.warnings
    );
}

/// The source files that place, compile or check workholding, each up to its tests.
fn sources() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = vec![];
    for d in [
        "kernel/forge-workholding/src",
        "kernel/forge-cam/src",
        "interface/forge-cam-lang/src",
        "interface/forge-workholding-builder/src",
        "interface/forge-render/src",
    ] {
        let mut stack = vec![root.join(d)];
        while let Some(p) = stack.pop() {
            for e in std::fs::read_dir(&p).unwrap().flatten() {
                let q = e.path();
                if q.is_dir() {
                    stack.push(q);
                } else if q.extension().is_some_and(|x| x == "rs") {
                    let t = std::fs::read_to_string(&q).unwrap();
                    let code = t.split("#[cfg(test)]").next().unwrap_or("").to_string();
                    out.push((q, code));
                }
            }
        }
    }
    out
}

#[test]
fn no_code_path_names_a_particular_model() {
    let lib = forge_workholding::Library::builtin();
    let mut names: Vec<String> = forge_workholding::model::builtin()
        .iter()
        .map(|m| m.name.clone())
        .collect();
    for (r, _) in &lib.records {
        if r.words.contains_key("model") || r.words.contains_key("replaces") {
            names.push(r.name.clone());
        }
        if let Some(x) = r.words.get("replaces") {
            names.push(x.clone());
        }
    }
    assert!(names.len() >= 4, "{names:?}");
    let src = sources();
    assert!(src.len() > 100, "{}", src.len());
    for (p, code) in &src {
        for n in &names {
            assert!(
                !code.contains(&format!("\"{n}\"")),
                "{} names `{n}`: the data says which model, the code must not",
                p.display()
            );
        }
        // nor a Haas part number written into code
        assert!(
            !code.contains("05-04"),
            "{} names a Haas part number",
            p.display()
        );
    }
}
