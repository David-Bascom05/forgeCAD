//! `forgecad workholding list [KIND] | show NAME | schema | new NAME --from X [field=value ..]`:
//! (JSON, text) replies, the library read in search order (built-in, the user's, the project's).
use crate::schema::{new_from, schema_json};
use crate::{Kind, Library};
use std::path::Path;

fn js(s: &str) -> String {
    format!("{s:?}")
}

/// Runs a workholding command. `user` is the user's `<config>/forgecad` directory, `project` the project's
/// root; `new` writes `<project>/cam/workholding/NAME.toml` (refusing to overwrite).
pub fn tool(
    args: &[String],
    user: Option<&Path>,
    project: Option<&Path>,
) -> Result<(String, String), String> {
    let lib = Library::search(user, project).map_err(|e| e.to_string())?;
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a[..] {
        ["list"] | ["list", _] => {
            let k = match a.get(1) {
                Some(s) => Some(Kind::parse(s).ok_or_else(|| format!("no kind `{s}`"))?),
                None => None,
            };
            let mut j = Vec::new();
            let mut t = String::new();
            for (r, f) in &lib.records {
                if k.is_some_and(|k| k != r.kind) {
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
                    "{{\"name\":{},\"kind\":{},\"vendor\":{},\"product\":{},\"provenance\":{},\"unknown\":{},\"file\":{}}}",
                    js(&r.name),
                    js(r.kind.name()),
                    js(&r.vendor),
                    js(&r.product),
                    js(prov),
                    r.unknown.len(),
                    js(f)
                ));
                t += &format!(
                    "{:32} {:14} {:8} {}{}\n",
                    r.name,
                    r.kind.name(),
                    prov,
                    if r.vendor.is_empty() {
                        String::new()
                    } else {
                        format!("{} {}", r.vendor, r.product)
                    },
                    if r.unknown.is_empty() {
                        String::new()
                    } else {
                        format!("  (unknown: {})", r.unknown.join(", "))
                    }
                );
            }
            Ok((format!("{{\"workholding\":[{}]}}", j.join(",")), t))
        }
        ["show", name] => {
            let r = lib.get(name).map_err(|e| e.to_string())?;
            let t = r.to_toml();
            Ok((format!("{{\"name\":{},\"toml\":{}}}", js(&r.name), js(&t)), t))
        }
        ["schema"] => {
            let j = schema_json();
            let mut t = String::new();
            for k in crate::schema::schema() {
                t += &format!("{} — {}\n", k.name, k.about);
                for f in &k.fields {
                    t += &format!(
                        "  {:14} {:6} {:9} {}\n",
                        f.name,
                        f.unit,
                        if f.required { "required" } else { "optional" },
                        f.meaning
                    );
                }
            }
            Ok((j, t))
        }
        ["new", name, "--from", from, ref rest @ ..] => {
            let mut given = Vec::new();
            for kv in rest {
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| format!("`{kv}`: give field=value"))?;
                let v: f64 = v.parse().map_err(|_| format!("`{kv}`: a number"))?;
                given.push((k, v));
            }
            let r = new_from(&lib, name, from, &given).map_err(|e| e.to_string())?;
            let text = r.to_toml();
            let dir = project
                .ok_or("no project directory")?
                .join("cam")
                .join("workholding");
            let path = dir.join(format!("{name}.toml"));
            if path.exists() {
                return Err(format!("{} exists", path.display()));
            }
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(&path, &text).map_err(|e| e.to_string())?;
            Ok((
                format!(
                    "{{\"written\":{},\"toml\":{}}}",
                    js(&path.display().to_string()),
                    js(&text)
                ),
                format!("wrote {}\n{text}", path.display()),
            ))
        }
        _ => Err(
            "forgecad workholding list [KIND] | show NAME | schema | new NAME --from X [field=value ..]"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_show_schema_new() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let (j, t) = tool(&s(&["list", "chuck"]), None, None).unwrap();
        assert!(j.contains("kitagawa-b208") && t.contains("standin-chuck-200") && !t.contains("kurt"));
        assert!(
            tool(&s(&["show", "kurt-dx6"]), None, None)
                .unwrap()
                .1
                .contains("jaw_width = 152.4")
        );
        assert!(
            tool(&s(&["schema"]), None, None)
                .unwrap()
                .0
                .contains("workholding-schema/1")
        );
        let dir = forge_dirs::temp_dir().join(format!("wh-cli-{}", forge_dirs::process_id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (_, t) = tool(
            &s(&["new", "my-dx6", "--from", "kurt-dx6", "jaw_thickness=22"]),
            None,
            Some(&dir),
        )
        .unwrap();
        assert!(t.contains("jaw_thickness = 22"), "{t}");
        // the project level now holds it, above the built-in set
        let l = Library::search(None, Some(&dir)).unwrap();
        assert_eq!(l.get("my-dx6").unwrap().get("jaw_thickness"), Ok(22.0));
        assert!(tool(&s(&["new", "my-dx6", "--from", "kurt-dx6"]), None, Some(&dir)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
