//! A small shaded picture of a part: orthographic, z-buffered, edges and outline drawn, handles
//! labelled.

use forge_math::{Point3, Vec3};
use forge_mesh::TriMesh;
use forge_topo::Body;

const BG: u8 = 0;
pub const INK: u8 = 1;
const LABEL: u8 = 2;
const SHADES: u8 = 240;
const FIRST_SHADE: u8 = 3;

pub const VIEWS: [&str; 7] = ["iso", "front", "top", "right", "back", "left", "bottom"];

/// The direction toward the camera and the screen's up.
fn camera(view: &str) -> Option<(Vec3, Vec3)> {
    let z = Vec3::Z;
    Some(match view {
        "iso" => (Vec3::new(1.0, -1.0, 1.0), z),
        "front" => (Vec3::new(0.0, -1.0, 0.0), z),
        "back" => (Vec3::Y, z),
        "right" => (Vec3::X, z),
        "left" => (Vec3::new(-1.0, 0.0, 0.0), z),
        "top" => (Vec3::Z, Vec3::Y),
        "bottom" => (Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
        _ => return None,
    })
}

/// 5x7 glyphs, rows top down, for the hex digits of handles (drawn upper case).
fn glyph(c: char) -> Option<[u8; 7]> {
    let i = c.to_digit(16)? as usize;
    const G: [[u8; 7]; 16] = [
        [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
    ];
    Some(G[i])
}

/// A PNG at most `size` pixels on its long side, each label drawn at its point unless the part
/// hides it.
pub fn render(
    body: &Body,
    mesh: &TriMesh,
    view: &str,
    size: usize,
    labels: &[(String, Point3)],
) -> Result<Vec<u8>, String> {
    let p = picture(body, mesh, view, size, labels)?;
    Ok(crate::png::encode(p.w, p.h, &p.palette, &p.px))
}

/// A drawn picture: palette indices, row by row.
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
    pub palette: Vec<[u8; 3]>,
    /// Where a point lands on the picture, and its depth (larger is nearer).
    pub to_px: Box<dyn Fn(Point3) -> (f64, f64, f64)>,
}

impl Picture {
    /// Whether this pixel is an edge or outline.
    pub fn ink(&self, x: usize, y: usize) -> bool {
        self.px[y * self.w + x] == INK
    }
}

/// What the z-buffer keeps per pixel: the nearest surface's depth there, its face, and how its
/// depth changes per pixel across and down, so its plane can be followed off the pixel centre.
#[derive(Clone, Copy)]
struct Near {
    z: f64,
    face: u32,
    dz: (f64, f64),
}

const NO_FACE: u32 = u32::MAX;

/// The picture `render` encodes.
pub fn picture(
    body: &Body,
    mesh: &TriMesh,
    view: &str,
    size: usize,
    labels: &[(String, Point3)],
) -> Result<Picture, String> {
    let (eye, up) = camera(view).ok_or_else(|| format!("view is one of {}", VIEWS.join(" ")))?;
    let e = eye.normalize().ok_or("eye")?.get();
    let u = up.cross(e).normalize().ok_or("up")?.get();
    let v = e.cross(u);
    let proj = move |p: Point3| {
        let d = p - Point3::ORIGIN;
        (d.dot(u), d.dot(v), d.dot(e))
    };
    let pts: Vec<(f64, f64, f64)> = mesh.positions.iter().map(|&p| proj(p)).collect();
    if pts.is_empty() {
        return Err("nothing to draw".into());
    }
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    let (mut zlo, mut zhi) = (f64::MAX, f64::MIN);
    for p in &pts {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
        zlo = zlo.min(p.2);
        zhi = zhi.max(p.2);
    }
    let size = size.clamp(32, 4096);
    let margin = (size / 20) as f64;
    let span = (hi.0 - lo.0).max(hi.1 - lo.1).max(1e-9);
    let s = (size as f64 - 2.0 * margin) / span;
    let w = ((hi.0 - lo.0) * s + 2.0 * margin).ceil() as usize;
    let h = ((hi.1 - lo.1) * s + 2.0 * margin).ceil() as usize;
    let screen = move |p: (f64, f64, f64)| ((p.0 - lo.0) * s + margin, (hi.1 - p.1) * s + margin, p.2);
    // depths that differ by less than rounding in the scene's own size are the same depth: a
    // face flush with another (a PEM head in its sheet) keeps whichever was drawn first
    let same = 1e-9 * (span + (zhi - zlo).abs() + zhi.abs().max(zlo.abs()));
    let mut px = vec![BG; w * h];
    let mut near = vec![
        Near {
            z: f64::NEG_INFINITY,
            face: NO_FACE,
            dz: (0.0, 0.0),
        };
        w * h
    ];
    let light = (e + v * 0.6 + u * 0.3).normalize().ok_or("light")?.get();
    // vertex normals averaged over one face's triangles only, so curved faces shade smoothly and edges stay sharp
    let mut smooth: std::collections::HashMap<(u32, u32), Vec3> = std::collections::HashMap::new();
    for (k, t) in mesh.triangles.iter().enumerate() {
        let n = mesh.triangle_normal(*t);
        for i in t {
            *smooth.entry((*i, mesh.face_of[k])).or_insert(Vec3::ZERO) += n;
        }
    }
    for (k, t) in mesh.triangles.iter().enumerate() {
        let n = match mesh.triangle_normal(*t).normalize() {
            Some(n) => n.get(),
            None => continue,
        };
        if n.dot(e) <= 0.0 {
            continue;
        }
        let face = mesh.face_of[k];
        let ns = t.map(|i| smooth[&(i, face)].normalize().map_or(n, |m| m.get()));
        let [a, b, c] = t.map(|i| screen(pts[i as usize]));
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        if area.abs() < 1e-12 {
            continue;
        }
        let dz = (
            ((b.2 - a.2) * (c.1 - a.1) - (c.2 - a.2) * (b.1 - a.1)) / area,
            ((c.2 - a.2) * (b.0 - a.0) - (b.2 - a.2) * (c.0 - a.0)) / area,
        );
        let x0 = a.0.min(b.0).min(c.0).floor().max(0.0) as usize;
        let x1 = (a.0.max(b.0).max(c.0).ceil() as usize).min(w - 1);
        let y0 = a.1.min(b.1).min(c.1).floor().max(0.0) as usize;
        let y1 = (a.1.max(b.1).max(c.1).ceil() as usize).min(h - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px_, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let w0 = ((b.0 - px_) * (c.1 - py) - (b.1 - py) * (c.0 - px_)) / area;
                let w1 = ((c.0 - px_) * (a.1 - py) - (c.1 - py) * (a.0 - px_)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                let k = y * w + x;
                if z > near[k].z + same {
                    near[k] = Near { z, face, dz };
                    let m = ns[0] * w0 + ns[1] * w1 + ns[2] * w2;
                    let l = m.normalize().map_or(n, |m| m.get()).dot(light).max(0.0);
                    px[k] = FIRST_SHADE + ((0.15 + 0.85 * l) * (SHADES - 1) as f64).round() as u8;
                }
            }
        }
    }
    // A point at (x, y, z) on screen shows if nothing nearer covers the pixel it lands on: the
    // surface there, its plane followed to the point, is not in front of it. The surface is
    // exact (an f64 z-buffer, no bias), so a sheet 0.1 mm thick hides what is behind it at any
    // scale. A point of an edge also shows where one of its own faces is the surface on a pixel
    // next to it, within a pixel of depth: an edge lies on the boundary of its faces, where pixel
    // centres fall either side of it, and a curved face's facets stray from the edge.
    let pixel = 1.0 / s;
    let plane_at = |k: usize, x: f64, y: f64| {
        let n = &near[k];
        let (cx, cy) = ((k % w) as f64 + 0.5, (k / w) as f64 + 0.5);
        n.z + n.dz.0 * (x - cx) + n.dz.1 * (y - cy)
    };
    let shows = |x: f64, y: f64, z: f64, own: &[u32], reach: f64| {
        if x < 0.0 || y < 0.0 || x >= w as f64 || y >= h as f64 {
            return false;
        }
        let (xi, yi) = (x as isize, y as isize);
        let k = yi as usize * w + xi as usize;
        if near[k].face == NO_FACE || z >= plane_at(k, x, y) - same {
            return true;
        }
        (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                let (xx, yy) = (xi + dx, yi + dy);
                if xx < 0 || yy < 0 || xx as usize >= w || yy as usize >= h {
                    return false;
                }
                let k = yy as usize * w + xx as usize;
                own.contains(&near[k].face) && z >= plane_at(k, x, y) - reach
            })
        })
    };
    // outline where the part meets the background
    let solid: Vec<bool> = px.iter().map(|&c| c != BG).collect();
    for y in 0..h {
        for x in 0..w {
            let k = y * w + x;
            if solid[k]
                && (x == 0
                    || y == 0
                    || x + 1 == w
                    || y + 1 == h
                    || !solid[k - 1]
                    || !solid[k + 1]
                    || !solid[k - w]
                    || !solid[k + w])
            {
                px[k] = INK;
            }
        }
    }
    for ed in body.edge_ids() {
        let own: Vec<u32> = body
            .edge(ed)
            .coedges
            .iter()
            .map(|&c| body.coedge_face(c).0)
            .collect();
        let p: Vec<_> = body
            // a helix or blend edge can wind round many times
            .edge_points(
                ed,
                if matches!(
                    body.edge(ed).curve,
                    forge_geom::Curve::Line(_) | forge_geom::Curve::Circle(_)
                ) {
                    24
                } else {
                    400
                },
            )
            .into_iter()
            .map(|q| screen(proj(q)))
            .collect();
        for seg in p.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as usize).max(1);
            for i in 0..=n {
                let t = i as f64 / n as f64;
                let (x, y, z) = (
                    a.0 + (b.0 - a.0) * t,
                    a.1 + (b.1 - a.1) * t,
                    a.2 + (b.2 - a.2) * t,
                );
                if shows(x, y, z, &own, pixel) {
                    px[y as usize * w + x as usize] = INK;
                }
            }
        }
    }
    let mut boxes: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (text, p) in labels {
        let (x, y, z) = screen(proj(*p));
        if !shows(x, y, z, &[], 0.0) && !shows(x, y, z + 2.0 * pixel, &[], 0.0) {
            continue;
        }
        let colour = LABEL;
        let digits: Vec<[u8; 7]> = text.chars().filter_map(glyph).collect();
        let (bw, bh) = (digits.len() * 12 + 2, 18);
        let mut bx = (x as usize + 3).min(w.saturating_sub(bw));
        let mut by = (y as usize).saturating_sub(bh / 2).min(h.saturating_sub(bh));
        for _ in 0..8 {
            if !boxes
                .iter()
                .any(|b| bx < b.0 + b.2 && b.0 < bx + bw && by < b.1 + b.3 && b.1 < by + bh)
            {
                break;
            }
            by = (by + bh).min(h.saturating_sub(bh));
            if by + bh >= h {
                bx = bx.saturating_sub(bw);
            }
        }
        boxes.push((bx, by, bw, bh));
        let mut put = |x: usize, y: usize, c: u8| {
            if x < w && y < h {
                px[y * w + x] = c;
            }
        };
        for dy in 0..3 {
            for dx in 0..3 {
                put(
                    (x as usize + dx).saturating_sub(1),
                    (y as usize + dy).saturating_sub(1),
                    colour,
                );
            }
        }
        for yy in by..by + bh {
            for xx in bx..bx + bw {
                put(xx, yy, BG);
            }
        }
        for (i, g) in digits.iter().enumerate() {
            for (r, row) in g.iter().enumerate() {
                for col in 0..5 {
                    if row >> (4 - col) & 1 == 1 {
                        for (sx, sy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            put(bx + 2 + i * 12 + col * 2 + sx, by + 2 + r * 2 + sy, colour);
                        }
                    }
                }
            }
        }
    }
    let mut palette = vec![[255, 255, 255], [20, 20, 28], [200, 30, 30]];
    for i in 0..SHADES {
        let t = i as f64 / (SHADES - 1) as f64;
        let c = |lo: f64, hi: f64| (lo + (hi - lo) * t).round() as u8;
        palette.push([c(40.0, 205.0), c(60.0, 220.0), c(90.0, 240.0)]);
    }
    Ok(Picture {
        w,
        h,
        px,
        palette,
        to_px: Box::new(move |p| screen(proj(p))),
    })
}
