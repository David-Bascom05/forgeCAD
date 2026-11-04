//! The standard parts library: every table parses and agrees with every bundled source it was
//! compared with (or the difference is resolved, with its reason, in `stdparts/resolutions`),
//! every family's generator builds valid parts across its sizes and lengths, and `std(..)`
//! places them in an assembly with typed errors and BOM properties.

use forge_lang::error::Op;
use forge_lang::stdparts::{self, Spec};
use forge_lang::{Options, Run};
use std::collections::BTreeMap;

/// The bundled copies of other sources' tables, as `tools/stdparts/import.py` wrote them.
const THIRD_PARTY: &[(&str, &str, &str)] = &[
    (
        "B",
        "asme_b18_3_shcs",
        include_str!("../stdparts/third_party/bd_warehouse/asme_b18_3_shcs.csv"),
    ),
    (
        "B",
        "asme_clearance",
        include_str!("../stdparts/third_party/bd_warehouse/asme_clearance.csv"),
    ),
    (
        "B",
        "din471",
        include_str!("../stdparts/third_party/bd_warehouse/din471.csv"),
    ),
    (
        "B",
        "din472",
        include_str!("../stdparts/third_party/bd_warehouse/din472.csv"),
    ),
    (
        "B",
        "iso10642",
        include_str!("../stdparts/third_party/bd_warehouse/iso10642.csv"),
    ),
    (
        "B",
        "iso15_bearings",
        include_str!("../stdparts/third_party/bd_warehouse/iso15_bearings.csv"),
    ),
    (
        "B",
        "iso273",
        include_str!("../stdparts/third_party/bd_warehouse/iso273.csv"),
    ),
    (
        "B",
        "iso4014",
        include_str!("../stdparts/third_party/bd_warehouse/iso4014.csv"),
    ),
    (
        "B",
        "iso4017",
        include_str!("../stdparts/third_party/bd_warehouse/iso4017.csv"),
    ),
    (
        "B",
        "iso4026",
        include_str!("../stdparts/third_party/bd_warehouse/iso4026.csv"),
    ),
    (
        "B",
        "iso4032",
        include_str!("../stdparts/third_party/bd_warehouse/iso4032.csv"),
    ),
    (
        "B",
        "iso4035",
        include_str!("../stdparts/third_party/bd_warehouse/iso4035.csv"),
    ),
    (
        "B",
        "iso4762",
        include_str!("../stdparts/third_party/bd_warehouse/iso4762.csv"),
    ),
    (
        "B",
        "iso7089",
        include_str!("../stdparts/third_party/bd_warehouse/iso7089.csv"),
    ),
    (
        "B",
        "iso7090",
        include_str!("../stdparts/third_party/bd_warehouse/iso7090.csv"),
    ),
    (
        "B",
        "iso7093_1",
        include_str!("../stdparts/third_party/bd_warehouse/iso7093_1.csv"),
    ),
    (
        "B",
        "iso7379",
        include_str!("../stdparts/third_party/bd_warehouse/iso7379.csv"),
    ),
    (
        "B",
        "iso7380_1",
        include_str!("../stdparts/third_party/bd_warehouse/iso7380_1.csv"),
    ),
    (
        "B",
        "iso7380_2",
        include_str!("../stdparts/third_party/bd_warehouse/iso7380_2.csv"),
    ),
    (
        "B",
        "tap_drill_metric",
        include_str!("../stdparts/third_party/bd_warehouse/tap_drill_metric.csv"),
    ),
    (
        "B",
        "tap_drill_unified",
        include_str!("../stdparts/third_party/bd_warehouse/tap_drill_unified.csv"),
    ),
];

const RESOLUTIONS: &[(&str, &str)] = &[
    ("as568", include_str!("../stdparts/resolutions/as568.csv")),
    (
        "asme_b18_22_1",
        include_str!("../stdparts/resolutions/asme_b18_22_1.csv"),
    ),
    (
        "asme_b18_2_1_hex_cap",
        include_str!("../stdparts/resolutions/asme_b18_2_1_hex_cap.csv"),
    ),
    (
        "asme_b18_2_2_hex_nut",
        include_str!("../stdparts/resolutions/asme_b18_2_2_hex_nut.csv"),
    ),
    (
        "asme_b18_3_button",
        include_str!("../stdparts/resolutions/asme_b18_3_button.csv"),
    ),
    (
        "asme_b18_3_flat",
        include_str!("../stdparts/resolutions/asme_b18_3_flat.csv"),
    ),
    (
        "asme_b18_3_shcs",
        include_str!("../stdparts/resolutions/asme_b18_3_shcs.csv"),
    ),
    (
        "asme_clearance",
        include_str!("../stdparts/resolutions/asme_clearance.csv"),
    ),
    (
        "cbore_iso4762",
        include_str!("../stdparts/resolutions/cbore_iso4762.csv"),
    ),
    ("din471", include_str!("../stdparts/resolutions/din471.csv")),
    ("din472", include_str!("../stdparts/resolutions/din472.csv")),
    ("din6885", include_str!("../stdparts/resolutions/din6885.csv")),
    ("iso10511", include_str!("../stdparts/resolutions/iso10511.csv")),
    ("iso10642", include_str!("../stdparts/resolutions/iso10642.csv")),
    (
        "iso15_bearings",
        include_str!("../stdparts/resolutions/iso15_bearings.csv"),
    ),
    ("iso273", include_str!("../stdparts/resolutions/iso273.csv")),
    ("iso4014", include_str!("../stdparts/resolutions/iso4014.csv")),
    ("iso4017", include_str!("../stdparts/resolutions/iso4017.csv")),
    ("iso4026", include_str!("../stdparts/resolutions/iso4026.csv")),
    ("iso4027", include_str!("../stdparts/resolutions/iso4027.csv")),
    ("iso4028", include_str!("../stdparts/resolutions/iso4028.csv")),
    ("iso4029", include_str!("../stdparts/resolutions/iso4029.csv")),
    ("iso4032", include_str!("../stdparts/resolutions/iso4032.csv")),
    ("iso4035", include_str!("../stdparts/resolutions/iso4035.csv")),
    ("iso4762", include_str!("../stdparts/resolutions/iso4762.csv")),
    ("iso7090", include_str!("../stdparts/resolutions/iso7090.csv")),
    ("iso7093_1", include_str!("../stdparts/resolutions/iso7093_1.csv")),
    ("iso7379", include_str!("../stdparts/resolutions/iso7379.csv")),
    ("iso7380_1", include_str!("../stdparts/resolutions/iso7380_1.csv")),
    ("iso7380_2", include_str!("../stdparts/resolutions/iso7380_2.csv")),
    ("pem_fh", include_str!("../stdparts/resolutions/pem_fh.csv")),
    ("pem_s", include_str!("../stdparts/resolutions/pem_s.csv")),
    ("pem_so", include_str!("../stdparts/resolutions/pem_so.csv")),
    (
        "tap_drill_metric",
        include_str!("../stdparts/resolutions/tap_drill_metric.csv"),
    ),
    (
        "tap_drill_unified",
        include_str!("../stdparts/resolutions/tap_drill_unified.csv"),
    ),
];

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
                stdparts::TABLES.iter().chain(stdparts::HELD).any(|t| t.0 == stem),
                "stdparts/{n} is neither in TABLES nor HELD"
            );
        }
    }
}

#[test]
fn tables_parse() {
    for f in stdparts::families() {
        assert!(!f.rows.is_empty(), "{}", f.id);
        if f.kind == "hole" {
            assert!(!f.placeable(), "{}: a hole table makes no part", f.id);
        } else {
            let header = f.get("gen").unwrap_or_else(|| panic!("{}: no gen header", f.id));
            let g = header.split('|').next().unwrap().trim();
            assert!(
                stdparts::GENERATORS.iter().any(|x| x.0 == g),
                "{}: no generator {g}",
                f.id
            );
            // every material it can be made of is the library's, with its density
            let mut names: Vec<&str> = f.get("material").into_iter().collect();
            if let Some((_, grades)) = f.options.iter().find(|o| o.0 == "grade") {
                for g in grades {
                    names.push(
                        f.get(&format!("grade {g}"))
                            .unwrap_or_else(|| panic!("{}: grade {g} names no material", f.id)),
                    );
                }
            }
            assert!(!names.is_empty(), "{}: no material", f.id);
            for n in names {
                assert!(
                    forge_material::find(n).is_some(),
                    "{}: material {n:?} is not in the library",
                    f.id
                );
            }
        }
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
            if f.has_lengths() && f.placeable() {
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
    for f in stdparts::families().iter().filter(|f| f.placeable()) {
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
    assert_eq!(k, "std:ISO 4762|M6|20|thread=modelled");
    let back = stdparts::spec_of(&k).unwrap();
    assert_eq!(stdparts::key(&back), k);
    assert!(stdparts::text_of(&k).is_some());
    assert!(stdparts::text_of("std:ISO 4762|M6|21").is_none());
}

fn asm_dir(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let d = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stdparts-{tag}"));
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

fn asm_run(tag: &str, src: &str) -> Run {
    let d = asm_dir(tag, &[("plate.fg", PLATE)]);
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
        "bom",
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
    let key = "std:ISO 4762|M6|20|grade=10.9";
    assert!(out.bom.contains(&(key.to_string(), 2)), "{:?}", out.bom);
    let rows = forge_lang::props::bom_rows(&out);
    let (_, n, p, material, mass) = rows.iter().find(|r| r.0 == key).expect("the screw's BOM row");
    assert_eq!(*n, 2);
    assert_eq!(p.get("part_number"), Some("ISO 4762 M6x20"));
    assert_eq!(
        p.custom,
        vec![("Standard".to_string(), "ISO 4762:2004".to_string())]
    );
    assert_eq!(material.as_deref(), Some("ISO 898-1 class 10.9"));
    // 957.5545 mm3 of steel at 7.85 g/cm3, in kg
    assert!((mass.unwrap() - 0.0075168).abs() < 1e-6, "{mass:?}");
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
        let r = asm_run(
            kind,
            &format!("assembly\nbase = part(\"plate.fg\", ground: true)\nscr = {call}\n"),
        );
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

/// The kernel's hole table (what `hole(screw: ..)` drills, in .fg and .fc parts alike) against
/// these tables: every difference is a row of resolutions/hole_sizes.csv saying why it stands.
#[test]
fn hole_sizes_agree_or_are_resolved() {
    let res = rows(include_str!("../stdparts/resolutions/hole_sizes.csv"));
    let mut open = vec![];
    for k in stdparts::hole_screws() {
        let t = stdparts::tables_hole(k);
        let mut check = |what: &str, a: f64, b: f64| {
            if (a - b).abs() > 1e-6 && !res.iter().any(|r| r[1] == k.name && r[2] == what) {
                open.push(format!("{} {what}: kernel {a} tables {b}", k.name));
            }
        };
        for i in 0..3 {
            check(["close", "normal", "loose"][i], k.clearance[i], t.clearance[i]);
        }
        for i in 0..2 {
            let c = ["iso", "ansi"][i];
            check(&format!("tap {c}"), k.tap[i], t.tap[i]);
            check(&format!("cbore {c}"), k.counterbore[i].0, t.counterbore[i].0);
            check(
                &format!("cbore depth {c}"),
                k.counterbore[i].1,
                t.counterbore[i].1,
            );
        }
        if let (Some(a), Some(b)) = (k.countersink, t.countersink) {
            check("csink", a.0, b.0);
        }
    }
    assert!(open.is_empty(), "unresolved hole sizes:\n{}", open.join("\n"));
    assert_eq!(stdparts::hole_screw("#10-32").map(|s| s.name), Some("10-32"));
}
