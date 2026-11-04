//! The standard parts library: every table parses and agrees with every bundled source it was
//! compared with (or the difference is resolved, with its reason, in `stdparts/resolutions`),
//! every family's generator builds valid parts across its sizes and lengths, and `std(..)`
//! places them in an assembly with typed errors and BOM properties.

use forge_lang::error::Op;
use forge_lang::stdparts::{self, Spec};
use forge_lang::{Options, Run};
use std::collections::BTreeMap;

/// The bundled copies of other sources' tables, as `tools/stdparts/import.py` wrote them.
const THIRD_PARTY: &[(&str, &str, &str)] = &[(
    "B",
    "iso4762",
    include_str!("../stdparts/third_party/bd_warehouse/iso4762.csv"),
)];

const RESOLUTIONS: &[(&str, &str)] = &[("iso4762", include_str!("../stdparts/resolutions/iso4762.csv"))];

fn rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split(',').map(|c| c.trim().to_string()).collect())
        .collect()
}

#[test]
fn every_file_is_listed() {
    let Ok(dir) = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/stdparts")) else {
        return; // no file system here (wasm): the native run checks it
    };
    for e in dir.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if let Some(stem) = n.strip_suffix(".csv") {
            assert!(
                stdparts::TABLES.iter().any(|t| t.0 == stem),
                "stdparts/{n} is not in TABLES"
            );
        }
    }
}

#[test]
fn tables_parse() {
    for f in stdparts::families() {
        assert!(!f.rows.is_empty(), "{}", f.id);
        let header = f.get("gen").unwrap_or_else(|| panic!("{}: no gen header", f.id));
        let g = header.split('|').next().unwrap().trim();
        assert!(
            stdparts::GENERATORS.iter().any(|x| x.0 == g),
            "{}: no generator {g}",
            f.id
        );
        let checked = f
            .col("checked")
            .unwrap_or_else(|| panic!("{}: no checked column", f.id));
        for r in &f.rows {
            assert!(!r[f.col("src").unwrap()].is_empty(), "{} {}: no src", f.id, r[0]);
            // cross-checked: at least two sources agree on the row
            assert!(
                r[checked].split_whitespace().count() >= 2,
                "{} {}: checked by fewer than two sources",
                f.id,
                r[0]
            );
            if f.has_lengths() {
                assert!(!f.lengths_of(r).is_empty(), "{} {}: no lengths", f.id, r[0]);
            }
        }
    }
}

#[test]
fn bundled_sources_agree_or_are_resolved() {
    let mut open = vec![];
    for (src, fam, text) in THIRD_PARTY {
        let f = stdparts::family(fam).unwrap_or_else(|| panic!("{fam}"));
        let res: Vec<Vec<String>> = RESOLUTIONS
            .iter()
            .filter(|r| r.0 == *fam)
            .flat_map(|r| rows(r.1).into_iter().skip(1))
            .collect();
        for r in res.iter().filter(|r| r[6].starts_with("open")) {
            open.push(format!("{fam} {} {} {}: {}", r[1], r[2], r[3], r[6]));
        }
        let t = rows(text);
        let head = &t[0];
        for r in &t[1..] {
            let Some(ours) = f.row(&r[0]) else {
                continue;
            };
            for (c, v) in head.iter().zip(r).skip(1) {
                let (Ok(theirs), Some(o)) = (v.parse::<f64>(), f.value(ours, c)) else {
                    continue;
                };
                if (theirs - o).abs() > 1e-9
                    && !res.iter().any(|x| x[1] == r[0] && x[2] == *c && x[3] == *src)
                {
                    open.push(format!("{fam} {} {c}: {src} {theirs} ours {o}", r[0]));
                }
            }
        }
    }
    assert!(open.is_empty(), "unresolved disagreements:\n{}", open.join("\n"));
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn spec(family: &str, size: &str, length: Option<f64>, opts: &[(&str, &str)]) -> Spec {
    Spec {
        family: family.into(),
        size: size.into(),
        length,
        options: opts
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}

/// The first, middle and last size of each family at its shortest and longest length build
/// valid, one solid, with no errors; the middle size with its thread modelled too.
#[test]
fn generators_build_across_sizes() {
    for f in stdparts::families() {
        let n = f.rows.len();
        for (k, r) in [0, n / 2, n - 1].iter().map(|&k| (k, &f.rows[k])) {
            let ls = f.lengths_of(r);
            let lengths: Vec<Option<f64>> = if ls.is_empty() {
                vec![None]
            } else {
                vec![ls.first().copied(), ls.last().copied()]
            };
            for l in lengths {
                let mut variants = vec![vec![]];
                if k == n / 2 && f.options.iter().any(|o| o.0 == "thread") {
                    variants.push(vec![("thread", "modelled")]);
                }
                for o in variants {
                    let s = spec(&f.name, &r[0], l, &o);
                    let g = stdparts::generate(&s).unwrap_or_else(|e| panic!("{s:?}: {e:?}"));
                    let run = run(&g.text);
                    assert!(
                        run.errors.is_empty(),
                        "{}: {:?}",
                        g.key,
                        run.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
                    );
                    let m = run
                        .measure
                        .as_ref()
                        .unwrap_or_else(|| panic!("{}: no body", g.key));
                    assert!(
                        m.problems == 0 && m.bodies == 1 && m.volume > 0.0,
                        "{}: {m:?}",
                        g.key
                    );
                    assert!(run.expects.iter().all(|e| e.ok), "{}", g.key);
                }
            }
        }
    }
}

/// M6 x 20 against its volume worked by hand: the head less its bearing-face chamfer, the
/// under-head fillet, the shank and its tip chamfer, less the socket and its drill point.
#[test]
fn iso4762_m6x20_volume() {
    let g = stdparts::generate(&spec("ISO 4762", "M6", Some(20.0), &[])).unwrap();
    let v = run(&g.text).measure.unwrap().volume;
    // tools/stdparts/volumes.py, iso4762 M6 20
    assert!((v - 957.5545).abs() < 1e-3, "{v}");
}

#[test]
fn keys_round_trip() {
    let s = spec(
        "ISO 4762",
        "M6",
        Some(20.0),
        &[("thread", "modelled"), ("grade", "12.9")],
    );
    let k = stdparts::key(&s);
    assert_eq!(k, "std:ISO 4762/M6/20/thread=modelled");
    let back = stdparts::spec_of(&k).unwrap();
    assert_eq!(stdparts::key(&back), k);
    assert!(stdparts::text_of(&k).is_some());
    assert!(stdparts::text_of("std:ISO 4762/M6/21").is_none());
}

fn asm_dir(files: &[(&str, &str)]) -> std::path::PathBuf {
    let d = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stdparts");
    std::fs::create_dir_all(&d).unwrap();
    for (n, t) in files {
        std::fs::write(d.join(n), t).unwrap();
    }
    d
}

const PLATE: &str = "shared t = 8
sketch s(on: xy) {
  o = rect(center: (0, 0), size: (40, 40))
  c = circle((0, 0), 3.2)
}
p = extrude(s, t)
top = anchor(plane(at: s.c.center, normal: Z, x: X))
";

fn asm_run(src: &str) -> Run {
    let d = asm_dir(&[("plate.fg", PLATE)]);
    let opts = Options {
        dir: Some(d.clone()),
        root: Some(d),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

#[test]
fn std_in_an_assembly_is_one_bom_line_with_properties() {
    let r = asm_run(
        "assembly
base = part(\"plate.fg\", ground: true)
a = std(\"ISO 4762\", size: \"M6\", length: 20, grade: \"10.9\")
b = std(\"ISO 4762\", size: \"M6\", length: 20, grade: \"10.9\")
ja = fastened(a.head, base.top)
jb = fastened(b.head, base.top)
",
    );
    assert!(
        r.errors.is_empty(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let out = r.asm.unwrap();
    let key = "std:ISO 4762/M6/20/grade=10.9";
    assert!(out.bom.contains(&(key.to_string(), 2)), "{:?}", out.bom);
    let (p, mass) = &out.props[key];
    assert_eq!(p.part_number.as_deref(), Some("ISO 4762 M6x20"));
    assert_eq!(p.standard.as_deref(), Some("ISO 4762:2004"));
    assert_eq!(p.material.as_deref(), Some("Steel, property class 10.9"));
    // 957.5545 mm3 of steel at 7.85 g/cm3
    assert!((mass.unwrap() - 7.5168).abs() < 1e-3, "{mass:?}");
}

#[test]
fn std_errors_are_typed_with_fixes() {
    let cases = [
        (
            "std(\"ISO 4762\", size: \"M6x1\", length: 20)",
            "StdUnknownSize",
            Some(("size", "\"M6\"")),
        ),
        (
            "std(\"ISO 4762\", size: \"M6\", length: 21)",
            "StdUnknownLength",
            Some(("length", "20")),
        ),
        (
            "std(\"ISO 4762\", size: \"M6\")",
            "StdNeedsLength",
            Some(("length", "10")),
        ),
        (
            "std(\"ISO 4762\", size: \"M7\", length: 20)",
            "StdUnknownSize",
            None,
        ),
        (
            "std(\"ISO 4762\", size: \"M6\", length: 20, grade: \"9.9\")",
            "StdUnknownOption",
            None,
        ),
        (
            "std(\"ISO 9999\", size: \"M6\", length: 20)",
            "StdUnknownFamily",
            None,
        ),
    ];
    for (call, kind, fix) in cases {
        let r = asm_run(&format!(
            "assembly\nbase = part(\"plate.fg\", ground: true)\nscr = {call}\n"
        ));
        let e = r.errors.iter().find(|e| e.kind() == kind).unwrap_or_else(|| {
            panic!(
                "{call}: {:?}",
                r.errors.iter().map(|e| e.kind()).collect::<Vec<_>>()
            )
        });
        let want: Vec<Op> = fix
            .map(|(arg, text)| Op::SetArg {
                id: "scr".into(),
                arg: arg.into(),
                text: text.into(),
            })
            .into_iter()
            .collect();
        assert_eq!(e.fixes(), want, "{call}");
    }
}
