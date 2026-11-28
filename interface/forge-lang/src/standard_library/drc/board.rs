//! Standard board rules share copper broad-phase and drill measurements.
use super::*;
pub(crate) fn copper<'a>(board: &'a Board, report: &mut Check<'a>) {
    report.rules.extend([Rule::CopperOutsideEdge, Rule::ShortCircuit]);
    if let Some(p) = &board.process {
        report.rules.push(Rule::MinCopperSpacing);
        if p.min_mask_web_nm.is_some() {
            report.rules.push(Rule::MinMaskWeb);
        }
        if p.silk_clearance_nm.is_some() {
            report.rules.push(Rule::SilkToMask);
        }
    }
    if board.process.is_none() && !board.classes.is_empty() {
        report.rules.push(Rule::MinCopperSpacing);
    }
    if board.process.as_ref().is_some_and(|p| p.min_edge_nm.is_some())
        || board
            .layout
            .layer_graphics
            .values()
            .any(|g| g.layer == PlotLayer::Drawing(DrawingRole::Margin))
    {
        report.rules.push(Rule::MinEdgeClearance);
    }
    if board.layout.placements.values().any(|p| p.courtyard.is_some()) {
        report.rules.push(Rule::CourtyardOverlap);
        let placed: Vec<_> = board
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
    if !board.layout.pads.is_empty() {
        report.rules.push(Rule::UnroutedNet);
    }
    if !board.layout.areas.is_empty() {
        report.rules.push(Rule::CopperInKeepout);
    }
    if !board.layout.cutouts.is_empty() {
        report.rules.push(Rule::CopperInCutout);
    }
    if !board.layout.holes.is_empty() {
        report.rules.extend([Rule::CopperAtHole, Rule::HoleOutsideBoard]);
    }
    if !board.layout.pads.is_empty()
        || !board.layout.graphics.is_empty()
        || board
            .vias
            .values()
            .any(|v| v.mask_top.is_some() || v.mask_bottom.is_some())
    {
        report.rules.push(Rule::ArtworkOutsideBoard);
    }
    spacing::inspect(board, report);
    spacing::surfaces(board, report);
}
pub(crate) fn manufacturing<'a>(board: &'a Board, report: &mut Check<'a>) {
    if board.process.is_some() {
        report
            .rules
            .extend([Rule::MinTrackWidth, Rule::MinViaDrill, Rule::MinAnnularRing]);
    } else if !board.classes.is_empty() {
        report.rules.push(Rule::MinTrackWidth);
    }

    if !board.layout.holes.is_empty() && board.process.is_some() {
        report.rules.push(Rule::MinHoleDrill);
    }
    let drills = board.drills();
    if let Some(gap) = board.process.as_ref().and_then(|p| p.inner_drill_clearance_nm) {
        report.rules.push(Rule::MinInnerDrillClearance);
        for l in board.layers.iter().filter(|l| board.is_inner_copper(&l.id)) {
            let mut items = board.copper(&l.id);
            let boundary = items.len();
            items.extend(board.drills().into_iter().filter(|d| d.net.is_some()));
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
    if let Some(min) = board.process.as_ref().and_then(|p| p.min_slot_nm) {
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
        spacing::overlapping_drills(&drills, report);
    }
    for (id, h) in &board.layout.holes {
        if let Some(p) = &board.process {
            report.minimum(id, h.span, Rule::MinHoleDrill, 2 * h.drill, p.min_drill_nm);
        }
        let bore = Shape::Stroke {
            axis: [h.at, h.to.unwrap_or(h.at)],
            width: h.drill,
        };
        if !bore.within(&board.outline)
            || board
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
    for (id, pad) in &board.layout.pads {
        if let (Some(drill), Some(p)) = (pad.drill, &board.process) {
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
    for (id, t) in &board.tracks {
        let min_width = board
            .classes
            .values()
            .filter(|c| c.nets.contains(&t.net))
            .map(|c| c.min_width_nm)
            .max()
            .unwrap_or(0)
            .max(board.process.as_ref().map_or(0, |p| p.min_width_nm));
        report.minimum(id, t.span, Rule::MinTrackWidth, 2 * t.width_nm, min_width);
        for (i, pair) in t.points.windows(2).enumerate() {
            if !geometry::contains_copper(&board.outline, [pair[0], pair[1]], t.width_nm) {
                report.outside(id, t.span, Some(i));
            }
        }
    }
    for (id, v) in &board.vias {
        if !geometry::contains_copper(&board.outline, [v.at, v.at], v.diameter_nm) {
            report.outside(id, v.span, None);
        }
        if let Some(p) = &board.process {
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
}
