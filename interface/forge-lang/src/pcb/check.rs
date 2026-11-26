//! Exact local checks on the supported rigid-board primitives.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
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
    pub fn name(self) -> &'static str {
        match self {
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
            rules: vec![Rule::CopperOutsideEdge, Rule::ShortCircuit],
            findings: Vec::new(),
        };
        if let Some(p) = &self.process {
            report.rules.push(Rule::MinCopperSpacing);
            if p.min_mask_web_nm.is_some() {
                report.rules.push(Rule::MinMaskWeb);
            }
            if p.silk_clearance_nm.is_some() {
                report.rules.push(Rule::SilkToMask);
            }
            report
                .rules
                .extend([Rule::MinTrackWidth, Rule::MinViaDrill, Rule::MinAnnularRing]);
        }
        if self.process.is_none() && !self.classes.is_empty() {
            report.rules.extend([Rule::MinTrackWidth, Rule::MinCopperSpacing]);
        }
        if self.process.as_ref().is_some_and(|p| p.min_edge_nm.is_some())
            || self
                .layout
                .layer_graphics
                .values()
                .any(|g| g.layer == PlotLayer::Drawing(DrawingRole::Margin))
        {
            report.rules.push(Rule::MinEdgeClearance);
        }
        if self.layout.placements.values().any(|p| p.courtyard.is_some()) {
            report.rules.push(Rule::CourtyardOverlap);
            let placed: Vec<_> = self
                .layout
                .placements
                .iter()
                .filter_map(|(id, p)| Some((id, p, p.courtyard.as_ref()?)))
                .collect();
            for (i, (id, a, shape)) in placed.iter().enumerate() {
                for (other, b, body) in &placed[i + 1..] {
                    if a.pose.side == b.pose.side && shape.near(body, 0, true) {
                        report.findings.push(Finding {
                            rule: Rule::CourtyardOverlap,
                            object: id,
                            span: a.span,
                            witness: Witness::Pair {
                                other,
                                layer: a.pose.side.name(),
                            },
                        });
                    }
                }
            }
        }
        if !self.layout.pads.is_empty() {
            report.rules.push(Rule::UnroutedNet);
        }
        if !self.layout.areas.is_empty() {
            report.rules.push(Rule::CopperInKeepout);
        }
        if !self.layout.cutouts.is_empty() {
            report.rules.push(Rule::CopperInCutout);
        }
        if !self.layout.holes.is_empty() {
            report.rules.extend([Rule::CopperAtHole, Rule::HoleOutsideBoard]);
        }
        if !self.layout.pads.is_empty()
            || !self.layout.graphics.is_empty()
            || self
                .vias
                .values()
                .any(|v| v.mask_top.is_some() || v.mask_bottom.is_some())
        {
            report.rules.push(Rule::ArtworkOutsideBoard);
        }
        spacing::inspect(self, &mut report);
        spacing::surfaces(self, &mut report);
        if !self.layout.holes.is_empty() && self.process.is_some() {
            report.rules.push(Rule::MinHoleDrill);
        }
        let drills = self.drills();
        if let Some(gap) = self.process.as_ref().and_then(|p| p.inner_drill_clearance_nm) {
            report.rules.push(Rule::MinInnerDrillClearance);
            for l in self.layers.iter().filter(|l| self.is_inner_copper(&l.id)) {
                let mut items = self.copper(&l.id);
                let boundary = items.len();
                items.extend(self.drills().into_iter().filter(|d| d.net.is_some()));
                spacing::pairs(&items, gap, |i, j| {
                    let (a, b) = if i < boundary && j >= boundary {
                        (&items[i], &items[j])
                    } else if j < boundary && i >= boundary {
                        (&items[j], &items[i])
                    } else {
                        return;
                    };
                    if a.net != b.net && a.shape.near(&b.shape, gap, false) {
                        report.spacing(Rule::MinInnerDrillClearance, a, b, &l.id, gap);
                    }
                });
            }
        }
        if let Some(min) = self.process.as_ref().and_then(|p| p.min_slot_nm) {
            report.rules.push(Rule::MinSlotWidth);
            for d in &drills {
                if let Shape::Stroke { axis: [a, b], width } = &d.shape
                    && a != b
                {
                    report.minimum(d.id, d.span, Rule::MinSlotWidth, 2 * width, min);
                }
            }
        }
        if drills.len() > 1 {
            report.rules.push(Rule::DrillOverlap);
            spacing::overlapping_drills(&drills, &mut report);
        }
        for (id, h) in &self.layout.holes {
            if let Some(p) = &self.process {
                report.minimum(id, h.span, Rule::MinHoleDrill, 2 * h.drill, p.min_drill_nm);
            }
            let bore = Shape::Stroke {
                axis: [h.at, h.to.unwrap_or(h.at)],
                width: h.drill,
            };
            if !bore.within(&self.outline)
                || self
                    .layout
                    .cutouts
                    .values()
                    .any(|c| bore.near(&Shape::Region(c.clone()), 0, true))
            {
                report.findings.push(Finding {
                    rule: Rule::HoleOutsideBoard,
                    object: id,
                    span: h.span,
                    witness: Witness::Outside { segment: None },
                });
            }
        }
        for (id, pad) in &self.layout.pads {
            if let (Some(drill), Some(p)) = (pad.drill, &self.process) {
                report.minimum(id, pad.span, Rule::MinViaDrill, 2 * drill, p.min_drill_nm);
                report.minimum(
                    id,
                    pad.span,
                    Rule::MinAnnularRing,
                    pad.min_diameter - drill,
                    p.min_annular_ring_nm,
                );
            }
        }
        for (id, t) in &self.tracks {
            let min_width = self
                .classes
                .values()
                .filter(|c| c.nets.contains(&t.net))
                .map(|c| c.min_width_nm)
                .max()
                .unwrap_or(0)
                .max(self.process.as_ref().map_or(0, |p| p.min_width_nm));
            report.minimum(id, t.span, Rule::MinTrackWidth, 2 * t.width_nm, min_width);
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
