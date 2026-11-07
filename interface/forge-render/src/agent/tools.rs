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
            "create_file",
            "Make a new .fg file (a new part, assembly or drawing) in the open file's folder; the app opens it. `name` is a plain file name, not a path, and mustn't exist yet.",
            vec![("name", string("e.g. window.fg")), ("text", string("the whole file"))],
            vec!["name", "text"],
        ),
        tool(
            "query",
            "Numbers from the open file, built: `holes`, `section z=67`, `aligned [A B]`, `gap A B`, `interference`, `frame SKETCH`, `faces`; assembly coordinates (`in OCC` for a part's), names in an assembly OCC/NAME.",
            vec![("query", string("e.g. holes, section z=67, gap rack/ear.bottom tray/right.bottom"))],
            vec!["query"],
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
        "create_file" => (|| {
            let name = args
                .get("name")
                .and_then(Json::str)
                .ok_or("create_file takes `name`")?;
            let text = args
                .get("text")
                .and_then(Json::str)
                .ok_or("create_file takes `text`")?;
            let plain = !name.is_empty()
                && !name.starts_with('.')
                && name.ends_with(".fg")
                && !name.contains(['/', '\\'])
                && !name.contains("..");
            if !plain {
                return Err(format!("`{name}` isn't a plain .fg file name"));
            }
            let dir = file.parent().ok_or("the open file has no folder")?;
            let new = dir.join(name);
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new)
                .map_err(|e| format!("{}: {e}", new.display()))?;
            std::io::Write::write_all(&mut f, text.as_bytes())
                .map_err(|e| format!("{}: {e}", new.display()))?;
            Ok(format!("Made {name}.\n{}", check(text, &new)))
        })(),
        "run_check" => read().map(|t| check(&t, file)),
        "query" => (|| {
            let text = read()?;
            let q = args
                .get("query")
                .and_then(Json::str)
                .ok_or("query takes `query`")?;
            let a = forge_lang::query::ask(&text, file.parent().map(|d| d.to_path_buf()), None, q);
            if a.ok { Ok(a.text) } else { Err(a.text) }
        })(),
        other => Err(format!("no tool `{other}`")),
    };
    match got {
        Ok(s) => (s, false),
        Err(e) => (e, true),
    }
}

/// The part built and checked, as `fg run` reports it.
pub fn check(text: &str, file: &Path) -> String {
    // a drawing: made as the app makes it, its views, annotations and problems
    if forge_lang::drawing::is_drawing(text) {
        let dir = file.parent().unwrap_or(Path::new("."));
        let d = forge_lang::drawing::regen::make_in(text, dir, Some(dir));
        let mut s = String::new();
        forge_lang::drawing::regen::json(&d, text).write(&mut s);
        return s;
    }
    let opts = forge_lang::Options {
        build: true,
        dir: file.parent().map(|d| d.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(text, opts, Default::default());
    forge_lang::report::text(&run, text)
}

/// A file name the agent gave, as a file in `dir`: plain names (and sub-folders) only, no `..`,
/// no absolute path, a `.fg`, and still in `dir` once links are followed.
pub fn in_folder(dir: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    let p = Path::new(name);
    let plain = !name.is_empty()
        && !name.starts_with('-')
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
        && name.ends_with(".fg");
    if !plain {
        return Err(format!(
            "`{name}` isn't a .fg file name in the folder (no `..`, no absolute path, no flags)"
        ));
    }
    let root = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let full = std::fs::canonicalize(root.join(p)).map_err(|e| format!("{name}: {e}"))?;
    if !full.starts_with(&root) {
        return Err(format!("`{name}` leads out of the folder"));
    }
    Ok(full)
}

/// The fg CLI's work, in the app: run, check, edit, handles, reference, each on a file named in
/// the open document's folder. Its answer and whether it failed.
pub fn fg_call(name: &str, args: &Json, dir: &Path) -> (String, bool) {
    if name == "fg_reference" {
        return (forge_lang::REFERENCE.to_string(), false);
    }
    let got = (|| {
        let f = in_folder(dir, args.get("file").and_then(Json::str).unwrap_or(""))?;
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        match name {
            "fg_run" => Ok(check(&text, &f)),
            "fg_query" => {
                let q = args
                    .get("query")
                    .and_then(Json::str)
                    .ok_or("fg_query takes `query`")?;
                let a = forge_lang::query::ask(
                    &text,
                    f.parent().map(|d| d.to_path_buf()),
                    Some(dir.to_path_buf()),
                    q,
                );
                if a.ok { Ok(a.text) } else { Err(a.text) }
            }
            "fg_check" if forge_lang::drawing::is_drawing(&text) => Ok(check(&text, &f)),
            "fg_check" => {
                let opts = forge_lang::Options {
                    build: false,
                    dir: f.parent().map(|d| d.to_path_buf()),
                    ..forge_lang::Options::default()
                };
                let (run, _) = forge_lang::run(&text, opts, Default::default());
                Ok(forge_lang::report::text(&run, &text))
            }
            "fg_handles" => Ok(forge_lang::edit::handles(&text)
                .into_iter()
                .map(|(k, l)| format!("{k} = {}  ({})\n", l.text(), l.dim.name()))
                .collect()),
            "fg_edit" => {
                let op = args.get("op").ok_or("fg_edit takes `op`")?;
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
                std::fs::write(&f, &new).map_err(|e| format!("{}: {e}", f.display()))?;
                Ok(format!("Applied.\n{}", check(&new, &f)))
            }
            other => Err(format!("no tool `{other}`")),
        }
    })();
    match got {
        Ok(s) => (s, false),
        Err(e) => (e, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_asks_the_folders_files_for_numbers() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sheetmetal");
        let args = Json::obj(vec![
            ("file", "tray_asm.fg".into()),
            (
                "query",
                "aligned tray/right_holes.faces#1 rack/right_holes.faces#0".into(),
            ),
        ]);
        let (text, failed) = fg_call("fg_query", &args, &dir);
        assert!(!failed && text.starts_with("coaxial"), "{text}");
        let (text, failed) = call(
            "query",
            &Json::obj(vec![("query", "holes".into())]),
            &dir.join("load_tray.fg"),
        );
        assert!(!failed && text.starts_with("9 holes"), "{text}");
        let (_, failed) = fg_call(
            "fg_query",
            &Json::obj(vec![("file", "tray_asm.fg".into()), ("query", "gap".into())]),
            &dir,
        );
        assert!(failed);
    }
}
