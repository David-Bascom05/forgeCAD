//! STEP for fabricators (`fg run --step OUT.step`), written as forgecad's `export F ap214` writes
//! it. A part: its folded body, without any flat pattern an `unfold` made, in OUT; each flat
//! pattern in a file of its own beside it (`OUT-<unfold>.step`). An assembly: its structure,
//! one product per part and per subassembly, each use placed where the solve put it.

use crate::Run;
use forge_topo::Body;

/// One STEP file: what goes after OUT's stem (empty for OUT itself), and its text.
pub struct StepFile {
    pub suffix: String,
    pub text: String,
    pub solids: usize,
}

/// The part as it is made, less its flat patterns: what an assembly places of it too.
pub fn folded(run: &Run) -> Option<Body> {
    if run.sheets.iter().all(|s| s.unfolds.is_empty()) {
        return run.body.clone();
    }
    // a part that unfolds a sheet holds sheets only: the folded bodies are the part
    let mut out: Option<Body> = None;
    for b in run.sheets.iter().filter_map(|s| s.folded.as_ref()) {
        let b = forge_model::finish(b.clone());
        match &mut out {
            None => out = Some(b),
            Some(all) => all.append(&b),
        }
    }
    out
}

fn part(body: &Body, name: &str) -> Result<forge_step::Export, String> {
    let opt = forge_step::Options {
        schema: forge_step::Schema::Ap214,
        name: name.into(),
        ..forge_step::Options::default()
    };
    forge_step::export(body, &opt).map_err(|e| e.to_string())
}

/// The STEP files of a run, `name` the product's name (the file's stem).
pub fn step(run: &Run, name: &str) -> Result<Vec<StepFile>, String> {
    if let Some(a) = &run.asm {
        return match &a.step {
            Some(Ok(text)) => Ok(vec![StepFile {
                suffix: String::new(),
                text: text.clone(),
                solids: a.occurrences.len(),
            }]),
            Some(Err(e)) => Err(e.clone()),
            None => Err("the assembly was not solved, so it has no STEP".into()),
        };
    }
    let body = folded(run).ok_or_else(|| match run.errors.first() {
        Some(e) => format!("the part did not build: {}: {}", e.kind(), e.text()),
        None => "the part has no solid".to_string(),
    })?;
    let e = part(&body, name)?;
    let mut out = vec![StepFile {
        suffix: String::new(),
        text: e.text,
        solids: e.solids,
    }];
    for s in &run.sheets {
        for (id, flat) in &s.unfolds {
            let Some(flat) = flat else { continue };
            let e = part(&forge_model::finish(flat.clone()), &format!("{name}-{id}"))?;
            out.push(StepFile {
                suffix: format!("-{id}"),
                text: e.text,
                solids: e.solids,
            });
        }
    }
    Ok(out)
}
