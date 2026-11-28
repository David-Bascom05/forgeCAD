//! Exact local checks on the supported rigid-board primitives.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Library(&'static str),
    MinSlotWidth,
    MinInnerDrillClearance,
    MinHoleDrill,
    DrillOverlap,
    CourtyardOverlap,
    MinEdgeClearance,
    ShortCircuit,
    UnroutedNet,
    CopperInCutout,
    CopperAtHole,
    CopperInKeepout,
    MinMaskWeb,
    SilkToMask,
    ArtworkOutsideBoard,
    HoleOutsideBoard,
    CopperOutsideEdge,
    MinTrackWidth,
    MinCopperSpacing,
    MinViaDrill,
    MinAnnularRing,
}

impl Rule {
    pub const ALL: &[Self] = &[
        Self::MinSlotWidth,
        Self::MinInnerDrillClearance,
        Self::MinHoleDrill,
        Self::DrillOverlap,
        Self::CourtyardOverlap,
        Self::MinEdgeClearance,
        Self::ShortCircuit,
        Self::UnroutedNet,
        Self::CopperInCutout,
        Self::CopperAtHole,
        Self::CopperInKeepout,
        Self::MinMaskWeb,
        Self::SilkToMask,
        Self::ArtworkOutsideBoard,
        Self::HoleOutsideBoard,
        Self::CopperOutsideEdge,
        Self::MinTrackWidth,
        Self::MinCopperSpacing,
        Self::MinViaDrill,
        Self::MinAnnularRing,
    ];
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|r| r.name() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Library(id) => id,
            Self::MinSlotWidth => "MinSlotWidth",
            Self::MinInnerDrillClearance => "MinInnerDrillClearance",
            Self::MinHoleDrill => "MinHoleDrill",
            Self::DrillOverlap => "DrillOverlap",
            Self::CourtyardOverlap => "CourtyardOverlap",
            Self::MinEdgeClearance => "MinEdgeClearance",
            Self::ShortCircuit => "ShortCircuit",
            Self::UnroutedNet => "UnroutedNet",
            Self::CopperInCutout => "CopperInCutout",
            Self::CopperAtHole => "CopperAtHole",
            Self::CopperInKeepout => "CopperInKeepout",
            Self::MinMaskWeb => "MinMaskWeb",
            Self::SilkToMask => "SilkToMask",
            Self::ArtworkOutsideBoard => "ArtworkOutsideBoard",
            Self::HoleOutsideBoard => "HoleOutsideBoard",
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
    Library {
        message: String,
        objects: Vec<String>,
    },
    Pair {
        other: &'a str,
        layer: &'a str,
    },
    Disconnected {
        other: &'a str,
        net: &'a str,
    },
    Spacing {
        other: &'a str,
        layer: &'a str,
        segment: Option<usize>,
        other_segment: Option<usize>,
        measured_floor_nm: i64,
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
    pub(super) fn minimum(
        &mut self,
        object: &'a str,
        span: Span,
        rule: Rule,
        twice_measured_nm: i64,
        minimum_nm: i64,
    ) {
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

    pub(super) fn spacing(
        &mut self,
        rule: Rule,
        a: &artwork::Mark<'a>,
        b: &artwork::Mark<'a>,
        layer: &'a str,
        minimum_nm: i64,
    ) {
        // The caller already established the violation. Quantify it using the same
        // exact predicate; odd-width pads and diagonal gaps can fall between grid units.
        let (mut lo, mut hi) = (0, minimum_nm);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if a.shape.near(&b.shape, mid, false) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        self.findings.push(Finding {
            rule,
            object: a.id,
            span: a.span,
            witness: Witness::Spacing {
                other: b.id,
                layer,
                segment: a.segment,
                other_segment: b.segment,
                minimum_nm,
                measured_floor_nm: lo,
            },
        });
    }
    pub(super) fn pair(&mut self, rule: Rule, a: &artwork::Mark<'a>, other: &'a str, layer: &'a str) {
        self.findings.push(Finding {
            rule,
            object: a.id,
            span: a.span,
            witness: Witness::Pair { other, layer },
        });
    }
    pub(super) fn outside(&mut self, object: &'a str, span: Span, segment: Option<usize>) {
        self.findings.push(Finding {
            rule: Rule::CopperOutsideEdge,
            object,
            span,
            witness: Witness::Outside { segment },
        });
    }
}

impl Board {
    pub(super) fn class_spacing(&self, net: &str) -> i64 {
        self.classes
            .values()
            .filter(|c| c.nets.contains(net))
            .map(|c| c.min_spacing_nm)
            .max()
            .unwrap_or(0)
    }
    pub fn check(&self) -> Check<'_> {
        let mut report = Check {
            rules: Vec::new(),
            findings: Vec::new(),
        };
        self.packages.board(self, &mut report);
        report
    }
}
