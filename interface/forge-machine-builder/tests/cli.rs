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
    assert!(o.out.contains(" user"), "{}", o.out);
    assert!(!o.out.contains("uncited"), "{}", o.out);
    // the cradle and head move relative to each other through X Y Z B; the check reports it,
    // never as a hit inside the corrected travel (Z [-280, 0])
    assert!(o.out.contains("cradle / head (via X Y Z B):"), "{}", o.out);
    assert!(!o.out.contains("cradle / head (via X Y Z B): HIT"), "{}", o.out);

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
