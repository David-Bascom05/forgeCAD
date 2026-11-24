//! Spindle interfaces: the toolholder shank's geometry (steep 7:24 tapers BT and CAT, HSK), as
//! cited dimension tables for drawing realistic holders (CAM-SIM). The standards (MAS 403 / JIS
//! B 6339, ASME B5.50, ISO 12164 / DIN 69893) are paywalled: every value is restated from a
//! maker's published catalogue page or drawing, and what no maker prints is absent (UNKNOWN),
//! never a guess. Holders name their interface (`interface = "BT40"`); `Library::interface`
//! finds its table by id or alias.
//!
//! Axial positions are from the gauge line (steep tapers: the taper's gauge diameter; HSK: the
//! face-contact plane), towards the spindle (taper side) or the tool (flange side) as named.
//!
//! ```toml
//! [[interface]]
//! id = "BT40"; aliases = ["BT-40"]; kind = "steep"   # steep | hsk
//! standard = "..."; taper = "7/24"
//! gauge_d = 44.45          # taper diameter at the gauge line
//! taper_length = 65.4      # gauge line to the taper's small end
//! flange_d = 63; flange_t = 25
//! gauge_to_flange_back = 2 # gauge line to the flange's spindle-side face
//! gauge_to_flange_front = 26.9
//! key_slot_w = 16.1; pull_stud_thread = "M16"
//! # hsk: collar_t (flange/collar thickness from the contact face)
//! ```
use crate::toml::{Table, num, text, texts};
use crate::{Cite, LibError, cite};

/// The numeric dimensions an interface may carry, mm.
pub const DIMS: [&str; 8] = [
    "gauge_d",
    "taper_length",
    "flange_d",
    "flange_t",
    "gauge_to_flange_back",
    "gauge_to_flange_front",
    "key_slot_w",
    "collar_t",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Interface {
    pub id: String,
    pub aliases: Vec<String>,
    /// "steep" (7:24: BT, CAT) or "hsk".
    pub kind: String,
    pub standard: String,
    /// The taper as printed ("7/24", "1/10").
    pub taper: String,
    /// (name, mm) of the dimensions printed, in `DIMS` order.
    pub dims: Vec<(String, f64)>,
    pub pull_stud_thread: String,
    pub source: String,
    pub taken: String,
    pub note: String,
    pub real: bool,
    pub cites: Vec<Cite>,
}

impl Interface {
    /// A dimension by name (`DIMS`), or None when no maker prints it.
    pub fn dim(&self, name: &str) -> Option<f64> {
        self.dims.iter().find(|(k, _)| k == name).map(|(_, v)| *v)
    }

    /// Whether `name` (a holder's `interface`) names this one: its id or an alias, ignoring
    /// case, dashes and spaces.
    pub fn is(&self, name: &str) -> bool {
        let n = |s: &str| {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_uppercase()
        };
        let want = n(name);
        n(&self.id) == want || self.aliases.iter().any(|a| n(a) == want)
    }

    pub fn from_table(t: &Table) -> Result<Interface, LibError> {
        let id = text(t, "id").ok_or_else(|| LibError::Missing {
            record: "[[interface]]".into(),
            field: "id",
            fix: "give `id`, e.g. \"BT40\"".into(),
        })?;
        let rec = format!("interface `{id}`");
        let kind = text(t, "kind").unwrap_or("");
        if kind != "steep" && kind != "hsk" {
            return Err(LibError::Bad {
                record: rec,
                field: "kind",
                why: format!("`{kind}` is not \"steep\" | \"hsk\""),
            });
        }
        let mut dims = vec![];
        let mut required: Vec<String> = vec![];
        for k in DIMS {
            if let Some(v) = num(t, k) {
                if !(v > 0.0 && v.is_finite()) {
                    return Err(LibError::Bad {
                        record: rec,
                        field: k,
                        why: format!("must be > 0 (is {v})"),
                    });
                }
                dims.push((k.to_string(), v));
                required.push(k.to_string());
            }
        }
        for k in ["taper", "pull_stud_thread"] {
            if text(t, k).is_some() {
                required.push(k.to_string());
            }
        }
        let (real, cites) = cite::read(t, &rec, &required, &["id", "aliases", "kind", "standard"])?;
        let s = |k: &str| text(t, k).unwrap_or("").to_string();
        Ok(Interface {
            id: id.to_string(),
            aliases: texts(t, "aliases").unwrap_or_default(),
            kind: kind.to_string(),
            standard: s("standard"),
            taper: s("taper"),
            dims,
            pull_stud_thread: s("pull_stud_thread"),
            source: s("source"),
            taken: s("taken"),
            note: s("note"),
            real,
            cites,
        })
    }
}
