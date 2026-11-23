//! A built part or assembly as a mesh file: STL (binary or ASCII), 3MF or OBJ (`forgecad export F
//! stl|3mf|obj`, the app's File > Export). A part gives one object per solid, named by the
//! feature that made it, with its material's name and colour; an assembly one object per part
//! and a 3MF build item per occurrence at its pose, or with `flat` each occurrence an object of
//! its own where it is. STL and OBJ, which have no placements, are always flat.
//!
//! The mesh is the kernel's (`forge_mesh::tessellate`): each B-rep edge cut once and shared by
//! the faces on either side, so every object is closed, manifold and consistently oriented by
//! construction. Each object is checked all the same: closed and oriented (no open edge), facing
//! out (positive volume), and manifold (one fan of triangles round every vertex), and the reply
//! says which.

use crate::Run;
use forge_mesh::scene::{Item, Material, Object, Scene, Solids, frame_matrix};
use forge_mesh::{MeshCheck, ObjOptions, Quality, body_objects};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Stl,
    ThreeMf,
    Obj,
}

impl Format {
    pub fn of(word: &str) -> Option<Format> {
        match word {
            "stl" => Some(Format::Stl),
            "3mf" => Some(Format::ThreeMf),
            "obj" => Some(Format::Obj),
            _ => None,
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Format::Stl => "stl",
            Format::ThreeMf => "3mf",
            Format::Obj => "obj",
        }
    }

    /// What a file dialog calls it.
    pub fn label(self) -> &'static str {
        match self {
            Format::Stl => "STL meshes",
            Format::ThreeMf => "3MF (3D printing)",
            Format::Obj => "Wavefront OBJ",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Stl => "model/stl",
            Format::ThreeMf => "model/3mf",
            Format::Obj => "model/obj",
        }
    }
}

/// The default chord tolerance: the most a triangle strays from the exact surface, in mm.
pub const DEFAULT_TOL: f64 = 0.01;
/// The default angle tolerance: the most one chord turns through, in degrees.
pub const DEFAULT_ANGLE: f64 = 15.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub format: Format,
    /// Chord tolerance (sag), mm.
    pub tol: f64,
    /// Angle tolerance, degrees.
    pub angle: f64,
    /// STL: ASCII instead of binary.
    pub ascii: bool,
    /// OBJ: the exact surface's normal at each corner.
    pub normals: bool,
    /// OBJ: a group per body face.
    pub faces: bool,
    /// 3MF of an assembly: each occurrence an object of its own where it is, not a part placed
    /// by a build item.
    pub flat: bool,
}

impl Options {
    pub fn new(format: Format) -> Options {
        Options {
            format,
            tol: DEFAULT_TOL,
            angle: DEFAULT_ANGLE,
            ascii: false,
            normals: false,
            faces: false,
            flat: false,
        }
    }

    /// `FORMAT [tol MM] [angle DEG] [ascii] [normals] [faces] [flat]`.
    pub fn parse(words: &[String]) -> Result<Options, String> {
        let usage = "export F stl|3mf|obj [tol MM] [angle DEG] [ascii] [normals] [faces] [flat]";
        let mut w = words.iter();
        let f = w
            .next()
            .and_then(|f| Format::of(f))
            .ok_or_else(|| format!("{usage}: no format"))?;
        let mut o = Options::new(f);
        while let Some(x) = w.next() {
            let mut val = |what: &str| -> Result<f64, String> {
                w.next()
                    .and_then(|t| {
                        t.trim_end_matches("mm")
                            .trim_end_matches("deg")
                            .parse::<f64>()
                            .ok()
                    })
                    .filter(|t| t.is_finite() && *t > 0.0)
                    .ok_or_else(|| format!("{x} takes {what}"))
            };
            match x.as_str() {
                "tol" => o.tol = val("a length in mm, over 0")?,
                "angle" => {
                    o.angle = val("degrees, over 0 and up to 90")?;
                    if o.angle > 90.0 {
                        return Err("angle takes degrees, over 0 and up to 90".into());
                    }
                }
                "ascii" if f == Format::Stl => o.ascii = true,
                "binary" if f == Format::Stl => o.ascii = false,
                "normals" if f == Format::Obj => o.normals = true,
                "faces" if f == Format::Obj => o.faces = true,
                "flat" => o.flat = true,
                _ => {
                    return Err(format!(
                        "export: unknown word '{x}' ({usage}; ascii is STL's, normals and faces OBJ's)"
                    ));
                }
            }
        }
        Ok(o)
    }

    pub fn quality(&self) -> Quality {
        Quality {
            chord: self.tol,
            max_angle: self.angle.to_radians(),
        }
    }
}

/// A file written: its bytes, the scene it holds, and each placed object's check.
pub struct Exported {
    pub bytes: Vec<u8>,
    pub scene: Scene,
    pub checks: Vec<MeshCheck>,
}

impl Exported {
    pub fn watertight(&self) -> bool {
        self.checks.iter().all(MeshCheck::watertight)
    }

    pub fn manifold(&self) -> bool {
        self.checks.iter().all(MeshCheck::manifold)
    }

    pub fn triangles(&self) -> usize {
        self.checks.iter().map(|c| c.triangles).sum()
    }

    pub fn volume(&self) -> f64 {
        self.checks.iter().map(|c| c.volume).sum()
    }
}

fn colour(libs: &forge_material::Libraries, name: &str) -> Material {
    Material {
        name: name.to_string(),
        rgb: libs.find(name).map_or([0x80; 3], |m| m.1.colour),
    }
}

fn material_index(mats: &mut Vec<Material>, m: Material) -> usize {
    match mats.iter().position(|x| x.name == m.name) {
        Some(k) => k,
        None => {
            mats.push(m);
            mats.len() - 1
        }
    }
}

/// The run's part or assembly as a scene: `name` is the file's stem, `dir` where its material
/// libraries are. An assembly's run must have been made with `Options::render` (its parts).
pub fn scene(run: &Run, name: &str, dir: Option<&Path>, o: &Options) -> Result<Scene, String> {
    let q = o.quality();
    let tess = |e: forge_mesh::TessError| format!("meshing failed: {}", crate::error::words(&e));
    if let Some(asm) = &run.asm {
        if asm.occ_part.iter().all(Option::is_none) {
            return Err("the assembly has no parts to export (built without its parts' bodies)".into());
        }
        let mut objects: Vec<Object> = Vec::new();
        let mut materials = Vec::new();
        // each part's body meshed once
        let mut of_part: Vec<Option<usize>> = vec![None; asm.parts.len()];
        let mut items = Vec::new();
        for (k, (path, doc, pose)) in asm.occurrences.iter().enumerate() {
            let reference = asm.occ_body.get(k).is_some_and(|b| b.1);
            let Some(Some(p)) = asm.occ_part.get(k) else {
                continue;
            };
            if reference {
                continue;
            }
            let object = match of_part[*p] {
                Some(i) => i,
                None => {
                    // a part file's stem, or an in-file component as `#lid` names it
                    let part_name = Path::new(doc.trim_start_matches('#'))
                        .file_stem()
                        .map_or_else(|| doc.clone(), |s| s.to_string_lossy().into_owned());
                    let by = Solids {
                        name: &part_name,
                        solids: &[],
                    };
                    let pieces = body_objects(&asm.parts[*p], &q, &by, false).map_err(tess)?;
                    // a part is one object, however many pieces its body is in
                    let mut one = Object {
                        name: part_name,
                        ..Object::default()
                    };
                    for x in pieces {
                        let base = one.positions.len() as u32;
                        one.positions.extend(x.positions);
                        one.triangles
                            .extend(x.triangles.iter().map(|t| t.map(|i| i + base)));
                        one.face_of.extend(x.face_of);
                    }
                    let mat = asm.part_docs.get(doc).and_then(|d| {
                        let mut ms = d.solids.iter().filter_map(|s| s.1.as_ref());
                        let first = ms.next()?;
                        Some(Material {
                            name: first.name.clone(),
                            rgb: first.colour,
                        })
                    });
                    one.material = mat.map(|m| material_index(&mut materials, m));
                    objects.push(one);
                    of_part[*p] = Some(objects.len() - 1);
                    objects.len() - 1
                }
            };
            items.push(Item {
                object,
                transform: frame_matrix(pose),
                name: path.clone(),
            });
        }
        let s = Scene {
            title: name.to_string(),
            objects,
            materials,
            items,
        };
        return Ok(if o.flat { s.flatten() } else { s });
    }
    let body = run.body.as_ref().ok_or_else(|| match run.errors.first() {
        Some(e) => format!("the part did not build: {}: {}", e.kind(), e.text()),
        None => "the part has no solid".to_string(),
    })?;
    let solids: Vec<(String, std::ops::Range<u32>)> = run
        .solids
        .iter()
        .map(|s| {
            let id = run
                .feats
                .iter()
                .find(|f| f.idx == s.solid)
                .map_or_else(|| name.to_string(), |f| f.id.clone());
            (id, s.faces.clone())
        })
        .collect();
    let by = Solids {
        name,
        solids: &solids,
    };
    let mut objects = body_objects(body, &q, &by, o.normals).map_err(tess)?;
    let mut materials = Vec::new();
    let libs = std::cell::OnceCell::new();
    for ob in &mut objects {
        let m = run
            .physical
            .as_ref()
            .and_then(|p| p.solids.iter().find(|s| s.id == ob.name))
            .and_then(|s| s.material.clone())
            .or_else(|| run.props.material.clone());
        if let Some(m) = m {
            let libs = libs.get_or_init(|| forge_material::Libraries::cached(dir));
            ob.material = Some(material_index(&mut materials, colour(libs, &m)));
        }
    }
    // one solid is named for the part
    if objects.len() == 1 && run.solids.len() <= 1 {
        objects[0].name = name.to_string();
    }
    Ok(Scene::of_objects(name, objects, materials))
}

/// The scene's file in the format asked for.
pub fn write(scene: &Scene, o: &Options) -> Result<Vec<u8>, String> {
    Ok(match o.format {
        Format::Stl if o.ascii => forge_mesh::scene_ascii_stl(scene).into_bytes(),
        Format::Stl => forge_mesh::scene_binary_stl(scene),
        Format::Obj => forge_mesh::to_obj(
            scene,
            &ObjOptions {
                normals: o.normals,
                face_groups: o.faces,
            },
        )
        .into_bytes(),
        Format::ThreeMf => forge_mesh::to_3mf(scene).map_err(|e| e.to_string())?,
    })
}

/// The run as a mesh file.
pub fn export(run: &Run, name: &str, dir: Option<&Path>, o: &Options) -> Result<Exported, String> {
    let scene = scene(run, name, dir, o)?;
    let bytes = write(&scene, o)?;
    let checks = scene.checks();
    Ok(Exported { bytes, scene, checks })
}
