//! Another system's model file (STEP, IGES, OpenCascade .brep, Parasolid .x_t/.x_b) brought in
//! as a part, or turned into STEP: read by `forge_step::import`, which knows each by what it
//! holds, as a part file's `import()` reads it.
use forge_agent::json::Json;
use std::path::{Path, PathBuf};

/// The extensions a model file goes by: the same as `import()` finds beside a part.
pub const MODEL_EXTS: &[&str] = &[
    "step", "stp", "brep", "igs", "iges", "x_t", "x_b", "xmt_txt", "xmt_bin",
];

pub fn is_model_name(p: &Path) -> bool {
    p.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| MODEL_EXTS.contains(&x.to_ascii_lowercase().as_str()))
}

/// A file's solids, or why there are none: the reader's refusals as they are.
fn read(src: &Path) -> Result<forge_step::Imported, String> {
    let name = src.display();
    let data = std::fs::read(src).map_err(|e| format!("{name}: {e}"))?;
    let imp =
        forge_step::import(&data, &forge_step::Limits::default()).map_err(|e| format!("{name}: {e}"))?;
    if imp.solids.is_empty() {
        let why: Vec<String> = imp
            .refusals
            .iter()
            .take(3)
            .map(|r| format!("{} #{}: {}", r.kind, r.entity, r.why))
            .collect();
        return Err(format!("{name}: no solid it reads ({})", why.join("; ")));
    }
    Ok(imp)
}

fn report(imp: &forge_step::Imported) -> Vec<(&'static str, Json)> {
    let refusals: Vec<Json> = imp
        .refusals
        .iter()
        .map(|r| {
            Json::obj(vec![
                ("entity", (r.entity as f64).into()),
                ("kind", r.kind.as_str().into()),
                ("why", r.why.as_str().into()),
            ])
        })
        .collect();
    vec![
        ("solids", (imp.solids.len() as f64).into()),
        ("refused", Json::Arr(refusals)),
        ("system", imp.system.as_str().into()),
        ("schema", imp.schema.as_str().into()),
    ]
}

/// `src` as a part in `dir`: the file copied there (if not there already) and `<stem>.fg`
/// beside it holding `body = import("<file>")`. Nothing is overwritten.
pub fn import(src: &Path, dir: &Path) -> Result<(Json, String), String> {
    let imp = read(src)?;
    let file = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("a file name is needed")?;
    if file.contains('"') || file.contains('\\') {
        return Err(format!("{file}: a name a part file can quote is needed"));
    }
    let stem: String = file
        .split('.')
        .next()
        .unwrap_or("part")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = if stem.is_empty() { "part".to_string() } else { stem };
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let beside: PathBuf = dir.join(file);
    let same = match (std::fs::canonicalize(src), std::fs::canonicalize(&beside)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !same {
        if beside.exists() {
            return Err(format!("{}: there already; not overwritten", beside.display()));
        }
        std::fs::copy(src, &beside).map_err(|e| format!("{}: {e}", beside.display()))?;
    }
    let part = dir.join(format!("{stem}.fg"));
    if part.exists() {
        return Err(format!("{}: there already; not overwritten", part.display()));
    }
    let text = format!(
        "# {file}: {} solid(s){}\nbody = import(\"{file}\")\n",
        imp.solids.len(),
        if imp.system.is_empty() {
            String::new()
        } else {
            format!(" from {}", imp.system)
        }
    );
    std::fs::write(&part, text).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut f = vec![
        ("ok", true.into()),
        ("part", part.display().to_string().into()),
        ("model", beside.display().to_string().into()),
    ];
    f.extend(report(&imp));
    let line = format!(
        "{}: {} solid(s), {} refused -> {}",
        src.display(),
        imp.solids.len(),
        imp.refusals.len(),
        part.display()
    );
    Ok((Json::obj(f), line))
}

/// `src` written as a STEP file at `out`, its solids together (AP214, or AP242 with `ap242`).
pub fn convert(src: &Path, out: &Path, ap242: bool) -> Result<(Json, String), String> {
    let imp = read(src)?;
    let mut body = forge_topo::Body::new();
    for s in &imp.solids {
        body.append(&s.body);
    }
    let mut opt = forge_step::Options {
        name: src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("part")
            .to_string(),
        ..forge_step::Options::default()
    };
    if ap242 {
        opt.schema = forge_step::Schema::Ap242;
    }
    let e = forge_step::export(&body, &opt).map_err(|e| format!("{}: {e}", src.display()))?;
    std::fs::write(out, &e.text).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut f = vec![("ok", true.into()), ("wrote", out.display().to_string().into())];
    f.extend(report(&imp));
    let line = format!(
        "wrote {} ({} solid(s), {} refused)",
        out.display(),
        imp.solids.len(),
        imp.refusals.len()
    );
    Ok((Json::obj(f), line))
}
