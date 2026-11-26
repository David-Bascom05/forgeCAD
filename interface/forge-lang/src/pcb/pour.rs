//! Solid pours share ForgeCAD's planar Boolean and constrained triangulation kernels.
//! Round clearance envelopes circumscribe copper. Their small outward grid allowance
//! keeps a snapped fill from rounding across the requested clearance.
use super::*;
use crate::electronics::{Fault, Result};
use forge_cam_geom::{
    boolean::{Op, boolean},
    region::{Loop, Region, Seg, p2},
};
use forge_math::Point2;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Zone {
    pub layer: String,
    pub net: String,
    pub boundary: Vec<Point>,
    pub gap: i64,
    pub priority: i32,
    pub thermal: Option<(i64, i64)>,
    pub span: Span,
}
#[derive(Debug)]
pub struct Island {
    pub layer: String,
    pub net: String,
    pub triangles: Vec<Vec<Point>>,
    pub span: Span,
}

fn signed(poly: &[Point]) -> i128 {
    geometry::sides(poly)
        .map(|[a, b]| i128::from(a[0]) * i128::from(b[1]) - i128::from(a[1]) * i128::from(b[0]))
        .sum()
}
fn region(ps: &[Point]) -> Region {
    let pt = |p: Point| p2(p[0] as f64 / 1e6, p[1] as f64 / 1e6);
    let l = Loop {
        segs: geometry::sides(ps)
            .map(|[a, b]| Seg::Line { a: pt(a), b: pt(b) })
            .collect(),
    };
    Region {
        loops: vec![if signed(ps) < 0 { l.reversed() } else { l }],
    }
}
fn combine(a: Region, b: &Region, op: Op, span: Span) -> Result<Region> {
    if a.loops.is_empty() {
        return Ok(if op == Op::Union { b.clone() } else { a });
    }
    if b.loops.is_empty() {
        return Ok(if op == Op::Inter { Region::default() } else { a });
    }
    let loops = boolean(&a, b, op).map_err(|e| {
        Fault::new(
            "ZoneBoolean",
            span,
            format!("cannot resolve zone boundary: {e:?}"),
        )
    })?;
    Ok(Region {
        loops: loops.into_iter().map(|l| Loop { segs: l.segs }).collect(),
    })
}
fn cross(a: Point, b: Point, c: Point) -> i128 {
    i128::from(b[0] - a[0]) * i128::from(c[1] - a[1]) - i128::from(b[1] - a[1]) * i128::from(c[0] - a[0])
}
fn capsule([a, b]: [Point; 2], radius: f64) -> Region {
    // Bound the radial overcut to about 2 µm for ordinary PCB features.
    let count = ((std::f64::consts::PI / forge_math::dmath::acos(radius / (radius + 2000.))).ceil() as usize)
        .clamp(16, 512);
    let r = (radius + 3.) / forge_math::dmath::cos(std::f64::consts::PI / count as f64);
    let mut cloud = Vec::with_capacity(2 * count);
    for i in 0..count {
        let (s, c) = forge_math::dmath::sin_cos(std::f64::consts::TAU * i as f64 / count as f64);
        for p in [a, b] {
            cloud.push([p[0] + (r * c).round() as i64, p[1] + (r * s).round() as i64]);
        }
    }
    cloud.sort_unstable();
    cloud.dedup();
    let chain = |ps: Vec<Point>| {
        let mut h = Vec::new();
        for p in ps {
            while h.len() >= 2 && cross(h[h.len() - 2], h[h.len() - 1], p) <= 0 {
                h.pop();
            }
            h.push(p);
        }
        h.pop();
        h
    };
    let mut h = chain(cloud.clone());
    cloud.reverse();
    h.extend(chain(cloud));
    region(&h)
}
fn subtract(mut fill: Region, shape: &Shape, gap: i64, span: Span) -> Result<Region> {
    match shape {
        Shape::Stroke { axis, width } => combine(
            fill,
            &capsule(*axis, *width as f64 / 2. + gap as f64),
            Op::Diff,
            span,
        ),
        Shape::Region(ps) => {
            fill = combine(fill, &region(ps), Op::Diff, span)?;
            for s in geometry::sides(ps) {
                fill = combine(fill, &capsule(s, gap as f64), Op::Diff, span)?;
            }
            Ok(fill)
        }
    }
}
fn islands(r: &Region, span: Span) -> Result<Vec<Vec<Vec<Point>>>> {
    if r.loops.is_empty() {
        return Ok(Vec::new());
    }
    let mut points = Vec::new();
    let mut loops = Vec::new();
    for l in &r.loops {
        let mut coords: Vec<_> = l
            .segs
            .iter()
            .map(|s| {
                let p = s.start();
                [(p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64]
            })
            .collect();
        coords.dedup();
        if coords.len() > 1 && coords[0] == *coords.last().unwrap() {
            coords.pop();
        }
        if coords.len() < 3 || signed(&coords) == 0 {
            continue;
        }
        let start = points.len() as u32;
        points.extend(coords.iter().map(|p| Point2::new(p[0] as f64, p[1] as f64)));
        loops.push((start..points.len() as u32).collect());
    }
    if loops.is_empty() {
        return Ok(Vec::new());
    }
    let mesh = forge_mesh::cdt::triangulate(&points, &loops, &[]).map_err(|e| {
        Fault::new(
            "ZoneTriangulation",
            span,
            format!("cannot triangulate filled copper: {e:?}"),
        )
    })?;
    let mut neighbors = vec![Vec::new(); mesh.triangles.len()];
    let mut edges = BTreeMap::new();
    for (i, t) in mesh.triangles.iter().enumerate() {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if let Some(j) = edges.insert((a.min(b), a.max(b)), i) {
                neighbors[i].push(j);
                neighbors[j].push(i);
            }
        }
    }
    let mut visited = vec![false; neighbors.len()];
    let mut groups = Vec::new();
    for i in 0..neighbors.len() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let mut todo = vec![i];
        let mut tris = Vec::new();
        while let Some(k) = todo.pop() {
            tris.push(
                mesh.triangles[k]
                    .iter()
                    .map(|&n| {
                        let p = mesh.points[n as usize];
                        [p.x as i64, p.y as i64]
                    })
                    .collect(),
            );
            for &j in &neighbors[k] {
                if !visited[j] {
                    visited[j] = true;
                    todo.push(j);
                }
            }
        }
        groups.push(tris);
    }
    Ok(groups)
}
impl Board {
    pub fn refill(&mut self) -> Result<()> {
        let mut result = BTreeMap::new();
        let mut zones: Vec<_> = self.layout.zones.iter().collect();
        zones.sort_by_key(|(id, z)| (std::cmp::Reverse(z.priority), *id));
        for (k, (id, z)) in zones.iter().enumerate() {
            let gap = z
                .gap
                .max(self.process.as_ref().map_or(0, |p| p.min_spacing_nm))
                .max(self.class_spacing(&z.net));
            let edge_gap = gap.max(self.process.as_ref().and_then(|p| p.min_edge_nm).unwrap_or(0));
            let footprint = Shape::Region(z.boundary.clone());
            let overlaps = |s: &Shape, gap: i64| {
                let [a, b] = footprint.bounds2();
                let [c, d] = s.bounds2();
                (0..2).all(|j| a[j] <= d[j] + 2 * gap + 10000 && c[j] <= b[j] + 2 * gap + 10000)
            };
            let mut fill = combine(region(&z.boundary), &region(&self.outline), Op::Inter, z.span)?;
            for axis in geometry::sides(&self.outline) {
                let border = Shape::Stroke { axis, width: 0 };
                if overlaps(&border, edge_gap) {
                    fill = subtract(fill, &border, edge_gap, z.span)?;
                }
            }
            let mut obstacles: Vec<_> = self
                .base_copper(&z.layer)
                .into_iter()
                .filter(|m| m.net != Some(&z.net))
                .map(|m| (m.shape, gap.max(self.class_spacing(m.net.unwrap()))))
                .collect();
            obstacles.extend(
                self.layout
                    .cutouts
                    .values()
                    .map(|p| (Shape::Region(p.clone()), edge_gap)),
            );
            obstacles.extend(self.layout.holes.values().map(|h| {
                (
                    Shape::Stroke {
                        axis: [h.at, h.to.unwrap_or(h.at)],
                        width: h.drill,
                    },
                    gap.max(
                        self.process
                            .as_ref()
                            .and_then(|p| p.hole_clearance_nm)
                            .unwrap_or(0),
                    ),
                )
            }));
            obstacles.extend(
                self.layout
                    .areas
                    .values()
                    .filter(|a| a.layer == z.layer && a.net.is_none())
                    .map(|a| (Shape::Region(a.polygon.clone()), gap)),
            );
            obstacles.extend(
                zones[..k]
                    .iter()
                    .filter(|(_, other)| other.layer == z.layer && other.net != z.net)
                    .map(|(_, other)| {
                        (
                            Shape::Region(other.boundary.clone()),
                            gap.max(self.class_spacing(&other.net)),
                        )
                    }),
            );
            for (s, gap) in obstacles {
                if overlaps(&s, gap) {
                    fill = subtract(fill, &s, gap, z.span)?;
                }
            }
            if let Some((relief, width)) = z.thermal {
                // Restrict every spoke to legal copper already present after all foreign clearances.
                for pad in self
                    .layout
                    .pads
                    .values()
                    .filter(|p| p.net == z.net && p.drill.is_some() && overlaps(&p.copper, relief))
                {
                    let original = fill;
                    fill = subtract(original.clone(), &pad.copper, relief, z.span)?;
                    let [lo, hi] = pad.copper.bounds2();
                    let [x, y] = pad.at;
                    let below = width / 2;
                    let above = width - below;
                    let reach = relief + 10_000;
                    for [x0, y0, x1, y1] in [
                        [lo[0] / 2 - reach, y - below, hi[0] / 2 + reach, y + above],
                        [x - below, lo[1] / 2 - reach, x + above, hi[1] / 2 + reach],
                    ] {
                        let strip = region(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
                        let bridge = combine(original.clone(), &strip, Op::Inter, z.span)?;
                        fill = combine(fill, &bridge, Op::Union, z.span)?;
                    }
                }
            }
            for (index, triangles) in islands(&fill, z.span)?.into_iter().enumerate() {
                result.insert(
                    format!("{id}#{index}"),
                    Island {
                        layer: z.layer.clone(),
                        net: z.net.clone(),
                        triangles,
                        span: z.span,
                    },
                );
            }
        }
        self.layout.fills = result;
        Ok(())
    }
}
