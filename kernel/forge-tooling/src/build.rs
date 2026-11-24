//! Custom tools, holders and collets from library records (MACHINE-BUILDER's library builder,
//! research/cam/library-builder.md): `derive` copies a record under a new id with the user's
//! overrides, and the checks report what a record or an assembly gets wrong, each with the
//! measured value, the limit and the fix.
//!
//! Provenance of a derived record: an overridden field is cited `user`; a field the source cited
//! keeps its cite; a required field the source did not cite is cited `unknown`, "copied from
//! <from>". The derived record is never `real` (it is not the maker's product).
//!
//! The ISO 1832 / 5608 / 6261 code checks of inserts and turning holders live in forge-turn
//! (`decode_insert`, `decode_holder`, `decode_bar`), which depends on this crate.
use crate::toml::{Table, Val};
use crate::turning::{Blade, GrooveInsert, Insert, ToolBlock, TurnHolder};
use crate::{Collet, ColletNut, Holder, LibError, Library, PullStud, Tool};

/// One library record of any kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    Tool(Tool),
    Holder(Holder),
    Collet(Collet),
    ColletNut(ColletNut),
    PullStud(PullStud),
    Insert(Insert),
    TurnHolder(TurnHolder),
    GrooveInsert(GrooveInsert),
    Blade(Blade),
    ToolBlock(ToolBlock),
}

impl Record {
    pub fn id(&self) -> &str {
        match self {
            Record::Tool(x) => &x.id,
            Record::Holder(x) => &x.id,
            Record::Collet(x) => &x.id,
            Record::ColletNut(x) => &x.id,
            Record::PullStud(x) => &x.id,
            Record::Insert(x) => &x.id,
            Record::TurnHolder(x) => &x.id,
            Record::GrooveInsert(x) => &x.id,
            Record::Blade(x) => &x.id,
            Record::ToolBlock(x) => &x.id,
        }
    }

    /// The record's cites (so a caller need not match every kind).
    pub fn cites(&self) -> &[crate::Cite] {
        match self {
            Record::Tool(x) => &x.cites,
            Record::Holder(x) => &x.cites,
            Record::Collet(x) => &x.cites,
            Record::ColletNut(x) => &x.cites,
            Record::PullStud(x) => &x.cites,
            Record::Insert(x) => &x.cites,
            Record::TurnHolder(x) => &x.cites,
            Record::GrooveInsert(x) => &x.cites,
            Record::Blade(x) => &x.cites,
            Record::ToolBlock(x) => &x.cites,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeriveError {
    /// No record of this kind with this id.
    Unknown {
        kind: String,
        from: String,
        near: Vec<String>,
    },
    /// The kind is not one `derive` knows.
    Kind(String),
    /// The new id is taken.
    Taken(String),
    /// An override names a field the kind does not have, or the result does not read.
    Bad(LibError),
}

/// The TOML keys `derive` knows, as `kind`.
pub const KINDS: [&str; 10] = [
    "tool",
    "holder",
    "collet",
    "collet_nut",
    "pull_stud",
    "insert",
    "turn_holder",
    "groove_insert",
    "blade",
    "tool_block",
];

/// Fields a record may carry beyond those it cites (bookkeeping, not values).
const META: [&str; 9] = [
    "id",
    "real",
    "cite",
    "source",
    "taken",
    "note",
    "vendor",
    "product_id",
    "maker",
];

/// A new record `id` of `kind` from the library record `from`, with `overrides` (field, value).
pub fn derive(
    lib: &Library,
    kind: &str,
    from: &str,
    id: &str,
    overrides: &[(String, Val)],
) -> Result<Record, DeriveError> {
    if !KINDS.contains(&kind) {
        return Err(DeriveError::Kind(kind.to_string()));
    }
    let Some((_, _, src)) = lib.sources.iter().find(|(k, i, _)| k == kind && i == from) else {
        let mut near: Vec<String> = lib
            .sources
            .iter()
            .filter(|(k, _, _)| k == kind)
            .map(|(_, i, _)| i.clone())
            .filter(|i| i.contains(from) || from.contains(i.as_str()))
            .take(5)
            .collect();
        near.sort();
        return Err(DeriveError::Unknown {
            kind: kind.into(),
            from: from.into(),
            near,
        });
    };
    if lib.sources.iter().any(|(k, i, _)| k == kind && i == id) {
        return Err(DeriveError::Taken(id.into()));
    }
    let mut t: Table = src.clone();
    for (k, v) in overrides {
        if META.contains(&k.as_str()) {
            return Err(DeriveError::Bad(LibError::Bad {
                record: format!("{kind} `{id}`"),
                field: "override",
                why: format!("`{k}` is bookkeeping, not a value to override"),
            }));
        }
        t.insert(k.clone(), v.clone());
    }
    t.insert("id".into(), Val::Str(id.into()));
    t.insert("real".into(), Val::Bool(false));
    t.insert("source".into(), Val::Str(format!("derived from {kind} `{from}`")));
    // cites: the source's, minus overridden paths; the overrides as `user`
    let mut cites: Vec<Table> = match src.get("cite") {
        Some(Val::Tables(v)) => v
            .iter()
            .filter(|c| {
                !overrides
                    .iter()
                    .any(|(k, _)| matches!(c.get("path"), Some(Val::Str(p)) if p == k))
            })
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    let cite = |path: &str, status: &str, basis: Option<String>| {
        let mut c = Table::new();
        c.insert("path".into(), Val::Str(path.into()));
        c.insert("status".into(), Val::Str(status.into()));
        if let Some(b) = basis {
            c.insert("basis".into(), Val::Str(b));
        }
        c
    };
    for (k, _) in overrides {
        cites.push(cite(k, "user", None));
    }
    // every value field the record carries is cited: what the source left uncited is unknown
    let carried: Vec<String> = t
        .keys()
        .filter(|k| {
            !META.contains(&k.as_str())
                && k.as_str() != "type"
                && k.as_str() != "kind"
                && k.as_str() != "preset"
        })
        .cloned()
        .collect();
    for k in carried {
        if !cites
            .iter()
            .any(|c| matches!(c.get("path"), Some(Val::Str(p)) if *p == k))
        {
            cites.push(cite(
                &k,
                "unknown",
                Some(format!("copied from {kind} `{from}`, which does not cite it")),
            ));
        }
    }
    t.insert("cite".into(), Val::Tables(cites));
    let rec = match kind {
        "tool" => Tool::from_table(&t).map(Record::Tool),
        "holder" => Holder::from_table(&t).map(Record::Holder),
        "collet" => Collet::from_table(&t).map(Record::Collet),
        "collet_nut" => ColletNut::from_table(&t).map(Record::ColletNut),
        "pull_stud" => PullStud::from_table(&t).map(Record::PullStud),
        "insert" => Insert::from_table(&t).map(Record::Insert),
        "groove_insert" => GrooveInsert::from_table(&t).map(Record::GrooveInsert),
        "blade" => Blade::from_table(&t).map(Record::Blade),
        "tool_block" => ToolBlock::from_table(&t).map(Record::ToolBlock),
        _ => TurnHolder::from_table(&t).map(Record::TurnHolder),
    };
    rec.map_err(DeriveError::Bad)
}

/// How a finding stands: an error is a record that cannot be right; a warning is one the check
/// could not settle (it says what would settle it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// What a check found: the field, what it measured, the limit it broke, and the fix.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    pub path: String,
    pub measured: f64,
    pub limit: f64,
    pub fix: String,
}

/// A record's own consistency, and against the library (a holder's nose against the nuts of its
/// collet system).
pub fn check(lib: &Library, r: &Record) -> Vec<Finding> {
    let mut out = Vec::new();
    match r {
        Record::Tool(t) => {
            if t.re > t.dc / 2.0 + 1e-12 {
                out.push(Finding {
                    severity: Severity::Error,
                    path: "re".into(),
                    measured: t.re,
                    limit: t.dc / 2.0,
                    fix: "a corner radius is at most half the cutting diameter".into(),
                });
            }
            if t.tip_d > 0.0 && t.tip_d >= t.dc {
                out.push(Finding {
                    severity: Severity::Error,
                    path: "tip_d".into(),
                    measured: t.tip_d,
                    limit: t.dc,
                    fix: "the tip diameter is below the cutting diameter".into(),
                });
            }
        }
        Record::Holder(h) => {
            // the nose ring must cover the collet nut (the nut is what stands out): the holder's
            // own nut when it names one; otherwise the largest nut of its system, as a warning
            let own = h
                .nut
                .as_ref()
                .map(|id| lib.collet_nuts.iter().find(|n| &n.id == id));
            match own {
                Some(None) => out.push(Finding {
                    severity: Severity::Error,
                    path: "nut".into(),
                    measured: f64::NAN,
                    limit: f64::NAN,
                    fix: format!(
                        "nut `{}` is not a [[collet_nut]] in the library",
                        h.nut.as_deref().unwrap_or("")
                    ),
                }),
                Some(Some(n)) if h.profile[0][1] + 1e-9 < n.od => out.push(Finding {
                    severity: Severity::Error,
                    path: "profile".into(),
                    measured: h.profile[0][1],
                    limit: n.od,
                    fix: format!(
                        "the nose ring is smaller than its nut `{}`: widen it to the nut's OD",
                        n.id
                    ),
                }),
                Some(Some(_)) => {}
                None => {
                    let nut = lib
                        .collet_nuts
                        .iter()
                        .filter(|n| n.system.eq_ignore_ascii_case(&h.clamp))
                        .map(|n| n.od)
                        .fold(f64::NAN, f64::max);
                    if nut.is_finite() && h.profile[0][1] + 1e-9 < nut {
                        out.push(Finding {
                            severity: Severity::Warning,
                            path: "profile".into(),
                            measured: h.profile[0][1],
                            limit: nut,
                            fix: format!(
                                "not checked against this holder's nut: give `nut` (the largest {} nut in the library is wider than the nose ring)",
                                h.clamp
                            ),
                        });
                    }
                }
            }
        }
        Record::Collet(c) => {
            if let Some([lo, hi]) = c.clamp
                && !(lo <= c.bore + 1e-9 && c.bore <= hi + 1e-9)
            {
                out.push(Finding {
                    severity: Severity::Error,
                    path: "clamp".into(),
                    measured: c.bore,
                    limit: if c.bore < lo { lo } else { hi },
                    fix: "the nominal bore lies within the clamping range".into(),
                });
            }
        }
        _ => {}
    }
    out
}

/// A tool in a collet: the collet clamps the shank.
pub fn check_fit(tool: &Tool, collet: &Collet) -> Vec<Finding> {
    let [lo, hi] = collet.clamp.unwrap_or([collet.bore, collet.bore]);
    if tool.dcon + 1e-9 < lo || tool.dcon > hi + 1e-9 {
        vec![Finding {
            severity: Severity::Error,
            path: "dcon".into(),
            measured: tool.dcon,
            limit: if tool.dcon < lo { lo } else { hi },
            fix: format!(
                "collet `{}` clamps {lo}..{hi} mm: choose one whose range holds the shank",
                collet.id
            ),
        }]
    } else {
        vec![]
    }
}

/// A tool held at `stickout` (tip to the holder's nose): the flutes stand clear of the holder.
pub fn check_stickout(tool: &Tool, stickout: f64) -> Vec<Finding> {
    if stickout + 1e-9 < tool.lcf {
        vec![Finding {
            severity: Severity::Error,
            path: "stickout".into(),
            measured: stickout,
            limit: tool.lcf,
            fix: "hold the tool with at least its flute length out, or the holder rubs the cut".into(),
        }]
    } else {
        vec![]
    }
}
