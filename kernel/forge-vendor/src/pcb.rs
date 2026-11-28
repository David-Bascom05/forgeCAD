//! Rigid PCB service design rules. Lengths use the parent profile's units.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PcbService {
    pub id: String,
    pub source: String,
    pub layers: usize,
    pub outer_oz: f64,
    pub inner_oz: Option<f64>,
    pub min_width: f64,
    pub min_spacing: f64,
    pub min_drill: f64,
    pub min_annular_ring: f64,
    pub min_slot: Option<f64>,
    pub inner_drill_clearance: Option<f64>,
    pub min_edge: Option<f64>,
    pub min_mask_web: Option<f64>,
}

impl PcbService {
    pub(super) fn read(t: &Table) -> Result<Self, ReadError> {
        let mut r = Reader::new(t, "pcb");
        let mut positive = |k| {
            r.num(k)?
                .filter(|n| n.is_finite() && *n > 0.)
                .ok_or_else(|| bad(format!("pcb.{k} needs a positive number")))
        };
        let count = positive("layers")?;
        if count.fract() != 0. || count > 256. {
            return Err(bad("pcb.layers needs an integer in 1..256"));
        }
        let outer_oz = positive("outer_oz")?;
        let min_width = positive("min_width")?;
        let min_spacing = positive("min_spacing")?;
        let min_drill = positive("min_drill")?;
        let min_annular_ring = positive("min_annular_ring")?;
        let inner_oz = if count > 2. {
            Some(positive("inner_oz")?)
        } else {
            None
        };
        let id = r
            .str("id")?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad("pcb.id is missing"))?;
        let source = r
            .str("source")?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad("pcb.source is missing"))?;
        let result = Self {
            id,
            source,
            layers: count as usize,
            outer_oz,
            inner_oz,
            min_width,
            min_spacing,
            min_drill,
            min_annular_ring,
            min_slot: r.pos("min_slot")?,
            inner_drill_clearance: r.pos("inner_drill_clearance")?,
            min_edge: r.pos("min_edge")?,
            min_mask_web: r.pos("min_mask_web")?,
        };
        r.done()?;
        Ok(result)
    }
    pub(super) fn table(&self) -> Table {
        let mut t = Table::default();
        t.set("id", Item::Str(self.id.clone()));
        t.set("source", Item::Str(self.source.clone()));
        for (key, n) in [
            ("layers", self.layers as f64),
            ("outer_oz", self.outer_oz),
            ("min_width", self.min_width),
            ("min_spacing", self.min_spacing),
            ("min_drill", self.min_drill),
            ("min_annular_ring", self.min_annular_ring),
        ] {
            t.set(key, Item::Num(n));
        }
        for (key, n) in [
            ("min_slot", self.min_slot),
            ("inner_drill_clearance", self.inner_drill_clearance),
            ("inner_oz", self.inner_oz),
            ("min_edge", self.min_edge),
            ("min_mask_web", self.min_mask_web),
        ] {
            if let Some(n) = n {
                t.set(key, Item::Num(n));
            }
        }
        t
    }
}
