//! Derive positive, layer-specific images from the current board. Stencil apertures
//! and mask openings have no electrical identity. Plated lands share one object
//! identity across copper layers, allowing the connectivity checker to join them.
use super::*;
#[derive(Debug)]
pub struct Mark<'a> {
    pub id: &'a str,
    pub net: Option<&'a str>,
    pub span: Span,
    pub segment: Option<usize>,
    pub shape: Shape,
}
impl<'a> Mark<'a> {
    fn new(id: &'a str, net: Option<&'a str>, span: Span, shape: Shape) -> Self {
        Self {
            id,
            net,
            span,
            segment: None,
            shape,
        }
    }
}
impl Board {
    pub(super) fn surface_layer(&self, side: Side) -> Option<&str> {
        let layer = match side {
            Side::Top => self.layers.first(),
            Side::Bottom => self.layers.last(),
        }?;
        matches!(layer.material, LayerMaterial::Copper { .. }).then_some(&layer.id)
    }
    pub(super) fn drills(&self) -> Vec<Mark<'_>> {
        self.vias
            .iter()
            .map(|(id, v)| Mark::new(id, Some(&v.net), v.span, Shape::disk(v.at, v.drill_nm)))
            .chain(self.layout.pads.iter().filter_map(|(id, p)| {
                Some(Mark::new(
                    id,
                    Some(&p.net),
                    p.span,
                    Shape::Stroke {
                        axis: p.drill_axis,
                        width: p.drill?,
                    },
                ))
            }))
            .chain(self.layout.holes.iter().map(|(id, h)| {
                Mark::new(
                    id,
                    None,
                    h.span,
                    Shape::Stroke {
                        axis: [h.at, h.to.unwrap_or(h.at)],
                        width: h.drill,
                    },
                )
            }))
            .collect()
    }
    pub fn copper(&self, layer: &str) -> Vec<Mark<'_>> {
        let mut image = self.base_copper(layer);
        for (id, island) in self.layout.fills.iter().filter(|(_, f)| f.layer == layer) {
            image.extend(
                island
                    .triangles
                    .iter()
                    .map(|t| Mark::new(id, Some(&island.net), island.span, Shape::Region(t.clone()))),
            );
        }
        image
    }
    pub(super) fn base_copper(&self, layer: &str) -> Vec<Mark<'_>> {
        let mut image = Vec::new();
        for (id, t) in self.tracks.iter().filter(|(_, t)| t.layer == layer) {
            for (i, p) in t.points.windows(2).enumerate() {
                image.push(Mark {
                    id,
                    net: Some(&t.net),
                    span: t.span,
                    segment: Some(i),
                    shape: Shape::Stroke {
                        axis: [p[0], p[1]],
                        width: t.width_nm,
                    },
                });
            }
        }
        for (id, v) in &self.vias {
            image.push(Mark::new(
                id,
                Some(&v.net),
                v.span,
                Shape::disk(v.at, v.diameter_nm),
            ));
        }
        for (id, p) in &self.layout.pads {
            if p.drill.is_some() || self.surface_layer(p.side) == Some(layer) {
                image.push(Mark::new(id, Some(&p.net), p.span, p.copper.clone()));
            }
        }
        for (id, a) in &self.layout.areas {
            if a.layer == layer
                && let Some(net) = &a.net
            {
                image.push(Mark::new(id, Some(net), a.span, Shape::Region(a.polygon.clone())));
            }
        }
        image
    }
    pub fn film(&self, side: Side, film: Film) -> Vec<Mark<'_>> {
        let mut marks = Vec::new();
        for (id, p) in &self.layout.pads {
            let shape = match film {
                Film::Mask if p.side == side || p.drill.is_some() => p.mask.as_ref(),
                Film::Paste if p.side == side => p.paste.as_ref(),
                _ => None,
            };
            if let Some(shape) = shape {
                marks.push(Mark::new(id, None, p.span, shape.clone()));
            }
        }
        if film == Film::Mask {
            for (id, v) in &self.vias {
                let m = if side == Side::Top {
                    v.mask_top
                } else {
                    v.mask_bottom
                };
                if let Some(m) = m {
                    marks.push(Mark::new(
                        id,
                        None,
                        v.span,
                        Shape::disk(v.at, v.diameter_nm + 2 * m),
                    ));
                }
            }
        }
        for (id, g) in &self.layout.graphics {
            if g.film == film && g.side == side {
                marks.extend(g.shapes.iter().cloned().map(|s| Mark::new(id, None, g.span, s)));
            }
        }
        marks
    }
}
