//! The machine builder's commands end to end: a machine from a template and one cloned from the
//! library, saved at the project and user levels, listed with shadowing, checked (envelopes
//! with the real parts), byte-stable on disk, and used by a setup's simulation.
use forge_machine::def::{CiteStatus, MachineDef};
use forge_machine_builder::catalog::Level;
use forge_machine_builder::check::EnvelopeCheck;
use forge_machine_builder::cli::{self, Env};
use forge_machine_builder::write;
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("fmb-{name}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn envelopes(def: &MachineDef, dir: &Path) -> Vec<(String, EnvelopeCheck)> {
    forge_machine_lang::envelope_checks(def, dir)
}

fn env(root: &Path) -> Env<'static> {
    Env {
        project: root.join("proj"),
        user: Some(root.join("user")),
        today: "2026-11-23".into(),
        envelopes: &envelopes,
    }
}

fn run(e: &Env, args: &[&str]) -> cli::Out {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    cli::run(e, &a).expect("a builder command")
}

#[test]
fn template_clone_list_check_and_simulate() {
    let root = tmp("e2e");
    let e = env(&root);

    // from a template, at the project level: every value a cited placeholder
    let o = run(&e, &["new", "shop-vmc", "--template", "vmc3", "--project"]);
    assert_eq!(o.code, 0, "{}", o.err);
    let f = root.join("proj/cam/machines/shop-vmc.toml");
    let text = std::fs::read_to_string(&f).unwrap();
    let m = MachineDef::parse_all(&text).unwrap().remove(0);
    assert!(!m.real);
    for p in m.cite_paths() {
        let c = m
            .cites
            .iter()
            .find(|c| c.path == p)
            .unwrap_or_else(|| panic!("{p} uncited"));
        assert_eq!(c.status, CiteStatus::Unknown, "{p}");
    }
    // byte-stable: what was written is what the writer writes for what it reads
    let header = "shop-vmc — written by forgecad's machine builder (docs/cam-machine.md).";
    assert_eq!(write::file(header, std::slice::from_ref(&m)).unwrap(), text);
    // its parts came along and lie in their envelopes
    assert!(root.join("proj/cam/machines/parts/shop-vmc/head.fg").is_file());
    let o = run(&e, &["check", "shop-vmc", "--json"]);
    assert_eq!(o.code, 0, "{}", o.out);
    assert!(o.out.contains("\"level\":\"project\""), "{}", o.out);
    assert!(o.out.contains("\"status\":\"inside\""), "{}", o.out);
    assert!(!o.out.contains("\"status\":\"outside\""), "{}", o.out);
    assert!(!o.out.contains("\"travel\":\"hit\""), "{}", o.out);

    // a clone at the user level, values taken as the user's own
    let o = run(
        &e,
        &["new", "my-trunnion", "--from", "generic-trunnion-bc", "--own"],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    let o = run(&e, &["check", "my-trunnion"]);
    assert!(o.out.contains("(user, "), "{}", o.out);
    assert!(o.out.contains("provenance: 37 user"), "{}", o.out);
    assert!(!o.out.contains("uncited"), "{}", o.out);
    // the generic trunnion's Z travel stops above its cradle (cam/machine-train4: Z [-280, 0]):
    // the pair is swept, its lines clear, and no HIT
    assert!(o.out.contains("cradle / head (via X Y Z B): "), "{}", o.out);
    assert!(!o.out.contains("HIT"), "{}", o.out);
    assert_eq!(o.code, 0, "{}", o.out);

    // shadowing: a user machine named like a built-in hides it, and list says so
    let o = run(&e, &["new", "generic-vmc3", "--from", "generic-vmc3"]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("this hides the built-in machine"), "{}", o.out);
    let o = run(&e, &["list"]);
    assert_eq!(o.code, 0, "{}", o.out);
    let lines: Vec<&str> = o.out.lines().collect();
    assert!(
        lines[0].starts_with("shop-vmc ") && lines[0].contains("project"),
        "{}",
        o.out
    );
    assert!(
        o.out.contains("hides the built-in one in builtin/generic.toml"),
        "{}",
        o.out
    );
    let o = run(&e, &["list", "--json"]);
    assert!(
        o.out
            .contains("\"name\":\"my-trunnion\",\"kind\":\"table-table\""),
        "{}",
        o.out
    );
    // refusals: same name twice, bad name, unknown source
    assert_eq!(
        run(&e, &["new", "shop-vmc", "--template", "vmc3", "--project"]).code,
        1
    );
    assert_eq!(run(&e, &["new", "../evil", "--template", "vmc3"]).code, 1);
    assert_eq!(run(&e, &["new", "x", "--from", "nope"]).code, 2);

    // the project's machine in a setup's simulation (examples/machine/vmc_slot on shop-vmc)
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machine");
    let setup = std::fs::read_to_string(ex.join("vmc_slot.toml"))
        .unwrap()
        .replace("machine = \"generic-vmc3\"", "machine = \"shop-vmc\"");
    std::fs::write(root.join("proj/slot.toml"), &setup).unwrap();
    std::fs::copy(ex.join("vmc_slot.nc"), root.join("proj/vmc_slot.nc")).unwrap();
    let o = forge_machine_lang::run_setup(&root.join("proj/slot.toml")).unwrap();
    assert!(o.ok, "{}", o.text);
    assert!(o.text.contains("on shop-vmc"), "{}", o.text);

    // the user's own value changes the verdict: X travel cut to ±10 mm overtravels the slot
    let mut m2 = m.clone();
    let x = m2.joint_index("X").unwrap();
    m2.joints[x].range = Some([-10.0, 10.0]);
    m2.cites.retain(|c| c.path != "joint.X.range");
    std::fs::write(&f, write::file(header, &[m2]).unwrap()).unwrap();
    let o = forge_machine_lang::run_setup(&root.join("proj/slot.toml")).unwrap();
    assert!(!o.ok);
    assert!(o.text.to_lowercase().contains("overtravel"), "{}", o.text);
    let o = run(&e, &["check", "shop-vmc", "--json"]);
    assert!(
        o.out
            .contains("{\"path\":\"joint.X.range\",\"status\":[\"user\"]"),
        "{}",
        o.out
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn every_template_makes_a_valid_machine() {
    let root = tmp("templates");
    let e = env(&root);
    for t in forge_machine_builder::make::Template::ALL {
        let name = format!("t-{}", t.word());
        let o = run(&e, &["new", &name, "--template", t.word(), "--project"]);
        assert_eq!(o.code, 0, "{}: {}", t.word(), o.err);
        let o = run(&e, &["check", &name]);
        assert!(o.out.contains("structure: ok"), "{}", o.out);
        assert!(o.out.contains("home pose: every pair clear"), "{}", o.out);
    }
    let cat = e.catalog();
    assert_eq!(
        cat.entries.iter().filter(|x| x.level == Level::Project).count(),
        forge_machine_builder::make::Template::ALL.len()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_broken_file_is_listed_not_hiding_the_rest() {
    let root = tmp("broken");
    let e = env(&root);
    std::fs::create_dir_all(root.join("user")).unwrap();
    std::fs::write(root.join("user/bad.toml"), "[[machine]]\nname = \"half\"\n").unwrap();
    let o = run(&e, &["list"]);
    assert_eq!(o.code, 1);
    assert!(o.out.contains("generic-vmc3"), "{}", o.out);
    assert!(o.out.contains("NOT LOADED (user)"), "{}", o.out);
    let o = run(&e, &["check", root.join("user/bad.toml").to_str().unwrap()]);
    assert_eq!(o.code, 1);
    assert!(o.out.contains("structure: REFUSED"), "{}", o.out);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dates() {
    assert_eq!(cli::date_of_days(0), "1970-01-01");
    assert_eq!(cli::date_of_days(20_780), "2026-11-23");
    assert_eq!(cli::date_of_days(11_016), "2000-02-29");
}

/// `machine model ...`: a "downloaded" whole-machine STL (two separate parts, in inches) attached
/// with the user's licence note, split so its parts follow haas-vf-2's table and head, checked,
/// listed and removed; nothing written into the project; an empty note and an X_T refused.
#[test]
fn model_attach_split_check_list_remove() {
    use forge_machine_builder::model_files::Piece;
    let root = tmp("models");
    let e = env(&root);
    std::fs::create_dir_all(root.join("proj")).unwrap();
    let cube = |lo: [f64; 3], hi: [f64; 3]| {
        let c =
            |k: usize| -> [f64; 3] { std::array::from_fn(|a| if k >> a & 1 == 1 { hi[a] } else { lo[a] }) };
        let faces = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        Piece {
            pos: (0..8).map(c).collect(),
            tri: faces
                .iter()
                .flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]])
                .collect(),
            name: String::new(),
        }
    };
    // a 36 x 14 x 3.5 in table and a smaller head well above it, in one file, under an
    // enclosure a whole machine's size (a download's size says its units against the machine's)
    let stl = Piece::join(&[
        cube([0.0; 3], [36.0, 14.0, 3.5]),
        cube([10.0, 4.0, 20.0], [20.0, 10.0, 30.0]),
        cube([-30.0, -30.0, -36.0], [66.0, 44.0, 64.0]),
    ]);
    let dl = root.join("downloads");
    std::fs::create_dir_all(&dl).unwrap();
    std::fs::write(dl.join("VF-2.stl"), stl.to_stl("download")).unwrap();
    let file = dl.join("VF-2.stl");
    let file = file.to_str().unwrap();

    let o = run(&e, &["model", "attach", "haas-vf-2", file, "--from", " "]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Haas download, my licence"), "{}", o.err);
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "haas-vf-2",
            file,
            "--from",
            "Haas download, my licence",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(
        o.out.contains("inches") && o.out.contains("3 separate parts"),
        "{}",
        o.out
    );
    let kept = root.join("user/models/haas-vf-2");
    assert!(kept.join("VF-2.stl").is_file());
    assert!(!root.join("proj/cam").exists(), "nothing in the project");

    // its parts, largest first; then the table's and the head's
    let o = run(&e, &["model", "split", "haas-vf-2", "VF-2.stl"]);
    assert!(
        o.out.starts_with("part 1: 12 triangles, 36.0 x 14.0 x 3.5")
            && o.out.contains("part 3: 12 triangles, 96.0 x 74.0 x 100.0"),
        "{}",
        o.out
    );
    let o = run(&e, &["model", "split", "haas-vf-2", "VF-2.stl", "table", "nope"]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("no body `nope`"), "{}", o.err);
    // by name: an STL's parts have none, so `*` is the one rule that names them
    let o = run(&e, &["model", "split", "haas-vf-2", "VF-2.stl", "*=table", "head"]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("PART=BODY"), "{}", o.err);
    let o = run(
        &e,
        &["model", "split", "haas-vf-2", "VF-2.stl", "table", "head", "-"],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(!kept.join("VF-2.stl").exists(), "the whole file goes once split");
    let o = run(&e, &["model", "list", "haas-vf-2", "--json"]);
    assert!(o.out.contains("\"file\":\"VF-2-table.stl\",\"body\":\"table\",\"from\":\"Haas download, my licence\",\"scale\":25.4"), "{}", o.out);
    assert!(o.out.contains("\"body\":\"head\""), "{}", o.out);

    // checked against the envelopes: placed for the whole machine, they need not fit each body
    let o = run(&e, &["model", "check", "haas-vf-2"]);
    assert!(o.out.contains("VF-2-table.stl (table): "), "{}", o.out);
    // a model given its own offset and scale
    let o = run(&e, &["model", "remove", "haas-vf-2", "VF-2-head.stl"]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(!kept.join("VF-2-head.stl").exists());
    std::fs::write(dl.join("head.stl"), cube([0.0; 3], [10.0; 3]).to_stl("h")).unwrap();
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "haas-vf-2",
            dl.join("head.stl").to_str().unwrap(),
            "--body",
            "head",
            "--from",
            "my own drawing",
            "--scale",
            "1",
            "--rotate",
            "0,0,90",
            "--offset",
            "1,2,3",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    let o = run(&e, &["model", "list", "haas-vf-2", "--json"]);
    assert!(o.out.contains("\"file\":\"head.stl\",\"body\":\"head\",\"from\":\"my own drawing\",\"scale\":1,\"rotate\":[0,0,90],\"offset\":[1,2,3]"), "{}", o.out);
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "haas-vf-2",
            dl.join("head.stl").to_str().unwrap(),
            "--body",
            "head",
            "--from",
            "x",
        ],
    );
    assert_eq!(o.code, 1, "one model per body");

    std::fs::write(dl.join("vf2.x_t"), "**PART1;").unwrap();
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "haas-vf-2",
            dl.join("vf2.x_t").to_str().unwrap(),
            "--from",
            "x",
        ],
    );
    assert_eq!(o.code, 1);
    assert!(o.err.contains("STEP"), "{}", o.err);
    let _ = std::fs::remove_dir_all(&root);
}

/// A synthetic machine download (CAM-HAAS): a four-part STL split onto a machine's bed and its X,
/// Y and Z bodies, then a STEP with two solids split the same way; a display-only body's report
/// labelled and not a failure, a checked body's still one.
#[test]
fn model_split_onto_axes_step_solids_and_display_only() {
    use forge_machine_builder::model_files::{self as mf, Piece};
    let root = tmp("models-axes");
    let e = env(&root);
    std::fs::create_dir_all(root.join("proj")).unwrap();
    // a user machine from the generic VMC, with a display-only enclosure added
    let o = run(&e, &["new", "shop", "--from", "generic-vmc3", "--user"]);
    assert_eq!(o.code, 0, "{}", o.err);
    let f = root.join("user/shop.toml");
    let mut text = std::fs::read_to_string(&f).unwrap();
    text.push_str(
        "\n[[machine.body]]\nname = \"enclosure\"\nlink = \"base\"\ncheck = false\n[[machine.body.box]]\nmin = [-10, -10, -10]\nmax = [10, 10, 10]\n",
    );
    std::fs::write(&f, text).unwrap();

    let cube = |lo: [f64; 3], hi: [f64; 3]| {
        let c =
            |k: usize| -> [f64; 3] { std::array::from_fn(|a| if k >> a & 1 == 1 { hi[a] } else { lo[a] }) };
        let faces = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        Piece {
            pos: (0..8).map(c).collect(),
            tri: faces
                .iter()
                .flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]])
                .collect(),
            name: String::new(),
        }
    };
    // four parts, sizes in falling order: bed, saddle, table, head
    let four = Piece::join(&[
        cube([0.0; 3], [1000.0, 900.0, 150.0]),
        cube([0.0, 0.0, 200.0], [800.0, 500.0, 300.0]),
        cube([0.0, 0.0, 400.0], [700.0, 300.0, 450.0]),
        cube([0.0, 0.0, 600.0], [300.0, 300.0, 700.0]),
    ]);
    let dl = root.join("downloads");
    std::fs::create_dir_all(&dl).unwrap();
    std::fs::write(dl.join("vmc.stl"), four.to_stl("vmc")).unwrap();
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "shop",
            dl.join("vmc.stl").to_str().unwrap(),
            "--from",
            "a test download",
            "--scale",
            "1",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("4 separate parts"), "{}", o.out);
    let o = run(&e, &["model", "split", "shop", "vmc.stl"]);
    assert_eq!(o.out.lines().count(), 4, "{}", o.out);
    let o = run(
        &e,
        &[
            "model", "split", "shop", "vmc.stl", "bed", "saddle", "table", "head",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    let ms = forge_machine_builder::models::load_in(&root.join("user/models/shop")).unwrap();
    let mut bodies: Vec<_> = ms.iter().filter_map(|m| m.body.clone()).collect();
    bodies.sort();
    assert_eq!(bodies, ["bed", "head", "saddle", "table"]);
    // the same placement for each part, so they sit where the download had them
    assert!(
        ms.windows(2)
            .all(|w| w[0].offset == w[1].offset && w[0].scale == w[1].scale)
    );

    // a STEP with two solids: a solid each
    let src = "sketch a(on: xy) {\n  r = rect(corner: (0, 0), size: (40, 20))\n}\nlo = extrude(a, 10)\nsketch b(on: xy) {\n  r = rect(corner: (100, 0), size: (10, 10))\n}\nhi = extrude(b, 30)\n";
    let (run_, _) = forge_lang::run(
        src,
        forge_lang::Options::default(),
        forge_lang::build::Cache::default(),
    );
    assert!(run_.ok(), "{:?}", run_.errors.first());
    let step = forge_step::export(&run_.body.unwrap(), &forge_step::Options::default()).unwrap();
    let parts = mf::read("two.step", step.text.as_bytes()).unwrap();
    assert_eq!(parts.len(), 2, "a STEP solid each");

    // display only: a model sticking out of the enclosure's envelope is labelled, not a failure
    std::fs::write(dl.join("shell.stl"), cube([0.0; 3], [500.0; 3]).to_stl("shell")).unwrap();
    let o = run(
        &e,
        &[
            "model",
            "attach",
            "shop",
            dl.join("shell.stl").to_str().unwrap(),
            "--body",
            "enclosure",
            "--from",
            "my drawing",
            "--scale",
            "1",
            "--offset",
            "0,0,0",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    let o = run(&e, &["model", "remove", "shop", "vmc-head.stl"]);
    assert_eq!(o.code, 0, "{}", o.err);
    let o = run(&e, &["model", "check", "shop"]);
    assert!(
        o.out.contains("shell.stl (enclosure): outside its envelope") && o.out.contains(mf::DISPLAY_ONLY),
        "{}",
        o.out
    );
    // the parts placed for the whole machine stick out of the checked bodies' envelopes: exit 1
    assert_eq!(o.code, 1, "{}", o.out);
    for f in ["vmc-bed.stl", "vmc-saddle.stl", "vmc-table.stl"] {
        let o = run(&e, &["model", "remove", "shop", f]);
        assert_eq!(o.code, 0, "{}", o.err);
    }
    let o = run(&e, &["model", "check", "shop"]);
    assert_eq!(o.code, 0, "display only alone is not a failure: {}", o.out);
    let _ = std::fs::remove_dir_all(&root);
}
