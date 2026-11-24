//! `forgecad tool log ..`: the physical tools on the shelf (forge_tooling::instances), what they
//! cut and how they wore. Exit 0 ok, 1 a refusal, 2 usage.
//!
//! ```text
//! forgecad tool log add LABEL --tool ID [--serial S] [--note TEXT]
//! forgecad tool log list [--json]
//! forgecad tool log observe|worn|chipped|broken|regrind LABEL [--vb MM] [--note TEXT]
//! forgecad tool log predict JOB.fg [--use T1=LABEL ..]
//! forgecad tool log run JOB.fg --use T1=LABEL [--use T2=LABEL ..] [--posted] [--program NAME]
//! forgecad tool log calibrate
//!   any: [--project] (a new instance at the project level; default the user's), [--date YYYY-MM-DD]
//! Instances are kept by a `Store` (forge_tooling::instances): on disk, one file per instance in
//! `cam/tool-instances/` (forge_cam_lang::toolstore::LevelStore).
//! ```
//! `run` adds each named tool's predicted use of the job (its cut, binned by condition) to the
//! instance; `--posted` records it as posted rather than run. Life use is a prediction from
//! cited data (forge_tooling::life), never a certification; a tool with no cited row shows
//! minutes only.
use forge_tooling::instances::{self, Event, Instance, Store};
use forge_tooling::life::{self, Limit, Row};
use forge_tooling::{Library, toml};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Out {
    pub code: u8,
    pub out: String,
    pub err: String,
}

fn fail(code: u8, err: impl Into<String>) -> Out {
    Out {
        code,
        out: String::new(),
        err: err.into(),
    }
}

pub const USAGE: &str = "usage: forgecad tool log add LABEL --tool ID [--serial S] [--note TEXT]
       forgecad tool log list [--json]
       forgecad tool log observe|worn|chipped|broken|regrind LABEL [--vb MM] [--note TEXT]
       forgecad tool log predict JOB.fg [--use T1=LABEL ..]
       forgecad tool log run JOB.fg --use T1=LABEL [--use T2=LABEL ..] [--posted] [--program NAME]
       forgecad tool log mount T1=LABEL [T2=LABEL ..] | unmount T1 [T2 ..]   (.forgecad/cam-mounts.toml; --use overrides)
       forgecad tool log calibrate
       (any: --project to keep a new instance with the project, --date YYYY-MM-DD)";

/// The project folder (jobs are read from it) and today's date.
pub struct Env {
    pub project: PathBuf,
    /// Today's date (YYYY-MM-DD), for entries given no --date.
    pub today: String,
}

struct Args {
    words: Vec<String>,
    opts: BTreeMap<String, Vec<String>>,
    flags: Vec<String>,
}

fn parse(argv: &[String]) -> Result<Args, String> {
    const VALUED: &[&str] = &[
        "--tool",
        "--serial",
        "--note",
        "--vb",
        "--use",
        "--program",
        "--date",
    ];
    const FLAGS: &[&str] = &["--json", "--user", "--project", "--posted"];
    let mut a = Args {
        words: vec![],
        opts: BTreeMap::new(),
        flags: vec![],
    };
    let mut i = 0;
    while i < argv.len() {
        let w = &argv[i];
        if VALUED.contains(&w.as_str()) {
            let v = argv.get(i + 1).ok_or(format!("{w} needs a value"))?;
            a.opts.entry(w.clone()).or_default().push(v.clone());
            i += 2;
        } else if FLAGS.contains(&w.as_str()) {
            a.flags.push(w.clone());
            i += 1;
        } else if w.starts_with("--") {
            return Err(format!("unknown option {w}"));
        } else {
            a.words.push(w.clone());
            i += 1;
        }
    }
    Ok(a)
}

impl Args {
    fn opt(&self, k: &str) -> Option<&str> {
        self.opts.get(k).and_then(|v| v.last()).map(|s| s.as_str())
    }
    fn flag(&self, k: &str) -> bool {
        self.flags.iter().any(|f| f == k)
    }
}

/// The coating text of a library tool record.
fn coating(lib: &Library, id: &str) -> String {
    lib.sources
        .iter()
        .find(|(k, i, _)| k == "tool" && i == id)
        .and_then(|(_, _, t)| toml::text(t, "coating"))
        .unwrap_or("")
        .to_string()
}

/// The work names to look a material up by: the name, its forge-material name and aliases.
fn work_names(m: &str) -> Vec<String> {
    vec![crate::life::work_name(m)]
}

/// The row for an instance's current life: its tool's substrate and coating, the material of
/// its runs (all the same, else None).
fn row_of<'a>(lib: &Library, rows: &'a [Row], i: &Instance) -> Option<&'a Row> {
    let t = lib.tool(&i.tool).ok()?;
    let runs = i.current_runs();
    let work = runs.first().map(|r| r.work.clone())?;
    if runs.iter().any(|r| r.work != work) {
        return None;
    }
    let names = work_names(&work);
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    life::find(rows, "milling", &t.material, &coating(lib, &i.tool), &refs)
}

fn jstr(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jnum(x: Option<f64>) -> String {
    x.filter(|v| v.is_finite())
        .map(|v| format!("{v}"))
        .unwrap_or_else(|| "null".into())
}

fn list(lib: &Library, rows: &[Row], all: &[Instance], json: bool) -> String {
    let mut o = String::new();
    if json {
        o.push('[');
    }
    for (k, i) in all.iter().enumerate() {
        let row = row_of(lib, rows, i);
        let used = row.and_then(|r| i.used(r));
        let last_vb = i.log.iter().rev().find_map(|l| l.vb);
        if json {
            if k > 0 {
                o.push(',');
            }
            o += &format!(
                "{{\"label\":{},\"tool\":{},\"serial\":{},\"state\":{},\"minutes\":{},\"programs\":{},\"regrinds\":{},\"life_used\":{},\"row\":{},\"last_vb\":{}}}",
                jstr(&i.label),
                jstr(&i.tool),
                jstr(&i.serial),
                jstr(&i.state),
                jnum(Some(i.minutes())),
                i.runs.len(),
                i.regrinds(),
                jnum(used),
                row.map(|r| jstr(&r.id)).unwrap_or("null".into()),
                jnum(last_vb)
            );
        } else {
            o += &format!(
                "{}  {}  {:.1} min in cut, {} program{}, {} regrind{}, life used {}{}\n",
                i.label,
                i.tool,
                i.minutes(),
                i.runs.len(),
                if i.runs.len() == 1 { "" } else { "s" },
                i.regrinds(),
                if i.regrinds() == 1 { "" } else { "s" },
                used.map(|u| format!("{:.0}%", 100.0 * u))
                    .unwrap_or("not estimated".into()),
                last_vb.map(|v| format!(", last VB {v} mm")).unwrap_or_default()
            );
        }
    }
    if json {
        o.push_str("]\n");
    }
    o
}

/// `--use T1=LABEL` pairs.
fn mounts_path(env: &Env) -> PathBuf {
    env.project.join(instances::MOUNTS_FILE)
}

fn load_mounts(env: &Env) -> Result<instances::Mounts, String> {
    match forge_dirs::fs::read_to_string(mounts_path(env)) {
        Ok(s) => instances::Mounts::read(&s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(instances::Mounts::default()),
        Err(e) => Err(format!("{}: {e}", mounts_path(env).display())),
    }
}

fn save_mounts(env: &Env, m: &instances::Mounts) -> Result<(), String> {
    let p = mounts_path(env);
    if let Some(d) = p.parent() {
        forge_dirs::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    forge_dirs::fs::write(&p, m.write()).map_err(|e| format!("{}: {e}", p.display()))
}

/// `--use T1=LABEL` pairs over the project's mounts (`.forgecad/cam-mounts.toml`).
fn uses(env: &Env, a: &Args) -> Result<BTreeMap<u32, String>, String> {
    let mut m = load_mounts(env)?.by_t;
    for u in a.opts.get("--use").into_iter().flatten() {
        let (t, l) = u
            .split_once('=')
            .ok_or(format!("--use {u}: give T<number>=LABEL"))?;
        let n: u32 = t
            .trim_start_matches(['T', 't'])
            .parse()
            .map_err(|_| format!("--use {u}: `{t}` is not a tool number"))?;
        m.insert(n, l.to_string());
    }
    Ok(m)
}

/// The job's tool-life report (and the instances it would add to).
fn job(env: &Env, a: &Args, all: &mut [Instance], write_runs: bool) -> Result<String, String> {
    let fg = a.words.get(1).ok_or("give the job file (JOB.fg)")?;
    let path = env.project.join(fg);
    let src = forge_dirs::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let cam = crate::run(&src, dir);
    if !cam.errors.is_empty() {
        return Err(format!("the job does not build: {}", cam.errors.join("; ")));
    }
    let rows = life::builtin();
    let u = uses(env, a)?;
    let mut limits = BTreeMap::new();
    for (t, l) in &u {
        let i = all.iter().find(|i| &i.label == l).ok_or(format!(
            "no tool instance `{l}` (add it: forgecad tool log add {l} --tool ID)"
        ))?;
        let ti = cam.tool_number_id(*t).ok_or(format!("the job loads no T{t}"))?;
        if i.tool != ti {
            return Err(format!("`{l}` is a {} but T{t} is a {ti}", i.tool));
        }
        let start = row_of(cam.library(), &rows, i)
            .and_then(|r| i.used(r))
            .unwrap_or(0.0);
        limits.insert(*t, Limit { at: 1.0, start });
    }
    let rep = cam.tool_life(&rows, &[], &limits);
    let work = cam
        .material()
        .map(|m| crate::life::work_name(&m))
        .unwrap_or_default();
    let mut o = String::new();
    for r in &rep {
        o += &format!(
            "T{} {}: {:.1} min in cut, life {}{}\n  {}\n",
            r.t,
            r.tool,
            r.life.minutes_in_cut,
            r.life
                .used
                .map(|x| format!("{:.0}% used", 100.0 * x))
                .unwrap_or("not estimated".into()),
            if r.unsampled_minutes > 0.0 {
                format!(
                    " ({:.2} min of plunges/ramps not in the estimate)",
                    r.unsampled_minutes
                )
            } else {
                String::new()
            },
            r.life.note
        );
        if let Some(w) = &r.warning {
            o += &format!("  warning: {}\n", w.why);
        }
        // entry load: per check, the count and the worst
        let mut by: BTreeMap<&str, (usize, &forge_tooling::entry::Finding)> = BTreeMap::new();
        for f in &r.entry {
            let e = by.entry(f.check.name()).or_insert((0, f));
            e.0 += 1;
            let worse = if f.check.max() {
                f.value > e.1.value
            } else {
                f.value < e.1.value
            };
            if worse {
                e.1 = f;
            }
        }
        for (k, (n, f)) in by {
            o += &format!(
                "  entry load (makers' guidance, not tool life): {n} × {k}, worst in op {} move {}: {}\n",
                f.op + 1,
                f.mv + 1,
                f.why
            );
        }
        for n in &r.entry_not_checked {
            o += &format!("  entry not checked: {n}\n");
        }
        if write_runs && let Some(l) = u.get(&r.t) {
            let cuts = cam.tool_cuts(r.t);
            let i = all.iter_mut().find(|i| &i.label == l).expect("checked above");
            i.record(
                a.opt("--program").unwrap_or(fg),
                a.opt("--date").unwrap_or(&env.today),
                &work,
                if a.flag("--posted") { "posted" } else { "run" },
                &cuts,
            );
            o += &format!("  logged on {l}\n");
        }
    }
    Ok(o)
}

pub fn run(env: &Env, store: &mut dyn Store, argv: &[String]) -> Out {
    let a = match parse(argv) {
        Ok(a) => a,
        Err(e) => return fail(2, format!("{e}\n{USAGE}")),
    };
    let Some(cmd) = a.words.first().cloned() else {
        return fail(2, USAGE);
    };
    let mut all = match store.load() {
        Ok(v) => v,
        Err(e) => return fail(1, e),
    };
    let orig = all.clone();
    let lib = Library::builtin();
    let rows = life::builtin();
    let date = a.opt("--date").unwrap_or(&env.today).to_string();
    let label = a.words.get(1).cloned();
    let res: Result<(String, bool), String> = match cmd.as_str() {
        "add" => (|| {
            let l = label.ok_or("give the instance's LABEL")?;
            let t = a
                .opt("--tool")
                .ok_or("give --tool ID (the library tool record)")?;
            lib.tool(t).map_err(|e| e.to_string())?;
            if all.iter().any(|i| i.label == l) {
                return Err(format!("`{l}` is already logged"));
            }
            let mut i = Instance::new(&l, t);
            i.serial = a.opt("--serial").unwrap_or("").into();
            i.note = a.opt("--note").unwrap_or("").into();
            all.push(i);
            Ok((format!("added {l} ({t})\n"), true))
        })(),
        "list" => Ok((list(&lib, &rows, &all, a.flag("--json")), false)),
        ev @ ("observe" | "worn" | "chipped" | "broken" | "regrind") => (|| {
            let l = label.ok_or("give the instance's LABEL")?;
            let vb = match a.opt("--vb") {
                Some(v) => Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|x| *x >= 0.0 && x.is_finite())
                        .ok_or(format!("--vb {v}: give the flank wear in mm"))?,
                ),
                None => None,
            };
            let i = all
                .iter_mut()
                .find(|i| i.label == l)
                .ok_or(format!("no tool instance `{l}`"))?;
            i.event(
                Event::parse(ev).expect("listed"),
                &date,
                vb,
                a.opt("--note").unwrap_or(""),
            );
            Ok((format!("{l}: {ev} logged\n"), true))
        })(),
        "predict" => job(env, &a, &mut all, false).map(|o| (o, false)),
        "mount" | "unmount" => (|| {
            let mut m = load_mounts(env)?;
            for w in &a.words[1..] {
                if cmd == "mount" {
                    let (t, l) = w
                        .split_once('=')
                        .ok_or(format!("mount {w}: give T<number>=LABEL"))?;
                    let n: u32 = t
                        .trim_start_matches(['T', 't'])
                        .parse()
                        .map_err(|_| format!("mount {w}: `{t}` is not a tool number"))?;
                    if !all.iter().any(|i| i.label == l) {
                        return Err(format!(
                            "no tool instance `{l}` (add it: forgecad tool log add {l} --tool ID)"
                        ));
                    }
                    m.by_t.insert(n, l.to_string());
                } else {
                    let n: u32 = w
                        .trim_start_matches(['T', 't'])
                        .parse()
                        .map_err(|_| format!("unmount {w}: not a tool number"))?;
                    m.by_t.remove(&n);
                }
            }
            save_mounts(env, &m)?;
            let list: Vec<String> = m.by_t.iter().map(|(t, l)| format!("T{t}={l}")).collect();
            Ok((
                format!(
                    "mounted: {}\n",
                    if list.is_empty() {
                        "none".into()
                    } else {
                        list.join(" ")
                    }
                ),
                false,
            ))
        })(),
        "run" => {
            if !a.opts.contains_key("--use") && load_mounts(env).map(|m| m.by_t.is_empty()).unwrap_or(true) {
                Err("give --use T<n>=LABEL for each tool to log, or mount them (forgecad tool log mount T1=LABEL)".into())
            } else {
                job(env, &a, &mut all, true).map(|o| (o, true))
            }
        }
        "calibrate" => {
            let mut o = String::new();
            for r in &rows {
                let tools: Vec<&str> = lib
                    .tools
                    .iter()
                    .filter(|t| {
                        t.material.eq_ignore_ascii_case(&r.tool)
                            && (r.coating.is_empty() || coating(&lib, &t.id).eq_ignore_ascii_case(&r.coating))
                    })
                    .map(|t| t.id.as_str())
                    .collect();
                if let Some(c) = instances::calibrate(&all, &tools, r) {
                    o += &format!("{}: {} (geometric sd ×{:.2})\n", r.id, c.note, c.gsd);
                }
            }
            if o.is_empty() {
                o = "no calibration yet: log tools taken out worn (forgecad tool log worn LABEL --vb MM) after runs of a material with cited data\n".into();
            }
            Ok((o, false))
        }
        _ => Err(format!("unknown `{cmd}`\n{USAGE}")),
    };
    match res {
        Ok((out, dirty)) => {
            if dirty {
                for i in all.iter().filter(|i| !orig.contains(i)) {
                    if let Err(e) = store.save(i) {
                        return fail(1, e);
                    }
                }
            }
            Out {
                code: 0,
                out,
                err: String::new(),
            }
        }
        Err(e) => fail(if e.contains("usage") { 2 } else { 1 }, e),
    }
}
