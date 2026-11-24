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
//!
//! [[turn_holder]]
//! id = "maker-PCLNR2525M12"
//! code = "PCLNR2525M12"              # ISO 5608 (holder) or ISO 6261 (bar)
//! kind = "holder"                    # holder | bar
//! maker = "..."
//! h = 25; b = 25; l1 = 150; f = 32   # f: nose to shank side
//! kr = 95; hand = "R"
//! # bars: dmm = 20; min_bore = 25
//! ```
use crate::toml::{self, Table, num, tables, text};
use crate::{Cite, LibError, cite};

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

impl Insert {
    pub fn from_table(t: &Table) -> Result<Insert, LibError> {
        let (id, code) = id_and(t, "insert")?;
        let rec = format!("insert `{id}`");
        let mut required: Vec<String> = ["code", "ic", "s", "re"].iter().map(|x| x.to_string()).collect();
        opt_required(
            t,
            &["l", "eps", "clearance", "grade", "chipbreaker"],
            &mut required,
        );
        let (real, cites) = cite::read(t, &rec, &required, &["id", "maker", "cutting"])?;
        Ok(Insert {
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
