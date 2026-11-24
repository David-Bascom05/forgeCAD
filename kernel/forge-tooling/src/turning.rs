//! Turning inserts, holders and boring bars as cited maker data (CAM-TURN's request: forge-turn
//! decodes the ISO code, checks it against the dimensions stated here, refuses a mismatch by
//! name, and uses the stated holder offsets in place of its generic supersets). All mm and
//! degrees; each value cited (cite.rs). The ISO codes are the keys forge-turn decodes; nothing
//! here decodes them.
//!
//! ```toml
//! [[insert]]
//! id = "maker-CNMG120408-PM-4325"   # unique
//! code = "CNMG120408"               # ISO 1832
//! maker = "..."
//! grade = "4325"
//! chipbreaker = "PM"
//! ic = 12.7                          # inscribed circle
//! s = 4.76                           # thickness
//! re = 0.8                           # corner radius
//! l = 12.9                           # cutting edge length (optional)
//! eps = 80                           # point angle (optional)
//! clearance = 0                      # (optional)
//! [[insert.preset]]                  # the maker's cutting data (optional), cited `preset.<name>`
//! name = "P PM 4325"; group = "P"; vc = 300; fz = 0.15; ap = 1.5   # fz: feed, mm/rev
//!
//! [[turn_holder]]
//! id = "maker-PCLNR2525M12"
//! code = "PCLNR2525M12"              # ISO 5608 (holder) or ISO 6261 (bar)
//! kind = "holder"                    # holder | bar
//! maker = "..."
//! h = 25; b = 25; l1 = 150; f = 32   # f: nose to shank side
//! kr = 95; hand = "R"
//! # bars: dmm = 20; min_bore = 25
//!
//! [[groove_insert]]                  # parting / grooving: the maker's code, not ISO 1832
//! id = "maker-..."; code = "..."; width = 3; re = 0.2   # + clearance, head, seat, grade, chipbreaker
//! [[blade]]                          # a parting blade
//! id = "..."; code = "..."; h = 26; t_blade = 2.4; l1 = 110; cdx = 40; hand = "N"   # + max_bar, seat, insert
//! [[tool_block]]                     # the block a blade clamps in
//! id = "..."; code = "..."; h = 25; b = 25; l1 = 110   # + blade_h
//! ```
use crate::toml::{self, Table, num, tables, text};
use crate::{Cite, LibError, Preset, cite, material_score, preset_paths, read_presets};

#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    pub id: String,
    /// The ISO 1832 code, as the maker writes it without spaces.
    pub code: String,
    pub maker: String,
    pub grade: String,
    pub chipbreaker: String,
    pub ic: f64,
    pub s: f64,
    pub re: f64,
    pub l: Option<f64>,
    pub eps: Option<f64>,
    pub clearance: Option<f64>,
    /// The maker's cutting data, `[[insert.preset]]`: vc m/min, `fz` the feed in mm/rev, ap mm.
    pub presets: Vec<Preset>,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderKind {
    /// An external turning holder (ISO 5608).
    Holder,
    /// A boring bar (ISO 6261).
    Bar,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnHolder {
    pub id: String,
    pub code: String,
    pub kind: HolderKind,
    pub maker: String,
    /// Shank height and width (holders), mm.
    pub h: Option<f64>,
    pub b: Option<f64>,
    /// Bar diameter (bars), mm.
    pub dmm: Option<f64>,
    pub l1: f64,
    /// Nose to the shank's side face (functional width), mm.
    pub f: f64,
    /// Entering angle, degrees.
    pub kr: f64,
    /// "R" | "L" | "N".
    pub hand: String,
    /// Smallest bore a bar enters, mm.
    pub min_bore: Option<f64>,
    /// The insert code it takes (as the maker states it).
    pub insert: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

fn pos(rec: &str, k: &'static str, t: &Table) -> Result<f64, LibError> {
    let v = num(t, k).ok_or_else(|| LibError::Missing {
        record: rec.to_string(),
        field: k,
        fix: format!("give `{k}` (mm or degrees)"),
    })?;
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(LibError::Bad {
            record: rec.to_string(),
            field: k,
            why: format!("must be > 0 (is {v})"),
        })
    }
}

fn id_and(t: &Table, kind: &str) -> Result<(String, String), LibError> {
    let id = text(t, "id").ok_or_else(|| LibError::Missing {
        record: format!("[[{kind}]]"),
        field: "id",
        fix: "give `id`".into(),
    })?;
    let rec = format!("{kind} `{id}`");
    let code = text(t, "code").ok_or_else(|| LibError::Missing {
        record: rec.clone(),
        field: "code",
        fix: "give the ISO code, e.g. `code = \"CNMG120408\"`".into(),
    })?;
    Ok((id.to_string(), code.replace(' ', "")))
}

fn s(t: &Table, k: &str) -> String {
    text(t, k).unwrap_or("").to_string()
}

fn opt_required(t: &Table, keys: &[&str], into: &mut Vec<String>) {
    for k in keys {
        if num(t, k).is_some() || text(t, k).is_some() {
            into.push((*k).to_string());
        }
    }
}

/// An insert preset as picked for a cut: the preset, its cite, and how it was chosen.
#[derive(Clone, Debug)]
pub struct InsertPick<'a> {
    /// The row the speed (and depth) is taken from.
    pub preset: &'a Preset,
    pub cite: Option<&'a Cite>,
    /// The feed, mm/rev: the speed row's own where a material's row was picked; else the
    /// 10th percentile of the same rows' feeds (picked separately: `f_row`).
    pub f: f64,
    pub f_row: &'a Preset,
    pub f_cite: Option<&'a Cite>,
    pub why: String,
}

impl Insert {
    /// The insert's preset for a group (CAM-HAAS's names: "<G> <chipbreaker> <grade>" the
    /// maker's lead row for the grade, "<G> <chipbreaker> <grade> <material>" each printed
    /// material). The grade is the one asked for (the insert record names none: a code is sold
    /// in several grades); none asked, the grade whose lead row is slowest, so an unstated grade
    /// never runs faster than any of them. Within the grade, the maker's row naming the stock
    /// material (`material_score`), else the lead row. Err: the grades the insert has for the
    /// group, where the one asked is not among them.
    pub fn preset_for(
        &self,
        group: &str,
        grade: Option<&str>,
        material: Option<&str>,
    ) -> Result<Option<InsertPick<'_>>, Vec<String>> {
        let head = format!("{group} {} ", self.chipbreaker);
        // (grade, material text) of each preset of the group
        let parts: Vec<(&Preset, &str, &str)> = self
            .presets
            .iter()
            .filter(|p| p.group == group)
            .map(|p| {
                let rest = p.name.strip_prefix(&head).unwrap_or("");
                let (g, m) = rest.split_once(' ').unwrap_or((rest, ""));
                (p, g, m)
            })
            .collect();
        if parts.is_empty() {
            return Ok(None);
        }
        let mut grades: Vec<&str> = parts.iter().map(|x| x.1).collect();
        grades.sort();
        grades.dedup();
        // the 10th-percentile row by speed of a set (an actual cited row: nearest rank)
        fn p10(rows: Vec<&Preset>) -> Option<&Preset> {
            let q = crate::feeds::Quantiles::of(rows.iter().map(|p| p.vc).collect())?;
            rows.into_iter().find(|p| p.vc == q.p10)
        }
        fn p10_f(rows: Vec<&Preset>) -> Option<&Preset> {
            let q = crate::feeds::Quantiles::of(rows.iter().map(|p| p.fz).collect())?;
            rows.into_iter().find(|p| p.fz == q.p10)
        }
        // a grade's material rows (its lead row where it prints none)
        let rows_of = |g: Option<&str>| -> Vec<&Preset> {
            let m: Vec<&Preset> = parts
                .iter()
                .filter(|x| g.is_none_or(|g| x.1 == g) && !x.2.is_empty())
                .map(|x| x.0)
                .collect();
            if m.is_empty() {
                parts
                    .iter()
                    .filter(|x| g.is_none_or(|g| x.1 == g))
                    .map(|x| x.0)
                    .collect()
            } else {
                m
            }
        };
        let grade = match grade {
            Some(g) if grades.contains(&g) => Some(g),
            Some(_) => return Err(grades.iter().map(|g| g.to_string()).collect()),
            None => None,
        };
        let matched = match (grade, material) {
            (Some(g), Some(m)) => parts
                .iter()
                .filter(|x| x.1 == g && !x.2.is_empty())
                .map(|x| (material_score(x.2, m), x.0))
                .filter(|x| x.0 > 0)
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.vc.total_cmp(&a.1.vc)))
                .map(|x| x.1),
            _ => None,
        };
        let (p, fr, why) = match (matched, grade) {
            (Some(p), Some(g)) => (p, p, format!("grade {g}; the maker's row for the stock material")),
            (_, Some(g)) => {
                let rows = rows_of(Some(g));
                let n = rows.len();
                let (Some(p), Some(fr)) = (p10(rows.clone()), p10_f(rows)) else {
                    return Ok(None);
                };
                (
                    p,
                    fr,
                    format!(
                        "grade {g}; {}: the 10th percentiles by speed and, separately, by feed of the grade's {n} rows",
                        if material.is_some() {
                            "no row names the stock material"
                        } else {
                            "no stock material named"
                        }
                    ),
                )
            }
            (_, None) => {
                let rows = rows_of(None);
                let n = rows.len();
                let (Some(p), Some(fr)) = (p10(rows.clone()), p10_f(rows)) else {
                    return Ok(None);
                };
                (
                    p,
                    fr,
                    format!(
                        "grade not stated: the 10th percentiles by speed and, separately, by feed of the insert's {n} rows for {group} over its {} grades ({}); give the insert's grade (and the stock material) to use its own",
                        grades.len(),
                        grades.join(", ")
                    ),
                )
            }
        };
        let cite_of = |p: &Preset| {
            let path = format!("preset.{}", p.name);
            self.cites.iter().find(|c| c.path == path)
        };
        Ok(Some(InsertPick {
            preset: p,
            cite: cite_of(p),
            f: fr.fz,
            f_row: fr,
            f_cite: cite_of(fr),
            why,
        }))
    }

    pub fn from_table(t: &Table) -> Result<Insert, LibError> {
        let (id, code) = id_and(t, "insert")?;
        let rec = format!("insert `{id}`");
        let mut required: Vec<String> = ["code", "ic", "s", "re"].iter().map(|x| x.to_string()).collect();
        opt_required(
            t,
            &["l", "eps", "clearance", "grade", "chipbreaker"],
            &mut required,
        );
        let presets = read_presets(t, &rec)?;
        required.extend(preset_paths(&presets));
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker", "cutting"])?;
        Ok(Insert {
            presets,
            ic: pos(&rec, "ic", t)?,
            s: pos(&rec, "s", t)?,
            re: num(t, "re")
                .filter(|v| *v >= 0.0)
                .ok_or_else(|| LibError::Missing {
                    record: rec.clone(),
                    field: "re",
                    fix: "give `re`, the corner radius (0 for a sharp corner)".into(),
                })?,
            l: num(t, "l"),
            eps: num(t, "eps"),
            clearance: num(t, "clearance"),
            maker: s(t, "maker"),
            grade: s(t, "grade"),
            chipbreaker: s(t, "chipbreaker"),
            source: s(t, "source"),
            taken: s(t, "taken"),
            note: s(t, "note"),
            real,
            cites,
            code,
            id,
        })
    }
}

impl TurnHolder {
    pub fn from_table(t: &Table) -> Result<TurnHolder, LibError> {
        let (id, code) = id_and(t, "turn_holder")?;
        let rec = format!("turning holder `{id}`");
        let kind = match text(t, "kind") {
            Some("holder") => HolderKind::Holder,
            Some("bar") => HolderKind::Bar,
            other => {
                return Err(LibError::Bad {
                    record: rec,
                    field: "kind",
                    why: format!("{other:?} is not \"holder\" | \"bar\""),
                });
            }
        };
        let hand = s(t, "hand");
        if !["R", "L", "N"].contains(&hand.as_str()) {
            return Err(LibError::Bad {
                record: rec,
                field: "hand",
                why: format!("`{hand}` is not R | L | N"),
            });
        }
        let (h, b, dmm) = match kind {
            HolderKind::Holder => (Some(pos(&rec, "h", t)?), Some(pos(&rec, "b", t)?), None),
            HolderKind::Bar => (None, None, Some(pos(&rec, "dmm", t)?)),
        };
        let mut required: Vec<String> = ["code", "l1", "f", "kr", "hand"]
            .iter()
            .map(|x| x.to_string())
            .collect();
        match kind {
            HolderKind::Holder => required.extend(["h".to_string(), "b".to_string()]),
            HolderKind::Bar => required.push("dmm".into()),
        }
        opt_required(t, &["min_bore", "insert"], &mut required);
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker", "kind"])?;
        Ok(TurnHolder {
            l1: pos(&rec, "l1", t)?,
            f: pos(&rec, "f", t)?,
            kr: pos(&rec, "kr", t)?,
            min_bore: num(t, "min_bore"),
            maker: s(t, "maker"),
            insert: s(t, "insert"),
            source: s(t, "source"),
            taken: s(t, "taken"),
            note: s(t, "note"),
            h,
            b,
            dmm,
            hand,
            kind,
            real,
            cites,
            code,
            id,
        })
    }
}

/// A grooving or parting insert (`[[groove_insert]]`): no ISO 1832 code; the cut is `width`.
#[derive(Clone, Debug, PartialEq)]
pub struct GrooveInsert {
    pub id: String,
    /// The maker's designation.
    pub code: String,
    pub maker: String,
    pub grade: String,
    pub chipbreaker: String,
    /// Cutting width, mm.
    pub width: f64,
    /// Corner radius, mm.
    pub re: f64,
    /// Side clearance angle, degrees (optional).
    pub clearance: Option<f64>,
    /// Length of the insert along the blade, mm (optional).
    pub head: Option<f64>,
    /// The seat size it fits, as the maker prints it (optional).
    pub seat: String,
    /// The maker's cutting data: vc m/min, `fz` the feed in mm/rev.
    pub presets: Vec<Preset>,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// A parting/grooving blade (`[[blade]]`), clamped in a tool block.
#[derive(Clone, Debug, PartialEq)]
pub struct Blade {
    pub id: String,
    pub code: String,
    pub maker: String,
    /// Blade height, mm.
    pub h: f64,
    /// Blade thickness, mm.
    pub t_blade: f64,
    /// Blade length, mm.
    pub l1: f64,
    /// Maximum depth of cut (reach), mm.
    pub cdx: f64,
    /// Largest bar it parts off, mm (optional).
    pub max_bar: Option<f64>,
    /// The insert seat, as printed.
    pub seat: String,
    /// The insert codes it takes, as printed.
    pub insert: String,
    /// "R" | "L" | "N".
    pub hand: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

/// The block a blade clamps in (`[[tool_block]]`).
#[derive(Clone, Debug, PartialEq)]
pub struct ToolBlock {
    pub id: String,
    pub code: String,
    pub maker: String,
    /// Shank height and width, block length, mm.
    pub h: f64,
    pub b: f64,
    pub l1: f64,
    /// The blade height it takes, mm (optional).
    pub blade_h: Option<f64>,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

fn hand_of(rec: &str, t: &Table) -> Result<String, LibError> {
    let hand = s(t, "hand");
    if ["R", "L", "N"].contains(&hand.as_str()) {
        Ok(hand)
    } else {
        Err(LibError::Bad {
            record: rec.to_string(),
            field: "hand",
            why: format!("`{hand}` is not R | L | N"),
        })
    }
}

impl GrooveInsert {
    pub fn from_table(t: &Table) -> Result<GrooveInsert, LibError> {
        let (id, code) = id_and(t, "groove_insert")?;
        let rec = format!("groove insert `{id}`");
        let mut required: Vec<String> = ["code", "width", "re"].iter().map(|x| x.to_string()).collect();
        opt_required(
            t,
            &["clearance", "head", "seat", "grade", "chipbreaker"],
            &mut required,
        );
        let presets = read_presets(t, &rec)?;
        required.extend(preset_paths(&presets));
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker"])?;
        Ok(GrooveInsert {
            width: pos(&rec, "width", t)?,
            re: num(t, "re")
                .filter(|v| *v >= 0.0)
                .ok_or_else(|| LibError::Missing {
                    record: rec.clone(),
                    field: "re",
                    fix: "give `re`, the corner radius (0 for a sharp corner)".into(),
                })?,
            clearance: num(t, "clearance"),
            head: num(t, "head"),
            seat: s(t, "seat"),
            maker: s(t, "maker"),
            grade: s(t, "grade"),
            chipbreaker: s(t, "chipbreaker"),
            source: s(t, "source"),
            taken: s(t, "taken"),
            note: s(t, "note"),
            presets,
            real,
            cites,
            code,
            id,
        })
    }
}

impl Blade {
    pub fn from_table(t: &Table) -> Result<Blade, LibError> {
        let (id, code) = id_and(t, "blade")?;
        let rec = format!("blade `{id}`");
        let hand = hand_of(&rec, t)?;
        let mut required: Vec<String> = ["code", "h", "t_blade", "l1", "cdx", "hand"]
            .iter()
            .map(|x| x.to_string())
            .collect();
        opt_required(t, &["max_bar", "seat", "insert"], &mut required);
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker"])?;
        Ok(Blade {
            h: pos(&rec, "h", t)?,
            t_blade: pos(&rec, "t_blade", t)?,
            l1: pos(&rec, "l1", t)?,
            cdx: pos(&rec, "cdx", t)?,
            max_bar: num(t, "max_bar"),
            seat: s(t, "seat"),
            insert: s(t, "insert"),
            maker: s(t, "maker"),
            source: s(t, "source"),
            taken: s(t, "taken"),
            note: s(t, "note"),
            hand,
            real,
            cites,
            code,
            id,
        })
    }
}

impl ToolBlock {
    pub fn from_table(t: &Table) -> Result<ToolBlock, LibError> {
        let (id, code) = id_and(t, "tool_block")?;
        let rec = format!("tool block `{id}`");
        let mut required: Vec<String> = ["code", "h", "b", "l1"].iter().map(|x| x.to_string()).collect();
        opt_required(t, &["blade_h"], &mut required);
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker"])?;
        Ok(ToolBlock {
            h: pos(&rec, "h", t)?,
            b: pos(&rec, "b", t)?,
            l1: pos(&rec, "l1", t)?,
            blade_h: num(t, "blade_h"),
            maker: s(t, "maker"),
            source: s(t, "source"),
            taken: s(t, "taken"),
            note: s(t, "note"),
            real,
            cites,
            code,
            id,
        })
    }
}

/// Every `[[insert]]` and `[[turn_holder]]` in a file (forge-turn's entry point).
pub fn parse(src: &str) -> Result<(Vec<Insert>, Vec<TurnHolder>), LibError> {
    let t = toml::parse(src).map_err(|e| LibError::Toml {
        file: String::new(),
        line: e.line,
        what: e.what,
    })?;
    let inserts = tables(&t, "insert")
        .iter()
        .map(Insert::from_table)
        .collect::<Result<Vec<_>, _>>()?;
    let holders = tables(&t, "turn_holder")
        .iter()
        .map(TurnHolder::from_table)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((inserts, holders))
}
