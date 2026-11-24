//! `forgecad tool|holder|collet|collet_nut|pull_stud|insert|turn_holder list|new|check`: the
//! machine builder's words for forge-tooling's records (exit 0 ok, 1 problems or a refusal, 2
//! usage).
use crate::{ColletNuts, Collets, Holders, Inserts, PullStuds, Rec, Tools, TurnHolders};
use forge_library::{Catalog, Kind, Level};
use forge_tooling::build::{self, DeriveError};
use forge_tooling::toml::Val;
use std::path::{Path, PathBuf};

pub struct Env {
    /// The project folder (`cam/<folder>` under it is the project level).
    pub project: PathBuf,
    /// The user's `cam` folder (each kind's folder under it); None: no user level.
    pub user_cam: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct Out {
    pub code: u8,
    pub out: String,
    pub err: String,
}

fn fail(code: u8, err: String) -> Out {
    Out {
        code,
        out: String::new(),
        err,
    }
}

pub fn usage(word: &str) -> String {
    format!(
        "forgecad {word} list [--json]\n       \
forgecad {word} new ID --from ID [FIELD=VALUE ..] [--project | --user]\n       \
forgecad {word} check ID|FILE.toml [--json]"
    )
}

/// A value as typed on the command line: a TOML value, or else a word.
pub fn value_of(s: &str) -> Val {
    forge_tooling::toml::parse(&format!("v = {s}"))
        .ok()
        .and_then(|t| t.get("v").cloned())
        .unwrap_or_else(|| Val::Str(s.to_string()))
}

fn jstr(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn status_name(s: forge_tooling::CiteStatus) -> String {
    format!("{s:?}").to_lowercase()
}

/// Each cite of a record's table: (path, status).
fn cites(r: &Rec) -> Vec<(String, String)> {
    match r.table.get("cite") {
        Some(Val::Tables(v)) => v
            .iter()
            .map(|c| {
                (
                    forge_tooling::toml::text(c, "path").unwrap_or("?").to_string(),
                    forge_tooling::toml::text(c, "status").unwrap_or("?").to_string(),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn user_dir<K: Kind>(env: &Env) -> Option<PathBuf> {
    env.user_cam.as_ref().map(|u| u.join(K::FOLDER))
}

fn list<K: Kind<Record = Rec>>(env: &Env, json: bool) -> Out {
    let cat = Catalog::<K>::load(Some(&env.project), user_dir::<K>(env).as_deref());
    let mut o = String::new();
    if json {
        let rows: Vec<String> = cat
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"id\":{},\"level\":{},\"file\":{},\"hides\":{}}}",
                    jstr(&e.def.id),
                    jstr(e.level.name()),
                    jstr(&e.file),
                    e.hides.len()
                )
            })
            .collect();
        o = format!(
            "{{\"format\":\"forgecad.library/1\",\"kind\":{},\"records\":[{}],\"errors\":{}}}\n",
            jstr(K::WORD),
            rows.join(","),
            cat.errors.len()
        );
    } else {
        let w = cat.entries.iter().map(|e| e.def.id.len()).max().unwrap_or(0);
        for e in &cat.entries {
            o.push_str(&format!("{:w$}  {:8} {}\n", e.def.id, e.level.name(), e.file));
            for (l, f) in &e.hides {
                o.push_str(&format!("{:w$}    hides the {} one in {f}\n", "", l.name()));
            }
        }
        for (l, f, e) in &cat.errors {
            o.push_str(&format!("NOT LOADED ({}): {f}: {e}\n", l.name()));
        }
    }
    Out {
        code: if cat.errors.is_empty() { 0 } else { 1 },
        out: o,
        err: String::new(),
    }
}

fn new<K: Kind<Record = Rec>>(env: &Env, argv: &[String]) -> Out {
    let (mut id, mut from, mut level) = (None, None, Level::User);
    let mut over: Vec<(String, Val)> = Vec::new();
    let mut it = argv.iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--from" => from = it.next().cloned(),
            "--project" => level = Level::Project,
            "--user" => level = Level::User,
            w if w.starts_with("--") => {
                return fail(2, format!("unknown option {w}\nusage: {}", usage(K::WORD)));
            }
            w if w.contains('=') => {
                let (k, v) = w.split_once('=').unwrap();
                over.push((k.trim().to_string(), value_of(v.trim())));
            }
            w => {
                if id.replace(w.to_string()).is_some() {
                    return fail(2, format!("one ID only\nusage: {}", usage(K::WORD)));
                }
            }
        }
    }
    let (Some(id), Some(from)) = (id, from) else {
        return fail(
            2,
            format!("give the new ID and --from ID\nusage: {}", usage(K::WORD)),
        );
    };
    if let Err(e) = forge_library::check_id(&id) {
        return fail(1, e);
    }
    let dir = match level {
        Level::Project => forge_library::project_dir::<K>(&env.project),
        _ => match user_dir::<K>(env) {
            Some(d) => d,
            None => return fail(2, "no user folder (no config directory): use --project".into()),
        },
    };
    let (lib, _) = crate::library(Some(&env.project), env.user_cam.as_deref());
    let typed = match build::derive(&lib, K::WORD, &from, &id, &over) {
        Ok(r) => r,
        Err(DeriveError::Unknown { kind, from, near }) => {
            return fail(2, format!("no {kind} `{from}`; near: {}", near.join(", ")));
        }
        Err(e) => return fail(1, format!("{e:?}")),
    };
    // the derived record's table: the source's with the overrides and the bookkeeping derive set,
    // read back from the record's text (derive validated it through the kind's reader)
    let table = match derived_table(&lib, K::WORD, &from, &id, &over) {
        Ok(t) => t,
        Err(e) => return fail(1, e),
    };
    let rec = Rec {
        id: id.clone(),
        table,
        typed,
    };
    match forge_library::save::<K>(&rec, &[], &dir, level, false) {
        Ok(f) => {
            let user = cites(&rec).iter().filter(|(_, s)| s == "user").count();
            let unknown = cites(&rec).iter().filter(|(_, s)| s == "unknown").count();
            Out {
                code: 0,
                out: format!(
                    "wrote {} ({} level): {user} values yours (user), {unknown} copied (unknown)\n",
                    f.display(),
                    level.name()
                ),
                err: String::new(),
            }
        }
        Err(e) => fail(1, e),
    }
}

/// The table `build::derive` makes (it returns the typed record; the table is made the same way
/// here so it can be written): the record's text through a one-record library.
/// `derived_table`, for the UI's derive.
pub fn derived_table_pub(
    lib: &forge_tooling::Library,
    kind: &str,
    from: &str,
    id: &str,
    over: &[(String, Val)],
) -> Result<forge_tooling::toml::Table, String> {
    derived_table(lib, kind, from, id, over)
}

fn derived_table(
    lib: &forge_tooling::Library,
    kind: &str,
    from: &str,
    id: &str,
    over: &[(String, Val)],
) -> Result<forge_tooling::toml::Table, String> {
    use forge_tooling::toml::Table;
    let (_, _, src) = lib
        .sources
        .iter()
        .find(|(k, i, _)| k == kind && i == from)
        .ok_or_else(|| format!("no {kind} `{from}`"))?;
    let mut t: Table = src.clone();
    for (k, v) in over {
        t.insert(k.clone(), v.clone());
    }
    t.insert("id".into(), Val::Str(id.into()));
    t.insert("real".into(), Val::Bool(false));
    t.insert("source".into(), Val::Str(format!("derived from {kind} `{from}`")));
    // the cites as derive makes them: its typed record holds them
    let typed = build::derive(lib, kind, from, id, over).map_err(|e| format!("{e:?}"))?;
    let cs = record_cites(&typed);
    let tables: Vec<Table> = cs
        .iter()
        .map(|c| {
            let mut ct = Table::new();
            ct.insert("path".into(), Val::Str(c.path.clone()));
            ct.insert("status".into(), Val::Str(status_name(c.status)));
            for (k, v) in [
                ("source", &c.source),
                ("taken", &c.taken),
                ("value", &c.value),
                ("basis", &c.basis),
                ("note", &c.note),
            ] {
                if !v.is_empty() {
                    ct.insert(k.into(), Val::Str(v.clone()));
                }
            }
            ct
        })
        .collect();
    if tables.is_empty() {
        t.remove("cite");
    } else {
        t.insert("cite".into(), Val::Tables(tables));
    }
    Ok(t)
}

fn record_cites(r: &build::Record) -> Vec<forge_tooling::Cite> {
    use build::Record as R;
    match r {
        R::Tool(x) => x.cites.clone(),
        R::Holder(x) => x.cites.clone(),
        R::Collet(x) => x.cites.clone(),
        R::ColletNut(x) => x.cites.clone(),
        R::PullStud(x) => x.cites.clone(),
        R::Insert(x) => x.cites.clone(),
        R::TurnHolder(x) => x.cites.clone(),
    }
}

fn check<K: Kind<Record = Rec>>(env: &Env, argv: &[String]) -> Out {
    let json = argv.iter().any(|a| a == "--json");
    let Some(target) = argv.iter().find(|a| !a.starts_with("--")) else {
        return fail(2, format!("give an ID or a FILE.toml\nusage: {}", usage(K::WORD)));
    };
    let (lib, _) = crate::library(Some(&env.project), env.user_cam.as_deref());
    let recs: Vec<(Rec, String)> = if target.ends_with(".toml") || forge_dirs::fs::is_file(Path::new(target))
    {
        match forge_dirs::fs::read_to_string(target)
            .map_err(|e| e.to_string())
            .and_then(|t| K::parse(&t))
        {
            Ok(rs) => rs.into_iter().map(|r| (r, target.clone())).collect(),
            Err(e) => {
                return Out {
                    code: 1,
                    out: if json {
                        format!(
                            "{{\"format\":\"forgecad.library-check/1\",\"ok\":false,\"refused\":{}}}\n",
                            jstr(&e)
                        )
                    } else {
                        format!("{target}: REFUSED: {e}\n")
                    },
                    err: String::new(),
                };
            }
        }
    } else {
        let cat = Catalog::<K>::load(Some(&env.project), user_dir::<K>(env).as_deref());
        match cat.get(target) {
            Some(e) => vec![(e.def.clone(), format!("{}, {}", e.level.name(), e.file))],
            None => return fail(2, format!("no {} `{target}`", K::WORD)),
        }
    };
    let mut ok = true;
    let mut out = String::new();
    for (r, from) in &recs {
        let found = build::check(&lib, &r.typed);
        ok &= found.is_empty();
        let cs = cites(r);
        if json {
            let f: Vec<String> = found
                .iter()
                .map(|x| {
                    format!(
                        "{{\"path\":{},\"measured\":{},\"limit\":{},\"fix\":{}}}",
                        jstr(&x.path),
                        x.measured,
                        x.limit,
                        jstr(&x.fix)
                    )
                })
                .collect();
            let c: Vec<String> = cs
                .iter()
                .map(|(p, s)| format!("{{\"path\":{},\"status\":{}}}", jstr(p), jstr(s)))
                .collect();
            out.push_str(&format!(
                "{{\"format\":\"forgecad.library-check/1\",\"kind\":{},\"id\":{},\"from\":{},\"ok\":{},\"findings\":[{}],\"cites\":[{}]}}\n",
                jstr(K::WORD),
                jstr(&r.id),
                jstr(from),
                found.is_empty(),
                f.join(","),
                c.join(",")
            ));
        } else {
            out.push_str(&format!(
                "{} `{}` ({from}): {}\n",
                K::WORD,
                r.id,
                if found.is_empty() { "ok" } else { "PROBLEMS" }
            ));
            out.push_str("  structure: ok (read by forge-tooling)\n");
            for x in &found {
                out.push_str(&format!(
                    "  {}: {} (limit {}): {}\n",
                    x.path, x.measured, x.limit, x.fix
                ));
            }
            let mut counts: Vec<(String, usize)> = Vec::new();
            for (_, s) in &cs {
                match counts.iter_mut().find(|(k, _)| k == s) {
                    Some(c) => c.1 += 1,
                    None => counts.push((s.clone(), 1)),
                }
            }
            out.push_str(&format!(
                "  provenance: {}\n",
                if counts.is_empty() {
                    "no cites".to_string()
                } else {
                    counts
                        .iter()
                        .map(|(k, n)| format!("{n} {k}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ));
            let unk: Vec<&str> = cs
                .iter()
                .filter(|(_, s)| s == "unknown")
                .map(|(p, _)| p.as_str())
                .collect();
            if !unk.is_empty() {
                out.push_str(&format!(
                    "    UNKNOWN (not checked against): {}\n",
                    unk.join(", ")
                ));
            }
        }
    }
    Out {
        code: if ok { 0 } else { 1 },
        out,
        err: String::new(),
    }
}

/// `forgecad WORD list|new|check ..` for a tooling kind; None if WORD is not one.
pub fn run(env: &Env, word: &str, argv: &[String]) -> Option<Out> {
    fn go<K: Kind<Record = Rec>>(env: &Env, argv: &[String]) -> Out {
        let rest = argv.get(1..).unwrap_or(&[]);
        match argv.first().map(String::as_str) {
            Some("list") => list::<K>(env, rest.iter().any(|a| a == "--json")),
            Some("new") => new::<K>(env, rest),
            Some("check") => check::<K>(env, rest),
            _ => fail(2, format!("usage: {}", usage(K::WORD))),
        }
    }
    Some(match word {
        "tool" => go::<Tools>(env, argv),
        "holder" => go::<Holders>(env, argv),
        "collet" => go::<Collets>(env, argv),
        "collet_nut" | "collet-nut" => go::<ColletNuts>(env, argv),
        "pull_stud" | "pull-stud" => go::<PullStuds>(env, argv),
        "insert" => go::<Inserts>(env, argv),
        "turn_holder" | "turn-holder" => go::<TurnHolders>(env, argv),
        _ => return None,
    })
}
