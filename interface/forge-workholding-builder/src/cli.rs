//! `forgecad workholding list [KIND] | new ID --from X [field=value ..] [--user] | check [ID ..] |
//! show ID | schema` [--json]: exit 0 ok, 1 problems, 2 usage (the library builder's
//! convention). The catalog is the project's `cam/workholding`, the user's
//! `<config>/forgecad/cam/workholding`, then the built-in set.
use crate::{Workholding, check, fields};
use forge_library::{Catalog, Level, project_dir, save, user_dir};
use std::path::Path;

pub struct Out {
    pub code: u8,
    pub text: String,
}

fn js(s: &str) -> String {
    format!("{s:?}")
}

const USAGE: &str = "forgecad workholding list [KIND] | new ID --from X [field=value ..] [--user] | check [ID ..] | show ID | schema [--json]";

/// Runs the command; `project` is the project folder (the current directory), `user` the user's
/// workholding folder (None: `forge_library::user_dir`).
pub fn run(args: &[String], project: &Path, user: Option<&Path>) -> Out {
    let json = args.iter().any(|a| a == "--json");
    let to_user = args.iter().any(|a| a == "--user");
    let a: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json" && *a != "--user")
        .collect();
    let udir = user.map(Path::to_path_buf).or_else(user_dir::<Workholding>);
    let cat = Catalog::<Workholding>::load(Some(project), udir.as_deref());
    let out = |code: u8, j: String, t: String| Out {
        code,
        text: if json { j } else { t },
    };
    match a[..] {
        ["list"] | ["list", _] => {
            let kind = match a.get(1) {
                Some(s) => match forge_workholding::Kind::parse(s) {
                    Some(k) => Some(k),
                    None => {
                        return out(
                            2,
                            format!("{{\"error\":{}}}", js(&format!("no kind `{s}`"))),
                            format!("no kind `{s}`\n"),
                        );
                    }
                },
                None => None,
            };
            let mut j = Vec::new();
            let mut t = String::new();
            for e in &cat.entries {
                let r = &e.def;
                if kind.is_some_and(|k| k != r.kind) {
                    continue;
                }
                let prov = if r.standin {
                    "standin"
                } else if r.cites.iter().any(|c| c.source.starts_with("user")) {
                    "user"
                } else {
                    "cited"
                };
                j.push(format!(
                    "{{\"id\":{},\"kind\":{},\"level\":{},\"file\":{},\"provenance\":{},\"unknown\":{},\"conflict\":{}}}",
                    js(&r.name),
                    js(r.kind.name()),
                    js(e.level.name()),
                    js(&e.file),
                    js(prov),
                    r.unknown.len(),
                    r.conflict.len()
                ));
                t += &format!(
                    "{:32} {:13} {:8} {:8} {} {}\n",
                    r.name,
                    r.kind.name(),
                    e.level.name(),
                    prov,
                    r.vendor,
                    r.product
                );
            }
            for (l, f, why) in &cat.errors {
                t += &format!("error ({}) {f}: {why}\n", l.name());
            }
            out(
                u8::from(!cat.errors.is_empty()),
                format!("{{\"workholding\":[{}]}}", j.join(",")),
                t,
            )
        }
        ["show", id] => match cat.get(id) {
            Some(e) => out(
                0,
                format!("{{\"id\":{},\"toml\":{}}}", js(id), js(&e.def.to_toml())),
                e.def.to_toml(),
            ),
            None => out(
                1,
                format!("{{\"error\":{}}}", js(&format!("no workholding `{id}`"))),
                format!("no workholding `{id}`\n"),
            ),
        },
        ["schema"] => {
            let mut t = String::new();
            for k in forge_workholding::Kind::all() {
                t += &format!("{}\n", k.name());
                for f in fields(k) {
                    t += &format!("  {:14} {:5} {:8} {}\n", f.path, f.unit.suffix(), f.group, f.help);
                }
            }
            out(0, forge_workholding::schema::schema_json(), t)
        }
        ["check", ref ids @ ..] => {
            let mut bad = !cat.errors.is_empty();
            let mut t = String::new();
            let mut j = Vec::new();
            for e in &cat.entries {
                if !ids.is_empty() && !ids.contains(&e.def.name.as_str()) {
                    continue;
                }
                let f = check(&e.def);
                bad |= f.iter().any(|x| !x.not_checked);
                for x in &f {
                    t += &format!(
                        "{}: {} ({}) {}\n",
                        e.def.name,
                        x.what,
                        x.fix,
                        if x.not_checked { "[not checked]" } else { "" }
                    );
                    j.push(format!(
                        "{{\"id\":{},\"path\":{},\"what\":{},\"fix\":{},\"not_checked\":{}}}",
                        js(&e.def.name),
                        js(&x.path),
                        js(&x.what),
                        js(&x.fix),
                        x.not_checked
                    ));
                }
            }
            for id in ids {
                if cat.get(id).is_none() {
                    bad = true;
                    t += &format!("no workholding `{id}`\n");
                }
            }
            for (l, f, why) in &cat.errors {
                t += &format!("error ({}) {f}: {why}\n", l.name());
            }
            out(u8::from(bad), format!("{{\"findings\":[{}]}}", j.join(",")), t)
        }
        ["new", id, "--from", from, ref rest @ ..] => {
            let mut given = Vec::new();
            for kv in rest {
                let Some((k, v)) = kv.split_once('=') else {
                    return out(
                        2,
                        format!("{{\"error\":{}}}", js(USAGE)),
                        format!("`{kv}`: field=value\n{USAGE}\n"),
                    );
                };
                let Ok(v) = v.parse::<f64>() else {
                    return out(
                        2,
                        format!("{{\"error\":{}}}", js(USAGE)),
                        format!("`{kv}`: a number\n"),
                    );
                };
                given.push((k, v));
            }
            let lib = forge_workholding::Library {
                records: cat
                    .entries
                    .iter()
                    .map(|e| (e.def.clone(), e.file.clone()))
                    .collect(),
            };
            let r = match forge_workholding::schema::new_from(&lib, id, from, &given) {
                Ok(r) => r,
                Err(e) => {
                    return out(
                        1,
                        format!("{{\"error\":{}}}", js(&e.to_string())),
                        format!("{e}\n"),
                    );
                }
            };
            let (dir, level) = if to_user {
                match &udir {
                    Some(d) => (d.clone(), Level::User),
                    None => {
                        return out(
                            1,
                            "{\"error\":\"no user folder\"}".into(),
                            "no user folder\n".into(),
                        );
                    }
                }
            } else {
                (project_dir::<Workholding>(project), Level::Project)
            };
            match save::<Workholding>(&r, &[], &dir, level, false) {
                Ok(p) => {
                    let f = check(&r);
                    let mut t = format!("wrote {}\n", p.display());
                    for x in &f {
                        t += &format!("{}: {} ({})\n", r.name, x.what, x.fix);
                    }
                    out(0, format!("{{\"written\":{}}}", js(&p.display().to_string())), t)
                }
                Err(e) => out(1, format!("{{\"error\":{}}}", js(&e)), format!("{e}\n")),
            }
        }
        _ => out(2, format!("{{\"usage\":{}}}", js(USAGE)), format!("{USAGE}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_new_check_show_and_usage() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let dir = forge_dirs::temp_dir().join(format!("wh-builder-{}", forge_dirs::process_id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let user = dir.join("user");
        let o = run(&s(&["list", "chuck"]), &dir, Some(&user));
        assert_eq!(o.code, 0);
        assert!(o.text.contains("kitagawa-b208") && !o.text.contains("kurt-dx6"));
        let o = run(
            &s(&[
                "new",
                "my-dx6",
                "--from",
                "kurt-dx6",
                "jaw_thickness=22",
                "height=117.1",
            ]),
            &dir,
            Some(&user),
        );
        assert_eq!(o.code, 0, "{}", o.text);
        assert!(dir.join("cam/workholding/my-dx6.toml").exists());
        let o = run(&s(&["show", "my-dx6"]), &dir, Some(&user));
        assert!(o.text.contains("jaw_thickness = 22"), "{}", o.text);
        let o = run(&s(&["check", "my-dx6"]), &dir, Some(&user));
        assert_eq!(o.code, 0, "{}", o.text);
        // again: refused (the file exists)
        assert_eq!(
            run(&s(&["new", "my-dx6", "--from", "kurt-dx6"]), &dir, Some(&user)).code,
            1
        );
        assert_eq!(run(&s(&["frobnicate"]), &dir, Some(&user)).code, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
