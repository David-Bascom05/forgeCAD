//! `name = use("file.fg")`: another file's shared values and functions as a read-only namespace.
//! Values keep their provenance (driven by `name.param`), a SetParam on one is refused as an
//! edit of the used file, functions see their own file's names, and missing files, cycles and
//! misplaced uses are typed errors.

use forge_lang::edit::apply;
use forge_lang::error::{Error, Literal, Op, ParamKey};
use forge_lang::uses::UseErr;
use forge_lang::values::Dim;
use forge_lang::{Options, Run, report};
use std::path::{Path, PathBuf};

/// A directory of this test's own under cargo's test temp dir, with these files in it.
fn dir(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("uses-{tag}"));
    std::fs::create_dir_all(&d).expect("temp dir");
    for (name, text) in files {
        std::fs::write(d.join(name), text).expect("write");
    }
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        root: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

const SHOP: &str = "shared t = 1.5
shared hole_d = 6.6
shared pitch = 44.45
mat = use(\"material.fg\")
fn rack_units(n) {
  return n * pitch
}
fn stiffness() {
  return mat.k * t
}
sketch nothing(on: xy) {
  o = rect(corner: (0, 0), size: (1, 1))
}
";

const MATERIAL: &str = "shared k = 0.42\n";

const PART: &str = "steel = use(\"shop.fg\")
shared w = 100
shared pitch = 1
shared x = 10
sketch plate(on: xy) {
  o = rect(corner: (0, 0), size: (w, steel.rack_units(2)))
  h = circle((x, x), steel.hole_d / 2)
}
panel = extrude(plate, steel.t)
";

fn use_err(e: &Error) -> &UseErr {
    match e {
        Error::Use(u) => u,
        o => panic!("not a use error: {o:?}"),
    }
}

#[test]
fn a_used_files_values_and_functions() {
    let d = dir("ok", &[("shop.fg", SHOP), ("material.fg", MATERIAL)]);
    let r = run_in(&d, PART);
    assert!(r.errors.is_empty(), "{}", report::text(&r, PART));
    // the function reads its own file's pitch (44.45), not the part's (1)
    let want = (100.0 * 2.0 * 44.45 - std::f64::consts::PI * 3.3 * 3.3) * 1.5;
    let v = r.measure.as_ref().expect("measured").volume;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
    // the depth is driven by the used file's param, named under the binding
    let panel = r.feats.iter().find(|f| f.id == "panel").expect("panel");
    let src = format!("{:?}", panel.dims);
    assert!(src.contains("steel") && src.contains("\"t\""), "{src}");
    // the used file's sketch is not part of this part; its params are not this part's
    assert!(r.feats.iter().all(|f| f.id != "nothing"));
    assert!(r.shared.iter().all(|p| !p.key.to_string().starts_with("steel")));
    // the same text runs to the same report
    let again = run_in(&d, PART);
    let steady = |r: &Run| {
        report::text(r, PART)
            .lines()
            .filter(|l| !l.contains(" ms (parse "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(steady(&again), steady(&r));
}

#[test]
fn a_used_function_sees_only_its_own_file() {
    let d = dir("scope", &[("shop.fg", SHOP), ("material.fg", MATERIAL)]);
    // the part cannot name the used file's values unqualified
    let src = "steel = use(\"shop.fg\")\nshared w = hole_d * 2\n";
    let r = run_in(&d, src);
    assert!(
        r.errors
            .iter()
            .any(|e| matches!(e, Error::UnknownName { name, .. } if name == "hole_d")),
        "{:?}",
        r.errors
    );
    // a used function reaching the file its file uses: mat.k * t, driven by steel.mat.k and steel.t
    let src = "steel = use(\"shop.fg\")\nshared s = steel.stiffness()\n";
    let r = run_in(&d, src);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let s = r.shared.iter().find(|p| p.key.name == "s").expect("s");
    let links: Vec<String> = s.links.iter().map(|k| k.to_string()).collect();
    assert!(
        links.contains(&"steel.t".to_string()) && links.contains(&"steel.mat.k".to_string()),
        "{links:?}"
    );
}

#[test]
fn a_setparam_on_a_used_value_is_an_edit_of_that_file() {
    let d = dir("edit", &[("shop.fg", SHOP), ("material.fg", MATERIAL)]);
    let op = Op::SetParam {
        param: ParamKey::parse("steel.t").expect("a param"),
        value: Literal {
            value: 2.0,
            dim: Dim::Length,
        },
    };
    let e = apply(PART, &op).expect_err("refused here");
    assert_eq!(
        use_err(&e),
        &UseErr::ParamInUsedFile {
            param: ParamKey::parse("steel.t").expect("a param"),
            file: "shop.fg".into(),
            name: "t".into(),
        }
    );
    // made in the used file, it moves the part
    let shop = apply(
        SHOP,
        &Op::SetParam {
            param: ParamKey::parse("t").expect("a param"),
            value: Literal {
                value: 2.0,
                dim: Dim::Length,
            },
        },
    )
    .expect("applies to the shop file");
    std::fs::write(d.join("shop.fg"), &shop).expect("write");
    let r = run_in(&d, PART);
    let want = (100.0 * 2.0 * 44.45 - std::f64::consts::PI * 3.3 * 3.3) * 2.0;
    let v = r.measure.as_ref().expect("measured").volume;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
    // a param of the part itself with the same name is still the part's
    let own = Op::SetParam {
        param: ParamKey::parse("pitch").expect("a param"),
        value: Literal {
            value: 3.0,
            dim: Dim::Length,
        },
    };
    assert!(
        apply(PART, &own)
            .expect("the part's own")
            .contains("shared pitch = 3")
    );
}

#[test]
fn use_errors_are_typed() {
    let d = dir(
        "errors",
        &[
            ("a.fg", "b = use(\"b.fg\")\nshared x = 1\n"),
            ("b.fg", "a = use(\"a.fg\")\nshared y = 2\n"),
            ("bad.fg", "shared z = 2\nfn f() {\n  return nowhere * 2\n}\n"),
            ("shop.fg", SHOP),
            ("material.fg", MATERIAL),
            ("a.txt", "shared x = 1\n"),
        ],
    );
    let outside = d.parent().expect("temp").join("uses-outside.fg");
    std::fs::write(&outside, "shared x = 1\n").expect("write");
    let first = |src: &str| -> UseErr {
        let r = run_in(&d, src);
        use_err(r.errors.first().unwrap_or_else(|| panic!("no error for {src}"))).clone()
    };
    assert!(matches!(
        first("s = use(\"missing.fg\")\n"),
        UseErr::Missing { .. }
    ));
    // nothing outside the session's root, and only .fg files
    let up = format!(
        "s = use(\"../{}\")\n",
        outside.file_name().expect("name").to_string_lossy()
    );
    assert!(matches!(first(&up), UseErr::Missing { .. }));
    assert!(matches!(first("s = use(\"a.txt\")\n"), UseErr::Missing { .. }));
    // a cycle, reported where it closes, inside the files
    match first("s = use(\"a.fg\")\n") {
        UseErr::InFile { file, error, .. } => {
            assert_eq!(file, "b.fg");
            // the chain is the files as the documents named them, never a path of this machine
            match use_err(&error) {
                UseErr::Cycle { file, chain, .. } => {
                    assert_eq!(file, "a.fg");
                    assert_eq!(chain, &vec!["a.fg".to_string(), "b.fg".to_string()]);
                }
                o => panic!("{o:?}"),
            }
            let text = use_err(&error).text() + &format!("{:?}", use_err(&error).data());
            let home = d.to_string_lossy().to_string();
            assert!(!text.contains(&home) && !text.contains('/'), "{text}");
        }
        o => panic!("{o:?}"),
    }
    assert!(matches!(
        first("shared q = use(\"shop.fg\").t\n"),
        UseErr::NotBound { .. }
    ));
    assert!(matches!(
        first("fn g() {\n  s = use(\"shop.fg\")\n  return s\n}\nshared q = g()\n"),
        UseErr::NotTopLevel { .. }
    ));
    // an error in a used function: at the call here, with the file and its line there
    match first("bad = use(\"bad.fg\")\nshared q = bad.f()\n") {
        UseErr::InFile {
            file, error, line, ..
        } => {
            assert_eq!((file.as_str(), line), ("bad.fg", 3));
            assert!(matches!(*error, Error::UnknownName { ref name, .. } if name == "nowhere"));
        }
        o => panic!("{o:?}"),
    }
    // a used namespace is bound once, like any name
    let r = run_in(&d, "s = use(\"shop.fg\")\ns = use(\"shop.fg\")\n");
    assert!(!r.errors.is_empty());
}
