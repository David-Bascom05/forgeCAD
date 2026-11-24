//! A user's model files (models.rs) read, attached, checked and split, for the Machine Builder and
//! for `forgecad machine model ...`:
//! - `read`: a STEP / IGES (each solid, meshed as the app meshes a file's body) or STL file as
//!   pieces: a STEP solid each, an STL's connected parts each (triangles sharing a corner).
//! - `attach`: copies a file into the user's models folder for a machine, placed on its body's
//!   envelope (or the machine's bounds), its units read from its size; the user's own note of
//!   where it came from and their right to use it is required.
//! - `check`: a model's points against its body's envelope.
//! - `split`: one model's pieces made models of their own, each given to a body (or dropped), with
//!   the same placement, so a whole-machine download moves with the axes.
use crate::models::{self, Format, Model, Outside};
use forge_machine::def::MachineDef;
use forge_machine::kin::V3;
use std::path::{Path, PathBuf};

/// Triangles sharing corners by index, in the file's own coordinates and units.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Piece {
    pub pos: Vec<V3>,
    pub tri: Vec<[u32; 3]>,
}

impl Piece {
    pub fn bounds(&self) -> Option<(V3, V3)> {
        models::bounds(self.pos.iter().copied())
    }

    fn append(&mut self, o: &Piece) {
        let k = self.pos.len() as u32;
        self.pos.extend_from_slice(&o.pos);
        self.tri.extend(o.tri.iter().map(|t| t.map(|i| i + k)));
    }

    /// One piece of them all.
    pub fn join(ps: &[Piece]) -> Piece {
        let mut all = Piece::default();
        for p in ps {
            all.append(p);
        }
        all
    }

    /// Its triangles in groups that share a corner (a union-find over corners), largest first.
    pub fn components(&self) -> Vec<Piece> {
        let n = self.pos.len();
        let mut up: Vec<usize> = (0..n).collect();
        fn root(up: &mut [usize], mut i: usize) -> usize {
            while up[i] != i {
                up[i] = up[up[i]];
                i = up[i];
            }
            i
        }
        for t in &self.tri {
            let a = root(&mut up, t[0] as usize);
            for &v in &t[1..] {
                let b = root(&mut up, v as usize);
                if a != b {
                    up[b] = a;
                }
            }
        }
        let mut groups: std::collections::BTreeMap<usize, Piece> = Default::default();
        let mut remap: Vec<Option<u32>> = vec![None; n];
        for t in &self.tri {
            let r = root(&mut up, t[0] as usize);
            let g = groups.entry(r).or_default();
            let nt = t.map(|v| {
                let v = v as usize;
                *remap[v].get_or_insert_with(|| {
                    g.pos.push(self.pos[v]);
                    (g.pos.len() - 1) as u32
                })
            });
            g.tri.push(nt);
        }
        let mut out: Vec<Piece> = groups.into_values().collect();
        out.sort_by_key(|p| std::cmp::Reverse(p.tri.len()));
        out
    }

    /// A binary STL of it.
    pub fn to_stl(&self, header: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(84 + 50 * self.tri.len());
        let mut h = [0u8; 80];
        for (d, s) in h.iter_mut().zip(header.bytes()) {
            *d = s;
        }
        out.extend_from_slice(&h);
        out.extend_from_slice(&(self.tri.len() as u32).to_le_bytes());
        for t in &self.tri {
            let [a, b, c] = t.map(|i| self.pos[i as usize]);
            let n = forge_machine::kin::cross(forge_machine::kin::sub(b, a), forge_machine::kin::sub(c, a));
            let l = forge_machine::kin::norm(n);
            let n = if l > 0.0 { n.map(|x| x / l) } else { [0.0; 3] };
            for v in [n, a, b, c] {
                for x in v {
                    out.extend_from_slice(&(x as f32).to_le_bytes());
                }
            }
            out.extend_from_slice(&[0, 0]);
        }
        out
    }
}

/// A model file's pieces (its own coordinates and units): a STEP solid each; an STL's parts that
/// share no corner each.
pub fn read(name: &str, bytes: &[u8]) -> Result<Vec<Piece>, String> {
    let out = match Format::of(name)? {
        Format::Stl => {
            let (objs, _) = forge_mesh::read_stl(bytes)
                .map_err(|e| format!("{name}: not an STL this reads ({})", e.0))?;
            let all = Piece::join(
                &objs
                    .iter()
                    .map(|o| Piece {
                        pos: o.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
                        tri: o.triangles.clone(),
                    })
                    .collect::<Vec<_>>(),
            );
            all.components()
        }
        Format::Step | Format::Iges => {
            let imp = forge_step::import(bytes, &forge_step::Limits::default())
                .map_err(|_| format!("{name}: not a model this reads"))?;
            let mut v = Vec::new();
            let mut failed = 0;
            for s in &imp.solids {
                let q = forge_mesh::Quality {
                    chord: 5e-4 * s.body.faces_box().diagonal(),
                    ..forge_mesh::Quality::default()
                };
                match forge_mesh::tessellate::tessellate_tolerant(&s.body, &q) {
                    Ok(m) => v.push(Piece {
                        pos: m.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
                        tri: m.triangles.clone(),
                    }),
                    Err(_) => failed += 1,
                }
            }
            if v.is_empty() && failed > 0 {
                return Err(format!("{name}: none of its {failed} solids could be meshed"));
            }
            v
        }
    };
    if out.iter().all(|p| p.tri.is_empty()) {
        return Err(format!("{name}: no solid or triangle in it"));
    }
    Ok(out)
}

/// A file's pieces, read through the app's filesystem.
pub fn read_file(path: &Path) -> Result<Vec<Piece>, String> {
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let bytes = forge_dirs::fs::read(path).map_err(|e| format!("{name}: {e}"))?;
    read(&name, &bytes)
}

/// The note required with every model: where it came from and the user's right to use it.
pub fn require_from(from: &str) -> Result<(), String> {
    if from.trim().is_empty() {
        Err("say where the file came from and your right to use it (e.g. \"Haas download, my licence\"): it is kept with the model".into())
    } else {
        Ok(())
    }
}

/// What a model is placed on: its body's envelope, or (a whole-machine model) every body's.
pub fn target(def: &MachineDef, body: Option<&str>) -> Result<(V3, V3), String> {
    match body {
        Some(b) => {
            let b =
                def.bodies.iter().find(|x| x.name == b).ok_or_else(|| {
                    format!("{} has no body `{b}` (its bodies: {})", def.name, body_names(def))
                })?;
            models::prims_bounds(&b.prims)
                .ok_or_else(|| format!("`{}` has no envelope to place it on", b.name))
        }
        None => models::bounds(
            def.bodies
                .iter()
                .filter_map(|b| models::prims_bounds(&b.prims))
                .flat_map(|(lo, hi)| [lo, hi]),
        )
        .ok_or_else(|| format!("{} has no bodies to place it on", def.name)),
    }
}

pub fn body_names(def: &MachineDef) -> String {
    def.bodies
        .iter()
        .map(|b| b.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// How a model is placed on attach: the user's units and turn when given, else read from its size
/// and none; then sat on its target unless an offset is given.
#[derive(Clone, Debug, Default)]
pub struct Placing {
    pub scale: Option<f64>,
    pub rotate: Option<V3>,
    pub offset: Option<V3>,
}

/// Copies `file` into `dir` (a machine's models folder) as a model of `body` (None: the whole
/// machine), placed, with the user's note; returns it and what was done, in words. `taken`: the
/// machine's models so far (the new file's name is free among them). Nothing is left behind on a
/// refusal.
pub fn attach(
    def: &MachineDef,
    dir: &Path,
    taken: &[Model],
    body: Option<&str>,
    file: &Path,
    from: &str,
    how: &Placing,
) -> Result<(Model, String), String> {
    require_from(from)?;
    let target = target(def, body)?;
    let name = models::copy_in(dir, file, taken)?;
    let pieces = match read_file(&dir.join(&name)) {
        Ok(p) => p,
        Err(e) => {
            let _ = forge_dirs::fs::remove_file(dir.join(&name));
            return Err(e);
        }
    };
    let all = Piece::join(&pieces);
    let mut m = Model::new(body.map(str::to_string), name.clone());
    m.from = from.trim().to_string();
    let mut said = Vec::new();
    match how.scale {
        Some(s) => m.scale = s,
        None => {
            if let Some(own) = all.bounds()
                && let Some(s) = models::guess_scale(own, target)
            {
                m.scale = s;
                said.push(format!(
                    "its size reads as {}, so its units are set to {} (change them if not)",
                    if s == 25.4 { "inches" } else { "metres" },
                    if s == 25.4 { "in" } else { "m" }
                ));
            }
        }
    }
    if let Some(r) = how.rotate {
        m.rotate = r;
    }
    match how.offset {
        Some(o) => m.offset = o,
        None => {
            m.sit_on(&all.pos, target);
            said.push("placed on its envelope".into());
        }
    }
    if body.is_none() && pieces.len() > 1 {
        said.push(format!(
            "it has {} separate parts: split it to give each to a body, so they move with the axes",
            pieces.len()
        ));
    }
    let on = body.map_or("the whole machine".to_string(), |b| format!("`{b}`"));
    let tail = if said.is_empty() {
        String::new()
    } else {
        format!("; {}", said.join("; "))
    };
    Ok((m, format!("{name} attached to {on}{tail}")))
}

/// At most this many of a model's points are checked against its envelope (evenly spread).
pub const CHECKED: usize = 200_000;

/// A model's points placed (a sample of at most `CHECKED`).
pub fn placed_points(pos: &[V3], m: &Model) -> Vec<V3> {
    let step = pos.len().div_ceil(CHECKED).max(1);
    pos.iter().step_by(step).map(|p| m.place(*p)).collect()
}

/// What a model sticking out of a display-only body's envelope means (nothing).
pub const DISPLAY_ONLY: &str =
    "display only: not collision-checked, so a model sticking out of this envelope changes nothing";

/// A model against its body's envelope: (what is outside, the body is display only, `check =
/// false`). A whole-machine model: not checked, None.
pub fn check(def: &MachineDef, dir: &Path, m: &Model) -> Result<Option<(Outside, bool)>, String> {
    let Some(b) = m.body.as_deref() else {
        return Ok(None);
    };
    let body = def
        .bodies
        .iter()
        .find(|x| x.name == b)
        .ok_or_else(|| format!("{}: its body `{b}` is not in {} any more", m.file, def.name))?;
    let all = Piece::join(&read_file(&dir.join(&m.file))?);
    let o = models::outside(&body.prims, placed_points(&all.pos, m).into_iter());
    Ok(Some((o, !body.check)))
}

/// Splits model `i` of `ms` into its pieces: each piece given a body in `to` (None: dropped)
/// becomes a model of its own (`<stem>-<n>.stl` beside it), with the same placement and note;
/// the original goes. `to` has one entry per piece, in `read`'s order. Returns the new models.
pub fn split(dir: &Path, ms: &mut Vec<Model>, i: usize, to: &[Option<String>]) -> Result<Vec<Model>, String> {
    let m = ms.get(i).cloned().ok_or("no such model")?;
    let pieces = read_file(&dir.join(&m.file))?;
    if to.len() != pieces.len() {
        return Err(format!(
            "{} has {} parts; {} given",
            m.file,
            pieces.len(),
            to.len()
        ));
    }
    if to.iter().all(Option::is_none) {
        return Err("give at least one part a body (or remove the model)".into());
    }
    let stem = m
        .file
        .rsplit_once('.')
        .map_or(m.file.as_str(), |(s, _)| s)
        .to_string();
    let mut made = Vec::new();
    // pieces given the same body go in one file
    let mut bodies: Vec<&String> = to.iter().flatten().collect();
    bodies.sort();
    bodies.dedup();
    for b in bodies {
        let mine: Vec<Piece> = pieces
            .iter()
            .zip(to)
            .filter(|(_, t)| t.as_ref() == Some(b))
            .map(|(p, _)| p.clone())
            .collect();
        let others: Vec<Model> = ms.iter().chain(&made).cloned().collect();
        let base = format!("{stem}-{}.stl", models::safe(b));
        let name = std::iter::once(base.clone())
            .chain((2..).map(|k| format!("{stem}-{}-{k}.stl", models::safe(b))))
            .find(|n| !others.iter().any(|o| &o.file == n))
            .expect("an unbounded search");
        let bytes = Piece::join(&mine).to_stl(&format!("{} part {b}", m.file));
        forge_dirs::fs::write(dir.join(&name), bytes).map_err(|e| format!("{}: {e}", dir.display()))?;
        made.push(Model {
            body: Some(b.clone()),
            file: name,
            ..m.clone()
        });
    }
    ms.remove(i);
    ms.extend(made.iter().cloned());
    Ok(made)
}

/// The models folder of `machine` under the user's machines folder.
pub fn dir_for(user_machines: &Path, machine: &str) -> PathBuf {
    models::dir_in(user_machines, machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_piece(lo: V3, hi: V3) -> Piece {
        let c = |k: usize| -> V3 { std::array::from_fn(|a| if k >> a & 1 == 1 { hi[a] } else { lo[a] }) };
        let pos = (0..8).map(c).collect();
        let faces = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let tri = faces
            .iter()
            .flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]])
            .collect();
        Piece { pos, tri }
    }

    #[test]
    fn an_stl_reads_as_its_separate_parts() {
        let two = Piece::join(&[
            box_piece([0.0; 3], [1.0; 3]),
            box_piece([5.0, 0.0, 0.0], [7.0, 1.0, 1.0]),
        ]);
        let bytes = two.to_stl("two boxes");
        let ps = read("two.stl", &bytes).unwrap();
        assert_eq!(ps.len(), 2);
        assert_eq!(ps.iter().map(|p| p.tri.len()).sum::<usize>(), 24);
        let mut b: Vec<_> = ps.iter().map(|p| p.bounds().unwrap()).collect();
        b.sort_by(|x, y| x.0[0].total_cmp(&y.0[0]));
        assert_eq!(b[1], ([5.0, 0.0, 0.0], [7.0, 1.0, 1.0]));
    }

    #[test]
    fn a_note_is_required() {
        assert!(
            require_from("  ")
                .unwrap_err()
                .contains("Haas download, my licence")
        );
        assert!(require_from("Haas download, my licence").is_ok());
    }

    #[test]
    fn a_split_gives_each_part_to_a_body_with_the_same_placement() {
        let tmp = forge_dirs::temp_dir().join(format!("forge-model-split-{}", forge_dirs::process_id()));
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
        forge_dirs::fs::create_dir_all(&tmp).unwrap();
        let three = Piece::join(&[
            box_piece([0.0; 3], [10.0; 3]),
            box_piece([20.0, 0.0, 0.0], [22.0, 1.0, 1.0]),
            box_piece([30.0, 0.0, 0.0], [31.0, 1.0, 1.0]),
        ]);
        forge_dirs::fs::write(tmp.join("vf2.stl"), three.to_stl("x")).unwrap();
        let mut whole = Model::new(None, "vf2.stl".into());
        whole.from = "Haas download, my licence".into();
        whole.scale = 25.4;
        whole.offset = [1.0, 2.0, 3.0];
        let mut ms = vec![whole.clone()];
        // largest first: the 10 mm box, then the two small ones
        let made = split(
            &tmp,
            &mut ms,
            0,
            &[Some("base".into()), Some("table".into()), None],
        )
        .unwrap();
        assert_eq!(made.len(), 2);
        assert_eq!(ms.len(), 2);
        for m in &ms {
            assert_eq!(
                (m.scale, m.offset, m.from.as_str()),
                (25.4, [1.0, 2.0, 3.0], "Haas download, my licence")
            );
        }
        let t = ms.iter().find(|m| m.body.as_deref() == Some("table")).unwrap();
        assert_eq!(t.file, "vf2-table.stl");
        let tb = Piece::join(&read_file(&tmp.join(&t.file)).unwrap())
            .bounds()
            .unwrap();
        assert_eq!(tb.1[0], 22.0);
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
    }
}
