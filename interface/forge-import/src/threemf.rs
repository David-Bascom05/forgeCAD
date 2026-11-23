//! 3MF (3D Manufacturing Format, core specification 1.3): the model's meshes, placed as its build
//! says, as one triangle mesh in millimetres. A mesh for reference and measurement, not a boundary
//! representation (`forge_mesh::read`). A file is untrusted: a zip whose members are capped before
//! they inflate (`zip.rs`), components nested at most `MAX_NESTING` deep, and no more triangles
//! made, copies included, than `forge_mesh::read::MAX_TRIANGLES`.

use crate::zip::Zip;
use forge_math::Point3;
use forge_mesh::TriMesh;
use forge_mesh::read::{MAX_TRIANGLES, ReadError};
use std::collections::HashMap;

/// Components inside components.
pub const MAX_NESTING: usize = 16;

fn bad<T>(why: impl Into<String>) -> Result<T, ReadError> {
    Err(ReadError {
        at: 0,
        line: 0,
        why: why.into(),
    })
}

/// A 3MF transform: x' = x m00 + y m10 + z m20 + m30, and so on (row vectors, the spec's order).
#[derive(Clone, Copy)]
struct M([f64; 12]);

impl M {
    const ID: M = M([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let m = &self.0;
        [
            p[0] * m[0] + p[1] * m[3] + p[2] * m[6] + m[9],
            p[0] * m[1] + p[1] * m[4] + p[2] * m[7] + m[10],
            p[0] * m[2] + p[1] * m[5] + p[2] * m[8] + m[11],
        ]
    }
    /// `self` then `o`.
    fn then(&self, o: &M) -> M {
        let a = &self.0;
        let r = |i: usize| o.apply([a[3 * i], a[3 * i + 1], a[3 * i + 2]]);
        let lin = |v: [f64; 3]| {
            let t = o.apply(v);
            [t[0] - o.0[9], t[1] - o.0[10], t[2] - o.0[11]]
        };
        let (x, y, z) = (
            lin([a[0], a[1], a[2]]),
            lin([a[3], a[4], a[5]]),
            lin([a[6], a[7], a[8]]),
        );
        let t = r(3);
        M([
            x[0], x[1], x[2], y[0], y[1], y[2], z[0], z[1], z[2], t[0], t[1], t[2],
        ])
    }
    /// Whether it turns the mesh inside out (a mirror): then each triangle's corners go the other way.
    fn mirrors(&self) -> bool {
        let m = &self.0;
        let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]);
        det < 0.0
    }
}

fn transform(s: Option<&str>) -> Result<M, ReadError> {
    let Some(s) = s else { return Ok(M::ID) };
    let v: Vec<f64> = s
        .split_whitespace()
        .filter_map(|w| w.parse::<f64>().ok())
        .collect();
    if v.len() != 12 || v.iter().any(|x| !x.is_finite()) {
        return bad(format!("transform {s:?}: twelve finite numbers"));
    }
    let mut m = [0.0; 12];
    m.copy_from_slice(&v);
    Ok(M(m))
}

/// The model file the package's relationships name, else the usual one.
fn model_path(zip: &Zip) -> Result<String, ReadError> {
    if let Some(rels) = zip.read("_rels/.rels").and_then(|r| String::from_utf8(r).ok())
        && let Ok(doc) = roxmltree::Document::parse(&rels)
    {
        for r in doc.descendants().filter(|n| n.has_tag_name("Relationship")) {
            if r.attribute("Type").is_some_and(|t| t.ends_with("/3dmodel"))
                && let Some(t) = r.attribute("Target")
            {
                let t = t.trim_start_matches('/');
                if zip.has(t) {
                    return Ok(t.to_string());
                }
            }
        }
    }
    if zip.has("3D/3dmodel.model") {
        return Ok("3D/3dmodel.model".into());
    }
    bad("no 3D model in the package (_rels/.rels names none, and there is no 3D/3dmodel.model)")
}

enum Obj {
    Mesh(Vec<[f64; 3]>, Vec<[u32; 3]>),
    Parts(Vec<(String, M)>),
}

/// A 3MF package's build: every item's object, placed, as one mesh in millimetres.
pub fn read(data: &[u8]) -> Result<TriMesh, ReadError> {
    let zip = Zip::new(data).ok_or(ReadError {
        at: 0,
        line: 0,
        why: "not a zip package (or one whose members would inflate past the caps)".into(),
    })?;
    let path = model_path(&zip)?;
    let xml = zip.read(&path).ok_or(ReadError {
        at: 0,
        line: 0,
        why: format!("{path} would not read"),
    })?;
    let text = String::from_utf8(xml).map_err(|e| ReadError {
        at: e.utf8_error().valid_up_to(),
        line: 0,
        why: format!("{path} is not UTF-8"),
    })?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| {
        let p = e.pos();
        ReadError {
            at: 0,
            line: p.row as usize,
            why: format!("{path}: {e}"),
        }
    })?;
    let at = |n: roxmltree::Node| -> (usize, usize) {
        let p = doc.text_pos_at(n.range().start);
        (n.range().start, p.row as usize)
    };
    let fail = |n: roxmltree::Node, why: String| -> ReadError {
        let (a, l) = at(n);
        ReadError {
            at: a,
            line: l,
            why: format!("{path}: {why}"),
        }
    };
    let model = doc.root_element();
    if !model.has_tag_name("model") {
        return Err(fail(model, "the root is not a <model>".into()));
    }
    let scale = match model.attribute("unit").unwrap_or("millimeter") {
        "micron" => 1e-3,
        "millimeter" => 1.0,
        "centimeter" => 10.0,
        "inch" => 25.4,
        "foot" => 304.8,
        "meter" => 1000.0,
        u => return Err(fail(model, format!("unit {u:?} is not one of 3MF's"))),
    };
    let num = |n: roxmltree::Node, k: &str| -> Result<f64, ReadError> {
        n.attribute(k)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .ok_or_else(|| fail(n, format!("<{}> needs a finite {k}", n.tag_name().name())))
    };
    let mut objs: HashMap<String, Obj> = HashMap::new();
    let mut total = 0usize;
    for o in model.descendants().filter(|n| n.has_tag_name("object")) {
        let id = o
            .attribute("id")
            .ok_or_else(|| fail(o, "an object with no id".into()))?
            .to_string();
        let obj = if let Some(mesh) = o.children().find(|n| n.has_tag_name("mesh")) {
            let mut vs = vec![];
            let mut ts = vec![];
            for v in mesh
                .children()
                .filter(|n| n.has_tag_name("vertices"))
                .flat_map(|n| n.children())
            {
                if v.has_tag_name("vertex") {
                    vs.push([num(v, "x")?, num(v, "y")?, num(v, "z")?]);
                }
            }
            for t in mesh
                .children()
                .filter(|n| n.has_tag_name("triangles"))
                .flat_map(|n| n.children())
            {
                if !t.has_tag_name("triangle") {
                    continue;
                }
                let mut c = [0u32; 3];
                for (k, name) in ["v1", "v2", "v3"].iter().enumerate() {
                    let i = t.attribute(*name).and_then(|v| v.trim().parse::<usize>().ok());
                    match i {
                        Some(i) if i < vs.len() => c[k] = i as u32,
                        _ => return Err(fail(t, format!("{name} is not one of the {} vertices", vs.len()))),
                    }
                }
                ts.push(c);
            }
            total += ts.len();
            if total > MAX_TRIANGLES {
                return Err(fail(o, format!("past the {MAX_TRIANGLES} triangles read")));
            }
            Obj::Mesh(vs, ts)
        } else if let Some(cs) = o.children().find(|n| n.has_tag_name("components")) {
            let mut parts = vec![];
            for c in cs.children().filter(|n| n.has_tag_name("component")) {
                if c.attributes().any(|a| a.name() == "path") {
                    return Err(fail(
                        c,
                        "a component in another model file (production extension) is not read".into(),
                    ));
                }
                let of = c
                    .attribute("objectid")
                    .ok_or_else(|| fail(c, "a component with no objectid".into()))?;
                parts.push((
                    of.to_string(),
                    transform(c.attribute("transform")).map_err(|e| fail(c, e.why))?,
                ));
            }
            Obj::Parts(parts)
        } else {
            return Err(fail(o, "an object with neither a mesh nor components".into()));
        };
        if objs.insert(id.clone(), obj).is_some() {
            return Err(fail(o, format!("object {id} given twice")));
        }
    }
    let build = model
        .children()
        .find(|n| n.has_tag_name("build"))
        .ok_or_else(|| fail(model, "no <build>".into()))?;
    let mut out = TriMesh::default();
    // each object's triangles a copy costs, components counted through, so copies of copies are
    // refused before any is made
    fn cost(
        objs: &HashMap<String, Obj>,
        id: &str,
        depth: usize,
        memo: &mut HashMap<String, usize>,
    ) -> Result<usize, String> {
        if depth > MAX_NESTING {
            return Err(format!(
                "components nested past {MAX_NESTING} deep (or in a cycle)"
            ));
        }
        if let Some(&c) = memo.get(id) {
            return Ok(c);
        }
        let c = match objs.get(id).ok_or(format!("no object {id}"))? {
            Obj::Mesh(_, ts) => ts.len(),
            Obj::Parts(ps) => {
                let mut s = 0usize;
                for (p, _) in ps {
                    s = s.saturating_add(cost(objs, p, depth + 1, memo)?);
                }
                s
            }
        };
        memo.insert(id.to_string(), c);
        Ok(c)
    }
    fn put(objs: &HashMap<String, Obj>, id: &str, m: &M, out: &mut TriMesh) {
        match &objs[id] {
            Obj::Mesh(vs, ts) => {
                let base = out.positions.len() as u32;
                for &v in vs {
                    let p = m.apply(v);
                    out.add_vertex(Point3::new(p[0], p[1], p[2]));
                }
                let flip = m.mirrors();
                for t in ts {
                    let [a, b, c] = t.map(|i| base + i);
                    out.triangles.push(if flip { [a, c, b] } else { [a, b, c] });
                }
            }
            Obj::Parts(ps) => {
                for (p, pm) in ps {
                    put(objs, p, &pm.then(m), out);
                }
            }
        }
    }
    let mut memo = HashMap::new();
    let mut made = 0usize;
    let scale_m = M([scale, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0]);
    for item in build.children().filter(|n| n.has_tag_name("item")) {
        let id = item
            .attribute("objectid")
            .ok_or_else(|| fail(item, "an item with no objectid".into()))?;
        let c = cost(&objs, id, 0, &mut memo).map_err(|e| fail(item, e))?;
        made = made.saturating_add(c);
        if made > MAX_TRIANGLES {
            return Err(fail(
                item,
                format!("the build's copies make past the {MAX_TRIANGLES} triangles read"),
            ));
        }
        let m = transform(item.attribute("transform")).map_err(|e| fail(item, e.why))?;
        put(&objs, id, &m.then(&scale_m), &mut out);
    }
    if out.triangles.is_empty() {
        return bad("the build places no triangles");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip of stored members.
    fn zip(members: &[(&str, &str)]) -> Vec<u8> {
        let (mut out, mut dir) = (vec![], vec![]);
        let le16 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u16).to_le_bytes());
        let le32 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u32).to_le_bytes());
        for (name, data) in members {
            let off = out.len();
            le32(&mut out, 0x0403_4b50);
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            le32(&mut out, data.len());
            le32(&mut out, data.len());
            le16(&mut out, name.len());
            le16(&mut out, 0);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data.as_bytes());
            le32(&mut dir, 0x0201_4b50);
            dir.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            le32(&mut dir, data.len());
            le32(&mut dir, data.len());
            le16(&mut dir, name.len());
            dir.extend_from_slice(&[0; 12]);
            le32(&mut dir, off);
            dir.extend_from_slice(name.as_bytes());
        }
        let at = out.len();
        out.extend_from_slice(&dir);
        le32(&mut out, 0x0605_4b50);
        out.extend_from_slice(&[0; 4]);
        le16(&mut out, members.len());
        le16(&mut out, members.len());
        le32(&mut out, dir.len());
        le32(&mut out, at);
        le16(&mut out, 0);
        out
    }

    fn cube(unit: &str, build: &str) -> String {
        let vs: String = (0..8)
            .map(|i| {
                format!(
                    "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
                    i & 1,
                    (i >> 1) & 1,
                    (i >> 2) & 1
                )
            })
            .collect();
        // outward, counter-clockwise
        let ts = [
            [0, 2, 3],
            [0, 3, 1],
            [4, 5, 7],
            [4, 7, 6],
            [0, 1, 5],
            [0, 5, 4],
            [2, 6, 7],
            [2, 7, 3],
            [0, 4, 6],
            [0, 6, 2],
            [1, 3, 7],
            [1, 7, 5],
        ];
        let ts: String = ts
            .iter()
            .map(|t| format!("<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]))
            .collect();
        format!(
            "<?xml version=\"1.0\"?><model unit=\"{unit}\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources><object id=\"1\" type=\"model\"><mesh><vertices>{vs}</vertices><triangles>{ts}</triangles></mesh></object><object id=\"2\"><components><component objectid=\"1\" transform=\"1 0 0 0 1 0 0 0 1 5 0 0\"/><component objectid=\"1\" transform=\"-1 0 0 0 1 0 0 0 1 0 0 0\"/></components></object></resources><build>{build}</build></model>"
        )
    }

    #[test]
    fn a_cube_its_copies_and_its_unit() {
        let rels = "<Relationships><Relationship Target=\"/3D/3dmodel.model\" Id=\"r\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/></Relationships>";
        let one = read(&zip(&[
            ("_rels/.rels", rels),
            ("3D/3dmodel.model", &cube("millimeter", "<item objectid=\"1\"/>")),
        ]))
        .unwrap();
        let s = forge_mesh::read::summary(&one);
        assert!(s.closed && (s.volume.unwrap() - 1.0).abs() < 1e-12, "{s:?}");
        // two copies, one mirrored: both face out
        let two = read(&zip(&[(
            "3D/3dmodel.model",
            &cube("centimeter", "<item objectid=\"2\"/>"),
        )]))
        .unwrap();
        let s = forge_mesh::read::summary(&two);
        assert!((s.volume.unwrap() - 2000.0).abs() < 1e-9, "{s:?}");
        assert_eq!(s.bbox, ([-10.0, 0.0, 0.0], [60.0, 10.0, 10.0]));
    }

    #[test]
    fn what_is_wrong_is_said() {
        let bad_index = cube("millimeter", "<item objectid=\"1\"/>").replace("v3=\"6\"", "v3=\"99\"");
        let e = read(&zip(&[("3D/3dmodel.model", &bad_index)])).unwrap_err();
        assert!(e.why.contains("v3") && e.line > 0, "{e}");
        let cycle = cube("millimeter", "<item objectid=\"3\"/>").replace(
            "</resources>",
            "<object id=\"3\"><components><component objectid=\"3\"/></components></object></resources>",
        );
        assert!(
            read(&zip(&[("3D/3dmodel.model", &cycle)]))
                .unwrap_err()
                .why
                .contains("nested")
        );
        assert!(read(b"not a zip").is_err());
    }
}
