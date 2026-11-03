//! What's drawn over a Forge file's model when the view's switches ask: a ground grid, planes (a
//! sketch's, a work plane's), anchors as small axis triads, and an exploded part's trail. All world
//! triangles, as the origin's are.

use crate::camera::{Camera, along};
use crate::origin::{WorldVert, disc, polyline};

/// A round step (1, 2 or 5 times a power of ten) about a tenth of `span`.
pub fn round_step(span: f32) -> f32 {
    let raw = (span / 10.0).max(1e-3);
    let p = 10f32.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|k| k * p)
        .find(|s| *s >= raw)
        .unwrap_or(10.0 * p)
}

/// The ground grid (z = 0) under what's in view: every tenth-or-so of its size, each fifth line
/// stronger.
pub fn grid(
    center: [f32; 3],
    radius: f32,
    camera: &Camera,
    size: (u32, u32),
    px: f32,
    rgb: [f32; 3],
    out: &mut Vec<WorldVert>,
) {
    let step = round_step(radius * 2.4);
    let n = ((radius * 1.3) / step).ceil() as i32;
    let (cx, cy) = (
        (center[0] / step).round() * step,
        (center[1] / step).round() * step,
    );
    let (lo, hi) = (-n as f32 * step, n as f32 * step);
    for k in -n..=n {
        let at = k as f32 * step;
        let major = ((cx + at) / step).round() as i64 % 5 == 0;
        let a = if major { 0.30 } else { 0.14 };
        let c = [rgb[0], rgb[1], rgb[2], a];
        let w = px * if major { 1.25 } else { 1.0 };
        polyline(
            &[[cx + at, cy + lo, 0.0], [cx + at, cy + hi, 0.0]],
            w,
            c,
            camera,
            size,
            out,
        );
        let major = ((cy + at) / step).round() as i64 % 5 == 0;
        let c = [rgb[0], rgb[1], rgb[2], if major { 0.30 } else { 0.14 }];
        let w = px * if major { 1.25 } else { 1.0 };
        polyline(
            &[[cx + lo, cy + at, 0.0], [cx + hi, cy + at, 0.0]],
            w,
            c,
            camera,
            size,
            out,
        );
    }
}

/// A plane as a translucent square `half` from its origin each way, edged.
#[allow(clippy::too_many_arguments)]
pub fn plane(
    origin: [f32; 3],
    x: [f32; 3],
    y: [f32; 3],
    half: f32,
    rgb: [f32; 3],
    camera: &Camera,
    size: (u32, u32),
    px: f32,
    out: &mut Vec<WorldVert>,
) {
    let at = |a: f32, b: f32| along(along(origin, x, a * half), y, b * half);
    let fill = [rgb[0], rgb[1], rgb[2], 0.12];
    let q = [at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)];
    for k in [0, 1, 2, 0, 2, 3] {
        out.push(WorldVert {
            at: q[k],
            colour: fill,
        });
    }
    let edge = [rgb[0], rgb[1], rgb[2], 0.7];
    polyline(
        &[q[0], q[1], q[2], q[3], q[0]],
        px * 1.25,
        edge,
        camera,
        size,
        out,
    );
}

/// An anchor: red, green, blue strokes `len` long along its x, y, z, and a dot where it is.
#[allow(clippy::too_many_arguments)]
pub fn triad(
    origin: [f32; 3],
    axes: [[f32; 3]; 3],
    len: f32,
    camera: &Camera,
    size: (u32, u32),
    px: f32,
    out: &mut Vec<WorldVert>,
) {
    for (a, hex) in axes.iter().zip([0xE0554A, 0x3FAE6E, 0x5A9BE0]) {
        let rgb = crate::viewcube::rgb(hex);
        polyline(
            &[origin, along(origin, *a, len)],
            px * 2.0,
            [rgb[0], rgb[1], rgb[2], 1.0],
            camera,
            size,
            out,
        );
    }
    let rgb = crate::viewcube::rgb(0xE0A21B);
    disc(origin, px * 5.0, [rgb[0], rgb[1], rgb[2], 1.0], camera, size, out);
}

/// A trail from where a part sits to where the explode put it: a dashed line.
pub fn trail(
    from: [f32; 3],
    to: [f32; 3],
    camera: &Camera,
    size: (u32, u32),
    px: f32,
    rgb: [f32; 3],
    out: &mut Vec<WorldVert>,
) {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len < 1e-3 {
        return;
    }
    let dashes = ((len / 12.0) as usize).clamp(1, 8);
    let c = [rgb[0], rgb[1], rgb[2], 0.8];
    for k in 0..dashes {
        let (a, b) = (k as f32 / dashes as f32, (k as f32 + 0.6) / dashes as f32);
        let p = |t: f32| [from[0] + d[0] * t, from[1] + d[1] * t, from[2] + d[2] * t];
        polyline(&[p(a), p(b)], px * 1.25, c, camera, size, out);
    }
    disc(from, px * 4.0, c, camera, size, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_step_is_round_and_near_a_tenth() {
        assert_eq!(round_step(100.0), 10.0);
        assert_eq!(round_step(1500.0), 200.0);
        assert_eq!(round_step(40.0), 5.0);
        for s in [3.0, 77.0, 999.0, 12345.0] {
            let k = round_step(s);
            assert!(k >= s / 10.0 && k <= s / 2.0, "{s}: {k}");
        }
    }
}
