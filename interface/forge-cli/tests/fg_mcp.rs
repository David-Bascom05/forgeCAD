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

    // act is .fc's; edit is .fg's; help forge is the language
    let (err, r, _) = m.tool("act", vec![("file", f.into()), ("lines", "x".into())]);
    assert!(err && s(&r, "error").contains("typed ops"), "{r}");
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
