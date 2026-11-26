//! Namespaced, project-fenced footprint libraries use the same pad reader as inline footprints.
use super::*;
use crate::electronics::{Call, Fault, Params, Result, header, number, string};
use crate::syntax::{self, StmtKind};
use crate::values::Dim;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

type Library = BTreeMap<String, Footprint>;
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
                ],
            )?;
            if c.name == "library" {
                let file = string(c.req("file")?)?;
                for (key, def) in self.load(&file, dir, root, e.span)? {
                    lib.insert(format!("{name}.{key}"), def);
                }
            } else {
                lib.insert(name.clone(), reader_layout::package(&mut c, &params)?);
            }
            c.end()?;
        }
        Ok(lib)
    }
}

pub fn query(
    src: &str,
    dir: Option<&Path>,
    root: Option<&Path>,
    words: &[String],
) -> Result<forge_agent::json::Json> {
    use forge_agent::json::Json;
    if header(src) != Some("footprint_library") {
        return response(&compile(src, dir, root)?, words);
    }
    let lib = Imports::default().read(src, dir, root)?;
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
            "result",
            Json::Arr(
                lib.into_iter()
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
