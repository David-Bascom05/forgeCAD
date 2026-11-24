//! Models a user attaches to a machine: their own files (a maker's STEP or STL, say), shown in
//! place of a body's drawn shape while the body's primitives stay its collision envelope.
//!
//! - Kept in the user's library, never in a project and never fetched:
//!   `<config>/forgecad/cam/machines/models/<machine>/models.toml` and the files beside it. A
//!   maker's model is theirs; its terms decide where it may go, so forgecad keeps it on the
//!   user's machine only.
//! - Placed by a rigid move from the file's own coordinates to the machine's base frame at home:
//!   `p ↦ R·(scale·p) + offset`, R the rotations about x, then y, then z (degrees). A model starts
//!   placed on its body's envelope (centred over it, bottom on its bottom) and is nudged from there.
//! - Checked against the envelope: the points of the model outside every primitive, and how far,
//!   since collisions are found on the envelope only.
use crate::write::{num, string};
use forge_machine::def::Prim;
use forge_machine::kin::{self, Rigid, V3};
use forge_tooling::toml::{self, nums, tables, text};
use std::path::{Path, PathBuf};

/// The formats a model is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Step,
    Iges,
    Stl,
}

impl Format {
    /// The extensions the picker offers.
    pub const EXTS: [&'static str; 5] = ["step", "stp", "igs", "iges", "stl"];

    /// A file's format by its extension; the formats forgecad cannot read say what to send instead.
    pub fn of(name: &str) -> Result<Format, String> {
        let ext = name.rsplit_once('.').map_or("", |(_, e)| e).to_ascii_lowercase();
        match ext.as_str() {
            "step" | "stp" => Ok(Format::Step),
            "igs" | "iges" => Ok(Format::Iges),
            "stl" => Ok(Format::Stl),
            "x_t" | "x_b" | "xmt_txt" | "xmt_bin" => Err(format!(
                "{name}: Parasolid files are not read by forgecad; save the model as STEP (or STL) in the program that opens it"
            )),
            "sldprt" | "sldasm" | "f3d" | "ipt" | "iam" | "prt" | "catpart" => Err(format!(
                "{name}: a CAD program's own format; export it as STEP (or STL)"
            )),
            "3mf" | "obj" | "glb" | "gltf" => Err(format!(
                "{name}: not read as a machine model yet; export it as STEP or STL"
            )),
            _ => Err(format!("{name}: a machine model is a STEP, IGES or STL file")),
        }
    }
}

/// One attached model.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The body it is drawn as; `None`: the whole machine, drawn on `base` and not moving with
    /// the axes.
    pub body: Option<String>,
    /// Its file, in the machine's models folder.
    pub file: String,
    /// Where the user got it (a maker's page, "own drawing"...): kept with it, never checked.
    pub from: String,
    /// File units to mm: 1 (mm), 25.4 (inch), 1000 (m).
    pub scale: f64,
    /// Rotations about x, y, z in degrees, applied in that order.
    pub rotate: V3,
    /// Where the file's origin lands in the base frame (mm).
    pub offset: V3,
}

impl Model {
    pub fn new(body: Option<String>, file: String) -> Model {
        Model {
            body,
            file,
            from: String::new(),
            scale: 1.0,
            rotate: [0.0; 3],
            offset: [0.0; 3],
        }
    }

    /// The rotation part of the placement.
    pub fn rotation(&self) -> kin::M3 {
        let rx = kin::rot([1.0, 0.0, 0.0], self.rotate[0]);
        let ry = kin::rot([0.0, 1.0, 0.0], self.rotate[1]);
        let rz = kin::rot([0.0, 0.0, 1.0], self.rotate[2]);
        kin::mm(&rz, &kin::mm(&ry, &rx))
    }

    /// A file point placed in the base frame at home.
    pub fn place(&self, p: V3) -> V3 {
        kin::add(kin::mul(&self.rotation(), kin::scale(p, self.scale)), self.offset)
    }

    /// The rigid part (rotation and offset; scale is applied to points first).
    pub fn rigid(&self) -> Rigid {
        Rigid {
            r: self.rotation(),
            p: self.offset,
        }
    }

    /// `offset` set so the model (its file points `pts`) sits centred over `target` with its
    /// bottom on the target's bottom.
    pub fn sit_on(&mut self, pts: &[V3], target: (V3, V3)) {
        self.offset = [0.0; 3];
        let Some((lo, hi)) = bounds(pts.iter().map(|p| self.place(*p))) else {
            return;
        };
        let (tlo, thi) = target;
        self.offset = [
            (tlo[0] + thi[0]) / 2.0 - (lo[0] + hi[0]) / 2.0,
            (tlo[1] + thi[1]) / 2.0 - (lo[1] + hi[1]) / 2.0,
            tlo[2] - lo[2],
        ];
    }
}

/// The units a model's size suggests against what it should be about the size of: inches or
/// metres when the file is 25.4 or 1000 times too small (within a factor of 2.5), else none.
pub fn guess_scale(model: (V3, V3), target: (V3, V3)) -> Option<f64> {
    let big = |(lo, hi): (V3, V3)| (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max);
    let (m, t) = (big(model), big(target));
    if !(m > 0.0 && t > 0.0) {
        return None;
    }
    let r = t / m;
    [25.4, 1000.0].into_iter().find(|s| r > s / 2.5 && r < s * 2.5)
}

pub fn bounds(pts: impl Iterator<Item = V3>) -> Option<(V3, V3)> {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for p in pts {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// The bounds of a body's primitives (base frame, home).
pub fn prims_bounds(prims: &[Prim]) -> Option<(V3, V3)> {
    let corners = prims.iter().flat_map(|p| match p {
        Prim::Box { min, max } => vec![*min, *max],
        Prim::Cyl { base, axis, r, h } => {
            let top = kin::add(*base, kin::scale(*axis, *h));
            // the round's reach along each world axis: r·sqrt(1 - a_k²)
            let e: V3 = std::array::from_fn(|k| r * (1.0 - axis[k] * axis[k]).max(0.0).sqrt());
            vec![
                kin::sub(*base, e),
                kin::add(*base, e),
                kin::sub(top, e),
                kin::add(top, e),
            ]
        }
    });
    bounds(corners)
}

/// How far a point is outside a primitive (0 inside or on it).
pub fn outside_prim(p: &Prim, x: V3) -> f64 {
    match p {
        Prim::Box { min, max } => {
            let d: V3 = std::array::from_fn(|k| (min[k] - x[k]).max(x[k] - max[k]).max(0.0));
            kin::norm(d)
        }
        Prim::Cyl { base, axis, r, h } => {
            let v = kin::sub(x, *base);
            let t = kin::dot(v, *axis);
            let radial = kin::norm(kin::sub(v, kin::scale(*axis, t)));
            let a = (-t).max(t - h).max(0.0);
            let b = (radial - r).max(0.0);
            (a * a + b * b).sqrt()
        }
    }
}

/// What of a model lies outside its body's envelope.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outside {
    /// Points checked, and those outside every primitive by more than `TOL`.
    pub points: usize,
    pub out: usize,
    /// The farthest outside, and where (base frame, home).
    pub worst: f64,
    pub at: V3,
}

/// Points within this of the envelope count as inside (a tessellation's and a fit's slack).
pub const TOL: f64 = 0.1;

/// Placed points against an envelope.
pub fn outside(prims: &[Prim], pts: impl Iterator<Item = V3>) -> Outside {
    let mut o = Outside::default();
    for x in pts {
        o.points += 1;
        let d = prims
            .iter()
            .map(|p| outside_prim(p, x))
            .fold(f64::INFINITY, f64::min);
        if d > TOL {
            o.out += 1;
            if d > o.worst {
                o.worst = d;
                o.at = x;
            }
        }
    }
    o
}

/// The user's folder of models for a machine: `models/<machine>` in their machines folder.
pub fn dir(machine: &str) -> Option<PathBuf> {
    Some(dir_in(&crate::catalog::user_dir()?, machine))
}

/// A machine's models folder under a user machines folder (`<config>/forgecad/cam/machines`).
pub fn dir_in(user_machines: &Path, machine: &str) -> PathBuf {
    user_machines.join("models").join(safe(machine))
}

/// A name safe as one path component.
pub fn safe(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.starts_with('.') {
        format!("_{s}")
    } else {
        s
    }
}

pub const INDEX: &str = "models.toml";

pub fn parse(src: &str) -> Result<Vec<Model>, String> {
    let root = toml::parse(src).map_err(|e| format!("{INDEX}: {e:?}"))?;
    tables(&root, "model")
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let v3 = |k: &str, d: V3| -> Result<V3, String> {
                match nums(t, k) {
                    None => Ok(d),
                    Some(v) if v.len() == 3 => Ok([v[0], v[1], v[2]]),
                    Some(_) => Err(format!("model {}: `{k}` takes 3 numbers", i + 1)),
                }
            };
            let file = text(t, "file").ok_or(format!("model {}: no `file`", i + 1))?;
            let scale = toml::num(t, "scale").unwrap_or(1.0);
            if !(scale > 0.0 && scale.is_finite()) {
                return Err(format!("model {}: `scale` must be above 0", i + 1));
            }
            Ok(Model {
                body: text(t, "body").map(str::to_string),
                file: file.to_string(),
                from: text(t, "from").unwrap_or("").to_string(),
                scale,
                rotate: v3("rotate", [0.0; 3])?,
                offset: v3("offset", [0.0; 3])?,
            })
        })
        .collect()
}

pub fn write(models: &[Model]) -> Result<String, String> {
    let v3 = |v: &V3| format!("[{}, {}, {}]", num(v[0]), num(v[1]), num(v[2]));
    let mut s = String::from(
        "# Models attached to this machine in forgecad's Machine Builder: the user's own files,\n\
         # kept here and never in a project. Each is drawn as its body; collisions use the body's\n\
         # envelope.\n",
    );
    for m in models {
        s.push_str("\n[[model]]\n");
        if let Some(b) = &m.body {
            s.push_str(&format!("body = {}\n", string(b)?));
        }
        s.push_str(&format!("file = {}\n", string(&m.file)?));
        if !m.from.is_empty() {
            s.push_str(&format!("from = {}\n", string(&m.from)?));
        }
        s.push_str(&format!("scale = {}\n", num(m.scale)));
        s.push_str(&format!("rotate = {}\n", v3(&m.rotate)));
        s.push_str(&format!("offset = {}\n", v3(&m.offset)));
    }
    Ok(s)
}

/// A machine's models from `dir` (none when it has no index).
pub fn load_in(dir: &Path) -> Result<Vec<Model>, String> {
    let p = dir.join(INDEX);
    if !forge_dirs::fs::exists(&p) {
        return Ok(Vec::new());
    }
    let src = forge_dirs::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse(&src)
}

/// Writes the index (read back first), and removes the files no model names any more.
pub fn save_in(dir: &Path, models: &[Model]) -> Result<(), String> {
    let text = write(models)?;
    if parse(&text)? != models {
        return Err("the models index does not read back as written".into());
    }
    forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    forge_dirs::fs::write(dir.join(INDEX), text).map_err(|e| format!("{}: {e}", dir.display()))?;
    if let Ok(rd) = forge_dirs::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name != INDEX && Format::of(&name).is_ok() && !models.iter().any(|m| m.file == name) {
                let _ = forge_dirs::fs::remove_file(e.path());
            }
        }
    }
    Ok(())
}

/// Copies `src` into `dir` under a name no other model has; returns that name.
pub fn copy_in(dir: &Path, src: &Path, taken: &[Model]) -> Result<String, String> {
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("the file has no name")?;
    Format::of(name)?;
    let bytes = forge_dirs::fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let base = safe(name);
    let (stem, ext) = base.rsplit_once('.').unwrap_or((&base, ""));
    let free = |n: &String| !taken.iter().any(|m| &m.file == n);
    let name = std::iter::once(base.clone())
        .chain((2..).map(|k| format!("{stem}-{k}.{ext}")))
        .find(free)
        .expect("an unbounded search");
    forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    forge_dirs::fs::write(dir.join(&name), bytes).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(lo: f64, hi: f64) -> Vec<V3> {
        (0..8)
            .map(|k| std::array::from_fn(|a| if k >> a & 1 == 1 { hi } else { lo }))
            .collect()
    }

    #[test]
    fn formats_say_what_to_send_instead() {
        assert_eq!(Format::of("Head.STP"), Ok(Format::Step));
        assert_eq!(Format::of("a.iges"), Ok(Format::Iges));
        assert_eq!(Format::of("a.stl"), Ok(Format::Stl));
        assert!(
            Format::of("vf2.x_t")
                .unwrap_err()
                .contains("save the model as STEP")
        );
        assert!(Format::of("a.sldprt").unwrap_err().contains("export it as STEP"));
    }

    #[test]
    fn a_model_sits_centred_on_its_envelope() {
        let mut m = Model::new(Some("table".into()), "t.stl".into());
        let pts = cube(0.0, 10.0);
        m.sit_on(&pts, ([-100.0, -50.0, 200.0], [100.0, 50.0, 260.0]));
        let (lo, hi) = bounds(pts.iter().map(|p| m.place(*p))).unwrap();
        assert_eq!(lo, [-5.0, -5.0, 200.0]);
        assert_eq!(hi, [5.0, 5.0, 210.0]);
    }

    #[test]
    fn rotations_are_about_x_then_y_then_z() {
        let mut m = Model::new(None, "m.step".into());
        m.rotate = [90.0, 0.0, 90.0];
        // y → z by x+90, then z stays under z+90
        assert_eq!(m.place([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        // x stays under x+90, then x → y by z+90
        assert_eq!(m.place([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
        m.scale = 25.4;
        assert_eq!(m.place([1.0, 0.0, 0.0]), [0.0, 25.4, 0.0]);
    }

    #[test]
    fn inches_and_metres_are_suggested_by_size() {
        let t = ([0.0; 3], [700.0, 400.0, 300.0]);
        assert_eq!(guess_scale(([0.0; 3], [27.0, 15.0, 12.0]), t), Some(25.4));
        assert_eq!(guess_scale(([0.0; 3], [0.7, 0.4, 0.3]), t), Some(1000.0));
        assert_eq!(guess_scale(([0.0; 3], [650.0, 400.0, 300.0]), t), None);
    }

    #[test]
    fn what_sticks_out_of_the_envelope_is_measured() {
        let env = vec![
            Prim::Box {
                min: [0.0; 3],
                max: [10.0; 3],
            },
            Prim::Cyl {
                base: [5.0, 5.0, 10.0],
                axis: [0.0, 0.0, 1.0],
                r: 2.0,
                h: 5.0,
            },
        ];
        let o = outside(
            &env,
            [
                [5.0, 5.0, 14.0],
                [1.0, 1.0, 1.0],
                [5.0, 5.0, 18.0],
                [13.0, 14.0, 10.0],
            ]
            .into_iter(),
        );
        assert_eq!((o.points, o.out), (4, 2));
        assert_eq!(o.worst, 5.0);
        assert_eq!(o.at, [13.0, 14.0, 10.0]);
        assert_eq!(prims_bounds(&env), Some(([0.0; 3], [10.0, 10.0, 15.0])));
    }

    #[test]
    fn the_index_round_trips_and_files_land_in_the_users_folder() {
        let mut a = Model::new(Some("head".into()), "Head \"v2\".step".into());
        a.from = "the maker's download page".into();
        a.scale = 25.4;
        a.rotate = [0.0, 0.0, -90.0];
        a.offset = [1.5, -2.0, 610.0];
        let ms = vec![a, Model::new(None, "whole.stl".into())];
        let text = write(&ms).unwrap();
        assert_eq!(parse(&text).unwrap(), ms);
        assert_eq!(write(&parse(&text).unwrap()).unwrap(), text);
        let d = dir("haas/vf 2").unwrap();
        assert!(
            d.ends_with("forgecad/cam/machines/models/haas_vf_2"),
            "{}",
            d.display()
        );
        assert_eq!(safe(".."), "_..");
    }

    #[test]
    fn copies_get_free_names_and_unused_files_go() {
        let tmp = forge_dirs::temp_dir().join(format!("forge-models-{}", forge_dirs::process_id()));
        let src = tmp.join("src");
        forge_dirs::fs::create_dir_all(&src).unwrap();
        forge_dirs::fs::write(src.join("head.stl"), b"solid x\nendsolid x\n").unwrap();
        let dst = tmp.join("models");
        let n1 = copy_in(&dst, &src.join("head.stl"), &[]).unwrap();
        let m1 = Model::new(Some("head".into()), n1.clone());
        let n2 = copy_in(&dst, &src.join("head.stl"), std::slice::from_ref(&m1)).unwrap();
        assert_eq!((n1.as_str(), n2.as_str()), ("head.stl", "head-2.stl"));
        save_in(&dst, std::slice::from_ref(&m1)).unwrap();
        assert!(forge_dirs::fs::exists(dst.join("head.stl")));
        assert!(!forge_dirs::fs::exists(dst.join("head-2.stl")));
        assert_eq!(load_in(&dst).unwrap(), vec![m1]);
        assert!(copy_in(&dst, &src.join("nope.x_t"), &[]).is_err());
        let _ = forge_dirs::fs::remove_dir_all(&tmp);
    }
}
