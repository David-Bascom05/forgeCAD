//! Makers' models of devices, compiled for the checker (forge-workholding-builder's `compile`):
//! for each model, its bodies (`groups`) in the device's own frame and the compiled faces' text
//! (forge_cam::fixture_model's format), built in (data/models.toml, data/check/*.txt).
//!
//! The device's frame for a vise: x along the clamp axis, the fixed jaw's face at x = 0, the
//! moving jaw's at x = `opening` (the jaws as the model has them), y across the jaws centred
//! on them, z up with the bed (the jaw plates' underside) at z = 0.
//!
//! The model files themselves (STEP, the drawn meshes) are in Git LFS under data/models/: the
//! compiled faces are what CAM reads, so a clone without them still checks against the real
//! device.
use crate::{Role, V3, WhError};
use forge_tooling::toml::{self, Val, num, tables, text};
use std::sync::OnceLock;

/// How a body of the device moves when the jaws close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moves {
    Fixed,
    /// With the moving jaw.
    WithJaw,
    /// Not known (the model does not say, nor does the maker): the body is placed both ways,
    /// fixed and with the jaw, and both are checked.
    Unknown,
}

impl Moves {
    pub fn parse(s: &str) -> Option<Moves> {
        match s {
            "fixed" => Some(Moves::Fixed),
            "with_jaw" => Some(Moves::WithJaw),
            "unknown" => Some(Moves::Unknown),
            _ => None,
        }
    }
}

/// One body of a model.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub name: String,
    pub role: Role,
    pub moves: Moves,
    /// Its compiled faces' box in the device's frame (what the checker sees: an enclosure, so
    /// it can reach past the body itself).
    pub min: V3,
    pub max: V3,
    /// Its own box in the device's frame (its mesh grown by the chord, so the body is inside
    /// it): its envelope for drawing, and the device's height and where it sits on the table.
    pub tight_min: V3,
    pub tight_max: V3,
    /// Its drawn mesh (an STL in the device's frame): relative to data/models/ (Git LFS) for a
    /// built-in model, an absolute path for one compiled while running (`mesh_path` resolves
    /// either). Empty when there is none.
    pub mesh: String,
    /// The STEP's solids it is made of (in the reader's order).
    pub solids: Vec<usize>,
}

/// A compiled model.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub name: String,
    /// The moving jaw's face's x in the model (the opening it was drawn at).
    pub opening: f64,
    pub groups: Vec<Group>,
    /// The compiled faces (forge_cam::fixture_model's text).
    pub check: &'static str,
    /// The STEP it was compiled from, relative to data/models/, its sha256, and how.
    pub step: String,
    pub sha256: String,
    pub how: String,
    /// The device's frame in the model's: each device axis as a model axis with its sign
    /// (`x,-z,y`: the device's x is the model's x, its y the model's −z, its z the model's y)...
    pub rot: [(usize, f64); 3],
    /// ... about this point of the model's (mm).
    pub origin: V3,
    /// forge-mesh's chord for faces left to the mesh (mm).
    pub chord: f64,
}

/// `x,-z,y` as (axis, sign) per device axis.
pub fn parse_rot(s: &str) -> Option<[(usize, f64); 3]> {
    let mut out = [(0, 1.0); 3];
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        let (sign, a) = match p.strip_prefix('-') {
            Some(a) => (-1.0, a),
            None => (1.0, p.strip_prefix('+').unwrap_or(p)),
        };
        out[i] = (["x", "y", "z"].iter().position(|x| *x == a)?, sign);
    }
    Some(out)
}

/// A group's box as the compiled text gives it (its `box` line under `group NAME`).
fn group_box(text: &str, group: &str, key: &str) -> Option<(V3, V3)> {
    let mut lines = text.lines();
    lines.find(|l| l.strip_prefix("group ") == Some(group))?;
    let b: Vec<f64> = lines
        .find_map(|l| l.strip_prefix(key))?
        .split_ascii_whitespace()
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    (b.len() == 6).then(|| ([b[0], b[1], b[2]], [b[3], b[4], b[5]]))
}

// every compiled model, by name (build.rs: one per data/check/*.txt, none named in code)
include!(concat!(env!("OUT_DIR"), "/checks.rs"));

/// The compiled faces of each built-in model, by name.
fn check_text(name: &str) -> Option<&'static str> {
    CHECKS.iter().find(|c| c.0 == name).map(|c| c.1)
}

fn parse(src: &str, compiled: bool) -> Result<Vec<Model>, WhError> {
    let bad = |why: String| WhError::Bad {
        record: "data/models.toml".into(),
        field: "model".into(),
        why,
    };
    let t = toml::parse(src).map_err(|e| WhError::Toml {
        file: "data/models.toml".into(),
        line: e.line,
        what: e.what,
    })?;
    let v3 = |g: &toml::Table, k: &str| -> Result<V3, WhError> {
        match g.get(k) {
            Some(Val::Arr(a)) if a.len() == 3 => {
                let mut o = [0.0; 3];
                for (i, x) in a.iter().enumerate() {
                    o[i] = match x {
                        Val::Num(n) => *n,
                        _ => return Err(bad(format!("{k}: numbers"))),
                    };
                }
                Ok(o)
            }
            _ => Err(bad(format!("{k}: three numbers"))),
        }
    };
    let mut out: Vec<Model> = Vec::new();
    for m in tables(&t, "model") {
        let name = text(m, "name")
            .ok_or_else(|| bad("model.name".into()))?
            .to_string();
        out.push(Model {
            check: if compiled {
                check_text(&name).ok_or_else(|| bad(format!("{name}: no compiled faces built in")))?
            } else {
                ""
            },
            opening: num(m, "opening").ok_or_else(|| bad(format!("{name}: opening")))?,
            step: text(m, "step")
                .ok_or_else(|| bad(format!("{name}: step")))?
                .to_string(),
            sha256: text(m, "sha256")
                .ok_or_else(|| bad(format!("{name}: sha256")))?
                .to_string(),
            how: text(m, "how").unwrap_or_default().to_string(),
            rot: text(m, "rot")
                .and_then(parse_rot)
                .ok_or_else(|| bad(format!("{name}: rot (as `x,-z,y`)")))?,
            origin: v3(m, "origin")?,
            chord: num(m, "chord").ok_or_else(|| bad(format!("{name}: chord")))?,
            name,
            groups: vec![],
        });
    }
    for g in tables(&t, "group") {
        let model = text(g, "model")
            .ok_or_else(|| bad("group.model".into()))?
            .to_string();
        let name = text(g, "name")
            .ok_or_else(|| bad("group.name".into()))?
            .to_string();
        let role = text(g, "role")
            .and_then(Role::parse)
            .ok_or_else(|| bad(format!("{model}/{name}: role")))?;
        let moves = text(g, "moves")
            .and_then(Moves::parse)
            .ok_or_else(|| bad(format!("{model}/{name}: moves (fixed, with_jaw or unknown)")))?;
        let boxes = |key: &str| -> Result<(V3, V3), WhError> {
            if !compiled {
                return Ok(([0.0; 3], [0.0; 3]));
            }
            out.iter()
                .find(|m| m.name == model)
                .and_then(|m| group_box(m.check, &name, key))
                .ok_or_else(|| {
                    bad(format!(
                        "{model}/{name}: no `{key}` in the compiled faces (compile it)"
                    ))
                })
        };
        let (min, max) = boxes("box ")?;
        let (tight_min, tight_max) = boxes("tight ")?;
        let gr = Group {
            role,
            moves,
            min,
            max,
            tight_min,
            tight_max,
            mesh: text(g, "mesh").unwrap_or_default().to_string(),
            solids: toml::nums(g, "solids")
                .ok_or_else(|| bad(format!("{model}/{name}: solids")))?
                .into_iter()
                .map(|x| x as usize)
                .collect(),
            name,
        };
        out.iter_mut()
            .find(|m| m.name == model)
            .ok_or_else(|| bad(format!("group of no model `{model}`")))?
            .groups
            .push(gr);
    }
    Ok(out)
}

/// The built-in models.
pub fn builtin() -> &'static [Model] {
    static M: OnceLock<Vec<Model>> = OnceLock::new();
    M.get_or_init(|| parse(SPECS, true).expect("data/models.toml"))
}

/// The built-in models' specs (data/models.toml) as text.
pub const SPECS: &str = include_str!("../data/models.toml");

/// The models' specs alone, before they are compiled: what to compile (no faces, no boxes).
pub fn specs(src: &str) -> Result<Vec<Model>, WhError> {
    parse(src, false)
}

/// Where a group's drawn mesh is, if it has one.
pub fn mesh_path(g: &Group) -> Option<std::path::PathBuf> {
    if g.mesh.is_empty() {
        return None;
    }
    let p = std::path::Path::new(&g.mesh);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/models")
            .join(p)
    })
}

/// Models compiled while running (a user's own imported fixture), each kept once, by name.
fn runtime() -> &'static std::sync::Mutex<Vec<&'static Model>> {
    static R: OnceLock<std::sync::Mutex<Vec<&'static Model>>> = OnceLock::new();
    R.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// A model by name: built in, or compiled while running.
pub fn get(name: &str) -> Option<&'static Model> {
    builtin()
        .iter()
        .find(|m| m.name == name)
        .or_else(|| runtime().lock().ok()?.iter().copied().find(|m| m.name == name))
}

/// Keeps a model compiled while running: its spec (as `specs` reads one, its groups' boxes
/// read from the compiled text) with its compiled faces. A model of that name already kept is
/// given back (a name is its content's hash, so the same file is compiled once a run). Each is
/// kept for the life of the process: they are few, one per distinct file a job names.
pub fn register(mut m: Model, check: String) -> Result<&'static Model, WhError> {
    if let Some(k) = get(&m.name) {
        return Ok(k);
    }
    let who = m.name.clone();
    let bad = |why: String| WhError::Bad {
        record: who.clone(),
        field: "model".into(),
        why,
    };
    if check.lines().next() != Some("forge-workholding check 1") {
        return Err(bad("not a compiled workholding model".into()));
    }
    for g in &mut m.groups {
        let (a, b) =
            group_box(&check, &g.name, "box ").ok_or_else(|| bad(format!("{}: not compiled", g.name)))?;
        let (c, d) =
            group_box(&check, &g.name, "tight ").ok_or_else(|| bad(format!("{}: not compiled", g.name)))?;
        (g.min, g.max, g.tight_min, g.tight_max) = (a, b, c, d);
    }
    m.check = Box::leak(check.into_boxed_str());
    let k: &'static Model = Box::leak(Box::new(m));
    let mut r = runtime()
        .lock()
        .map_err(|_| bad("the model list is poisoned".into()))?;
    if let Some(prev) = r.iter().copied().find(|x| x.name == k.name) {
        return Ok(prev);
    }
    r.push(k);
    Ok(k)
}
