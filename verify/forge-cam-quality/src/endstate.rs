//! The end state: what a program leaves against the part. The part's top surface (the highest
//! part over each cell of the stock's height map, from its tessellation) against the map after
//! every op has swept it: material left above the part (leftover) and below its top (a gouge,
//! or the two not lined up). The same grid and model for any program, ours or another system's:
//! two programs are compared only when they leave the same part.
use crate::map::HeightMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EndState {
    /// Material above the part's top by more than `tol` (mm³), its largest height (mm), and the
    /// area where it is more than 0.05 mm (mm²).
    pub left_vol: f64,
    pub left_max: f64,
    pub left_area: f64,
    /// Material below the part's top by more than `tol` (mm³) and the deepest (mm).
    pub gouge_vol: f64,
    pub gouge_max: f64,
    pub tol: f64,
}

/// The part (a forge-lang `.fg` file, its imports read beside it) as triangles, shifted by `shift`
/// into the map's frame.
pub fn part_triangles(path: &std::path::Path, shift: [f64; 3]) -> Result<Vec<[[f64; 3]; 3]>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let opts = forge_lang::Options {
        dir: path.parent().map(|p| p.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Default::default());
    let body = run.body.ok_or("the part has no body")?;
    let q = forge_mesh::tessellate::Quality {
        chord: 0.01,
        ..Default::default()
    };
    let mesh = forge_mesh::tessellate::tessellate_tolerant(&body, &q).map_err(|e| format!("{e:?}"))?;
    let p = |i: u32| {
        let v = mesh.positions[i as usize];
        [v.x + shift[0], v.y + shift[1], v.z + shift[2]]
    };
    Ok(mesh
        .triangles
        .iter()
        .map(|t| [p(t[0]), p(t[1]), p(t[2])])
        .collect())
}

/// The triangles' xy box: xmin ymin xmax ymax.
pub fn footprint(tris: &[[[f64; 3]; 3]]) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for t in tris {
        for p in t {
            b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
        }
    }
    b
}

/// The part's top over each cell centre of `m` (−∞: no part there).
pub fn part_tops(tris: &[[[f64; 3]; 3]], m: &HeightMap) -> Vec<f64> {
    let mut top = vec![f64::NEG_INFINITY; m.nx * m.ny];
    for t in tris {
        let [a, b, c] = *t;
        let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        // a wall seen from above covers no cell centre
        if det.abs() < 1e-12 {
            continue;
        }
        let (x0, x1) = (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
        let (y0, y1) = (a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]));
        let i0 = (((x0 - m.x0) / m.h - 0.5).floor().max(0.0)) as usize;
        let j0 = (((y0 - m.y0) / m.h - 0.5).floor().max(0.0)) as usize;
        let i1 = ((((x1 - m.x0) / m.h - 0.5).ceil() + 1.0).max(0.0) as usize).min(m.nx);
        let j1 = ((((y1 - m.y0) / m.h - 0.5).ceil() + 1.0).max(0.0) as usize).min(m.ny);
        for j in j0..j1 {
            let y = m.y0 + (j as f64 + 0.5) * m.h;
            for i in i0..i1 {
                let x = m.x0 + (i as f64 + 0.5) * m.h;
                // barycentric (edges inclusive)
                let u = ((x - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (y - a[1])) / det;
                let v = ((b[0] - a[0]) * (y - a[1]) - (x - a[0]) * (b[1] - a[1])) / det;
                if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
                    continue;
                }
                let z = a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]);
                let k = j * m.nx + i;
                if z > top[k] {
                    top[k] = z;
                }
            }
        }
    }
    top
}

/// The map against the part's tops, over the part's footprint (`foot`: xmin ymin xmax ymax, cell
/// centres within it); cells there with no part are compared with the map's bottom (all of the
/// stock there is to go). Outside it the map may run past the stock by a cell: not counted.
pub fn end_state(m: &HeightMap, tops: &[f64], foot: [f64; 4], tol: f64) -> EndState {
    let mut e = EndState {
        tol,
        ..EndState::default()
    };
    let a = m.h * m.h;
    for (k, &z) in m.z.iter().enumerate() {
        let (x, y) = (
            m.x0 + ((k % m.nx) as f64 + 0.5) * m.h,
            m.y0 + ((k / m.nx) as f64 + 0.5) * m.h,
        );
        if x < foot[0] || y < foot[1] || x > foot[2] || y > foot[3] {
            continue;
        }
        let t = if tops[k].is_finite() { tops[k] } else { m.bottom };
        let d = z - t;
        if std::env::var_os("FORGE_QUALITY_END").is_some() && d.abs() > 0.5 {
            eprintln!("end {x:.1} {y:.1} {d:.2}");
        }
        if d > tol {
            e.left_vol += d * a;
            e.left_max = e.left_max.max(d);
            if d > 0.05 {
                e.left_area += a;
            }
        } else if -d > tol {
            e.gouge_vol += -d * a;
            e.gouge_max = e.gouge_max.max(-d);
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_and_a_step() {
        // a 10 x 10 stock to z 5; the part a 10 x 10 slab to z 3 (two triangles on top)
        let mut m = HeightMap::new([0.0, 0.0, 0.0], [10.0, 10.0, 5.0], 0.1);
        let tris = vec![
            [[0.0, 0.0, 3.0], [10.0, 0.0, 3.0], [10.0, 10.0, 3.0]],
            [[0.0, 0.0, 3.0], [10.0, 10.0, 3.0], [0.0, 10.0, 3.0]],
        ];
        let tops = part_tops(&tris, &m);
        assert!(tops.iter().all(|t| (t - 3.0).abs() < 1e-9));
        let e = end_state(&m, &tops, [0.0, 0.0, 10.0, 10.0], 0.01);
        assert!(
            (e.left_vol - 200.0).abs() < 1e-6 && (e.left_max - 2.0).abs() < 1e-9,
            "{e:?}"
        );
        // cut it down to 3 over half, to 2.9 over the other half
        for (k, z) in m.z.iter_mut().enumerate() {
            *z = if k % m.nx < m.nx / 2 { 3.0 } else { 2.9 };
        }
        let e = end_state(&m, &tops, [0.0, 0.0, 10.0, 10.0], 0.01);
        assert_eq!(e.left_vol, 0.0);
        assert!(
            (e.gouge_vol - 5.0).abs() < 1e-6 && (e.gouge_max - 0.1).abs() < 1e-9,
            "{e:?}"
        );
    }
}
