//! Sweep copper envelopes in layer order. Twice-coordinate bounds keep odd widths
//! exact. Only overlapping expanded bounds reach the segment distance predicate.
use super::geometry::Segment;
use super::*;

struct Land<'a> {
    object: &'a str,
    net: &'a str,
    span: Span,
    segment: Option<usize>,
    axis: Segment,
    width: i64,
    lo: Point,
    hi: Point,
}

impl<'a> Land<'a> {
    fn new(
        object: &'a str,
        net: &'a str,
        span: Span,
        segment: Option<usize>,
        axis: Segment,
        width: i64,
        gap: i64,
    ) -> Self {
        let [a, b] = axis;
        let lo = std::array::from_fn(|k| 2 * a[k].min(b[k]) - width - gap);
        let hi = std::array::from_fn(|k| 2 * a[k].max(b[k]) + width + gap);
        Self {
            object,
            net,
            span,
            segment,
            axis,
            width,
            lo,
            hi,
        }
    }
}

pub(super) fn check<'a>(board: &'a Board, gap: i64, report: &mut Check<'a>) {
    for layer in board
        .layers
        .iter()
        .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }))
    {
        let mut lands = Vec::new();
        for (id, t) in board.tracks.iter().filter(|(_, t)| t.layer == layer.id) {
            for (i, p) in t.points.windows(2).enumerate() {
                lands.push(Land::new(
                    id,
                    &t.net,
                    t.span,
                    Some(i),
                    [p[0], p[1]],
                    t.width_nm,
                    gap,
                ));
            }
        }
        for (id, v) in &board.vias {
            lands.push(Land::new(
                id,
                &v.net,
                v.span,
                None,
                [v.at, v.at],
                v.diameter_nm,
                gap,
            ));
        }
        lands.sort_by_key(|l| l.lo[0]);
        for (i, a) in lands.iter().enumerate() {
            for b in &lands[i + 1..] {
                if b.lo[0] >= a.hi[0] {
                    break;
                }
                if a.net == b.net || a.hi[1] <= b.lo[1] || b.hi[1] <= a.lo[1] {
                    continue;
                }
                if geometry::segments_too_close(a.axis, b.axis, a.width + b.width + 2 * gap) {
                    report.findings.push(Finding {
                        rule: Rule::MinCopperSpacing,
                        object: a.object,
                        span: a.span,
                        witness: Witness::Spacing {
                            other: b.object,
                            layer: &layer.id,
                            segment: a.segment,
                            other_segment: b.segment,
                            minimum_nm: gap,
                        },
                    });
                }
            }
        }
    }
}
