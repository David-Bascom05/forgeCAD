//! Simplify creates a new exact model and refreshes it through the saved recipe.
#![cfg(not(target_os = "wasi"))]
use forge_agent::json::Json;
use std::{fs, path::Path, process::Command};

const PLATE: &str = r#"
shared w = 40
shared h = 30
shared t = 6
shared r = 1
shared big = 3
shared x = 10
sketch plan(on: xy) {
  outline = rect(center: (0, 0), size: (w, h))
  small = circle((-x, 0), r)
  large = circle((x, 0), big)
}
plate = extrude(plan, t)
"#;

fn run(dir: &Path, args: &[&str]) -> (bool, Json) {
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .current_dir(dir)
        .args(args)
        .arg("--json")
        .output()
        .unwrap();
    let out_text = String::from_utf8_lossy(&out.stdout);
    let json = Json::parse(out_text.trim()).unwrap_or_else(|e| {
        panic!(
            "{args:?}: {e}: {out_text} {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.success(), json)
}

fn ok(dir: &Path, args: &[&str]) -> Json {
    let (success, json) = run(dir, args);
    assert!(success, "{args:?}: {json}");
    json
}

fn scratch(name: &str) -> std::path::PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forgecad-simplify-{name}-{}", forge_dirs::process_id()));
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn linked_step_plate_refresh_is_repeatable_and_failures_keep_the_last_snapshot() {
    let d = scratch("step");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    ok(&d, &["export", "source.fg", "-o", "source.step"]);
    let preview = ok(&d, &["simplify", "preview", "source.step", "--holes", "2"]);
    let Some(Json::Arr(changes)) = preview.get("changes") else {
        panic!("{preview}")
    };
    assert!(
        changes
            .iter()
            .any(|c| c.get("action").and_then(Json::str) == Some("fill bore")),
        "{preview}"
    );
    assert!(
        changes
            .iter()
            .any(|c| c.get("action").and_then(Json::str) == Some("keep bore")),
        "{preview}"
    );
    ok(
        &d,
        &[
            "simplify",
            "create",
            "source.step",
            "--holes",
            "2",
            "-o",
            "derived.fg",
        ],
    );
    ok(&d, &["build", "derived.fg"]);
    let first = fs::read_to_string(d.join("derived.fg")).unwrap();
    let snapshot = first.lines().nth(1).unwrap().split('"').nth(1).unwrap();
    let bytes = fs::read(d.join(snapshot)).unwrap();
    let solid = forge_step::import(&bytes, &forge_step::Limits::default()).unwrap();
    let volume = forge_topo::mass::mass(&solid.solids[0].body).unwrap().volume;
    let expected = (40.0 * 30.0 - 9.0 * forge_math::dmath::PI) * 6.0;
    assert!((volume - expected).abs() < 1e-5, "{volume} != {expected}");
    ok(&d, &["simplify", "refresh", "derived.fg"]);
    assert_eq!(first, fs::read_to_string(d.join("derived.fg")).unwrap());

    fs::write(d.join("source.fg"), PLATE.replace("shared t = 6", "shared t = 8")).unwrap();
    ok(&d, &["export", "source.fg", "-o", "revision.step"]);
    fs::copy(d.join("revision.step"), d.join("source.step")).unwrap();
    ok(&d, &["simplify", "pause", "derived.fg"]);
    assert!(!run(&d, &["simplify", "refresh", "derived.fg"]).0);
    ok(&d, &["simplify", "resume", "derived.fg"]);
    ok(&d, &["simplify", "refresh", "derived.fg"]);
    let revised = fs::read_to_string(d.join("derived.fg")).unwrap();
    assert_ne!(first, revised);
    assert_ne!(first.lines().nth(1), revised.lines().nth(1));
    fs::write(d.join("source.step"), "invalid STEP").unwrap();
    assert!(!run(&d, &["simplify", "refresh", "derived.fg"]).0);
    assert_eq!(revised, fs::read_to_string(d.join("derived.fg")).unwrap());
    ok(&d, &["build", "derived.fg"]);
    ok(&d, &["simplify", "detach", "derived.fg"]);
    assert!(!run(&d, &["simplify", "refresh", "derived.fg"]).0);
    ok(&d, &["build", "derived.fg"]);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn source_rules_can_preserve_a_bore_or_create_a_solid_envelope() {
    let d = scratch("native");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    let preview = ok(&d, &["simplify", "preview", "source.fg", "--holes", "2"]);
    let Some(Json::Arr(changes)) = preview.get("changes") else {
        panic!("{preview}")
    };
    let bore = changes
        .iter()
        .find(|c| c.get("action").and_then(Json::str) == Some("fill bore"))
        .unwrap();
    let Some(Json::Arr(faces)) = bore.get("faces") else {
        panic!("{bore}")
    };
    let keep = format!("part={}", faces[0].str().unwrap());
    let kept = ok(
        &d,
        &[
            "simplify",
            "preview",
            "source.fg",
            "--holes",
            "2",
            "--keep-face",
            &keep,
        ],
    );
    let Some(Json::Arr(changes)) = kept.get("changes") else {
        panic!("{kept}")
    };
    assert!(
        !changes
            .iter()
            .any(|c| c.get("action").and_then(Json::str) == Some("fill bore"))
    );
    let result = ok(
        &d,
        &[
            "simplify",
            "create",
            "source.fg",
            "--envelope",
            "part",
            "-o",
            "box.fg",
        ],
    );
    assert_eq!(result.get("faces").and_then(Json::num), Some(6.0));
    ok(&d, &["build", "box.fg"]);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn native_assembly_refresh_watches_part_files_and_keeps_occurrence_placement() {
    let d = scratch("assembly");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    fs::write(
        d.join("assembly.fg"),
        r#"
assembly
shared separation = 70
a = part("source.fg", ground: true)
b = part("source.fg", ground: true, at: pose(at: (separation, 0, 0)))
"#,
    )
    .unwrap();
    let preview = ok(
        &d,
        &[
            "simplify",
            "create",
            "assembly.fg",
            "--exclude",
            "a",
            "--envelope",
            "b",
            "-o",
            "clearance.fg",
        ],
    );
    assert_eq!(preview.get("faces").and_then(Json::num), Some(6.0));
    let before = fs::read_to_string(d.join("clearance.fg")).unwrap();
    ok(&d, &["build", "clearance.fg"]);
    fs::write(
        d.join("source.fg"),
        PLATE.replace("shared w = 40", "shared w = 46"),
    )
    .unwrap();
    ok(&d, &["simplify", "refresh", "clearance.fg"]);
    let after = fs::read_to_string(d.join("clearance.fg")).unwrap();
    assert_ne!(before.lines().nth(1), after.lines().nth(1));
    ok(&d, &["build", "clearance.fg"]);
    let file = after.lines().nth(1).unwrap().split('"').nth(1).unwrap();
    let bytes = fs::read(d.join(file)).unwrap();
    let read = forge_step::import(&bytes, &forge_step::Limits::default()).unwrap();
    let bounds = forge_topo::extent::exact_box(&read.solids[0].body);
    assert!((bounds.x.lo - 47.0).abs() < 1e-6, "{bounds:?}");
    assert!((bounds.x.hi - 93.0).abs() < 1e-6, "{bounds:?}");
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn repeated_step_product_paths_allow_selecting_each_solid_independently() {
    let d = scratch("repeated-step-product");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    fs::write(
        d.join("assembly.fg"),
        r#"
assembly
shared separation = 70
a = part("source.fg", ground: true)
b = part("source.fg", ground: true, at: pose(at: (separation, 0, 0)))
"#,
    )
    .unwrap();
    ok(&d, &["simplify", "create", "assembly.fg", "-o", "combined.fg"]);
    ok(&d, &["export", "combined.fg", "-o", "repeated.step"]);
    let bytes = fs::read(d.join("repeated.step")).unwrap();
    let imported = forge_step::import(&bytes, &forge_step::Limits::default()).unwrap();
    assert_eq!(imported.occurrences.len(), 2);
    assert_eq!(imported.occurrences[0].path, imported.occurrences[1].path);
    let preview = ok(&d, &["simplify", "preview", "repeated.step"]);
    let Some(Json::Arr(changes)) = preview.get("changes") else {
        panic!("{preview}");
    };
    assert_eq!(changes.len(), 2);
    let first = changes[0].get("part").and_then(Json::str).unwrap();
    let second = changes[1].get("part").and_then(Json::str).unwrap();
    assert_ne!(first, second);
    let result = ok(
        &d,
        &[
            "simplify",
            "create",
            "repeated.step",
            "--exclude",
            first,
            "-o",
            "derived.fg",
        ],
    );
    assert_eq!(result.get("faces").and_then(Json::num), Some(8.0));
    let derived = ok(&d, &["build", "derived.fg"]);
    assert_eq!(
        derived.get("part").unwrap().get("bodies").and_then(Json::num),
        Some(1.0)
    );
    // Drop only the first use's placement; another use of the same part still resolves.
    let assembly = fs::read_to_string(d.join("assembly.fg")).unwrap();
    let options = forge_lang::Options {
        dir: Some(d.clone()),
        step: Some("uses".into()),
        ..Default::default()
    };
    let assembly_run = forge_lang::run(&assembly, options, Default::default()).0;
    let files = forge_lang::export::step(&assembly_run, "uses").unwrap();
    fs::write(d.join("uses.step"), &files[0].text).unwrap();
    ok(&d, &["simplify", "create", "uses.step", "-o", "linked.fg"]);
    let saved = fs::read(d.join("linked.fg")).unwrap();
    let text = fs::read_to_string(d.join("uses.step")).unwrap();
    assert_eq!(text.matches("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(").count(), 2);
    let partial = text.replacen("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(", "MISSING_PLACEMENT(", 1);
    let read = forge_step::import(partial.as_bytes(), &forge_step::Limits::default()).unwrap();
    assert_eq!(read.occurrences.len(), 1);
    assert!(read.stats.incomplete_placements);
    fs::write(d.join("uses.step"), partial).unwrap();
    let (success, reply) = run(&d, &["simplify", "refresh", "linked.fg"]);
    assert!(!success && reply.to_string().contains("unresolved placements"), "{reply}");
    assert_eq!(fs::read(d.join("linked.fg")).unwrap(), saved);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn automatic_planar_detail_rules_fill_pockets_and_remove_bosses() {
    let d = scratch("detail");
    fs::write(
        d.join("detail.fg"),
        r#"
shared w = 40
shared h = 30
shared t = 6
shared x = 10
shared s = 3
shared d = 2
sketch plan(on: xy) {
  outline = rect(center: (0, 0), size: (w, h))
}
plate = extrude(plan, t)
sketch cuts(on: plate.end) {
  outline = rect(center: (-x, 0), size: (s, s))
}
pocket = cut(cuts, d)
sketch additions(on: plate.end) {
  outline = rect(center: (x, 0), size: (s, s))
}
proud = extrude(additions, d)
"#,
    )
    .unwrap();
    let result = ok(
        &d,
        &[
            "simplify",
            "create",
            "detail.fg",
            "--pockets",
            "5",
            "--bosses",
            "5",
            "-o",
            "plain.fg",
        ],
    );
    let Some(Json::Arr(changes)) = result.get("changes") else {
        panic!("{result}")
    };
    for action in ["fill pocket", "remove boss"] {
        assert!(
            changes
                .iter()
                .any(|c| c.get("action").and_then(Json::str) == Some(action)),
            "{result}"
        );
    }
    assert_eq!(result.get("faces").and_then(Json::num), Some(6.0));
    ok(&d, &["build", "plain.fg"]);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn agent_preview_and_creation_use_the_same_rules_and_the_project_fence() {
    use std::io::Write;
    use std::process::Stdio;

    let d = scratch("mcp");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    let ask = |writable: bool, action: &str, file: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forgecad"));
        cmd.current_dir(&d)
            .args(["mcp", "--root", "."])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if writable {
            cmd.arg("--writable");
        }
        let mut child = cmd.spawn().unwrap();
        let params = Json::obj(vec![
            ("action", action.into()),
            ("file", file.into()),
            (
                "out",
                if action == "create" {
                    "new.fg".into()
                } else {
                    Json::Null
                },
            ),
            ("rules", Json::Arr(vec!["--holes".into(), "2".into()])),
        ]);
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"simplify","arguments":{params}}}}}"#).unwrap();
        drop(stdin);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        Json::parse(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
    };

    let preview = ask(false, "preview", "source.fg");
    assert!(preview.to_string().contains("fill bore"), "{preview}");
    let readonly = ask(false, "create", "source.fg");
    assert!(readonly.to_string().contains("read-only"), "{readonly}");
    assert!(!d.join("new.fg").exists());
    let outside = ask(true, "preview", env!("CARGO_MANIFEST_DIR"));
    assert!(
        outside.to_string().contains("under the project root"),
        "{outside}"
    );
    let create = ask(true, "create", "source.fg");
    assert!(d.join("new.fg").is_file(), "{create}");
    ok(&d, &["build", "new.fg"]);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn changed_imported_face_picks_require_review_even_through_a_native_wrapper() {
    let d = scratch("review");
    fs::write(d.join("source.fg"), PLATE).unwrap();
    ok(&d, &["export", "source.fg", "-o", "source.step"]);
    fs::write(d.join("wrapped.fg"), "plate = import(\"source.step\")\n").unwrap();
    let preview = ok(&d, &["simplify", "preview", "wrapped.fg", "--faces"]);
    let Some(Json::Arr(faces)) = preview.get("source_faces") else {
        panic!("{preview}");
    };
    let name = faces[0].get("face").and_then(Json::str).unwrap();
    let pick = format!("part={name}");
    ok(
        &d,
        &[
            "simplify",
            "create",
            "wrapped.fg",
            "--keep-face",
            &pick,
            "-o",
            "linked.fg",
        ],
    );
    let before = fs::read_to_string(d.join("linked.fg")).unwrap();
    fs::write(d.join("source.fg"), PLATE.replace("shared t = 6", "shared t = 8")).unwrap();
    ok(&d, &["export", "source.fg", "-o", "revised.step"]);
    fs::copy(d.join("revised.step"), d.join("source.step")).unwrap();
    let (accepted, reply) = run(&d, &["simplify", "refresh", "linked.fg"]);
    assert!(!accepted && reply.to_string().contains("Review"), "{reply}");
    assert_eq!(before, fs::read_to_string(d.join("linked.fg")).unwrap());
    ok(
        &d,
        &[
            "simplify",
            "refresh",
            "linked.fg",
            "--replace-rules",
            "--holes",
            "2",
        ],
    );
    ok(&d, &["build", "linked.fg"]);
    fs::remove_dir_all(d).unwrap();
}
