//! Namespaced, project-fenced footprint libraries use the same pad reader as inline footprints.
use super::*;
use crate::electronics::{Call, Fault, Params, Result, header, number, string};
use crate::syntax::{self, StmtKind};
use crate::values::Dim;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(super) struct Library {
    pub packages: crate::electronics_library::Loaded,
    pub footprints: BTreeMap<String, Footprint>,
    pub rules: BTreeMap<String, String>,
}
pub(super) fn policy(c: &mut Call<'_>, rules: &mut BTreeMap<String, String>) -> Result<()> {
    let e = c.req("rule")?;
    let name = string(e)?;

    let level = crate::electronics::rule_severity(c.req("severity")?)?;
    if rules.insert(name.clone(), level).is_some() {
        return Err(crate::electronics::fault(e, "DuplicateDrcRule", name));
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct Imports {
    active: BTreeSet<PathBuf>,
    bytes: usize,
    files: usize,
}
impl Imports {
    pub(super) fn load(
        &mut self,
        file: &str,
        dir: Option<&Path>,
        root: Option<&Path>,
        span: Span,
    ) -> Result<Library> {
        let bad = || {
            Fault::new(
                "FootprintLibraryUnavailable",
                span,
                format!("cannot read footprint library {file}"),
            )
        };
        if let Some(lib) = crate::electronics_library::resolve_import(file)
            .map_err(|m| Fault::new("RustLibraryUnavailable", span, m))?
        {
            let mut packages = crate::electronics_library::Loaded::default();
            packages
                .add(lib)
                .map_err(|m| Fault::new("InvalidLibrary", span, m))?;
            return Ok(Library {
                packages,
                footprints: BTreeMap::new(),
                rules: BTreeMap::new(),
            });
        }
        let base = dir.unwrap_or_else(|| Path::new("."));
        let fence = forge_dirs::fs::canonicalize(root.unwrap_or(base)).map_err(|_| bad())?;
        let path = forge_dirs::fs::canonicalize(base.join(file)).map_err(|_| bad())?;
        if !path.starts_with(&fence) || path.extension().and_then(|v| v.to_str()) != Some("fg") {
            return Err(bad());
        }
        if self.active.len() >= 64 || self.files >= 8192 {
            return Err(Fault::new("FootprintLibraryLimit", span, file));
        }
        if !self.active.insert(path.clone()) {
            return Err(Fault::new("FootprintLibraryCycle", span, file));
        }
        let bytes = crate::deps::read_plain(&path, 8 * 1024 * 1024).map_err(|_| bad())?;
        self.bytes += bytes.len();
        self.files += 1;
        if self.bytes > 64 * 1024 * 1024 {
            return Err(Fault::new("FootprintLibraryLimit", span, file));
        }
        crate::deps::read(&path, Some(&bytes));
        let src = std::str::from_utf8(&bytes).map_err(|_| bad())?;
        let r = self.read(src, path.parent(), Some(&fence)).map_err(|mut e| {
            e.file.get_or_insert_with(|| {
                path.strip_prefix(&fence)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned()
            });
            e
        });
        self.active.remove(&path);
        r
    }
    pub(super) fn read(&mut self, src: &str, dir: Option<&Path>, root: Option<&Path>) -> Result<Library> {
        let tree =
            syntax::parse(src).map_err(|e| Fault::new(e.kind(), e.span().unwrap_or_default(), e.text()))?;
        if header(src) != Some("footprint_library") {
            return Err(Fault::new(
                "ExpectedFootprintLibrary",
                Span::default(),
                "expected footprint_library",
            ));
        }
        let mut names = BTreeSet::new();
        let mut lib = BTreeMap::new();
        let mut rules = BTreeMap::new();
        let mut defaults = crate::electronics::RuleDefaults::default();
        let mut packages = crate::electronics_library::Loaded::standard();
        let mut params = Params::new();
        for s in tree.iter().skip(1) {
            let (name, e) = match &s.kind {
                StmtKind::Assign { name, value } | StmtKind::Shared { name, value } => (name, value),
                _ => {
                    return Err(Fault::new(
                        "ExpectedDeclaration",
                        s.span,
                        "use a footprint, shared length or nested library",
                    ));
                }
            };
            if !names.insert(name) {
                return Err(Fault::new("DuplicateName", s.span, name));
            }
            if matches!(s.kind, StmtKind::Shared { .. }) {
                let mut n = number(e, &params)?;
                if n.dim == Dim::Any {
                    n.dim = Dim::Length;
                }
                params.insert(name.clone(), n);
                continue;
            }
            let mut c = Call::with_signature(
                e,
                &[
                    ("footprint", reader_layout::FOOTPRINT_ARGS),
                    ("library", &["file"]),
                    ("drc_rule", &["rule", "severity"]),
                ],
            )?;
            if c.name == "library" {
                let file = string(c.req("file")?)?;
                let child = self.load(&file, dir, root, e.span)?;
                packages
                    .merge(&child.packages)
                    .map_err(|m| Fault::new("InvalidLibrary", e.span, m))?;
                defaults.import(name, child.rules);
                for (key, def) in child.footprints {
                    lib.insert(format!("{name}.{key}"), def);
                }
            } else if c.name == "drc_rule" {
                policy(&mut c, &mut rules)?;
            } else {
                lib.insert(name.clone(), reader_layout::package(&mut c, &params)?);
            }
            c.end()?;
        }
        defaults.resolve(&mut rules, Span::default())?;
        if self.active.is_empty() {
            validate_rules(&packages, &rules, Span::default())?;
        }
        Ok(Library {
            packages,
            footprints: lib,
            rules,
        })
    }
}

pub(super) fn validate_rules(
    packages: &crate::electronics_library::Loaded,
    rules: &BTreeMap<String, String>,
    span: Span,
) -> Result<()> {
    for name in rules.keys() {
        if Rule::parse(name).is_none() && !packages.drc.contains_key(name.as_str()) {
            return Err(Fault::new("UnknownDrcRule", span, name));
        }
    }
    Ok(())
}
pub fn query(
    src: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    words: &[String],
) -> Result<forge_agent::json::Json> {
    use forge_agent::json::Json;
    if header(src) != Some("footprint_library") {
        let board = compile(src, dir, root)?;
        if words.first().is_some_and(|s| s == "floorplan") {
            return floorplan::compare(&board, src, dir, root, &words[1..]);
        }
        if words.first().is_some_and(|s| s == "repack") {
            return packing::query(&board, src, dir, root, &words[1..]);
        }
        return response(&board, words);
    }
    let lib = Imports::default().read(src, dir, root)?;
    if matches!(words,[w] if w=="libraries") {
        return Ok(Json::obj(vec![
            ("ok", true.into()),
            ("result", lib.packages.json()),
        ]));
    }
    if !words.is_empty()
        && !matches!(words,[w] if matches!(w.as_str(),"summary"|"stat"|"check"|"drc"|"footprints"))
    {
        return Err(Fault::new(
            "UnknownLibraryQuery",
            Span::default(),
            "choose footprints",
        ));
    }
    Ok(Json::obj(vec![
        ("ok", true.into()),
        ("schema", "forgecad.footprint-library/1".into()),
        (
            "rule_severities",
            Json::Obj(lib.rules.into_iter().map(|(k, v)| (k, v.into())).collect()),
        ),
        (
            "result",
            Json::Arr(
                lib.footprints
                    .into_iter()
                    .map(|(id, f)| {
                        Json::obj(vec![
                            ("id", id.into()),
                            (
                                "pads",
                                Json::Arr(f.pads.into_iter().map(|p| p.number.into()).collect()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}
