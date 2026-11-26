//! Per-value provenance for library records (the machine library's rule, docs/cam-machine.md
//! "Provenance"): a record with `real = true` cites every value it must; each cite says where the
//! value comes from and how it stands. Shared by tools, holders, collets, nuts, pull studs and
//! the turning records.
use crate::LibError;
use crate::toml::{Table, boolean, tables, text};

/// How a cited value stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiteStatus {
    /// Stated by the source.
    Published,
    /// Not published: derived as `basis` says, erring the safe way.
    Estimate,
    /// Nothing to derive it from: a stand-in, `basis` says which way it errs.
    Unknown,
    /// Sources disagree: each is cited, `note` says which the record holds.
    Conflict,
    /// The user's own value (a custom tool, holder or collet).
    User,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    /// The field cited: `dc`, `lcf`, `profile`, `preset.Al`, ...
    pub path: String,
    pub status: CiteStatus,
    pub source: String,
    pub taken: String,
    /// The source's own statement of the value, restated.
    pub value: String,
    pub basis: String,
    pub note: String,
}

/// `real = true` and the record's `[[<kind>.cite]]` list, checked: each cite's status carries
/// what it needs, every `required` path is cited on a real record, and no cite names a field
/// outside `allowed` (nor `required`).
pub fn read(
    t: &Table,
    rec: &str,
    required: &[String],
    allowed: &[&str],
) -> Result<(bool, Vec<Cite>), LibError> {
    let real = boolean(t, "real").unwrap_or(false);
    let mut cites = Vec::new();
    for c in tables(t, "cite") {
        let path = text(c, "path")
            .ok_or_else(|| LibError::Missing {
                record: format!("{rec} cite"),
                field: "path",
                fix: "give `path`, the field cited (e.g. \"dc\")".into(),
            })?
            .to_string();
        let crec = format!("{rec} cite `{path}`");
        let st = text(c, "status").unwrap_or("");
        let status = match st {
            "published" => CiteStatus::Published,
            "estimate" => CiteStatus::Estimate,
            "unknown" => CiteStatus::Unknown,
            "conflict" => CiteStatus::Conflict,
            "user" => CiteStatus::User,
            _ => {
                return Err(LibError::Bad {
                    record: crec,
                    field: "status",
                    why: format!("`{st}` is not published | estimate | unknown | conflict | user"),
                });
            }
        };
        let s = |k: &str| text(c, k).unwrap_or("").to_string();
        let cite = Cite {
            path: path.clone(),
            status,
            source: s("source"),
            taken: s("taken"),
            value: s("value"),
            basis: s("basis"),
            note: s("note"),
        };
        let missing = |field: &'static str, fix: &str| {
            Err(LibError::Missing {
                record: crec.clone(),
                field,
                fix: fix.into(),
            })
        };
        match status {
            CiteStatus::Published | CiteStatus::Conflict if cite.source.is_empty() => {
                return missing("source", "give `source`, the URL or document the value is from");
            }
            CiteStatus::Estimate | CiteStatus::Unknown if cite.basis.is_empty() => {
                return missing("basis", "say how it was derived, or which way the stand-in errs");
            }
            CiteStatus::Conflict if cite.note.is_empty() => {
                return missing("note", "say what disagrees and which value the record holds");
            }
            _ => {}
        }
        if !cite.source.is_empty() && cite.taken.is_empty() {
            return missing("taken", "give `taken = \"YYYY-MM-DD\"`, when the source was read");
        }
        let head = path.split('.').next().unwrap_or("");
        if !required.contains(&path) && !allowed.contains(&head) {
            return Err(LibError::Bad {
                record: crec,
                field: "path",
                why: format!(
                    "names no field of this record; cite one of: {}",
                    required.join(", ")
                ),
            });
        }
        cites.push(cite);
    }
    if real {
        let un: Vec<&String> = required
            .iter()
            .filter(|p| !cites.iter().any(|c| &c.path == *p))
            .collect();
        if !un.is_empty() {
            return Err(LibError::Bad {
                record: rec.to_string(),
                field: "real",
                why: format!(
                    "a real record cites every value; uncited: {}",
                    un.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
            });
        }
    }
    Ok((real, cites))
}
