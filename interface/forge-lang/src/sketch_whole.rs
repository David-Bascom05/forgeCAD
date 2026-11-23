//! `Sk::whole_curve`, apart (see sketch.rs).
use super::Sk;

impl Sk {
    /// A loop that is one closed curve of the sketch (a closed spline or a whole ellipse), which
    /// a profile hands on cut into pieces (`ProfileLoop::smooth_pieces`: a solid's edge may not
    /// end where it starts), as that one curve again, one segment under its own id, run the way
    /// the loop runs. Sheet metal takes it whole: one source, one trim, one curve on the flat.
    #[inline(never)]
    pub fn whole_curve(&self, l: &forge_ops::ProfileLoop) -> Option<forge_ops::ProfileLoop> {
        const PIECE: u64 = 0xffff << 48;
        let base = l.segments.first()?.1 & !PIECE;
        if l.segments.len() < 2
            || !l
                .segments
                .iter()
                .all(|(g, id)| matches!(g, forge_ops::Segment::Spline(_)) && id & !PIECE == base)
        {
            return None;
        }
        // the sketch's closed curve the loop's pieces lie on: the one starting where the first
        // piece starts (a profile cuts a closed curve at its own interior breaks, so its start is
        // the first piece's), found by the curve, not the id (so the hot `seg_id` keeps its callers)
        let forge_ops::Segment::Spline(ref first) = l.segments[0].0 else {
            return None;
        };
        let at = first.start();
        let c = self
            .ents
            .iter()
            .filter_map(|x| self.s.curve(x.0))
            .find(|c| c.closed() && (c.start() == at || c.end() == at))?;
        let whole = forge_ops::Segment::Spline(c.clone());
        let along: f64 = l.segments.iter().map(|(g, _)| g.area_term()).sum();
        let c = if (whole.area_term() > 0.0) == (along > 0.0) {
            c
        } else {
            std::sync::Arc::new(c.reversed())
        };
        Some(forge_ops::ProfileLoop::new(vec![(
            forge_ops::Segment::Spline(c),
            base,
        )]))
    }
}
