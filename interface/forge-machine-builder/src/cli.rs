//! `forgecad machine list | new | check`: the command words, with what they need from outside
//! (the folders, today's date, the envelope check that needs forge-lang) passed in, so tests
//! drive them without a process.
use crate::catalog::{Catalog, Level, user_dir};
use crate::check::{self, CheckReport, EnvelopeCheck};
use crate::make::{self, Copied, Template};
use forge_machine::def::MachineDef;
use std::path::{Path, PathBuf};

/// A body's part checked against its envelope: (body, verdict) per body with a part. `dir` is
/// the folder its `part` paths are relative to.
pub type Envelopes<'a> = &'a dyn Fn(&MachineDef, &Path) -> Vec<(String, EnvelopeCheck)>;

pub struct Env<'a> {
    /// The project folder (`cam/machines` under it is the project level).
    pub project: PathBuf,
    /// The user's machines folder (None: no user level).
    pub user: Option<PathBuf>,
    /// YYYY-MM-DD.
    pub today: String,
    pub envelopes: Envelopes<'a>,
}

impl Env<'_> {
    pub fn catalog(&self) -> Catalog {
        Catalog::load(Some(&self.project), self.user.as_deref())
    }
}

#[derive(Debug, Default)]
pub struct Out {
    /// 0 ok, 1 the check found problems (or a refusal of the request), 2 usage / error.
    pub code: u8,
    pub out: String,
    pub err: String,
}

pub const USAGE: &str = "forgecad machine list [--json]\n       \
forgecad machine new NAME (--from MACHINE | --template vmc3|trunnion|head-table|lathe|mill-turn|gantry) [--project | --user] [--own | --keep-cites] [--force]\n       \
forgecad machine check FILE.toml|NAME [--json] [--no-parts]\n       \
forgecad machine model attach MACHINE FILE --from TEXT [--body BODY] [--scale S] [--rotate X,Y,Z] [--offset X,Y,Z]\n       \
forgecad machine model list|check MACHINE [--json]\n       \
forgecad machine model remove MACHINE FILE\n       \
forgecad machine model split MACHINE FILE BODY|-..   (one per part, largest first; - drops it)\n       \
forgecad machine model split MACHINE FILE PART=BODY..   (by name: the first PART in a part's name gives it BODY; - or no rule drops it)";

fn fail(code: u8, err: String) -> Out {
    Out {
        code,
        out: String::new(),
        err,
    }
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

/// `machine list`: every machine the search order finds, its level and file, and what it hides.
pub fn list(env: &Env, json: bool) -> Out {
    let cat = env.catalog();
    let mut o = String::new();
    if json {
        let rows: Vec<String> = cat
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"name\":{},\"kind\":{},\"control\":{},\"level\":{},\"file\":{},\"real\":{},\"hides\":[{}]}}",
                    jstr(&e.def.name),
                    jstr(e.def.kind.name()),
                    jstr(&e.def.control),
                    jstr(e.level.name()),
                    jstr(&e.file),
                    e.def.real,
                    e.hides
                        .iter()
                        .map(|(l, f)| format!("{{\"level\":{},\"file\":{}}}", jstr(l.name()), jstr(f)))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        let errs: Vec<String> = cat
            .errors
            .iter()
            .map(|(l, f, e)| {
                format!(
                    "{{\"level\":{},\"file\":{},\"error\":{}}}",
                    jstr(l.name()),
                    jstr(f),
                    jstr(e)
                )
            })
            .collect();
        o = format!(
            "{{\"format\":\"forgecad.machines/1\",\"machines\":[{}],\"errors\":[{}]}}\n",
            rows.join(","),
            errs.join(",")
        );
    } else {
        let w = cat.entries.iter().map(|e| e.def.name.len()).max().unwrap_or(0);
        for e in &cat.entries {
            o.push_str(&format!(
                "{:w$}  {:11} {:12} {:8} {}{}\n",
                e.def.name,
                e.def.kind.name(),
                e.def.control,
                e.level.name(),
                e.file,
                if e.def.real { "  (real)" } else { "" },
            ));
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

/// `machine new`.
pub fn new(env: &Env, argv: &[String]) -> Out {
    let (mut name, mut from, mut template) = (None, None, None);
    let (mut level, mut copied, mut force) = (Level::User, Copied::Mark, false);
    let mut it = argv.iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--from" => from = it.next().cloned(),
            "--template" => template = it.next().cloned(),
            "--project" => level = Level::Project,
            "--user" => level = Level::User,
            "--own" => copied = Copied::Own,
            "--keep-cites" => copied = Copied::KeepCites,
            "--force" => force = true,
            w if w.starts_with("--") => return fail(2, format!("unknown option {w}\nusage: {USAGE}")),
            w => {
                if name.replace(w.to_string()).is_some() {
                    return fail(2, format!("one NAME only\nusage: {USAGE}"));
                }
            }
        }
    }
    let Some(name) = name else {
        return fail(2, format!("give the new machine's NAME\nusage: {USAGE}"));
    };
    let dir = match level {
        Level::Project => crate::catalog::project_dir(&env.project),
        _ => match &env.user {
            Some(d) => d.clone(),
            None => return fail(2, "no user folder (no config directory): use --project".into()),
        },
    };
    let cat = env.catalog();
    let made = match (from, template) {
        (Some(_), Some(_)) | (None, None) => {
            return fail(
                2,
                format!("give one of --from MACHINE or --template KIND\nusage: {USAGE}"),
            );
        }
        (None, Some(t)) => {
            let Some(t) = Template::from_word(&t) else {
                let all: Vec<&str> = Template::ALL.iter().map(|t| t.word()).collect();
                return fail(2, format!("no template `{t}`; templates: {}", all.join(", ")));
            };
            make::from_template(t, &name, &env.today)
        }
        (Some(f), None) => {
            let Some(e) = cat.get(&f) else {
                return fail(2, format!("no machine `{f}`; known: {}", cat.names().join(", ")));
            };
            let src_dir = Path::new(&e.file)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            let builtin = e.level == Level::Builtin;
            make::clone_machine(
                &e.def,
                &format!("{} ({}, {})", e.def.name, e.level.name(), e.file),
                &name,
                copied,
                &env.today,
                &|p| {
                    if builtin {
                        make::builtin_part(p).map(str::to_string)
                    } else {
                        forge_dirs::fs::read_to_string(src_dir.join(p)).ok()
                    }
                },
            )
        }
    };
    let made = match made {
        Ok(m) => m,
        Err(e) => return fail(1, e),
    };
    // what the new name will hide (or what hides it)
    let mut warn = String::new();
    if let Some(e) = cat.get(&name)
        && e.level != level
    {
        warn = if e.level < level {
            format!(
                "note: the {} machine `{name}` in {} hides this one\n",
                e.level.name(),
                e.file
            )
        } else {
            format!(
                "note: this hides the {} machine `{name}` in {}\n",
                e.level.name(),
                e.file
            )
        };
    }
    match make::save(&made, &dir, level, force) {
        Ok(f) => {
            let unk = made
                .def
                .cites
                .iter()
                .filter(|c| c.status == forge_machine::def::CiteStatus::Unknown)
                .count();
            Out {
                code: 0,
                out: format!(
                    "{warn}wrote {} ({} level){}{}\n",
                    f.display(),
                    level.name(),
                    if made.parts.is_empty() {
                        String::new()
                    } else {
                        format!(", {} part files", made.parts.len())
                    },
                    if unk > 0 {
                        format!(
                            "; {unk} values are copied placeholders cited unknown: enter your machine's values"
                        )
                    } else {
                        String::new()
                    }
                ),
                err: String::new(),
            }
        }
        Err(e) => fail(1, e),
    }
}

/// `machine check`.
pub fn check(env: &Env, argv: &[String]) -> Out {
    let json = argv.iter().any(|a| a == "--json");
    let parts = !argv.iter().any(|a| a == "--no-parts");
    let Some(target) = argv.iter().find(|a| !a.starts_with("--")) else {
        return fail(2, format!("give a FILE.toml or a machine NAME\nusage: {USAGE}"));
    };
    let path = Path::new(target);
    let mut reports: Vec<(CheckReport, Vec<String>)> = Vec::new();
    if target.ends_with(".toml") || forge_dirs::fs::is_file(path) {
        let src = match forge_dirs::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return fail(2, format!("{target}: {e}")),
        };
        // the level a file is at, if it is in one of the searched folders
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let level = if canon(&dir) == canon(&crate::catalog::project_dir(&env.project)) {
            Some(Level::Project)
        } else if env.user.as_deref().is_some_and(|u| canon(u) == canon(&dir)) {
            Some(Level::User)
        } else {
            None
        };
        let defs = MachineDef::parse_all(&src);
        for mut r in check::check_text(&src, level, target) {
            let joints = match &defs {
                Ok(ds) => {
                    let d = ds.iter().find(|d| d.name == r.machine);
                    if let (true, Some(d)) = (parts, d) {
                        add_envelopes(&mut r, (env.envelopes)(d, &dir));
                    }
                    d.map(|d| d.joints.iter().map(|j| j.name.clone()).collect())
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            };
            reports.push((r, joints));
        }
    } else {
        let cat = env.catalog();
        let Some(e) = cat.get(target) else {
            return fail(
                2,
                format!("no machine `{target}`; known: {}", cat.names().join(", ")),
            );
        };
        let mut r = check::check(&e.def, Some(e.level), &e.file);
        if parts {
            if e.level == Level::Builtin {
                // the built-in parts, written where the envelope check can build them
                let tmp = forge_dirs::temp_dir()
                    .join(format!("forgecad-machine-check-{}", forge_dirs::process_id()));
                let mut ok = true;
                for b in &e.def.bodies {
                    if let Some(p) = &b.part
                        && let Some(t) = make::builtin_part(p)
                    {
                        let f = tmp.join(p);
                        ok &= f
                            .parent()
                            .is_some_and(|d| forge_dirs::fs::create_dir_all(d).is_ok())
                            && forge_dirs::fs::write(&f, t).is_ok();
                    }
                }
                if ok {
                    add_envelopes(&mut r, (env.envelopes)(&e.def, &tmp));
                }
                let _ = forge_dirs::fs::remove_dir_all(&tmp);
            } else {
                let dir = Path::new(&e.file)
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default();
                add_envelopes(&mut r, (env.envelopes)(&e.def, &dir));
            }
        }
        let joints = e.def.joints.iter().map(|j| j.name.clone()).collect();
        reports.push((r, joints));
    }
    let ok = reports.iter().all(|(r, _)| r.ok());
    let out = if json {
        let js: Vec<String> = reports.iter().map(|(r, _)| r.json()).collect();
        if js.len() == 1 {
            format!("{}\n", js[0])
        } else {
            format!("[{}]\n", js.join(","))
        }
    } else {
        reports
            .iter()
            .map(|(r, j)| r.text(j))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Out {
        code: if ok { 0 } else { 1 },
        out,
        err: String::new(),
    }
}

fn add_envelopes(r: &mut CheckReport, found: Vec<(String, EnvelopeCheck)>) {
    for (b, e) in found {
        r.envelopes.retain(|(x, _)| *x != b);
        r.envelopes.push((b, e));
    }
}

/// `machine model ...`: the user's own model files for a machine (model_files), kept in the user's
/// library only.
pub fn model(env: &Env, argv: &[String]) -> Out {
    use crate::model_files as mf;
    use crate::models;
    let usage = || fail(2, format!("usage: {USAGE}"));
    let Some(user) = env.user.clone() else {
        return fail(
            2,
            "no user folder (no config directory): models are kept in the user's library".into(),
        );
    };
    let (Some(verb), Some(name)) = (argv.first(), argv.get(1)) else {
        return usage();
    };
    let cat = env.catalog();
    let Some(e) = cat.get(name) else {
        return fail(2, format!("no machine `{name}`"));
    };
    let def = e.def.clone();
    let dir = mf::dir_for(&user, &def.name);
    let mut ms = match models::load_in(&dir) {
        Ok(m) => m,
        Err(e) => return fail(2, e),
    };
    let json = argv.iter().any(|a| a == "--json");
    let save = |ms: &[models::Model]| models::save_in(&dir, ms);
    let three = |s: &str| -> Option<[f64; 3]> {
        let v: Vec<f64> = s
            .split(',')
            .map(|x| x.trim().parse().ok())
            .collect::<Option<_>>()?;
        (v.len() == 3).then(|| [v[0], v[1], v[2]])
    };
    match verb.as_str() {
        "attach" => {
            let Some(file) = argv.get(2).filter(|f| !f.starts_with("--")) else {
                return usage();
            };
            let (mut body, mut from) = (None, String::new());
            let mut how = mf::Placing::default();
            let mut it = argv[3..].iter();
            while let Some(a) = it.next() {
                let v = it.next();
                match (a.as_str(), v) {
                    ("--body", Some(v)) => body = Some(v.clone()),
                    ("--from", Some(v)) => from = v.clone(),
                    ("--scale", Some(v)) => match v.parse::<f64>() {
                        Ok(x) if x > 0.0 => how.scale = Some(x),
                        _ => return fail(2, format!("--scale {v}: a number above 0 (25.4 for inches)")),
                    },
                    ("--rotate", Some(v)) => match three(v) {
                        Some(r) => how.rotate = Some(r),
                        None => return fail(2, format!("--rotate {v}: three angles, X,Y,Z in degrees")),
                    },
                    ("--offset", Some(v)) => match three(v) {
                        Some(o) => how.offset = Some(o),
                        None => return fail(2, format!("--offset {v}: three lengths, X,Y,Z in mm")),
                    },
                    _ => return usage(),
                }
            }
            if body.is_some() && ms.iter().any(|m| m.body == body) {
                return fail(
                    1,
                    format!(
                        "`{}` has a model already: remove it first (forgecad machine model remove {} FILE)",
                        body.unwrap_or_default(),
                        def.name
                    ),
                );
            }
            match mf::attach(&def, &dir, &ms, body.as_deref(), Path::new(file), &from, &how) {
                Ok((m, said)) => {
                    ms.push(m);
                    if let Err(e) = save(&ms) {
                        return fail(2, e);
                    }
                    Out {
                        code: 0,
                        out: format!("{said}\nkept in {}\n", dir.display()),
                        err: String::new(),
                    }
                }
                Err(e) => fail(1, e),
            }
        }
        "list" => {
            let out = if json {
                let rows: Vec<String> = ms
                    .iter()
                    .map(|m| {
                        format!(
                            "{{\"file\":{},\"body\":{},\"from\":{},\"scale\":{},\"rotate\":[{},{},{}],\"offset\":[{},{},{}]}}",
                            jstr(&m.file),
                            m.body.as_ref().map_or("null".into(), |b| jstr(b)),
                            jstr(&m.from),
                            m.scale,
                            m.rotate[0],
                            m.rotate[1],
                            m.rotate[2],
                            m.offset[0],
                            m.offset[1],
                            m.offset[2]
                        )
                    })
                    .collect();
                format!("[{}]\n", rows.join(","))
            } else if ms.is_empty() {
                format!("{}: no models attached\n", def.name)
            } else {
                ms.iter()
                    .map(|m| {
                        format!(
                            "{}  {}  scale {}  from: {}\n",
                            m.body.as_deref().unwrap_or("(whole machine)"),
                            m.file,
                            m.scale,
                            m.from
                        )
                    })
                    .collect()
            };
            Out {
                code: 0,
                out,
                err: String::new(),
            }
        }
        "check" => {
            let mut lines = Vec::new();
            let mut bad = false;
            for m in &ms {
                match mf::check(&def, &dir, m) {
                    Ok(None) => lines.push((
                        m,
                        "whole machine: not checked against an envelope".to_string(),
                        None,
                    )),
                    Ok(Some((o, true))) => lines.push((
                        m,
                        if o.out == 0 {
                            format!("inside its envelope ({} points); {}", o.points, mf::DISPLAY_ONLY)
                        } else {
                            format!(
                                "outside its envelope by up to {:.2} mm ({} of {} points); {}",
                                o.worst,
                                o.out,
                                o.points,
                                mf::DISPLAY_ONLY
                            )
                        },
                        Some(o),
                    )),
                    Ok(Some((o, false))) if o.out == 0 => {
                        lines.push((m, format!("inside its envelope ({} points)", o.points), Some(o)))
                    }
                    Ok(Some((o, false))) => {
                        bad = true;
                        lines.push((
                            m,
                            format!(
                                "outside its envelope by up to {:.2} mm ({} of {} points), farthest at ({:.1}, {:.1}, {:.1}): collisions there are not found",
                                o.worst, o.out, o.points, o.at[0], o.at[1], o.at[2]
                            ),
                            Some(o),
                        ))
                    }
                    Err(e) => {
                        bad = true;
                        lines.push((m, e, None));
                    }
                }
            }
            let out = if json {
                let rows: Vec<String> = lines
                    .iter()
                    .map(|(m, w, o)| match o {
                        Some(o) => format!(
                            "{{\"file\":{},\"body\":{},\"points\":{},\"outside\":{},\"worst_mm\":{},\"note\":{}}}",
                            jstr(&m.file),
                            jstr(m.body.as_deref().unwrap_or("")),
                            o.points,
                            o.out,
                            o.worst,
                            jstr(w)
                        ),
                        None => format!("{{\"file\":{},\"note\":{}}}", jstr(&m.file), jstr(w)),
                    })
                    .collect();
                format!("[{}]\n", rows.join(","))
            } else {
                lines
                    .iter()
                    .map(|(m, w, _)| {
                        format!(
                            "{} ({}): {w}\n",
                            m.file,
                            m.body.as_deref().unwrap_or("whole machine")
                        )
                    })
                    .collect()
            };
            Out {
                code: u8::from(bad),
                out,
                err: String::new(),
            }
        }
        "remove" => {
            let Some(file) = argv.get(2) else { return usage() };
            let before = ms.len();
            ms.retain(|m| &m.file != file);
            if ms.len() == before {
                return fail(1, format!("{}: no model `{file}`", def.name));
            }
            match save(&ms) {
                Ok(()) => Out {
                    code: 0,
                    out: format!("{file} removed (and its copy in your library)\n"),
                    err: String::new(),
                },
                Err(e) => fail(2, e),
            }
        }
        "split" => {
            let Some(file) = argv.get(2) else { return usage() };
            let Some(i) = ms.iter().position(|m| &m.file == file) else {
                return fail(1, format!("{}: no model `{file}`", def.name));
            };
            let mut to: Vec<Option<String>> =
                argv[3..].iter().map(|b| (b != "-").then(|| b.clone())).collect();
            // by name: PART=BODY rules, the first whose PART is in a part's name (any case; `*`
            // any part) giving it BODY, or dropping it with `-`; a part no rule names is dropped
            let mut read = None;
            if argv[3..].iter().any(|a| a.contains('=')) {
                let mut rules = Vec::new();
                for a in &argv[3..] {
                    let Some((p, b)) = a.rsplit_once('=') else {
                        return fail(1, format!("`{a}`: give every part rule as PART=BODY"));
                    };
                    rules.push((p.to_lowercase(), (b != "-").then(|| b.to_string())));
                }
                let ps = match mf::read_file(&dir.join(file)) {
                    Ok(ps) => ps,
                    Err(e) => return fail(1, e),
                };
                to = ps
                    .iter()
                    .map(|p| {
                        let n = p.name.to_lowercase();
                        rules
                            .iter()
                            .find(|(k, _)| k == "*" || n.contains(k.as_str()))
                            .and_then(|(_, b)| b.clone())
                    })
                    .collect();
                read = Some(ps);
            }
            if let Some(b) = to
                .iter()
                .flatten()
                .find(|b| !def.bodies.iter().any(|x| &x.name == *b))
            {
                return fail(
                    1,
                    format!(
                        "{} has no body `{b}` (its bodies: {})",
                        def.name,
                        mf::body_names(&def)
                    ),
                );
            }
            if to.is_empty() {
                // what there is to split: its parts, largest first
                return match mf::read_file(&dir.join(file)) {
                    Ok(ps) => Out {
                        code: 0,
                        out: ps
                            .iter()
                            .enumerate()
                            .map(|(k, p)| {
                                let (lo, hi) = p.bounds().unwrap_or(([0.0; 3], [0.0; 3]));
                                format!(
                                    "part {}: {} triangles, {:.1} x {:.1} x {:.1} (file units){}\n",
                                    k + 1,
                                    p.tri.len(),
                                    hi[0] - lo[0],
                                    hi[1] - lo[1],
                                    hi[2] - lo[2],
                                    if p.name.is_empty() {
                                        String::new()
                                    } else {
                                        format!(": {}", p.name)
                                    }
                                )
                            })
                            .collect(),
                        err: String::new(),
                    },
                    Err(e) => fail(1, e),
                };
            }
            let done = match &read {
                Some(ps) => mf::split_read(&dir, &mut ms, i, ps, &to),
                None => mf::split(&dir, &mut ms, i, &to),
            };
            match done {
                Ok(made) => match save(&ms) {
                    Ok(()) => Out {
                        code: 0,
                        out: made
                            .iter()
                            .map(|m| format!("{} -> {}\n", m.file, m.body.as_deref().unwrap_or("")))
                            .collect(),
                        err: String::new(),
                    },
                    Err(e) => fail(2, e),
                },
                Err(e) => fail(1, e),
            }
        }
        _ => usage(),
    }
}

/// `machine list|new|check ...`; None if `argv[0]` is none of those (a setup file).
pub fn run(env: &Env, argv: &[String]) -> Option<Out> {
    let rest = &argv[1..];
    Some(match argv.first()?.as_str() {
        "list" => list(env, rest.iter().any(|a| a == "--json")),
        "new" => new(env, rest),
        "check" => check(env, rest),
        "model" => model(env, rest),
        _ => return None,
    })
}

/// Today (UTC) as YYYY-MM-DD from the system clock (not on a browser page, whose caller gives it).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    date_of_days((secs / 86_400) as i64)
}

/// The civil date of a day count from 1970-01-01 (Howard Hinnant's days_from_civil inverse).
pub fn date_of_days(z: i64) -> String {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

/// The user's folder, for callers.
pub fn default_user() -> Option<PathBuf> {
    user_dir()
}
