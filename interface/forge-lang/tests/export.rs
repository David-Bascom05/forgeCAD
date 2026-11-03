//! `fg run --step`: a part's folded body and each flat pattern as STEP AP214 (forgecad's
//! export), an assembly's structure as forge-asm writes it; each read back by forge-step.

use forge_lang::{Options, Run, export};
use std::path::{Path, PathBuf};

fn dir(tag: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("export-{tag}"));
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

const L: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 40
param plan.W = 30
param up.h = 30
param flat.gap = 80
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
flat = unfold(base, on: offset(base.bottom, gap))
";

fn vol(b: &forge_topo::Body) -> f64 {
    forge_topo::mass::mass(b).expect("mass").volume
}

/// The solids a STEP text reads back as, and their volumes.
fn read_back(text: &str) -> Vec<f64> {
    let got = forge_step::import(text.as_bytes(), &forge_step::Limits::default()).expect("reads back");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    got.solids.iter().map(|s| vol(&s.body)).collect()
}

fn run_in(d: &Path, src: &str, step: Option<&str>) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        step: step.map(String::from),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

#[test]
fn a_sheet_part_is_its_folded_body_and_its_flat_beside_it() {
    let d = dir("sheet");
    let r = run_in(&d, L, None);
    assert!(r.ok(), "{}", forge_lang::report::text(&r, L));
    let folded = vol(r.sheets[0].folded.as_ref().expect("folded"));
    let flat = vol(r.sheets[0].unfolds[0].1.as_ref().expect("flat"));
    let files = export::step(&r, "l").expect("exports");
    assert_eq!(
        files.iter().map(|f| f.suffix.as_str()).collect::<Vec<_>>(),
        vec!["", "-flat"]
    );
    let v = read_back(&files[0].text);
    assert_eq!(v.len(), 1, "the folded file holds the folded body alone");
    assert!((v[0] - folded).abs() < 1e-3 * folded, "{} vs {folded}", v[0]);
    let v = read_back(&files[1].text);
    assert_eq!(v.len(), 1);
    assert!((v[0] - flat).abs() < 1e-3 * flat, "{} vs {flat}", v[0]);
    assert!(
        files[0].text.contains("AUTOMOTIVE_DESIGN"),
        "AP214, as forgecad's export ap214"
    );
}

#[test]
fn a_plain_part_is_one_file() {
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/parts/bracket.fg")).expect("part");
    let r = forge_lang::run(&src, Options::default(), Default::default()).0;
    let m = r.measure.as_ref().expect("measured").volume;
    let files = export::step(&r, "bracket").expect("exports");
    assert_eq!(files.len(), 1);
    let v = read_back(&files[0].text);
    assert_eq!(v.len(), 1);
    assert!((v[0] - m).abs() < 1e-3 * m, "{} vs {m}", v[0]);
}

#[test]
fn an_assembly_is_its_structure() {
    let d = dir("asm");
    let block = "shared s = 10
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (s, s))
  o = pt(0, 0)
}
body = extrude(plan, s)
top = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.outline[0].dir))
";
    std::fs::write(d.join("block.fg"), block).expect("write");
    std::fs::write(d.join("l.fg"), L).expect("write");
    let doc = "assembly

a = part(\"block.fg\", ground: true)
b = part(\"block.fg\", at: pose(at: (30, 0, 0)), ground: true)
c = part(\"l.fg\", at: pose(at: (0, 50, 0)), ground: true)
";
    let r = run_in(&d, doc, Some("pair"));
    assert!(r.ok(), "{}", forge_lang::report::text(&r, doc));
    let files = export::step(&r, "pair").expect("exports");
    assert_eq!(files.len(), 1);
    // one product per part (the block once, the L folded: an assembly places a sheet part
    // without its flat), one use per instance
    let folded = {
        let l = run_in(&d, L, None);
        vol(l.sheets[0].folded.as_ref().expect("folded"))
    };
    let mut v = read_back(&files[0].text);
    v.sort_by(f64::total_cmp);
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(
        (v[0] - 1000.0).abs() < 1e-6 * 1000.0 && (v[1] - folded).abs() < 1e-3 * folded,
        "{v:?}"
    );
    assert_eq!(
        files[0].text.matches("NEXT_ASSEMBLY_USAGE_OCCURRENCE(").count(),
        3
    );
    // without `step` asked, the assembly's run makes none
    let r = run_in(&d, doc, None);
    assert!(export::step(&r, "pair").is_err());
}

// (it runs the fg binary: native only, wasi has no processes)
#[cfg(not(target_os = "wasi"))]
#[test]
fn fg_run_writes_the_step_files() {
    let d = dir("cli");
    std::fs::write(d.join("l.fg"), L).expect("write");
    let out = d.join("l.step");
    let st = std::process::Command::new(env!("CARGO_BIN_EXE_fg"))
        .args([
            "run",
            &d.join("l.fg").to_string_lossy(),
            "--step",
            &out.to_string_lossy(),
        ])
        .output()
        .expect("fg runs");
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    assert_eq!(read_back(&std::fs::read_to_string(&out).expect("OUT")).len(), 1);
    let flat = d.join("l-flat.step");
    assert_eq!(
        read_back(&std::fs::read_to_string(&flat).expect("the flat beside it")).len(),
        1
    );
}

/// The DXF's TEXT entities on a layer: (text, x, y, rotation).
fn texts(dxf: &str, layer: &str) -> Vec<(String, f64, f64, f64)> {
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let mut out = vec![];
    let mut k = 0;
    while k + 1 < lines.len() {
        if lines[k] == "0" && lines[k + 1] == "TEXT" {
            let (mut on, mut t, mut x, mut y, mut r) = (false, String::new(), 0.0, 0.0, 0.0);
            let mut j = k + 2;
            while j + 1 < lines.len() && lines[j] != "0" {
                let v = lines[j + 1];
                match lines[j] {
                    "8" => on = v == layer,
                    "1" => t = v.to_string(),
                    "10" => x = v.parse().expect("x"),
                    "20" => y = v.parse().expect("y"),
                    "50" => r = v.parse().expect("rotation"),
                    _ => {}
                }
                j += 2;
            }
            if on {
                out.push((t, x, y, r));
            }
            k = j;
        } else {
            k += 2;
        }
    }
    out
}

/// Each bend line has its note on BEND_NOTES, at its middle and along it: UP for a flange turned
/// towards the drawing's viewer, DOWN for one turned away, the angle and the inside radius; and
/// the part's note names the material, the thickness and the bend rule.
#[test]
fn the_flat_pattern_dxf_carries_bend_notes() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/parts/sheet/z.fg"))
        .expect("part")
        .replace(
            "bend: kfactor(k))",
            "bend: kfactor(k), material: \"1008 CR steel\")",
        );
    let r = forge_lang::run(&src, Options::default(), Default::default()).0;
    assert!(r.ok(), "{}", forge_lang::report::text(&r, &src));
    let dxf = r.sheets[0].dxf().expect("a flat pattern");
    let notes = texts(&dxf, forge_sheet::NOTES_LAYER);
    assert_eq!(notes.len(), 3, "{notes:?}");
    // the web turns up, the lip (side: "down") turns down, both 90 degrees about a radius of 1
    assert_eq!(notes[0].0, "UP 90.0 DEG IR 1.00");
    assert_eq!(notes[1].0, "DOWN 90.0 DEG IR 1.00");
    assert_eq!(notes[2].0, "BASE  MATERIAL 1008 CR STEEL  T 1.50  K 0.330");
    // each bend note beside the middle of its bend line, written along it
    let lines: Vec<(f64, f64)> = dxf
        .split("  0\nLINE\n")
        .skip(1)
        .filter(|e| e.contains("\n  8\nBEND\n"))
        .map(|e| {
            let v: Vec<&str> = e.lines().collect();
            let get = |code: &str| -> f64 {
                let k = v.iter().position(|l| l.trim() == code).expect("code");
                v[k + 1].trim().parse().expect("number")
            };
            ((get("10") + get("11")) / 2.0, (get("20") + get("21")) / 2.0)
        })
        .collect();
    assert_eq!(lines.len(), 2);
    for ((_, x, y, rot), (mx, my)) in notes.iter().zip(&lines) {
        assert!(
            ((x - mx).powi(2) + (y - my).powi(2)).sqrt() < 3.0,
            "{x},{y} vs {mx},{my}"
        );
        assert!((rot - 90.0).abs() < 1e-9, "{rot}");
    }
    // the plain pattern writer is unchanged: no notes layer
    assert!(!forge_sheet::dxf(&forge_sheet::Pattern::default()).contains("BEND_NOTES"));
    // without a material the note says so
    let r = forge_lang::run(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/parts/sheet/z.fg")).expect("part"),
        Options::default(),
        Default::default(),
    )
    .0;
    assert!(r.sheets[0].note().contains("MATERIAL UNSPECIFIED"));
}
