//! Tool-entry guidance (data/entry.toml): cited makers' limits on the feed at entry and in a
//! plunge, the ramp angle and the helix diameter. A warning about load at entry, reported apart
//! from tool life (forge_tooling::life): Taylor's equation describes gradual flank wear, not
//! breakage at entry, and the validation (research/cam/toollife/validation.md) found entry
//! breakage is what failed.
use crate::LibError;
use crate::toml::{num, tables, text, texts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// Feed at first contact over the op's largest cutting feed.
    EntryFeed,
    /// Feed of a straight plunge into stock over the op's largest cutting feed.
    PlungeFeed,
    /// Ramp angle, degrees.
    RampAngle,
    /// Helix diameter over tool diameter.
    HelixDiameter,
}

impl Check {
    pub fn parse(s: &str) -> Option<Check> {
        Some(match s {
            "entry_feed" => Check::EntryFeed,
            "plunge_feed" => Check::PlungeFeed,
            "ramp_angle" => Check::RampAngle,
            "helix_diameter" => Check::HelixDiameter,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Check::EntryFeed => "entry_feed",
            Check::PlungeFeed => "plunge_feed",
            Check::RampAngle => "ramp_angle",
            Check::HelixDiameter => "helix_diameter",
        }
    }
    /// Whether a value over the limit is the bad side (else under).
    pub fn max(self) -> bool {
        !matches!(self, Check::HelixDiameter)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    pub check: Check,
    pub limit: f64,
    /// ISO 513 groups it applies to (empty: all).
    pub groups: Vec<String>,
    pub source: String,
    pub taken: String,
    pub value: String,
    pub note: String,
}

impl Rule {
    pub fn applies(&self, group: &str) -> bool {
        self.groups.is_empty() || self.groups.iter().any(|g| g.eq_ignore_ascii_case(group))
    }
}

pub fn read(src: &str, file: &str) -> Result<Vec<Rule>, LibError> {
    let t = crate::toml::parse(src).map_err(|e| LibError::Toml {
        file: file.to_string(),
        line: e.line,
        what: e.what,
    })?;
    let mut out = Vec::new();
    for r in tables(&t, "entry") {
        let id = text(r, "id").unwrap_or("").to_string();
        let rec = format!("entry {id}");
        let c = text(r, "check").unwrap_or("");
        let check = Check::parse(c).ok_or_else(|| LibError::Bad {
            record: rec.clone(),
            field: "check",
            why: format!("`{c}` is not entry_feed | plunge_feed | ramp_angle | helix_diameter"),
        })?;
        let limit = num(r, "limit").ok_or_else(|| LibError::Missing {
            record: rec.clone(),
            field: "limit",
            fix: "give `limit`".into(),
        })?;
        let source = text(r, "source").unwrap_or("").to_string();
        if source.is_empty() {
            return Err(LibError::Missing {
                record: rec,
                field: "source",
                fix: "cite the published guidance".into(),
            });
        }
        let s = |k: &str| text(r, k).unwrap_or("").to_string();
        out.push(Rule {
            check,
            limit,
            groups: texts(r, "groups").unwrap_or_default(),
            taken: s("taken"),
            value: s("value"),
            note: s("note"),
            source,
            id,
        });
    }
    Ok(out)
}

/// The built-in guidance.
pub fn builtin() -> Vec<Rule> {
    read(include_str!("../data/entry.toml"), "builtin/entry.toml").expect("the built-in entry data reads")
}

/// One finding: where, what was measured, the rule it is over.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub op: usize,
    pub path: usize,
    pub mv: usize,
    pub check: Check,
    pub value: f64,
    pub rule: String,
    pub limit: f64,
    pub why: String,
}

/// The rules for `check` that apply to `group`, the strictest first (none: not checked).
pub fn rule_for<'a>(rules: &'a [Rule], check: Check, group: &str) -> Option<&'a Rule> {
    let mut v: Vec<&Rule> = rules.iter().filter(|r| r.check == check && r.applies(group)).collect();
    v.sort_by(|a, b| {
        let (x, y) = if check.max() { (a.limit, b.limit) } else { (b.limit, a.limit) };
        x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
    });
    v.first().copied()
}

/// A finding when `value` is on the wrong side of the rule for `check`.
pub fn judge(rules: &[Rule], check: Check, group: &str, value: f64, at: (usize, usize, usize)) -> Option<Finding> {
    let r = rule_for(rules, check, group)?;
    let over = if check.max() { value > r.limit } else { value < r.limit };
    over.then(|| Finding {
        op: at.0,
        path: at.1,
        mv: at.2,
        check,
        value,
        rule: r.id.clone(),
        limit: r.limit,
        why: format!(
            "{}: {:.3} against {} {} ({}): {}",
            check.name(),
            value,
            if check.max() { "at most" } else { "at least" },
            r.limit,
            r.source,
            r.value
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_reads_and_groups_apply() {
        let r = builtin();
        assert!(r.iter().all(|x| !x.source.is_empty() && !x.value.is_empty()));
        assert_eq!(rule_for(&r, Check::RampAngle, "P").unwrap().limit, 3.0);
        assert_eq!(rule_for(&r, Check::RampAngle, "N").unwrap().limit, 10.0);
        // titanium: neither class of the source
        assert!(rule_for(&r, Check::RampAngle, "S").is_none());
        assert!(judge(&r, Check::EntryFeed, "S", 0.9, (0, 0, 3)).is_some());
        assert!(judge(&r, Check::EntryFeed, "S", 0.5, (0, 0, 3)).is_none());
        assert!(judge(&r, Check::HelixDiameter, "N", 1.05, (0, 0, 1)).is_some());
    }
}
