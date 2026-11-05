//! The tools ForgeCAD's own agent loop gives a model: the open file read, edited by text or by a
//! typed op, built and checked. Each works on the one open file and takes no path: nothing
//! else can be read, written or deleted.

use forge_agent::json::Json;
use forge_lang::error::Op;
use std::path::Path;

/// The tools, as chat completions' `tools` take them.
pub fn specs() -> Json {
    let tool = |name: &str, what: &str, props: Vec<(&str, Json)>, required: Vec<&str>| {
        Json::obj(vec![
            ("type", "function".into()),
            (
                "function",
                Json::obj(vec![
                    ("name", name.into()),
                    ("description", what.into()),
                    (
                        "parameters",
                        Json::obj(vec![
                            ("type", "object".into()),
                            ("properties", Json::obj(props)),
                            (
                                "required",
                                Json::Arr(required.into_iter().map(Json::from).collect()),
                            ),
                        ]),
                    ),
                ]),
            ),
        ])
    };
    let string = |what: &str| Json::obj(vec![("type", "string".into()), ("description", what.into())]);
    Json::Arr(vec![
        tool("read_file", "The open .fg file, each line numbered.", vec![], vec![]),
        tool(
            "replace_text",
            "Replace text in the open file: `old` must occur exactly once. Writes the file, then builds and checks it.",
            vec![("old", string("the exact text now in the file")), ("new", string("what replaces it"))],
            vec!["old", "new"],
        ),
        tool(
            "apply_op",
            "Apply a typed Forge edit op (JSON, as the check's fixes give them) to the open file; refused if the result wouldn't check.",
            vec![(
                "op",
                Json::obj(vec![
                    ("type", "object".into()),
                    ("description", "the op, e.g. {\"op\":\"SetParam\",\"param\":\"extrude1.d\",\"value\":{\"value\":22,\"dim\":\"Length\"}}".into()),
                ]),
            )],
            vec!["op"],
        ),
        tool(
            "run_check",
            "Build the open file and report its errors, warnings and measures.",
            vec![],
            vec![],
        ),
    ])
}

/// Runs a tool on the file: what it says back, and whether it failed.
pub fn call(name: &str, args: &Json, file: &Path) -> (String, bool) {
    let read = || std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()));
    let write = |text: &str| std::fs::write(file, text).map_err(|e| format!("{}: {e}", file.display()));
    let got = match name {
        "read_file" => read().map(|t| {
            t.lines()
                .enumerate()
                .map(|(k, l)| format!("{:>4}  {l}\n", k + 1))
                .collect::<String>()
        }),
        "replace_text" => (|| {
            let text = read()?;
            let old = args
                .get("old")
                .and_then(Json::str)
                .ok_or("replace_text takes `old`")?;
            let new = args
                .get("new")
                .and_then(Json::str)
                .ok_or("replace_text takes `new`")?;
            match text.matches(old).count() {
                0 => return Err("`old` isn't in the file: read it again".to_string()),
                1 => {}
                n => return Err(format!("`old` occurs {n} times: give more of the text around it")),
            }
            let text = text.replacen(old, new, 1);
            write(&text)?;
            Ok(format!("Written.\n{}", check(&text, file)))
        })(),
        "apply_op" => (|| {
            let text = read()?;
            let op = args.get("op").ok_or("apply_op takes `op`")?;
            let op = match op {
                Json::Str(s) => Json::parse(s).map_err(|e| format!("op: {e}"))?,
                o => o.clone(),
            };
            let op = Op::from_json(&op).ok_or("not a Forge edit op")?;
            let (new, _) = forge_lang::edit::apply_checked(&text, &op).map_err(|e| {
                let mut s = String::from("refused: ");
                e.json(&text).write(&mut s);
                s
            })?;
            write(&new)?;
            Ok(format!("Applied.\n{}", check(&new, file)))
        })(),
        "run_check" => read().map(|t| check(&t, file)),
        other => Err(format!("no tool `{other}`")),
    };
    match got {
        Ok(s) => (s, false),
        Err(e) => (e, true),
    }
}

/// The part built and checked, as `fg run` reports it.
pub fn check(text: &str, file: &Path) -> String {
    let opts = forge_lang::Options {
        build: true,
        dir: file.parent().map(|d| d.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(text, opts, Default::default());
    forge_lang::report::text(&run, text)
}
