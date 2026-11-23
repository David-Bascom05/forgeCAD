//! Wavefront OBJ: every item where it is placed (OBJ has no placements), an `o` and a `g` per
//! object, vertices shared (`v` once per vertex, indices counted across the file), and optionally
//! the exact surface's normals per corner (`vn`, `f v//vn`) and a group per body face.

use crate::scene::{Object, Scene, num};
use forge_math::{Point3, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ObjOptions {
    /// `vn` per corner from the exact surface (smooth shading across a curved face, sharp at its
    /// edges).
    pub normals: bool,
    /// A `g` per body face (`g NAME/f12`), as well as the object.
    pub face_groups: bool,
}

fn clean(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if s.is_empty() { "object".into() } else { s }
}

pub fn to_obj(scene: &Scene, opt: &ObjOptions) -> String {
    let flat = scene.flatten();
    let mut s = String::from("# ForgeCAD mesh, millimetres\n");
    if !scene.title.is_empty() {
        s += &format!("# {}\n", scene.title.replace('\n', " "));
    }
    let (mut v0, mut n0) = (1usize, 1usize);
    for o in &flat.objects {
        let name = clean(&o.name);
        s += &format!("o {name}\ng {name}\n");
        if let Some(m) = o.material.and_then(|m| scene.materials.get(m)) {
            s += &format!("usemtl {}\n", clean(&m.name));
        }
        for p in &o.positions {
            s += &format!("v {} {} {}\n", num(p.x), num(p.y), num(p.z));
        }
        let normals = opt.normals && o.corner_normals.len() == o.triangles.len();
        if normals {
            for n in &o.normals {
                s += &format!("vn {} {} {}\n", num(n.x), num(n.y), num(n.z));
            }
        }
        let mut face = None;
        for (k, t) in o.triangles.iter().enumerate() {
            if opt.face_groups
                && let Some(&f) = o.face_of.get(k)
                && face != Some(f)
            {
                face = Some(f);
                s += &format!("g {name} {name}/f{f}\n");
            }
            if normals {
                let c = o.corner_normals[k];
                s += &format!(
                    "f {}//{} {}//{} {}//{}\n",
                    t[0] as usize + v0,
                    c[0] as usize + n0,
                    t[1] as usize + v0,
                    c[1] as usize + n0,
                    t[2] as usize + v0,
                    c[2] as usize + n0
                );
            } else {
                s += &format!(
                    "f {} {} {}\n",
                    t[0] as usize + v0,
                    t[1] as usize + v0,
                    t[2] as usize + v0
                );
            }
        }
        v0 += o.positions.len();
        if normals {
            n0 += o.normals.len();
        }
    }
    s
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjError(pub String);

/// An OBJ file's objects: one per `o` (or the whole file, if it has none), each with the vertices
/// its faces use, in the order they first do; polygons fanned into triangles. Normals are read
/// per corner where given.
pub fn read_obj(text: &str) -> Result<Vec<Object>, ObjError> {
    let mut v: Vec<Point3> = Vec::new();
    let mut vn: Vec<Vec3> = Vec::new();
    let mut out: Vec<Object> = Vec::new();
    // per object: the file's vertex and normal indices to its own
    let mut map_v: std::collections::HashMap<usize, u32> = Default::default();
    let mut map_n: std::collections::HashMap<usize, u32> = Default::default();
    let bad = |l: usize, w: &str| ObjError(format!("line {l}: {w}"));
    for (l, line) in text.lines().enumerate() {
        let l = l + 1;
        let mut w = line.split_whitespace();
        let nums = |w: std::str::SplitWhitespace| -> Result<Vec<f64>, ObjError> {
            w.map(|t| t.parse::<f64>().map_err(|_| bad(l, "a number")))
                .collect()
        };
        match w.next() {
            Some("v") => {
                let x = nums(w)?;
                if x.len() < 3 {
                    return Err(bad(l, "v takes x y z"));
                }
                v.push(Point3::new(x[0], x[1], x[2]));
            }
            Some("vn") => {
                let x = nums(w)?;
                if x.len() != 3 {
                    return Err(bad(l, "vn takes x y z"));
                }
                vn.push(Vec3::new(x[0], x[1], x[2]));
            }
            Some("o") => {
                out.push(Object {
                    name: w.collect::<Vec<_>>().join(" "),
                    ..Object::default()
                });
                map_v.clear();
                map_n.clear();
            }
            Some("f") => {
                if out.is_empty() {
                    out.push(Object::default());
                }
                let o = out.last_mut().expect("an object");
                let mut corners = Vec::new();
                for c in w {
                    let mut parts = c.split('/');
                    let idx = |s: Option<&str>, n: usize| -> Result<Option<usize>, ObjError> {
                        match s.filter(|s| !s.is_empty()) {
                            None => Ok(None),
                            Some(s) => {
                                let i: i64 = s.parse().map_err(|_| bad(l, "an index"))?;
                                let k = if i < 0 { n as i64 + i } else { i - 1 };
                                if k < 0 || k as usize >= n {
                                    return Err(bad(l, "index out of range"));
                                }
                                Ok(Some(k as usize))
                            }
                        }
                    };
                    let vi = idx(parts.next(), v.len())?.ok_or_else(|| bad(l, "a vertex index"))?;
                    let _ = parts.next();
                    let ni = idx(parts.next(), vn.len())?;
                    let a = *map_v.entry(vi).or_insert_with(|| {
                        o.positions.push(v[vi]);
                        o.positions.len() as u32 - 1
                    });
                    let b = ni.map(|ni| {
                        *map_n.entry(ni).or_insert_with(|| {
                            o.normals.push(vn[ni]);
                            o.normals.len() as u32 - 1
                        })
                    });
                    corners.push((a, b));
                }
                if corners.len() < 3 {
                    return Err(bad(l, "a face of fewer than 3 corners"));
                }
                for k in 1..corners.len() - 1 {
                    let (a, b, c) = (corners[0], corners[k], corners[k + 1]);
                    o.triangles.push([a.0, b.0, c.0]);
                    if let (Some(x), Some(y), Some(z)) = (a.1, b.1, c.1) {
                        o.corner_normals.push([x, y, z]);
                    }
                }
            }
            _ => {}
        }
    }
    for o in &mut out {
        if o.corner_normals.len() != o.triangles.len() {
            o.corner_normals.clear();
            o.normals.clear();
        }
    }
    out.retain(|o| !o.triangles.is_empty());
    Ok(out)
}
