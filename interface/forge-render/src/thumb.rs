//! Library thumbnails (docs/ux/STANDARD.md §8): a record's real geometry drawn small, from the
//! standard 3/4 view, by a CPU rasteriser. It runs the same on native and wasm (no GPU pass, no
//! readback), takes well under a millisecond for a few thousand triangles at 96 px, and the
//! shell caches the result per record (`shell::ux::thumbnail`).

use crate::frame::{Bounds, Framing, ViewRect, frame};

/// A mesh to draw: positions (mm) and triangles, counter-clockwise from outside.
pub struct Mesh<'a> {
    pub pos: &'a [[f32; 3]],
    pub tri: &'a [[u32; 3]],
    /// Linear RGB base colour, 0..1 (from the record's appearance).
    pub colour: [f32; 3],
}

/// RGBA8, row major, `side`² pixels; transparent where nothing is drawn.
pub struct Image {
    pub side: u32,
    pub rgba: Vec<u8>,
}

/// Draw `meshes` filling `fill` of a `side`-pixel square, from the standard direction, with 2×2
/// supersampling. None when there is nothing to draw.
pub fn render(meshes: &[Mesh], side: u32, fill: f32) -> Option<Image> {
    let bounds = Bounds::of(meshes.iter().flat_map(|m| m.pos.iter().copied()))?;
    let ss = 2u32;
    let n = side * ss;
    let cam = frame(
        bounds,
        ViewRect::full(n as f32, n as f32),
        Framing {
            fill,
            ..Framing::default()
        },
        &[],
    )
    .camera;
    let (f, r, u) = cam.axes();
    let eye = cam.eye();
    let t = (cam.fovy * 0.5).tan();
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // camera space: x right, y up, z forward (depth)
    let to_cam = |p: [f32; 3]| {
        let v = [p[0] - eye[0], p[1] - eye[1], p[2] - eye[2]];
        [dot(v, r), dot(v, u), dot(v, f)]
    };
    let to_px = |c: [f32; 3]| {
        let nx = c[0] / (c[2] * t);
        let ny = c[1] / (c[2] * t);
        [(nx + 1.0) * 0.5 * n as f32, (1.0 - ny) * 0.5 * n as f32, c[2]]
    };
    // key light from upper left, in camera space; a sky/ground ambient
    let key = norm([-0.45, 0.65, -0.6]);
    let mut depth = vec![f32::INFINITY; (n * n) as usize];
    let mut col = vec![[0f32; 4]; (n * n) as usize];
    for m in meshes {
        let cpos: Vec<[f32; 3]> = m.pos.iter().map(|p| to_cam(*p)).collect();
        for tri in m.tri {
            let [a, b, c] = tri.map(|k| cpos.get(k as usize).copied().unwrap_or([0.0, 0.0, -1.0]));
            if a[2] <= 1e-4 || b[2] <= 1e-4 || c[2] <= 1e-4 {
                continue;
            }
            // the normal from world space (camera space is left-handed: x right, y up, z ahead)
            let [wa, wb, wc] = tri.map(|k| m.pos.get(k as usize).copied().unwrap_or([0.0; 3]));
            let wn = norm(cross(sub(wb, wa), sub(wc, wa)));
            let nrm = [dot(wn, r), dot(wn, u), dot(wn, f)];
            // facing the eye: the normal points against the view ray (towards negative z)
            let centre = [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ];
            if dot(nrm, centre) >= 0.0 {
                continue;
            }
            let diffuse = dot(nrm, key).max(0.0);
            let sky = 0.5 + 0.5 * nrm[1];
            let light = 0.28 + 0.12 * sky + 0.72 * diffuse;
            let shade = [0, 1, 2].map(|k| (m.colour[k] * light).min(1.0));
            let [pa, pb, pc] = [to_px(a), to_px(b), to_px(c)];
            raster(pa, pb, pc, n, &mut depth, &mut col, shade);
        }
    }
    // resolve 2×2 to linear → sRGB
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    for y in 0..side {
        for x in 0..side {
            let mut acc = [0f32; 4];
            for dy in 0..ss {
                for dx in 0..ss {
                    let c = col[((y * ss + dy) * n + x * ss + dx) as usize];
                    for k in 0..4 {
                        acc[k] += c[k];
                    }
                }
            }
            let s = (ss * ss) as f32;
            let a = acc[3] / s;
            let o = ((y * side + x) * 4) as usize;
            for k in 0..3 {
                let lin = if acc[3] > 0.0 { acc[k] / acc[3] } else { 0.0 };
                rgba[o + k] = (srgb(lin) * 255.0).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    Some(Image { side, rgba })
}

fn raster(
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    n: u32,
    depth: &mut [f32],
    col: &mut [[f32; 4]],
    shade: [f32; 3],
) {
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area.abs() < 1e-9 {
        return;
    }
    let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i64;
    let x1 = a[0].max(b[0]).max(c[0]).ceil().min(n as f32 - 1.0) as i64;
    let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i64;
    let y1 = a[1].max(b[1]).max(c[1]).ceil().min(n as f32 - 1.0) as i64;
    let edge =
        |p: [f32; 3], q: [f32; 3], x: f32, y: f32| (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            // perspective-correct depth: interpolate 1/z
            let iz = w0 / a[2] + w1 / b[2] + w2 / c[2];
            let z = 1.0 / iz;
            let i = (y as u32 * n + x as u32) as usize;
            if z < depth[i] {
                depth[i] = z;
                col[i] = [shade[0], shade[1], shade[2], 1.0];
            }
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 0.0 { a.map(|v| v / l) } else { a }
}

fn srgb(l: f32) -> f32 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// A box's triangles, for tests and for ghost thumbnails.
pub fn box_mesh(b: Bounds) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let c = b.corners();
    // outward, counter-clockwise from outside
    let quads: [[u32; 4]; 6] = [
        [0, 2, 3, 1], // -z
        [4, 5, 7, 6], // +z
        [0, 1, 5, 4], // -y
        [2, 6, 7, 3], // +y
        [0, 4, 6, 2], // -x
        [1, 3, 7, 5], // +x
    ];
    let tri = quads
        .iter()
        .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
        .collect();
    (c.to_vec(), tri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_fills_the_middle_and_leaves_the_corners_clear() {
        let (pos, tri) = box_mesh(Bounds::new([0.0; 3], [40.0, 20.0, 10.0]));
        let img = render(
            &[Mesh {
                pos: &pos,
                tri: &tri,
                colour: [0.6, 0.6, 0.62],
            }],
            48,
            0.8,
        )
        .unwrap();
        let px = |x: u32, y: u32| &img.rgba[((y * 48 + x) * 4) as usize..][..4];
        assert_eq!(px(24, 24)[3], 255, "centre drawn");
        assert_eq!(px(0, 0)[3], 0, "corner clear");
        // three faces show three different shades
        let mut shades: Vec<u8> = (0..48 * 48)
            .filter(|i| img.rgba[i * 4 + 3] == 255)
            .map(|i| img.rgba[i * 4])
            .collect();
        shades.sort();
        shades.dedup();
        assert!(shades.len() >= 3, "{shades:?}");
        // FORGE_THUMB_DUMP=FILE keeps the picture (raw RGBA, 48×48) for a look
        if let Ok(f) = std::env::var("FORGE_THUMB_DUMP") {
            std::fs::write(f, &img.rgba).unwrap();
        }
    }
}
