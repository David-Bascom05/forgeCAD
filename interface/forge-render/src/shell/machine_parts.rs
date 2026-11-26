//! A machine body's geometry from a file (research/cam/machine-builder.md §4, Bodies): an
//! existing `.fg` part, a STEP file (kept beside a `.fg` that imports it) or a mesh (STL: its
//! bounds only, until the format can name a display mesh, request F5); and the envelope fitted
//! to the part's bounds.
use forge_machine::def::{MachineDef, Prim};
use std::path::{Path, PathBuf};

/// How much a fitted box grows past the part's tessellated bounds: the mesh's vertices lie on the
/// exact surface but a curved face can bulge between them by up to the chord (0.01 mm here).
pub const FIT_MARGIN: f64 = 0.05;
const CHORD: f64 = 0.01;

/// The bounds of a `.fg` part (base frame, home pose), built with forge-lang.
pub fn part_bounds(fg: &Path) -> Result<([f64; 3], [f64; 3]), String> {
    let src = forge_dirs::fs::read_to_string(fg).map_err(|e| format!("{}: {e}", fg.display()))?;
    let opts = forge_lang::Options {
        dir: fg.parent().map(Path::to_path_buf),
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, forge_lang::build::Cache::default());
    if !run.ok() {
        return Err(format!(
            "{}: does not build: {:?}",
            fg.display(),
            run.errors.first()
        ));
    }
    let body = run.body.ok_or_else(|| format!("{}: no solid", fg.display()))?;
    let q = forge_mesh::Quality {
        chord: CHORD,
        ..forge_mesh::Quality::default()
    };
    let mesh = forge_mesh::tessellate(&body, &q).map_err(|e| format!("{}: {e:?}", fg.display()))?;
    bounds(mesh.positions.iter().map(|p| [p.x, p.y, p.z]))
}

fn bounds(pts: impl Iterator<Item = [f64; 3]>) -> Result<([f64; 3], [f64; 3]), String> {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for p in pts {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if (0..3).all(|k| lo[k] < hi[k]) {
        Ok((lo, hi))
    } else {
        Err("no volume: nothing to fit".into())
    }
}

/// An STL's bounds (binary or ASCII).
pub fn stl_bounds(bytes: &[u8]) -> Result<([f64; 3], [f64; 3]), String> {
    let ascii = bytes.starts_with(b"solid") && bytes.windows(6).any(|w| w == b"vertex");
    if ascii {
        let s = std::str::from_utf8(bytes).map_err(|_| "an ASCII STL that is not UTF-8")?;
        let pts: Vec<[f64; 3]> = s
            .lines()
            .filter_map(|l| l.trim().strip_prefix("vertex"))
            .filter_map(|r| {
                let v: Vec<f64> = r.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                (v.len() == 3).then(|| [v[0], v[1], v[2]])
            })
            .collect();
        return bounds(pts.into_iter());
    }
    if bytes.len() < 84 {
        return Err("too short for an STL".into());
    }
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    if bytes.len() < 84 + n * 50 {
        return Err(format!(
            "a binary STL of {n} triangles needs {} bytes",
            84 + n * 50
        ));
    }
    let f = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as f64;
    let pts = (0..n).flat_map(|t| {
        let b = 84 + t * 50 + 12;
        (0..3).map(move |v| [f(b + v * 12), f(b + v * 12 + 4), f(b + v * 12 + 8)])
    });
    bounds(pts)
}

/// The box a body's envelope becomes for bounds (grown by FIT_MARGIN).
pub fn fitted(lo: [f64; 3], hi: [f64; 3]) -> Prim {
    Prim::Box {
        min: lo.map(|x| x - FIT_MARGIN),
        max: hi.map(|x| x + FIT_MARGIN),
    }
}

fn safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Attaches `file` to body `b` of `def`, saved in `dir` (the machine file's folder): copies it to
/// `parts/<machine>/`, and for a STEP file writes `<body>.fg` importing it. Returns the message;
/// the body's `part` (and for a mesh its envelope) is set.
pub fn attach(def: &mut MachineDef, b: usize, dir: &Path, file: &Path) -> Result<String, String> {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("the file has no name")?
        .to_string();
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let rel_dir = PathBuf::from("parts").join(safe(&def.name));
    let abs_dir = dir.join(&rel_dir);
    forge_dirs::fs::create_dir_all(&abs_dir).map_err(|e| format!("{}: {e}", abs_dir.display()))?;
    let bytes = forge_dirs::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let body = def.bodies[b].name.clone();
    let rel = |p: &Path| p.to_string_lossy().replace('\\', "/");
    match ext.as_str() {
        "fg" => {
            let dst = rel_dir.join(safe(&name));
            if dir.join(&dst) != file {
                forge_dirs::fs::write(dir.join(&dst), &bytes).map_err(|e| e.to_string())?;
            }
            def.bodies[b].part = Some(rel(&dst));
            Ok(format!("`{body}`: part {}", rel(&dst)))
        }
        "step" | "stp" => {
            let step = rel_dir.join(safe(&name));
            forge_dirs::fs::write(dir.join(&step), &bytes).map_err(|e| e.to_string())?;
            let fg = rel_dir.join(format!("{}.fg", safe(&body)));
            let text = format!(
                "# {body} of machine {}: the STEP file beside this one, as a part.\nsolid = import(\"{}\", op: \"new\")\n",
                def.name,
                safe(&name)
            );
            forge_dirs::fs::write(dir.join(&fg), text).map_err(|e| e.to_string())?;
            def.bodies[b].part = Some(rel(&fg));
            Ok(format!("`{body}`: STEP {} as part {}", rel(&step), rel(&fg)))
        }
        "stl" => {
            let (lo, hi) = stl_bounds(&bytes)?;
            def.bodies[b].prims = vec![fitted(lo, hi)];
            Ok(format!(
                "`{body}`: envelope fitted to the mesh's bounds; the mesh itself is not shown (a body's display mesh needs the format, request F5)"
            ))
        }
        _ => Err(format!(
            "{name}: a body's geometry is a .fg part, a STEP file or an STL mesh"
        )),
    }
}

/// Body `b`'s envelope fitted to its part's bounds.
pub fn fit(def: &mut MachineDef, b: usize, dir: &Path) -> Result<String, String> {
    let part = def.bodies[b]
        .part
        .clone()
        .ok_or("the body has no part to fit to")?;
    let (lo, hi) = part_bounds(&dir.join(&part))?;
    def.bodies[b].prims = vec![fitted(lo, hi)];
    Ok(format!(
        "`{}`: envelope = the part's bounds + {FIT_MARGIN} mm ({:.1} x {:.1} x {:.1})",
        def.bodies[b].name,
        hi[0] - lo[0],
        hi[1] - lo[1],
        hi[2] - lo[2]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stl_bounds_both_kinds() {
        let a = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 10 0 0\nvertex 0 5 2\nendloop\nendfacet\nendsolid x\n";
        assert_eq!(stl_bounds(a).unwrap(), ([0.0, 0.0, 0.0], [10.0, 5.0, 2.0]));
        let mut b = vec![0u8; 80];
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 12]);
        for v in [[-1.0f32, 0.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]] {
            for x in v {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        b.extend_from_slice(&[0u8; 2]);
        assert_eq!(stl_bounds(&b).unwrap(), ([-1.0, 0.0, 0.0], [1.0, 2.0, 3.0]));
        assert!(stl_bounds(b"solid\n").is_err());
    }
}
