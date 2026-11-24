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
use forge_machine::kin::{self, V3};
use std::path::{Path, PathBuf};

/// Triangles sharing corners by index, in the file's own coordinates and units.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Piece {
    pub pos: Vec<V3>,
    pub tri: Vec<[u32; 3]>,
    /// What the file calls it: a STEP part's products from the top assembly down (`A / B / part`),
    /// or its solid's name; empty for an STL's.
    pub name: String,
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

    /// Fewer triangles for display: corners within one `cell` (mm, in the file's units) of a grid
    /// merged at their mean, triangles that close up dropped, and a triangle met twice kept once.
    pub fn decimated(&self, cell: f64) -> Piece {
        use std::collections::HashMap;
        let mut at: HashMap<[i64; 3], u32> = HashMap::new();
        let mut sum: Vec<([f64; 3], f64)> = Vec::new();
        let map: Vec<u32> = self
            .pos
            .iter()
            .map(|p| {
                let k = p.map(|x| (x / cell).floor() as i64);
                let i = *at.entry(k).or_insert_with(|| {
                    sum.push(([0.0; 3], 0.0));
                    (sum.len() - 1) as u32
                });
                let s = &mut sum[i as usize];
                for (d, x) in s.0.iter_mut().zip(p) {
                    *d += x;
                }
                s.1 += 1.0;
                i
            })
            .collect();
        let mut seen = std::collections::HashSet::new();
        let mut tri = Vec::new();
        for t in &self.tri {
            let n = t.map(|v| map[v as usize]);
            if n[0] == n[1] || n[1] == n[2] || n[0] == n[2] {
                continue;
            }
            let mut k = n;
            k.sort_unstable();
            if seen.insert(k) {
                tri.push(n);
            }
        }
        Piece {
            pos: sum.iter().map(|(s, n)| s.map(|x| x / n)).collect(),
            tri,
            name: self.name.clone(),
        }
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
                        name: String::new(),
                    })
                    .collect::<Vec<_>>(),
            );
            all.components()
        }
        Format::Step | Format::Iges => {
            // a display model: sixteen times the evaluations a part's check may spend, which
            // a machine maker's sheet-metal enclosure needs (Haas's VF-2: three solids more, no
            // longer to read)
            let d = forge_step::Limits::default();
            let lim = forge_step::Limits {
                max_evals: d.max_evals * 16,
                ..d
            };
            let imp =
                forge_step::import(bytes, &lim).map_err(|_| format!("{name}: not a model this reads"))?;
            // each solid meshed once, then placed where the assembly uses it (a solid no use
            // names stands as the file has it)
            let mut meshed: std::collections::BTreeMap<u32, Option<Piece>> = Default::default();
            let mut failed = 0;
            for s in &imp.solids {
                let q = forge_mesh::Quality {
                    chord: 5e-4 * s.body.faces_box().diagonal(),
                    ..forge_mesh::Quality::default()
                };
                let m = forge_mesh::tessellate::tessellate_tolerant(&s.body, &q)
                    .ok()
                    .map(|m| Piece {
                        pos: m.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
                        tri: m.triangles.clone(),
                        name: s.name.clone(),
                    });
                failed += usize::from(m.is_none());
                meshed.insert(s.entity, m);
            }
            let mut v = Vec::new();
            let mut placed = std::collections::BTreeSet::new();
            let mut uses: Vec<&forge_step::Occurrence> = imp.occurrences.iter().collect();
            uses.sort_by(|a, b| a.path.cmp(&b.path).then(a.solid.cmp(&b.solid)));
            for u in uses {
                let Some(Some(p)) = meshed.get(&u.solid) else {
                    continue;
                };
                placed.insert(u.solid);
                let f = u.frame;
                v.push(Piece {
                    pos: p
                        .pos
                        .iter()
                        .map(|&[x, y, z]| {
                            let q = f.point_to_world(forge_math::Point3::new(x, y, z));
                            [q.x, q.y, q.z]
                        })
                        .collect(),
                    tri: p.tri.clone(),
                    name: u.path.join(" / "),
                });
            }
            for (e, p) in meshed {
                if let (false, Some(p)) = (placed.contains(&e), p) {
                    v.push(p);
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
    let m = ms.get(i).ok_or("no such model")?;
    let pieces = read_file(&dir.join(&m.file))?;
    split_read(dir, ms, i, &pieces, to)
}

/// `split` with the model's pieces already read (`read_file` of it): a large file read once.
pub fn split_read(
    dir: &Path,
    ms: &mut Vec<Model>,
    i: usize,
    pieces: &[Piece],
    to: &[Option<String>],
) -> Result<Vec<Model>, String> {
    let m = ms.get(i).cloned().ok_or("no such model")?;
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

/// A machine's models made forgecad's own, for its built-in library (`models::shipped_dir`):
/// each model given a body, from `from_dir`, decimated to `cell` mm (`Piece::decimated`, in mm
/// whatever the file's units) and written to `to_dir` as `<body>.stl` with the same placement and
/// `source` as its note, beside a `models.toml`; a body named in `coarse` (display only: an
/// enclosure) at three times `cell`. Models on the whole machine are left out (a
/// shipped machine moves with its axes). Returns each written file with its size in bytes.
pub fn ship(
    from_dir: &Path,
    ms: &[Model],
    to_dir: &Path,
    cell: f64,
    coarse: &[String],
    source: &str,
) -> Result<Vec<(String, u64)>, String> {
    forge_dirs::fs::create_dir_all(to_dir).map_err(|e| format!("{}: {e}", to_dir.display()))?;
    let mut out = Vec::new();
    let mut shipped: Vec<Model> = Vec::new();
    let mut bodies: Vec<&str> = ms.iter().filter_map(|m| m.body.as_deref()).collect();
    bodies.sort_unstable();
    bodies.dedup();
    for b in bodies {
        let mine: Vec<&Model> = ms.iter().filter(|m| m.body.as_deref() == Some(b)).collect();
        // one file a body: the models given it, each placed as it was, joined in the first's frame
        let first = mine[0];
        let inv = first.rigid();
        let mut all = Vec::new();
        for m in &mine {
            let p = Piece::join(&read_file(&from_dir.join(&m.file))?);
            let (r, ri) = (m.rigid(), inv);
            all.push(Piece {
                pos: p
                    .pos
                    .iter()
                    .map(|&x| {
                        let w = kin::add(kin::mul(&r.r, kin::scale(x, m.scale)), r.p);
                        // back into the first model's file frame (its scale is applied on read)
                        let l = kin::mul(&kin::transpose(&ri.r), kin::sub(w, ri.p));
                        kin::scale(l, 1.0 / first.scale)
                    })
                    .collect(),
                tri: p.tri,
                name: String::new(),
            });
        }
        let c = if coarse.iter().any(|x| x == b) {
            3.0 * cell
        } else {
            cell
        };
        let piece = Piece::join(&all).decimated(c / first.scale);
        let name = format!("{}.stl", models::safe(b));
        let bytes = piece.to_stl(&format!("{b}: {source}"));
        forge_dirs::fs::write(to_dir.join(&name), &bytes)
            .map_err(|e| format!("{}: {e}", to_dir.display()))?;
        out.push((name.clone(), bytes.len() as u64));
        shipped.push(Model {
            body: Some(b.to_string()),
            file: name,
            from: source.to_string(),
            ..first.clone()
        });
    }
    if shipped.is_empty() {
        return Err("no model is given a body: split the machine's model first".into());
    }
    // its own header: forgecad's models, not a user's library
    let body: String = models::write(&shipped)?
        .lines()
        .skip_while(|l| l.starts_with('#') || l.is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    let text = format!(
        "# forgecad's own models of this machine, drawn in place of its bodies (collisions use the\n\
         # bodies' envelopes): written by `forgecad machine model ship`; a user's attached models win.\n\n{body}"
    );
    forge_dirs::fs::write(to_dir.join(models::INDEX), text).map_err(|e| format!("{}: {e}", to_dir.display()))?;
    Ok(out)
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
        Piece {
            pos,
            tri,
            name: String::new(),
        }
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

    #[test]
    fn decimation_merges_corners_within_a_cell_and_keeps_the_shape() {
        // a box cut in a fine grid of triangles along its top: merged back, its bounds kept
        let mut fine = box_piece([0.0; 3], [100.0, 50.0, 20.0]);
        let k = fine.pos.len() as u32;
        for i in 0..100 {
            for j in 0..50 {
                let (x, y) = (i as f64, j as f64);
                fine.pos
                    .extend([[x, y, 20.0], [x + 1.0, y, 20.0], [x + 1.0, y + 1.0, 20.0]]);
                let n = k + 3 * (i * 50 + j) as u32;
                fine.tri.push([n, n + 1, n + 2]);
            }
        }
        let d = fine.decimated(5.0);
        assert!(d.tri.len() < fine.tri.len() / 4, "{}", d.tri.len());
        let (lo, hi) = d.bounds().unwrap();
        for a in 0..3 {
            assert!(lo[a] < 5.0 && hi[a] > [100.0, 50.0, 20.0][a] - 5.0);
        }
        // no triangle closed up, none twice
        assert!(d.tri.iter().all(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]));
        // a cell smaller than any edge changes nothing but sharing
        assert_eq!(box_piece([0.0; 3], [10.0; 3]).decimated(0.5).tri.len(), 12);
    }

    #[test]
    fn ship_writes_a_mesh_a_body_and_its_index() {
        let tmp = forge_dirs::temp_dir().join(format!("forge-model-ship-{}", forge_dirs::process_id()));
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
        let (from, to) = (tmp.join("user"), tmp.join("shipped/maker/vmc"));
        forge_dirs::fs::create_dir_all(&from).unwrap();
        forge_dirs::fs::write(from.join("a.stl"), box_piece([0.0; 3], [10.0; 3]).to_stl("a")).unwrap();
        forge_dirs::fs::write(from.join("b.stl"), box_piece([0.0; 3], [2.0; 3]).to_stl("b")).unwrap();
        let mut a = Model::new(Some("table".into()), "a.stl".into());
        a.rotate = [90.0, 0.0, 0.0];
        a.offset = [5.0, 0.0, 0.0];
        let mut b = Model::new(Some("table".into()), "b.stl".into());
        b.offset = [100.0, 0.0, 0.0];
        let whole = Model::new(None, "w.stl".into());
        let files = ship(&from, &[a.clone(), b, whole], &to, 1.0, &[], "the maker's model").unwrap();
        assert_eq!(
            files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(),
            ["table.stl"]
        );
        let ms = models::load_in(&to).unwrap();
        assert_eq!(ms.len(), 1);
        assert_eq!(
            (ms[0].rotate, ms[0].offset, ms[0].from.as_str()),
            (a.rotate, a.offset, "the maker's model")
        );
        // both models in one file, each still where it was placed
        let p = Piece::join(&read_file(&to.join("table.stl")).unwrap());
        let placed = models::bounds(p.pos.iter().map(|x| ms[0].place(*x))).unwrap();
        assert!(
            (placed.1[0] - 102.0).abs() < 1e-6 && (placed.0[0] - 5.0).abs() < 1e-6,
            "{placed:?}"
        );
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
    }
}
