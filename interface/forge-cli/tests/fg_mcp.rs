//! Forge (.fg) parts over `forgecad mcp` and `forgecad serve`: typed JSON both ways, the round
//! trip an agent makes (read the errors and lints, send a fix back as it came, see it built).
//! Native only: they start `forgecad` as a process, which WASI cannot.
#![cfg(not(target_os = "wasi"))]

use forge_agent::json::Json;
use forge_lang::error::Op;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const PLATE: &str = "shared L = 60
shared t = 6

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)

param mark.d = 4
param mark.cy = 40
sketch mark(on: base.end) {
  c = circle((30, cy), d)
}
param dent.depth = 2
dent = cut(mark, depth)
";

struct Server {
    child: Child,
    to: ChildStdin,
    from: BufReader<ChildStdout>,
    id: u64,
}

impl Server {
    fn start(mode: &str, root: &std::path::Path) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_forgecad"))
            .args([mode, "--root", root.to_str().unwrap(), "--writable"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let to = child.stdin.take().unwrap();
        let from = BufReader::new(child.stdout.take().unwrap());
        Server {
            child,
            to,
            from,
            id: 0,
        }
    }

    fn ask(&mut self, method: &str, params: Json) -> Json {
        self.id += 1;
        let req = Json::obj(vec![
            ("jsonrpc", "2.0".into()),
            ("id", (self.id as usize).into()),
            ("method", method.into()),
            ("params", params),
        ]);
        writeln!(self.to, "{req}").unwrap();
        self.to.flush().unwrap();
        let mut line = String::new();
        self.from.read_line(&mut line).unwrap();
        let reply = Json::parse(&line).unwrap();
        assert!(reply.get("error").is_none(), "{line}");
        reply.get("result").cloned().unwrap()
    }

    /// An MCP tool call: (is it an error, its text content as JSON, whether it had an image).
    fn tool(&mut self, name: &str, args: Vec<(&str, Json)>) -> (bool, Json, bool) {
        let r = self.ask(
            "tools/call",
            Json::obj(vec![("name", name.into()), ("arguments", Json::obj(args))]),
        );
        let Some(Json::Arr(content)) = r.get("content") else {
            panic!("{r}")
        };
        let image = content
            .iter()
            .any(|c| c.get("type").and_then(Json::str) == Some("image"));
        let text = content
            .iter()
            .find(|c| c.get("type").and_then(Json::str) == Some("text"))
            .and_then(|c| c.get("text"))
            .and_then(Json::str)
            .unwrap();
        let err = r.get("isError") == Some(&Json::Bool(true));
        (
            err,
            Json::parse(text).unwrap_or_else(|e| panic!("{e}: {text}")),
            image,
        )
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-fg-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(d).unwrap()
}

fn arr(j: &Json, k: &str) -> Vec<Json> {
    match j.get(k) {
        Some(Json::Arr(xs)) => xs.clone(),
        _ => vec![],
    }
}

fn s(j: &Json, k: &str) -> String {
    j.get(k).and_then(Json::str).unwrap_or_default().to_string()
}

#[test]
fn an_agent_reads_a_lint_sends_its_fix_back_and_the_file_is_the_ops_edit() {
    let dir = scratch("mcp");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("mcp", &dir);
    let init = m.ask(
        "initialize",
        Json::obj(vec![("protocolVersion", "2025-06-18".into())]),
    );
    assert_eq!(
        init.get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(Json::str),
        Some("forgecad")
    );
    let tools = m.ask("tools/list", Json::obj(vec![]));
    let edit = arr(&tools, "tools")
        .into_iter()
        .find(|t| s(t, "name") == "edit")
        .unwrap();
    let op_schema = edit
        .get("inputSchema")
        .and_then(|i| i.get("properties"))
        .and_then(|p| p.get("op"))
        .unwrap();
    assert!(op_schema.get("type").is_none(), "an op is an object: {op_schema}");

    // the build: typed features, params, and the lint with its fixes
    let (err, stat, _) = m.tool("query", vec![("file", f.into()), ("sel", "stat".into())]);
    assert!(!err, "{stat}");
    assert_eq!(stat.get("ok"), Some(&Json::Bool(true)));
    let ids: Vec<String> = arr(&stat, "features").iter().map(|x| s(x, "id")).collect();
    assert_eq!(ids, ["plan", "base", "mark", "dent"]);
    let lint = arr(&stat, "warnings")
        .into_iter()
        .find(|w| s(w, "kind") == "LooseLiteral")
        .unwrap();
    assert_eq!(s(&lint, "severity"), "lint");
    let fix = arr(&lint, "fixes")[0].clone();
    assert_eq!(s(&fix, "op"), "LiftLiteral");

    // the fix, sent back as it came
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", fix.clone())]);
    assert!(!err, "{r}");
    assert_eq!(arr(&r, "applied").len(), 1);
    assert!(
        arr(&r, "warnings").iter().all(|w| s(w, "kind") != "LooseLiteral"),
        "{r}"
    );
    let want = forge_lang::edit::apply(PLATE, &Op::from_json(&fix).unwrap()).unwrap();
    let after_fix = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after_fix, want);

    // two ops at once, the second as JSON text: what SetParam reached comes back
    let set = Json::parse(r#"{"op":"SetParam","param":"t","value":{"value":8,"dim":"Length"}}"#).unwrap();
    let set2 = r#"{"op":"SetParam","param":"dent.depth","value":{"value":3,"dim":"Length"}}"#;
    let (err, r, _) = m.tool(
        "edit",
        vec![
            ("file", f.into()),
            ("ops", Json::Arr(vec![set.clone(), set2.into()])),
        ],
    );
    assert!(!err, "{r}");
    let applied = arr(&r, "applied");
    assert_eq!(applied.len(), 2);
    let reached: Vec<String> = arr(&applied[0], "features")
        .iter()
        .map(|x| x.str().unwrap().to_string())
        .collect();
    assert!(reached.contains(&"base".to_string()), "{reached:?}");
    let want = forge_lang::edit::apply(
        &forge_lang::edit::apply(&after_fix, &Op::from_json(&set).unwrap()).unwrap(),
        &Op::from_json(&Json::parse(set2).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), want);

    // refusals are typed, and nothing is written: a wrong dimension; a feature still read
    let bad = Json::parse(r#"{"op":"SetParam","param":"t","value":{"value":30,"dim":"Angle"}}"#).unwrap();
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", bad)]);
    assert!(err);
    assert_eq!(
        r.get("error").map(|e| s(e, "kind")).as_deref(),
        Some("EditDim"),
        "{r}"
    );
    let del = Json::parse(r#"{"op":"DeleteFeature","id":"base"}"#).unwrap();
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", del)]);
    assert!(err);
    let e = r.get("error").unwrap();
    assert_eq!(s(e, "kind"), "StillRead");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), want);

    // query, measure and render on the built part
    let (err, r, _) = m.tool("query", vec![("file", f.into()), ("sel", "params".into())]);
    assert!(!err);
    let t = arr(&r, "params")
        .into_iter()
        .find(|p| s(p, "param") == "t")
        .unwrap();
    assert_eq!(
        t.get("value").and_then(|v| v.get("value")).and_then(Json::num),
        Some(8.0)
    );
    let (err, r, _) = m.tool(
        "query",
        vec![("file", f.into()), ("sel", "faces plane normal +z".into())],
    );
    assert!(!err, "{r}");
    let found = arr(&r, "found");
    assert_eq!(found.len(), 2, "the top and the dent's floor: {r}");
    let h = s(&found[0], "h");
    let (err, r, _) = m.tool("measure", vec![("file", f.into()), ("what", h.as_str().into())]);
    assert!(!err, "{r}");
    let (err, r, _) = m.tool("measure", vec![("file", f.into()), ("what", "mass".into())]);
    assert!(!err);
    let v = r.get("volume").and_then(Json::num).unwrap();
    let want_v = 60.0 * 60.0 * 8.0 - std::f64::consts::PI * 16.0 * 3.0;
    assert!((v - want_v).abs() < 1e-3 * want_v, "{v} vs {want_v}");
    let (err, r, image) = m.tool("render", vec![("file", f.into()), ("px", Json::Num(200.0))]);
    assert!(!err && image, "{r}");

    // every feature as built, not "not built": features and check build (the stages kept)
    for q in ["features", "check"] {
        let (err, r, _) = m.tool("query", vec![("file", f.into()), ("sel", q.into())]);
        assert!(!err, "{q}: {r}");
        let st: Vec<String> = arr(&r, "features").iter().map(|x| s(x, "status")).collect();
        assert_eq!(st, ["ok"; 4], "{q}: {r}");
    }
    // a numbers question as one string
    let (err, r, _) = m.tool("query", vec![("file", f.into()), ("sel", "section z=1".into())]);
    assert!(!err, "{r}");

    // a part changes by edit alone: there is no `act` tool
    let names: Vec<String> = arr(&tools, "tools").iter().map(|t| s(t, "name")).collect();
    assert!(!names.iter().any(|n| n == "act" || n == "diff"), "{names:?}");
    let (err, r, _) = m.tool("act", vec![("file", f.into()), ("lines", "x".into())]);
    assert!(err && s(&r, "error").contains("edit F OP"), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn serve_keeps_the_stages_and_refuses_files_outside_the_root() {
    let dir = scratch("serve");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("serve", &dir);
    let first = m.ask("build", Json::obj(vec![("file", f.into())]));
    assert_eq!(first.get("ok"), Some(&Json::Bool(true)), "{first}");
    assert_eq!(first.get("reused").and_then(Json::num), Some(0.0));
    let again = m.ask("build", Json::obj(vec![("file", f.into())]));
    assert_eq!(again.get("rebuilt").and_then(Json::num), Some(0.0), "{again}");
    assert_eq!(again.get("reused").and_then(Json::num), Some(4.0));
    let total = again
        .get("timings_ms")
        .and_then(|t| t.get("total"))
        .and_then(Json::num)
        .unwrap();
    eprintln!("serve, unchanged .fg rebuild: {total:.2} ms");
    // a check never builds
    let check = m.ask("check", Json::obj(vec![("file", f.into())]));
    assert_eq!(check.get("rebuilt").and_then(Json::num), Some(0.0));
    assert!(check.get("part").is_none());
    // outside the root
    let out = std::env::temp_dir().join(format!("forgecad-fg-outside-{}.fg", std::process::id()));
    std::fs::write(&out, PLATE).unwrap();
    let r = m.ask("build", Json::obj(vec![("file", out.to_str().unwrap().into())]));
    assert_eq!(r.get("ok"), Some(&Json::Bool(false)));
    assert!(
        s(&r, "error").contains("not a .fg part file under the root"),
        "{r}"
    );
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_parts_flat_pattern_exports_as_dxf() {
    let dir = scratch("sheet");
    let file = dir.join("l.fg");
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/sheet/l.fg"),
        &file,
    )
    .unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("serve", &dir);
    let b = m.ask("build", Json::obj(vec![("file", f.into())]));
    assert_eq!(b.get("ok"), Some(&Json::Bool(true)), "{b}");
    assert_eq!(arr(&b, "sheets").len(), 1);
    let r = m.ask(
        "export",
        Json::obj(vec![("file", f.into()), ("what", "dxf".into())]),
    );
    assert_eq!(r.get("ok"), Some(&Json::Bool(true)), "{r}");
    assert!(r.get("png").is_some());
    // the CLI writes it beside the part
    let st = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["export", f, "dxf"])
        .status()
        .unwrap();
    assert!(st.success());
    let dxf = std::fs::read_to_string(dir.join("l.dxf")).unwrap();
    assert!(dxf.contains("BEND"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_part_exports_its_fitted_edges_tight() {
    // holes across a bend make fitted edges on fitted faces: written for sheet metal under 1e-7,
    // so a reader that sets each edge's tolerance from a few samples finds none over it
    let dir = scratch("sheet-step");
    let file = dir.join("holes.fg");
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/sheet/holes.fg"),
        &file,
    )
    .unwrap();
    let out = dir.join("holes.step");
    let st = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args([
            "export",
            file.to_str().unwrap(),
            "ap214",
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(st.success());
    let text = std::fs::read(&out).unwrap();
    let got = forge_step::import(&text, &Default::default()).unwrap();
    assert!(
        got.stats.curve_off_surface < 1e-6,
        "{:?}",
        got.stats.curve_off_surface
    );
    assert!(
        got.stats.pairs.keys().any(|k| k.contains("Procedural")),
        "{:?}",
        got.stats.pairs
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_build_summary_gives_its_measures_to_the_thousandth() {
    let dir = scratch("summary");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["build", file.to_str().unwrap()])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    let part = text.lines().find(|l| l.starts_with("part: volume ")).unwrap();
    let v = part.split(' ').nth(2).unwrap();
    assert!(v.split('.').nth(1).is_none_or(|d| d.len() <= 3), "{part}");
    let want = 60.0 * 60.0 * 6.0 - std::f64::consts::PI * 16.0 * 2.0;
    assert!((v.parse::<f64>().unwrap() - want).abs() < 1e-3, "{part}");
    // `exact`: every digit
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["measure", file.to_str().unwrap(), "mass", "exact"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let v: f64 = text.split(' ').nth(1).unwrap().parse().unwrap();
    assert!((v - want).abs() < 1e-9 * want, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn diff_says_what_the_second_part_changed() {
    let dir = scratch("diff");
    let (a, b) = (dir.join("a.fg"), dir.join("b.fg"));
    std::fs::write(&a, PLATE).unwrap();
    std::fs::write(&b, PLATE.replace("shared t = 6", "shared t = 8")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args([
            "diff",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "volume",
            "--json",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    let j = Json::parse(&text).unwrap();
    let params = arr(&j, "params");
    assert!(params.iter().any(|p| s(p, "name") == "t"), "{text}");
    // the exact volume added and removed, or why the kernel could not split it so
    assert!(
        j.get("volume").is_some() || j.get("volume_error").is_some(),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A plate, a boss on it and a hole at two points made in a loop (`pin[a]`, `pin[b]`).
const PINNED: &str = "shared L = 60
shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param top.r = 8
param top.e = 10
sketch top(on: base.end) {
  c = circle((L / 2, L / 2), r)
  p0 = pt(e, e)
  p1 = pt(L - e, e)
}
param boss.h = 10
boss = extrude(top.c, h)
param pin.d = 4
for k, p in {a: top.p0, b: top.p1} {
  pin = hole(at: p, d: d, depth: thru)
}
";

fn found(j: &Json) -> usize {
    arr(j, "found").len()
}

#[test]
fn query_measure_and_render_see_the_part_after_a_feature_and_select_by_feature() {
    let dir = scratch("at");
    let file = dir.join("pinned.fg");
    std::fs::write(&file, PINNED).unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("mcp", &dir);
    let q = |m: &mut Server, sel: &str, at: Option<&str>| {
        let mut a = vec![("file", f.into()), ("sel", sel.into())];
        if let Some(at) = at {
            a.push(("at", at.into()));
        }
        m.tool("query", a)
    };
    // the finished part, and as it stood after the plate and after the boss
    let (err, all, _) = q(&mut m, "faces plane", None);
    assert!(!err, "{all}");
    assert_eq!(found(&all), 7, "{all}");
    let (_, plate, _) = q(&mut m, "faces", Some("base"));
    assert_eq!(found(&plate), 6, "{plate}");
    let (_, bossed, _) = q(&mut m, "faces plane normal +z top", Some("boss"));
    assert_eq!(found(&bossed), 1, "{bossed}");
    let (err, mass, _) = m.tool(
        "measure",
        vec![("file", f.into()), ("what", "mass".into()), ("at", "base".into())],
    );
    assert!(!err, "{mass}");
    assert!((mass.get("volume").and_then(Json::num).unwrap() - 60.0 * 60.0 * 6.0).abs() < 1e-6);
    let (err, pic, image) = m.tool(
        "render",
        vec![
            ("file", f.into()),
            ("at", "boss".into()),
            ("labels", "faces of boss".into()),
        ],
    );
    assert!(!err && image, "{pic}");
    // by feature: one feature, a loop's features by their name, where two meet
    let (_, boss, _) = q(&mut m, "faces of boss", None);
    assert_eq!(found(&boss), 2, "{boss}");
    let (_, pins, _) = q(&mut m, "faces of pin", None);
    assert_eq!(found(&pins), 2, "{pins}");
    let (_, one, _) = q(&mut m, "faces of pin[a]", None);
    assert_eq!(found(&one), 1, "{one}");
    let (_, seat, _) = q(&mut m, "edges between boss base concave", None);
    assert_eq!(found(&seat), 1, "{seat}");
    let (_, rims, _) = q(&mut m, "edges between pin base", None);
    assert_eq!(found(&rims), 4, "{rims}");
    let (_, rim, _) = q(&mut m, "edges of boss convex", Some("boss"));
    assert_eq!(found(&rim), 1, "{rim}");
    // a ref by feature measures
    let (err, d, _) = m.tool(
        "measure",
        vec![
            ("file", f.into()),
            ("what", "(faces of boss plane)".into()),
            ("to", "(faces of base plane normal -z)".into()),
        ],
    );
    assert!(!err, "{d}");
    assert!(
        (d.get("distance").and_then(Json::num).unwrap() - 16.0).abs() < 1e-6,
        "{d}"
    );
    // what is not a feature, or not built, says so
    let (err, no, _) = q(&mut m, "faces", Some("nope"));
    assert!(err && s(&no, "error").contains("'nope'"), "{no}");
    let (err, no, _) = q(&mut m, "faces of nope", None);
    assert!(err && s(&no, "error").contains("boss"), "{no}");
    let (err, no, _) = q(&mut m, "stat", Some("base"));
    assert!(err, "{no}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_round_too_big_comes_back_with_the_largest_that_builds_and_an_edit_undoes() {
    let dir = scratch("fit");
    let file = dir.join("plate.fg");
    let big = format!("{PLATE}param round.r = 25\nround = fillet(edges(of: base.end), r)\n");
    std::fs::write(&file, &big).unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("mcp", &dir);
    let (err, r, _) = m.tool("build", vec![("file", f.into())]);
    assert!(err, "{r}");
    let e = arr(&r, "errors").into_iter().next().expect("refused");
    let fix = arr(&e, "fixes")
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{e}"));
    assert_eq!(s(&fix, "op"), "SetParam", "{fix}");
    let v = fix
        .get("value")
        .and_then(|v| v.get("value"))
        .and_then(Json::num)
        .unwrap();
    assert!(v > 0.0 && v < 25.0, "{fix}");
    // the fix sent back builds
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", fix.clone())]);
    assert!(!err, "{r}");
    let fixed = std::fs::read_to_string(&file).unwrap();
    assert_ne!(fixed, big);
    // taken back, and put back again
    let undo = Json::parse(r#"{"op":"Undo"}"#).unwrap();
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", undo)]);
    assert!(err, "the round too big again: {r}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), big);
    let redo = Json::parse(r#"{"op":"Redo"}"#).unwrap();
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", redo.clone())]);
    assert!(!err, "{r}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), fixed);
    // nothing left to redo: refused, nothing written
    let (err, r, _) = m.tool("edit", vec![("file", f.into()), ("op", redo)]);
    assert!(err && s(&r, "error").contains("redo"), "{r}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), fixed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_edit_against_a_version_the_file_has_moved_on_from_is_refused() {
    let dir = scratch("base");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("mcp", &dir);
    let (_, built, _) = m.tool("build", vec![("file", f.into())]);
    let v0 = s(&built, "version");
    assert_eq!(v0.len(), 16, "{built}");
    let set = |d: f64| {
        Json::parse(&format!(
            r#"{{"op":"SetParam","param":"dent.depth","value":{{"value":{d},"dim":"Length"}}}}"#
        ))
        .unwrap()
    };
    // at the version the reply gave: applied, and the reply gives the new one
    let (err, r, _) = m.tool(
        "edit",
        vec![("file", f.into()), ("op", set(3.0)), ("base", v0.as_str().into())],
    );
    assert!(!err, "{r}");
    let v1 = s(&r, "version");
    assert!(v1.len() == 16 && v1 != v0, "{r}");
    let after = std::fs::read_to_string(&file).unwrap();
    // the same version again: the file has moved on, so nothing is written
    let (err, r, _) = m.tool(
        "edit",
        vec![("file", f.into()), ("op", set(1.0)), ("base", v0.as_str().into())],
    );
    assert!(err, "{r}");
    let e = r.get("error").cloned().unwrap();
    assert_eq!(s(&e, "kind"), "StaleBase", "{r}");
    assert_eq!(s(&e, "version"), v1, "{r}");
    assert_eq!(s(&r, "version"), v1, "{r}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), after);
    // the version it gave goes back: applied
    let (err, r, _) = m.tool(
        "edit",
        vec![("file", f.into()), ("op", set(1.0)), ("base", v1.as_str().into())],
    );
    assert!(!err, "{r}");
    // the file's text and its version, as query file gives them
    let (_, q, _) = m.tool("query", vec![("file", f.into()), ("sel", "file".into())]);
    assert_eq!(s(&q, "version"), s(&r, "version"));
    // on the command line too
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["edit", f, &set(2.0).to_string(), "--base", &v0, "--json"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("StaleBase"));
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(["query", f, "faces", "--at", "base"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && text.starts_with("6 found"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_part_is_bend_simulated_over_mcp_and_the_cli() {
    let dir = scratch("bendsim");
    let file = dir.join("l.fg");
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/sheet/l.fg"),
        &file,
    )
    .unwrap();
    let f = file.to_str().unwrap();
    let mut m = Server::start("mcp", &dir);
    let _ = m.ask("initialize", Json::obj(vec![]));
    let (err, j, _) = m.tool(
        "query",
        vec![
            ("file", f.into()),
            ("sel", "bendsim".into()),
            ("tooling", "std-us".into()),
        ],
    );
    assert!(!err, "{j}");
    assert_eq!(s(&j, "schema"), "forgecad.bendsim/1");
    assert_eq!(s(&j, "tooling"), "std-us");
    assert_eq!(j.get("feasible"), Some(&Json::Bool(true)), "{j}");
    assert!(!arr(&j, "steps").is_empty());
    // a bend that is not there: typed, with the bends it could be
    let (err, j, _) = m.tool(
        "query",
        vec![
            ("file", f.into()),
            ("sel", "bendsim".into()),
            ("sequence", "nope".into()),
        ],
    );
    assert!(err, "{j}");
    // the CLI's flags
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args([
            "query",
            f,
            "bendsim",
            "--tooling",
            "std-eu",
            "--sequence",
            "1",
            "--json",
        ])
        .output()
        .unwrap();
    let j = Json::parse(String::from_utf8_lossy(&out.stdout).trim()).unwrap();
    assert_eq!(s(&j, "tooling"), "std-eu");
    assert_eq!(arr(&j, "sequence").len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
