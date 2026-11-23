//! `forgecad export F stl|3mf|obj` on the bench parts and real ones, as a slicer gets them: every
//! object closed, manifold and facing out, read back by our own readers, its volume within what
//! the chord tolerance allows of the exact one, the same bytes each time; an assembly as parts
//! placed by build items, or flat; and the same over MCP. Other readers (trimesh, numpy-stl, the
//! 3MF schema) judge the same files in tools/meshcheck/run.sh.
//! Native only: they start `forgecad` as a process.
#![cfg(not(target_os = "wasi"))]

use forge_agent::json::Json;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-mesh-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(d).unwrap()
}

/// `forgecad export F WORDS.. -o OUT --json`: the reply.
fn export(file: &Path, words: &[&str], out: &Path) -> Json {
    let o = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .arg("export")
        .arg(file)
        .args(words)
        .arg("-o")
        .arg(out)
        .arg("--json")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&o.stdout);
    let j = Json::parse(text.trim()).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert!(o.status.success(), "{}: {j}", file.display());
    j
}

fn num(j: &Json, k: &str) -> f64 {
    j.get(k)
        .and_then(Json::num)
        .unwrap_or_else(|| panic!("{k} in {j}"))
}

fn objects(j: &Json) -> Vec<Json> {
    match j.get("objects") {
        Some(Json::Arr(a)) => a.clone(),
        _ => panic!("{j}"),
    }
}

/// The file read back by forge-mesh's own reader: each object (as placed) closed and facing out;
/// their volumes summed.
fn read_back(path: &Path) -> (usize, f64) {
    let bytes = std::fs::read(path).unwrap();
    let checks: Vec<forge_mesh::MeshCheck> = match path.extension().and_then(|e| e.to_str()) {
        Some("3mf") => forge_mesh::read_3mf(&bytes).unwrap().checks(),
        Some("obj") => forge_mesh::read_obj(std::str::from_utf8(&bytes).unwrap())
            .unwrap()
            .iter()
            .map(forge_mesh::Object::check)
            .collect(),
        Some("stl") => {
            let (objs, wrong) = forge_mesh::read_stl(&bytes).unwrap();
            assert_eq!(wrong, 0, "{}: facet normals", path.display());
            objs.iter().map(forge_mesh::Object::check).collect()
        }
        e => panic!("{e:?}"),
    };
    for c in &checks {
        assert!(c.watertight() && c.manifold(), "{}: {c:?}", path.display());
    }
    (checks.len(), checks.iter().map(|c| c.volume).sum())
}

/// A bench part in every format: each closed, read back, within the tolerance of the exact volume.
fn every_format(name: &str) {
    let dir = scratch(name);
    let part = repo(&format!("interface/forge-lang/parts/bench/{name}.fg"));
    for (fmt, words) in [
        ("3mf", vec!["3mf"]),
        ("obj", vec!["obj", "normals"]),
        ("stl", vec!["stl", "ascii"]),
    ] {
        let out = dir.join(format!("{name}.{fmt}"));
        let j = export(&part, &words, &out);
        assert_eq!(j.get("watertight"), Some(&Json::Bool(true)), "{name} {fmt}: {j}");
        assert_eq!(j.get("manifold"), Some(&Json::Bool(true)), "{name} {fmt}: {j}");
        let obs = objects(&j);
        for o in &obs {
            assert_eq!(o.get("watertight"), Some(&Json::Bool(true)), "{name}: {o}");
            assert_eq!(num(o, "open_edges"), 0.0);
            assert_eq!(o.get("manifold"), Some(&Json::Bool(true)), "{name}: {o}");
            assert_eq!(num(o, "pinched_vertices"), 0.0);
        }
        // within the chord tolerance of the surface everywhere: within area x tol of the volume
        let area: f64 = obs.iter().map(|o| num(o, "area")).sum();
        let (v, exact) = (num(&j, "volume"), num(&j, "exact_volume"));
        assert!(
            (v - exact).abs() <= area * num(&j, "tol"),
            "{name}: {v} vs {exact}"
        );
        let (n, back) = read_back(&out);
        assert_eq!(n, obs.len(), "{name} {fmt}: one object per solid");
        assert!((back - v).abs() < 1e-4 * v, "{name} {fmt}: {back} vs {v}");
    }
    // binary STL: one mesh of every solid, closed where the solids don't touch
    let out = dir.join(format!("{name}-bin.stl"));
    let j = export(&part, &["stl"], &out);
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 84 + 50 * num(&j, "triangles") as usize);
    if objects(&j).len() == 1 {
        read_back(&out);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

macro_rules! bench {
    ($($name:ident)*) => {$(
        #[test]
        fn $name() {
            every_format(stringify!($name));
        }
    )*};
}

// the bench parts that build here (crank, heatsink and jack want a font beside them), but for the
// largest (frame, gearbox, manifold, volute: tools/meshcheck/run.sh has them)
bench!(bracket clamp conrod guard hub insert quadarm valve);

#[test]
fn the_same_part_gives_the_same_bytes() {
    let dir = scratch("same");
    let part = repo("interface/forge-lang/parts/bench/conrod.fg");
    for fmt in ["3mf", "stl", "obj"] {
        let (a, b) = (dir.join(format!("a.{fmt}")), dir.join(format!("b.{fmt}")));
        export(&part, &[fmt], &a);
        export(&part, &[fmt], &b);
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap(), "{fmt}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tolerances_set_how_fine_and_bad_words_are_refused() {
    let dir = scratch("tol");
    let part = repo("interface/forge-lang/parts/bench/hub.fg");
    let fine = export(&part, &["3mf"], &dir.join("fine.3mf"));
    let coarse = export(
        &part,
        &["3mf", "tol", "0.2", "angle", "30"],
        &dir.join("coarse.3mf"),
    );
    assert_eq!(num(&fine, "tol"), 0.01);
    assert_eq!(num(&fine, "angle"), 15.0);
    assert_eq!(num(&coarse, "tol"), 0.2);
    assert!(num(&coarse, "triangles") < num(&fine, "triangles") / 2.0);
    assert_eq!(coarse.get("watertight"), Some(&Json::Bool(true)));
    let exact = num(&fine, "exact_volume");
    assert!((num(&fine, "volume") - exact).abs() < (num(&coarse, "volume") - exact).abs());
    for bad in [
        &["3mf", "tol", "0"][..],
        &["3mf", "angle", "120"],
        &["3mf", "ascii"],
        &["obj", "wobble"],
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_forgecad"))
            .arg("export")
            .arg(&part)
            .args(bad)
            .output()
            .unwrap();
        assert!(!o.status.success(), "{bad:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_part_with_a_material_carries_its_name_and_colour() {
    let dir = scratch("material");
    let out = dir.join("knob.3mf");
    let j = export(&repo("examples/unified/knob.fg"), &["3mf"], &out);
    assert_eq!(objects(&j)[0].get("material").and_then(Json::str), Some("ABS"));
    let s = forge_mesh::read_3mf(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(s.objects[0].name, "knob");
    assert_eq!(s.objects[0].material, Some(0));
    assert_eq!(s.materials[0].name, "ABS");
    assert_eq!(s.materials[0].rgb, forge_material::find("ABS").unwrap().colour);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_assembly_is_its_parts_placed_or_flat() {
    let dir = scratch("asm");
    let asm = repo("examples/unified/peg_board.fg");
    let out = dir.join("placed.3mf");
    let j = export(&asm, &["3mf"], &out);
    assert_eq!(j.get("watertight"), Some(&Json::Bool(true)), "{j}");
    let placed = forge_mesh::read_3mf(&std::fs::read(&out).unwrap()).unwrap();
    // each part once, each occurrence an item at its pose
    assert!(placed.objects.len() <= placed.items.len());
    assert!(placed.items.iter().all(|i| !i.name.is_empty()));
    assert!(!placed.materials.is_empty());
    let flat_out = dir.join("flat.3mf");
    let fj = export(&asm, &["3mf", "flat"], &flat_out);
    let flat = forge_mesh::read_3mf(&std::fs::read(&flat_out).unwrap()).unwrap();
    assert_eq!(flat.objects.len(), placed.items.len());
    assert!(
        flat.items
            .iter()
            .all(|i| i.transform == forge_mesh::scene::IDENTITY)
    );
    assert!((num(&j, "volume") - num(&fj, "volume")).abs() < 1e-6 * num(&j, "volume"));
    // STL and OBJ: flat always, an object per occurrence
    let obj = dir.join("asm.obj");
    export(&asm, &["obj"], &obj);
    assert_eq!(read_back(&obj).0, placed.items.len());
    let (_, v) = read_back(&out);
    assert!((v - num(&j, "volume")).abs() < 1e-4 * v);
    let tray = dir.join("tray.3mf");
    let t = export(&repo("examples/sheetmetal/tray_asm.fg"), &["3mf"], &tray);
    assert_eq!(t.get("watertight"), Some(&Json::Bool(true)), "{t}");
    read_back(&tray);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mcp_exports_a_mesh_inline_or_to_a_file() {
    let dir = scratch("mcp");
    let part = dir.join("hub.fg");
    std::fs::copy(repo("interface/forge-lang/parts/bench/hub.fg"), &part).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["mcp", "--root", dir.to_str().unwrap(), "--writable"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut to = child.stdin.take().unwrap();
    let mut from = BufReader::new(child.stdout.take().unwrap());
    let mut ask = |id: usize, method: &str, params: Json| -> Json {
        let req = Json::obj(vec![
            ("jsonrpc", "2.0".into()),
            ("id", id.into()),
            ("method", method.into()),
            ("params", params),
        ]);
        writeln!(to, "{req}").unwrap();
        to.flush().unwrap();
        let mut line = String::new();
        from.read_line(&mut line).unwrap();
        Json::parse(&line).unwrap().get("result").cloned().unwrap()
    };
    let tools = ask(1, "tools/list", Json::obj(vec![]));
    assert!(tools.to_string().contains("\"export\""), "{tools}");
    let call =
        |args: Vec<(&str, Json)>| Json::obj(vec![("name", "export".into()), ("arguments", Json::obj(args))]);
    let r = ask(
        2,
        "tools/call",
        call(vec![
            ("file", part.to_str().unwrap().into()),
            ("format", "3mf".into()),
            ("options", "tol 0.05".into()),
        ]),
    );
    assert_eq!(r.get("isError"), Some(&Json::Bool(false)), "{r}");
    let Some(Json::Arr(content)) = r.get("content") else {
        panic!("{r}")
    };
    let res = content
        .iter()
        .find(|c| c.get("type").and_then(Json::str) == Some("resource"))
        .and_then(|c| c.get("resource"))
        .unwrap_or_else(|| panic!("{r}"));
    assert_eq!(res.get("mimeType").and_then(Json::str), Some("model/3mf"));
    assert_eq!(res.get("uri").and_then(Json::str), Some("forgecad:///hub.3mf"));
    let text = content
        .iter()
        .find(|c| c.get("type").and_then(Json::str) == Some("text"))
        .and_then(|c| c.get("text"))
        .and_then(Json::str)
        .unwrap();
    let j = Json::parse(text).unwrap();
    assert_eq!(num(&j, "tol"), 0.05);
    assert!(
        res.get("blob")
            .and_then(Json::str)
            .is_some_and(|b| b.len() > 1000)
    );
    let r = ask(
        3,
        "tools/call",
        call(vec![
            ("file", part.to_str().unwrap().into()),
            ("format", "stl".into()),
            ("out", "hub.stl".into()),
        ]),
    );
    assert_eq!(r.get("isError"), Some(&Json::Bool(false)), "{r}");
    read_back(&dir.join("hub.stl"));
    let r = ask(
        4,
        "tools/call",
        call(vec![
            ("file", part.to_str().unwrap().into()),
            ("format", "stl".into()),
            ("out", "../outside.stl".into()),
        ]),
    );
    assert_eq!(r.get("isError"), Some(&Json::Bool(true)), "{r}");
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
