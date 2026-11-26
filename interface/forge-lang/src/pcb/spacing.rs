//! One per-layer broad phase for electrical contact, copper spacing and film checks.
use super::{artwork::Mark, *};
use std::collections::{BTreeMap, BTreeSet};

fn pairs(items: &[Mark<'_>], gap: i64, mut visit: impl FnMut(usize, usize)) {
    let mut bounds: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, m)| (m.shape.bounds2(), i))
        .collect();
    bounds.sort_by_key(|(b, _)| b[0][0]);
    for (k, (a, i)) in bounds.iter().enumerate() {
        for (b, j) in &bounds[k + 1..] {
            if b[0][0] > a[1][0] + 2 * gap {
                break;
            }
            if a[0][1] <= b[1][1] + 2 * gap && b[0][1] <= a[1][1] + 2 * gap {
                visit(*i, *j);
            }
        }
    }
}
fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}
pub(super) type Islands<'a> = BTreeMap<&'a str, Vec<Vec<&'a str>>>;

pub(super) fn inspect<'a>(board: &'a Board, report: &mut Check<'a>) -> Islands<'a> {
    let process_gap = board.process.as_ref().map_or(0, |p| p.min_spacing_nm);
    let broad_gap = board
        .classes
        .values()
        .map(|c| c.min_spacing_nm)
        .max()
        .unwrap_or(0)
        .max(process_gap);
    let mut ids = BTreeMap::new();
    let mut parents = Vec::new();
    let mut nets = BTreeMap::new();
    for layer in board
        .layers
        .iter()
        .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }))
    {
        let items = board.copper(&layer.id);
        for a in &items {
            ids.entry(a.id).or_insert_with(|| {
                let n = parents.len();
                parents.push(n);
                n
            });
            nets.insert(a.id, a.net.unwrap());
            if (board.layout.pads.contains_key(a.id) || board.layout.areas.contains_key(a.id))
                && !a.shape.within(&board.outline)
            {
                report.outside(a.id, a.span, a.segment);
            }
            if let Some(gap) = board.process.as_ref().and_then(|p| p.min_edge_nm)
                && geometry::sides(&board.outline)
                    .any(|axis| a.shape.near(&Shape::Stroke { axis, width: 0 }, gap, false))
            {
                report.pair(Rule::MinEdgeClearance, a, "perimeter", &layer.id);
            }
            for (id, cut) in &board.layout.cutouts {
                if a.shape.near(
                    &Shape::Region(cut.clone()),
                    board.process.as_ref().and_then(|p| p.min_edge_nm).unwrap_or(0),
                    board.process.as_ref().and_then(|p| p.min_edge_nm).is_none(),
                ) {
                    report.pair(Rule::CopperInCutout, a, id, &layer.id);
                }
            }
            for (id, h) in &board.layout.holes {
                if a.shape.near(
                    &Shape::Stroke {
                        axis: [h.at, h.to.unwrap_or(h.at)],
                        width: h.drill,
                    },
                    board
                        .process
                        .as_ref()
                        .and_then(|p| p.hole_clearance_nm)
                        .unwrap_or(0),
                    board.process.as_ref().and_then(|p| p.hole_clearance_nm).is_none(),
                ) {
                    report.pair(Rule::CopperAtHole, a, id, &layer.id);
                }
            }
            for (id, area) in &board.layout.areas {
                if area.layer == layer.id
                    && area.net.is_none()
                    && a.shape.near(&Shape::Region(area.polygon.clone()), 0, true)
                {
                    report.pair(Rule::CopperInKeepout, a, id, &layer.id);
                }
            }
        }
        pairs(&items, broad_gap, |i, j| {
            let (a, b) = (&items[i], &items[j]);
            if a.id == b.id {
                return;
            }
            let gap = process_gap
                .max(board.class_spacing(a.net.unwrap()))
                .max(board.class_spacing(b.net.unwrap()));
            let contact = a.shape.near(&b.shape, 0, true);
            if a.net == b.net {
                if contact {
                    let left = root(&mut parents, ids[a.id]);
                    let right = root(&mut parents, ids[b.id]);
                    parents[right] = left;
                }
            } else if contact || (gap > 0 && a.shape.near(&b.shape, gap, false)) {
                if contact {
                    report.pair(Rule::ShortCircuit, a, b.id, &layer.id);
                }
                if gap > 0 {
                    report.spacing(Rule::MinCopperSpacing, a, b, &layer.id, gap);
                }
            }
        });
    }
    let mut groups: BTreeMap<_, BTreeMap<_, Vec<_>>> = BTreeMap::new();
    for (id, index) in ids {
        groups
            .entry(nets[id])
            .or_default()
            .entry(root(&mut parents, index))
            .or_default()
            .push(id);
    }
    let mut result = BTreeMap::new();
    for (net, group) in groups {
        let mut islands: Vec<_> = group.into_values().collect();
        islands.sort();
        let reps: Vec<_> = islands
            .iter()
            .filter_map(|g| g.iter().find(|id| board.layout.pads.contains_key(**id)).copied())
            .collect();
        if let Some((first, rest)) = reps.split_first() {
            for id in rest {
                report.findings.push(Finding {
                    rule: Rule::UnroutedNet,
                    object: id,
                    span: board.layout.pads[*id].span,
                    witness: Witness::Disconnected { other: first, net },
                });
            }
        }
        result.insert(net, islands);
    }
    result
}
pub(super) fn surfaces<'a>(board: &'a Board, report: &mut Check<'a>) {
    for side in Side::BOTH {
        let mut masks = board.film(side, Film::Mask);
        let mask_count = masks.len();
        let mut seen = BTreeSet::new();
        let process = board.process.as_ref();
        let web = process.and_then(|p| p.min_mask_web_nm);
        let clearance = process.and_then(|p| p.silk_clearance_nm);
        masks.extend(board.film(side, Film::Silk));
        if web.is_some() || clearance.is_some() {
            pairs(&masks, web.max(clearance).unwrap_or(0), |i, j| {
                let (rule, gap) = match (i < mask_count, j < mask_count) {
                    (true, true) => (Rule::MinMaskWeb, web),
                    (false, false) => return,
                    _ => (Rule::SilkToMask, clearance),
                };
                let (a, b) = (&masks[i], &masks[j]);
                if let Some(gap) = gap
                    && a.id != b.id
                    && a.shape.near(&b.shape, gap, false)
                    && seen.insert((rule.name(), a.id.min(b.id), a.id.max(b.id)))
                {
                    report.spacing(rule, a, b, side.name(), gap);
                }
            });
        }
        masks.extend(board.film(side, Film::Paste));
        for a in &masks {
            if (!a.shape.within(&board.outline)
                || board
                    .layout
                    .cutouts
                    .values()
                    .any(|c| a.shape.near(&Shape::Region(c.clone()), 0, true)))
                && seen.insert(("outside", a.id, ""))
            {
                report.pair(Rule::ArtworkOutsideBoard, a, "perimeter", side.name());
            }
        }
    }
}

pub(super) fn overlapping_drills<'a>(drills: &[Mark<'a>], report: &mut Check<'a>) {
    pairs(drills, 0, |i, j| {
        if drills[i].shape.near(&drills[j].shape, 0, true) {
            report.pair(Rule::DrillOverlap, &drills[i], drills[j].id, "through");
        }
    });
}
