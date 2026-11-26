//! Exact local checks on the supported rigid-board primitives.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    CopperOutsideEdge,
    MinTrackWidth,
    MinCopperSpacing,
    MinViaDrill,
    MinAnnularRing,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Self::CopperOutsideEdge => "CopperOutsideEdge",
            Self::MinCopperSpacing => "MinCopperSpacing",
            Self::MinTrackWidth => "MinTrackWidth",
            Self::MinViaDrill => "MinViaDrill",
            Self::MinAnnularRing => "MinAnnularRing",
        }
    }
}

#[derive(Debug)]
pub enum Witness<'a> {
    Spacing {
        other: &'a str,
        layer: &'a str,
        segment: Option<usize>,
        other_segment: Option<usize>,
        minimum_nm: i64,
    },
    Outside {
        segment: Option<usize>,
    },
    /// Twice the nanometre length retains half-nanometre rings on odd diameter differences.
    BelowMinimum {
        twice_measured_nm: i64,
        minimum_nm: i64,
    },
}

#[derive(Debug)]
pub struct Finding<'a> {
    pub rule: Rule,
    pub object: &'a str,
    pub span: Span,
    pub witness: Witness<'a>,
}

#[derive(Debug)]
pub struct Check<'a> {
    pub rules: Vec<Rule>,
    pub findings: Vec<Finding<'a>>,
}

impl<'a> Check<'a> {
    fn minimum(&mut self, object: &'a str, span: Span, rule: Rule, twice_measured_nm: i64, minimum_nm: i64) {
        if twice_measured_nm < 2 * minimum_nm {
            self.findings.push(Finding {
                rule,
                object,
                span,
                witness: Witness::BelowMinimum {
                    twice_measured_nm,
                    minimum_nm,
                },
            });
        }
    }

    fn outside(&mut self, object: &'a str, span: Span, segment: Option<usize>) {
        self.findings.push(Finding {
            rule: Rule::CopperOutsideEdge,
            object,
            span,
            witness: Witness::Outside { segment },
        });
    }
}

impl Board {
    pub fn check(&self) -> Check<'_> {
        let mut report = Check {
            rules: vec![Rule::CopperOutsideEdge],
            findings: Vec::new(),
        };
        if let Some(p) = &self.process {
            report.rules.push(Rule::MinCopperSpacing);
            super::spacing::check(self, p.min_spacing_nm, &mut report);
            report
                .rules
                .extend([Rule::MinTrackWidth, Rule::MinViaDrill, Rule::MinAnnularRing]);
        }
        for (id, t) in &self.tracks {
            if let Some(p) = &self.process {
                report.minimum(id, t.span, Rule::MinTrackWidth, 2 * t.width_nm, p.min_width_nm);
            }
            for (i, pair) in t.points.windows(2).enumerate() {
                if !geometry::contains_copper(&self.outline, [pair[0], pair[1]], t.width_nm) {
                    report.outside(id, t.span, Some(i));
                }
            }
        }
        for (id, v) in &self.vias {
            if !geometry::contains_copper(&self.outline, [v.at, v.at], v.diameter_nm) {
                report.outside(id, v.span, None);
            }
            if let Some(p) = &self.process {
                report.minimum(id, v.span, Rule::MinViaDrill, 2 * v.drill_nm, p.min_drill_nm);
                report.minimum(
                    id,
                    v.span,
                    Rule::MinAnnularRing,
                    v.diameter_nm - v.drill_nm,
                    p.min_annular_ring_nm,
                );
            }
        }
        report
    }
}
