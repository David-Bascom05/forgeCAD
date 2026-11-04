//! iProperties and materials: `properties(..)` read and checked, mass from the library's
//! density against the hand calculation, unknown materials with their nearest names as fixes,
//! SetProperty, multi-body and sheet materials, and an assembly's mass and centre of gravity.

use forge_lang::edit::{apply, apply_ast};
use forge_lang::error::{Error, Op};
use forge_lang::props::{Tab, asm_physical};
use forge_lang::{Options, Run, syntax};
use std::path::{Path, PathBuf};

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        render: true,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("iprops-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

const BLOCK: &str = "shared a = 10
shared b = 20
shared c = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (a, b))
}
block = extrude(plan, c)
";

fn kinds(r: &Run) -> Vec<&'static str> {
    r.errors.iter().map(Error::kind).collect()
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-12)
}

#[test]
fn a_block_of_6061_weighs_what_the_hand_calculation_says() {
    let src = format!(
        "properties(part_number: \"P-1\", description: \"block\", material: \"6061-T6\", estimated_cost: 12.5, creation_date: \"2026-11-04\", design_state: \"Pending\", custom: [(\"Finish\", \"anodised\")])\n{BLOCK}"
    );
    let r = run(&src);
    assert!(r.errors.is_empty(), "{:?}", kinds(&r));
    assert!(
        r.warnings.is_empty(),
        "the numbers in properties are no design numbers"
    );
    assert_eq!(r.props.get("part_number"), Some("P-1"));
    assert_eq!(r.props.get("estimated_cost"), Some("12.5"));
    assert_eq!(r.props.custom, [("Finish".to_string(), "anodised".to_string())]);
    assert!(r.props.title_block().contains(&("Material", "6061-T6".into())));
    assert!(
        forge_lang::props::FIELDS
            .iter()
            .any(|f| f.0 == "design_state" && f.2 == Tab::Status)
    );
    let p = r.physical.expect("a built part's physical properties");
    let rho = forge_material::find("6061-T6").unwrap().density;
    // 10 x 20 x 30 mm: 6000 mm^3
    let m = 6000.0 * rho * 1e-9;
    assert!(close(p.mass.unwrap(), m, 1e-9), "{:?} {m}", p.mass);
    assert!(close(p.volume, 6000.0, 1e-9));
    assert!(close(p.area, 2.0 * (200.0 + 300.0 + 600.0), 1e-9));
    for (k, want) in [5.0, 10.0, 15.0].into_iter().enumerate() {
        assert!(close(p.centroid[k], want, 1e-9), "{:?}", p.centroid);
    }
    // a box's moments about its centre: m (b^2 + c^2) / 12 ..
    let i = p.inertia.unwrap();
    let want = [
        m * (400.0 + 900.0) / 12.0,
        m * (100.0 + 900.0) / 12.0,
        m * (100.0 + 400.0) / 12.0,
    ];
    for k in 0..3 {
        assert!(close(i[k][k], want[k], 1e-7), "{i:?} {want:?}");
    }
    assert!(i[0][1].abs() < 1e-9 * want[0]);
    // .. and about the origin, by the parallel axis theorem
    let o = p.origin_inertia.unwrap();
    assert!(close(o[2][2], want[2] + m * (25.0 + 100.0), 1e-7));
    assert!(close(o[0][1], -m * 5.0 * 10.0, 1e-7), "{o:?}");
    let (pm, _) = p.principal.unwrap();
    assert!(
        close(pm[0], want[2], 1e-7) && close(pm[2], want[0], 1e-7),
        "{pm:?}"
    );
}

#[test]
fn no_material_gives_volume_and_centroid_but_no_mass() {
    let r = run(BLOCK);
    let p = r.physical.unwrap();
    assert_eq!(p.mass, None);
    assert_eq!(p.missing, ["block"]);
    assert!(close(p.centroid[2], 15.0, 1e-9));
    assert!(p.inertia.is_none());
}

#[test]
fn an_unknown_material_names_the_nearest_and_fixes_to_them() {
    let r = run(&format!("properties(material: \"6061-T5\")\n{BLOCK}"));
    let e = r
        .errors
        .iter()
        .find(|e| e.kind() == "UnknownMaterial")
        .expect("UnknownMaterial");
    let fixes = e.fixes();
    let fix = fixes
        .iter()
        .find(|f| matches!(f, Op::SetProperty { text: Some(t), .. } if t == "\"6061-T6\""))
        .expect("a fix to 6061-T6");
    let src = format!("properties(material: \"6061-T5\")\n{BLOCK}");
    let fixed = apply(&src, fix).unwrap();
    assert!(run(&fixed).errors.is_empty());
    assert!(e.text().contains("6061-T6"), "{}", e.text());
}

#[test]
fn set_property_puts_in_replaces_and_takes_out() {
    let ops = [
        Op::SetProperty {
            key: "material".into(),
            text: Some("\"AISI 304\"".into()),
        },
        Op::SetProperty {
            key: "part_number".into(),
            text: Some("\"RACK-101\"".into()),
        },
        Op::SetProperty {
            key: "material".into(),
            text: Some("\"AISI 1018\"".into()),
        },
        Op::SetProperty {
            key: "material".into(),
            text: None,
        },
    ];
    let mut src = format!("# a block\n\n{BLOCK}");
    let mut texts = vec![];
    for op in &ops {
        let next = apply(&src, op).unwrap();
        assert_eq!(syntax::parse(&next).unwrap(), apply_ast(&src, op).unwrap());
        src = next;
        texts.push(src.clone());
    }
    assert!(
        texts[0].starts_with("# a block\n\nproperties(material: \"AISI 304\")\nshared a"),
        "{}",
        texts[0]
    );
    assert!(texts[1].contains("properties(material: \"AISI 304\", part_number: \"RACK-101\")"));
    assert!(texts[2].contains("properties(material: \"AISI 1018\", part_number: \"RACK-101\")"));
    assert!(
        texts[3].contains("properties(part_number: \"RACK-101\")"),
        "{}",
        texts[3]
    );
    let r = run(&texts[2]);
    assert!(r.errors.is_empty(), "{:?}", kinds(&r));
    assert_eq!(r.props.material.as_deref(), Some("1018 cold drawn"));
    // only the arguments properties takes, and only values
    assert!(
        apply(
            &src,
            &Op::SetProperty {
                key: "colour".into(),
                text: Some("\"x\"".into())
            }
        )
        .is_err()
    );
    assert!(
        apply(
            &src,
            &Op::SetProperty {
                key: "title".into(),
                text: Some("a".into())
            }
        )
        .is_err()
    );
    // the op as JSON and back
    let j = ops[0].json();
    assert_eq!(Op::from_json(&j), Some(ops[0].clone()));
}

#[test]
fn properties_are_checked() {
    let bad = |args: &str| {
        let r = forge_lang::run(
            &format!("properties({args})\n{BLOCK}"),
            Options {
                build: false,
                ..Options::default()
            },
            Default::default(),
        )
        .0;
        kinds(&r)
    };
    assert_eq!(bad("creation_date: \"4 Nov\""), ["BadDate"]);
    assert_eq!(bad("design_state: \"Done\""), ["BadDesignState"]);
    assert_eq!(bad("title: 3"), ["WrongType"]);
    assert_eq!(bad("colour: \"red\""), ["UnknownArg"]);
    let twice = run(&format!(
        "properties(title: \"a\")\nproperties(title: \"b\")\n{BLOCK}"
    ));
    assert!(kinds(&twice).contains(&"PropertiesTwice"));
    let bound = run(&format!("p = properties(title: \"a\")\n{BLOCK}"));
    assert!(kinds(&bound).contains(&"PropertiesNotTopLevel"));
}

#[test]
fn each_body_weighs_by_its_own_material() {
    let src = "properties(material: \"AISI 1018\", bodies: {knob: \"ABS\"})
shared a = 10
shared h = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (a, a))
}
base = extrude(plan, a)
sketch top(on: base.end) {
  c = rect(corner: (0, 0), size: (a, a))
}
knob = extrude(top.c, h, new: true)
";
    let r = run(src);
    assert!(r.errors.is_empty(), "{:?}", kinds(&r));
    let p = r.physical.unwrap();
    let rho = |n: &str| forge_material::find(n).unwrap().density * 1e-9;
    let (m1, m2) = (1000.0 * rho("AISI 1018"), 2000.0 * rho("ABS"));
    assert_eq!(p.solids.len(), 2);
    assert!(close(p.mass.unwrap(), m1 + m2, 1e-9));
    // the centre of gravity weighs the steel base, not the volume
    let z = (m1 * 5.0 + m2 * 20.0) / (m1 + m2);
    assert!(close(p.centroid[2], z, 1e-9), "{:?} {z}", p.centroid);
    let wrong = run(&src.replace("knob: \"ABS\"", "lever: \"ABS\""));
    assert!(kinds(&wrong).contains(&"NoSuchBody"));
}

#[test]
fn a_sheet_material_is_a_library_material() {
    let sheet = |m: &str| {
        format!(
            "shared t = 1.5
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (40, 30))
}}
base = sheet(plan, t: t, r: 2, bend: kfactor(ratio(0.4)), material: \"{m}\")
"
        )
    };
    let r = run(&sheet("1008 CR steel"));
    assert!(r.errors.is_empty(), "{:?}", kinds(&r));
    let p = r.physical.unwrap();
    let rho = forge_material::find("1008 CR").unwrap().density;
    assert!(close(p.mass.unwrap(), 40.0 * 30.0 * 1.5 * rho * 1e-9, 1e-9));
    // the drawing's note keeps the name as written
    assert!(r.sheets[0].note().contains("1008 CR STEEL"));
    let bad = run(&sheet("1008 CR stel"));
    let e = bad.errors.iter().find(|e| e.kind() == "UnknownMaterial").unwrap();
    assert!(
        e.fixes()
            .iter()
            .any(|f| matches!(f, Op::SetArg { id, arg, .. } if id == "base" && arg == "material"))
    );
    // a sheet naming no stock is the part's material, in its note too
    let plain = sheet("x").replace(", material: \"x\"", "");
    let r = run(&format!("properties(material: \"5052-H32\")\n{plain}"));
    assert!(r.sheets[0].note().contains("5052-H32"), "{}", r.sheets[0].note());
}

#[test]
fn an_assembly_weighs_its_occurrences_at_their_poses() {
    let d = dir("asm");
    std::fs::write(
        d.join("block.fg"),
        format!("properties(material: \"AISI 304\", part_number: \"B-1\")\n{BLOCK}"),
    )
    .unwrap();
    let asm = "assembly
properties(title: \"two blocks\")
a = part(\"block.fg\", ground: true)
b = part(\"block.fg\", at: pose(at: (100, 0, 0), z: (0, 0, 1), x: (0, 1, 0)))
c = part(\"block.fg\", at: (0, 0, 500), reference: true)
";
    let r = run_in(&d, asm);
    let out = r.asm.as_ref().expect("an assembly");
    assert_eq!(r.props.get("title"), Some("two blocks"));
    let doc = out.part_docs.values().next().unwrap();
    assert_eq!(doc.props.get("part_number"), Some("B-1"));
    let p = asm_physical(out, None).unwrap();
    let m = 6000.0 * forge_material::find("AISI 304").unwrap().density * 1e-9;
    // the reference block counts nothing
    assert!(close(p.mass.unwrap(), 2.0 * m, 1e-9), "{:?}", p.mass);
    // a at (5, 10, 15); b turned a quarter about z and moved: its centre at (100 - 10, 5, 15)
    let want = [(5.0 + 90.0) / 2.0, (10.0 + 5.0) / 2.0, 15.0];
    for k in 0..3 {
        assert!(close(p.centroid[k], want[k], 1e-9), "{:?} {want:?}", p.centroid);
    }
    // b's own moments turn with it: about z, unchanged; its share about x is a's about y
    let own = |a: f64, b: f64| m * (a * a + b * b) / 12.0;
    let i = p.inertia.unwrap();
    let ixx = own(20.0, 30.0) + own(10.0, 30.0) + 2.0 * m * 2.5 * 2.5;
    assert!(close(i[0][0], ixx, 1e-7), "{} {ixx}", i[0][0]);
    let only_b = asm_physical(out, Some("b")).unwrap();
    assert!(close(only_b.mass.unwrap(), m, 1e-9));
    let mat = run_in(
        &d,
        &asm.replace("title: \"two blocks\"", "material: \"AISI 304\""),
    );
    assert!(kinds(&mat).contains(&"MaterialInAssembly"));
}
