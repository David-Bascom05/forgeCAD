//! The formed sheet's surface (stage 4, surface): each triangle's principal stretches and their
//! direction on the flat sheet, and distortion printing: the flat sheet's image that forms into
//! an artwork seen along the pull.
use crate::stretch::Stretch;
use forge_math::{Frame, Point3, dmath};

/// Each triangle's principal stretches (l1 >= l2, the thickness ratio is 1 / (l1 l2)) and the
/// direction of l1 on the flat sheet, radians from its x axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stretches {
    pub l1: Vec<f64>,
    pub l2: Vec<f64>,
    pub dir: Vec<f64>,
}

/// The flat sheet (the first step) in the trim plane's frame.
fn flat(st: &Stretch, frame: &Frame) -> Option<Vec<Point3>> {
    let s = st.steps.first()?;
    Some(
        s.nodes
            .iter()
            .map(|p| frame.point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)))
            .collect(),
    )
}

/// The principal stretches of `st`'s triangles (its frame `frame`), from the 3x2 Jacobian of
/// the formed triangle over the flat one.
pub fn stretches(st: &Stretch, frame: &Frame) -> Stretches {
    let Some(x0) = flat(st, frame) else {
        return Stretches::default();
    };
    let mut out = Stretches::default();
    for t in &st.tris {
        let [a, b, c] = t.map(|i| i as usize);
        let (p0, p1, p2) = (x0[a], x0[b], x0[c]);
        let (u1, u2) = ((p1.x - p0.x, p1.y - p0.y), (p2.x - p0.x, p2.y - p0.y));
        let det = u1.0 * u2.1 - u2.0 * u1.1;
        let (q0, q1, q2) = (st.nodes[a], st.nodes[b], st.nodes[c]);
        let (e1, e2) = (q1 - q0, q2 - q0);
        if det.abs() < 1e-12 {
            out.l1.push(1.0);
            out.l2.push(1.0);
            out.dir.push(0.0);
            continue;
        }
        // F = [e1 e2] [u1 u2]^-1, its columns the formed images of the flat x and y
        let fx = (e1 * u2.1 - e2 * u1.1) * (1.0 / det);
        let fy = (e2 * u1.0 - e1 * u2.0) * (1.0 / det);
        // C = F^T F, its eigenvalues the squared stretches
        let (cxx, cxy, cyy) = (fx.dot(fx), fx.dot(fy), fy.dot(fy));
        let half = 0.5 * (cxx - cyy);
        let disc = (half * half + cxy * cxy).sqrt();
        let (s1, s2) = (0.5 * (cxx + cyy) + disc, (0.5 * (cxx + cyy) - disc).max(0.0));
        out.l1.push(s1.sqrt());
        out.l2.push(s2.sqrt());
        out.dir.push(0.5 * dmath::atan2(2.0 * cxy, cxx - cyy));
    }
    out
}

/// A raster of the flat sheet, row 0 at its least y: each pixel's colour, and whether it can be
/// printed (its point reached the mould, or it is outside the artwork).
#[derive(Clone, Debug, PartialEq)]
pub struct Print {
    pub width: usize,
    pub height: usize,
    /// mm a pixel, and the flat sheet's corner (the trim plane's x, y) the raster starts at.
    pub pixel: f64,
    pub corner: (f64, f64),
    pub rgb: Vec<[u8; 3]>,
    /// The pixels whose point never reached the mould (it bridges there): the print lands where
    /// the forming has no say on it.
    pub bridged: Vec<bool>,
}

/// Distortion printing: the flat sheet's image at `pixel` mm that forms into `art`, the artwork
/// as seen along the pull (`art(x, y)` on the trim plane, None outside it; `background` there).
/// Each pixel's point on the flat sheet is carried to where the forming put it (its grid cell's
/// triangle, by barycentric weights) and coloured by the artwork above that point.
pub fn print(
    st: &Stretch,
    frame: &Frame,
    pixel: f64,
    art: &dyn Fn(f64, f64) -> Option<[u8; 3]>,
    background: [u8; 3],
) -> Option<Print> {
    let x0 = flat(st, frame)?;
    let (nx, ny) = st.grid;
    let corner = x0[0];
    let far = x0[(ny + 1) * (nx + 1) - 1];
    let (w, h) = (far.x - corner.x, far.y - corner.y);
    if nx == 0 || ny == 0 || w <= 0.0 || h <= 0.0 || pixel <= 0.0 {
        return None;
    }
    let (sx, sy) = (w / nx as f64, h / ny as f64);
    let (width, height) = ((w / pixel).round() as usize, (h / pixel).round() as usize);
    let formed: Vec<Point3> = st.nodes.iter().map(|p| frame.point_to_local(*p)).collect();
    let id = |i: usize, j: usize| j * (nx + 1) + i;
    let mut rgb = vec![background; width * height];
    let mut bridged = vec![false; width * height];
    for row in 0..height {
        for col in 0..width {
            let (x, y) = (
                corner.x + (col as f64 + 0.5) * pixel,
                corner.y + (row as f64 + 0.5) * pixel,
            );
            let (fi, fj) = ((x - corner.x) / sx, (y - corner.y) / sy);
            let (i, j) = ((fi as usize).min(nx - 1), (fj as usize).min(ny - 1));
            let (u, v) = (fi - i as f64, fj - j as f64);
            // the cell's two triangles, its diagonal alternating as the grid's
            let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
            let (tri, w3) = if (i + j) % 2 == 0 {
                if v <= u {
                    ([a, b, c], [1.0 - u, u - v, v])
                } else {
                    ([a, c, d], [1.0 - v, u, v - u])
                }
            } else if u + v <= 1.0 {
                ([a, b, d], [1.0 - u - v, u, v])
            } else {
                ([b, c, d], [1.0 - v, u + v - 1.0, 1.0 - u])
            };
            let p = tri
                .iter()
                .zip(w3)
                .fold(Point3::ORIGIN, |s, (&k, w)| s + (formed[k] - Point3::ORIGIN) * w);
            let k = row * width + col;
            bridged[k] = tri.iter().any(|&n| !st.touched[n]);
            if let Some(c) = art(p.x, p.y) {
                rgb[k] = c;
            }
        }
    }
    Some(Print {
        width,
        height,
        pixel,
        corner: (corner.x, corner.y),
        rgb,
        bridged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stretch::Step;
    use forge_math::Vec3;

    /// A flat grid of `n` cells a side, `s` mm apart, its triangles as `stretch` lays them.
    fn grid(n: usize, s: f64, map: impl Fn(Point3) -> Point3) -> Stretch {
        let id = |i: usize, j: usize| j * (n + 1) + i;
        let mut flat = vec![];
        for j in 0..=n {
            for i in 0..=n {
                flat.push(Point3::new(i as f64 * s, j as f64 * s, 0.0));
            }
        }
        let mut tris = vec![];
        for j in 0..n {
            for i in 0..n {
                let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
                let v = |x: [usize; 3]| x.map(|k| k as u32);
                if (i + j) % 2 == 0 {
                    tris.push(v([a, b, c]));
                    tris.push(v([a, c, d]));
                } else {
                    tris.push(v([a, b, d]));
                    tris.push(v([b, c, d]));
                }
            }
        }
        let m = flat.len();
        Stretch {
            nodes: flat.iter().map(|p| map(*p)).collect(),
            steps: vec![Step {
                nodes: flat
                    .iter()
                    .map(|p| [p.x as f32, p.y as f32, p.z as f32])
                    .collect(),
                ratio: vec![1.0; m],
                stage: "sheet",
            }],
            grid: (n, n),
            tris,
            ratio: vec![],
            node_ratio: vec![1.0; m],
            touched: vec![true; m],
            min: (1.0, Point3::ORIGIN),
            max: (1.0, Point3::ORIGIN),
            mean: 1.0,
            unformed: vec![],
            iterations: 0,
        }
    }

    #[test]
    fn a_uniform_stretch_is_its_stretches() {
        // 1.5 along x, 1.2 along y, lifted and tilted: every triangle the same
        let st = grid(6, 2.0, |p| {
            let q = Vec3::new(1.5 * p.x, 1.2 * p.y, 0.0);
            Point3::new(q.x, q.y * 0.8, q.y * 0.6 + 3.0)
        });
        let s = stretches(&st, &Frame::WORLD);
        for k in 0..st.tris.len() {
            assert!(
                (s.l1[k] - 1.5).abs() < 1e-9 && (s.l2[k] - 1.2).abs() < 1e-9,
                "{k}"
            );
            assert!(s.dir[k].abs() < 1e-9);
        }
    }

    #[test]
    fn an_unformed_sheet_prints_the_artwork_itself() {
        let st = grid(10, 1.0, |p| p);
        let art = |x: f64, y: f64| Some([(x * 20.0) as u8, (y * 20.0) as u8, 0]);
        let p = print(&st, &Frame::WORLD, 0.25, &art, [255; 3]).unwrap();
        assert_eq!((p.width, p.height), (40, 40));
        for row in 0..p.height {
            for col in 0..p.width {
                let (x, y) = ((col as f64 + 0.5) * 0.25, (row as f64 + 0.5) * 0.25);
                assert_eq!(p.rgb[row * p.width + col], art(x, y).unwrap());
            }
        }
        assert!(p.bridged.iter().all(|b| !b));
    }

    /// A sheet stretched twice along x prints the artwork squeezed to half along x: a flat
    /// point at x lands at 2x and takes the colour there.
    #[test]
    fn a_stretched_sheet_prints_the_artwork_squeezed() {
        let st = grid(10, 1.0, |p| Point3::new(2.0 * p.x, p.y, 0.0));
        let art = |x: f64, _y: f64| Some([if (x as i64) % 2 == 0 { 0 } else { 255 }; 3]);
        let p = print(&st, &Frame::WORLD, 0.25, &art, [9; 3]).unwrap();
        for col in 0..p.width {
            let x = (col as f64 + 0.5) * 0.25;
            assert_eq!(p.rgb[col], art(2.0 * x, 0.0).unwrap(), "col {col}");
        }
    }
}

/// Where the sheet webs (stage 2g): the triangles squeezed across (their lesser stretch under the
/// width uniaxial tension would leave, by `slack`: gathered material, the start of a web), their
/// area on the flat sheet, and the points where the sheet folds onto itself (two points within
/// the sheet's thickness that were more than three apart on the flat sheet).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Webbing {
    pub squeezed: Vec<usize>,
    pub squeezed_area: f64,
    pub folds: Vec<Point3>,
}

pub fn webbing(st: &Stretch, frame: &Frame, t: f64, slack: f64) -> Webbing {
    let Some(x0) = flat(st, frame) else {
        return Webbing::default();
    };
    let sv = stretches(st, frame);
    let mut w = Webbing::default();
    for (k, tri) in st.tris.iter().enumerate() {
        if sv.l2[k] < (1.0 - slack) / sv.l1[k].sqrt() {
            w.squeezed.push(k);
            let [a, b, c] = tri.map(|i| x0[i as usize]);
            w.squeezed_area += 0.5 * (b - a).cross(c - a).length();
        }
    }
    // the sheet on itself: a hash grid of cells t across over the formed nodes
    let key = |p: Point3| {
        (
            (p.x / t).floor() as i64,
            (p.y / t).floor() as i64,
            (p.z / t).floor() as i64,
        )
    };
    let mut cells: std::collections::HashMap<(i64, i64, i64), Vec<usize>> = Default::default();
    for (k, p) in st.nodes.iter().enumerate() {
        cells.entry(key(*p)).or_default().push(k);
    }
    let mut seen = vec![false; st.nodes.len()];
    for (k, p) in st.nodes.iter().enumerate() {
        if seen[k] {
            continue;
        }
        let (x, y, z) = key(*p);
        let mut hit = None;
        'near: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &q in cells.get(&(x + dx, y + dy, z + dz)).into_iter().flatten() {
                        if (st.nodes[q] - *p).length() < t && (x0[q] - x0[k]).length() > 3.0 * t {
                            hit = Some(q);
                            break 'near;
                        }
                    }
                }
            }
        }
        if let Some(q) = hit {
            seen[k] = true;
            seen[q] = true;
            w.folds.push(*p);
        }
    }
    w
}
