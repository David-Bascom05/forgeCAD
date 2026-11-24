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

/// An object, by the model part that holds it and its id there.
type Key = (String, String);

enum Obj {
    /// In millimetres (its part's unit applied).
    Mesh(Vec<[f64; 3]>, Vec<[u32; 3]>),
    Parts(Vec<(Key, M)>),
}

/// The extensions whose required use this reader honours: materials (colours and textures, the
/// geometry unchanged) and production (other model parts, `p:path`; UUIDs). Any other required
/// extension changes what the geometry is (beams, slices, booleans, displacement, encryption): a
/// consumer that does not read it must refuse the file (core 1.4, 2.3.1).
const READ_EXTENSIONS: &[&str] = &[
    "http://schemas.microsoft.com/3dmanufacturing/core/2015/02",
    "http://schemas.microsoft.com/3dmanufacturing/material/2015/02",
    "http://schemas.microsoft.com/3dmanufacturing/production/2015/06",
];

/// The production extension's namespace: `p:path`.
const PRODUCTION: &str = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";

/// Model parts one package may have read (the build's and its components').
pub const MAX_PARTS: usize = 4096;

/// A `p:path` as the zip member it names (absolute in the package, so the leading slash goes).
fn member(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

/// A build item: its object, its placement in millimetres, and where it is (offset, line).
type Item = (Key, M, (usize, usize));

/// What reading a package's model parts has made so far.
struct Parts<'z> {
    zip: &'z Zip<'z>,
    objs: HashMap<Key, Obj>,
    done: std::collections::HashSet<String>,
    triangles: usize,
}

impl Parts<'_> {
    /// The model part at zip member `path`: its objects into `objs`, and its build's items (each its
    /// object's key and placement, in millimetres) when it has a build.
    fn load(&mut self, path: &str) -> Result<Vec<Item>, ReadError> {
        let mut todo = vec![];
        let items = self.load_one(path, &mut todo)?.ok_or(ReadError {
            at: 0,
            line: 0,
            why: format!("{path}: no <build>"),
        })?;
        while let Some(p) = todo.pop() {
            if self.done.contains(&p) {
                continue;
            }
            self.load_one(&p, &mut todo)?;
        }
        Ok(items)
    }

    /// One model part: its objects, and its build's items if it has a build.
    fn load_one(&mut self, path: &str, todo: &mut Vec<String>) -> Result<Option<Vec<Item>>, ReadError> {
        self.done.insert(path.to_string());
        if self.done.len() > MAX_PARTS {
            return bad(format!("past the {MAX_PARTS} model parts read"));
        }
        let xml = self.zip.read(path).ok_or(ReadError {
            at: 0,
            line: 0,
            why: format!("{path}: no such part, or it would not read"),
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
        let fail = |n: roxmltree::Node, why: String| -> ReadError {
            let p = doc.text_pos_at(n.range().start);
            ReadError {
                at: n.range().start,
                line: p.row as usize,
                why: format!("{path}: {why}"),
            }
        };
        let model = doc.root_element();
        if !model.has_tag_name("model") {
            return Err(fail(model, "the root is not a <model>".into()));
        }
        // a required extension this reader does not read: refused, by name
        for prefix in model
            .attribute("requiredextensions")
            .unwrap_or("")
            .split_whitespace()
        {
            let Some(ns) = model.lookup_namespace_uri(Some(prefix)) else {
                return Err(fail(
                    model,
                    format!("required extension {prefix:?} has no namespace"),
                ));
            };
            if !READ_EXTENSIONS.contains(&ns) {
                return Err(fail(
                    model,
                    format!("requires extension {prefix:?} ({ns}), which this reader does not read"),
                ));
            }
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
        // a placement in this part's unit, made millimetres (its translation scaled)
        let place = |n: roxmltree::Node| -> Result<M, ReadError> {
            let mut m = transform(n.attribute("transform")).map_err(|e| fail(n, e.why))?;
            for t in &mut m.0[9..] {
                *t *= scale;
            }
            Ok(m)
        };
        // the part another object is in: this one, or the production extension's path
        let part_of = |n: roxmltree::Node, todo: &mut Vec<String>| -> String {
            match n.attribute((PRODUCTION, "path")) {
                Some(p) => {
                    let m = member(p);
                    todo.push(m.clone());
                    m
                }
                None => path.to_string(),
            }
        };
        let resources = model.children().find(|n| n.has_tag_name("resources"));
        for o in resources
            .iter()
            .flat_map(|r| r.children())
            .filter(|n| n.has_tag_name("object"))
        {
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
                        vs.push([num(v, "x")? * scale, num(v, "y")? * scale, num(v, "z")? * scale]);
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
                            _ => {
                                return Err(fail(
                                    t,
                                    format!("{name} is not one of the {} vertices", vs.len()),
                                ));
                            }
                        }
                    }
                    // core 1.3, 4.1.4: a triangle's three vertices are distinct
                    if c[0] == c[1] || c[1] == c[2] || c[0] == c[2] {
                        return Err(fail(
                            t,
                            format!("a triangle with a vertex twice ({} {} {})", c[0], c[1], c[2]),
                        ));
                    }
                    ts.push(c);
                }
                self.triangles += ts.len();
                if self.triangles > MAX_TRIANGLES {
                    return Err(fail(o, format!("past the {MAX_TRIANGLES} triangles read")));
                }
                Obj::Mesh(vs, ts)
            } else if let Some(cs) = o.children().find(|n| n.has_tag_name("components")) {
                let mut parts = vec![];
                for c in cs.children().filter(|n| n.has_tag_name("component")) {
                    let of = c
                        .attribute("objectid")
                        .ok_or_else(|| fail(c, "a component with no objectid".into()))?;
                    parts.push(((part_of(c, todo), of.to_string()), place(c)?));
                }
                Obj::Parts(parts)
            } else {
                return Err(fail(o, "an object with neither a mesh nor components".into()));
            };
            if self.objs.insert((path.to_string(), id.clone()), obj).is_some() {
                return Err(fail(o, format!("object {id} given twice")));
            }
        }
        let Some(build) = model.children().find(|n| n.has_tag_name("build")) else {
            return Ok(None);
        };
        let mut items = vec![];
        {
            for item in build.children().filter(|n| n.has_tag_name("item")) {
                let id = item
                    .attribute("objectid")
                    .ok_or_else(|| fail(item, "an item with no objectid".into()))?;
                let p = doc.text_pos_at(item.range().start);
                items.push((
                    (part_of(item, todo), id.to_string()),
                    place(item)?,
                    (item.range().start, p.row as usize),
                ));
            }
        }
        Ok(Some(items))
    }
}

/// A 3MF package's build: every item's object, placed, as one mesh in millimetres.
pub fn read(data: &[u8]) -> Result<TriMesh, ReadError> {
    let zip = Zip::new(data).map_err(|e| ReadError {
        at: 0,
        line: 0,
        why: format!("the package: {e}"),
    })?;
    // OPC: a package says its parts' content types
    if !zip.has("[Content_Types].xml") {
        return bad("no [Content_Types].xml: not an OPC package");
    }
    let path = model_path(&zip)?;
    let mut parts = Parts {
        zip: &zip,
        objs: HashMap::new(),
        done: Default::default(),
        triangles: 0,
    };
    let items = parts.load(&path)?;
    let objs = &parts.objs;
    let name = |k: &Key| {
        if k.0 == path {
            format!("object {}", k.1)
        } else {
            format!("object {} of /{}", k.1, k.0)
        }
    };
    // each object's triangles a copy costs, components counted through, so copies of copies are
    // refused before any is made
    fn cost(
        objs: &HashMap<Key, Obj>,
        id: &Key,
        depth: usize,
        memo: &mut HashMap<Key, usize>,
        name: &dyn Fn(&Key) -> String,
    ) -> Result<usize, String> {
        if depth > MAX_NESTING {
            return Err(format!(
                "components nested past {MAX_NESTING} deep (or in a cycle)"
            ));
        }
        if let Some(&c) = memo.get(id) {
            return Ok(c);
        }
        let c = match objs.get(id).ok_or_else(|| format!("no {}", name(id)))? {
            Obj::Mesh(_, ts) => ts.len(),
            Obj::Parts(ps) => {
                let mut s = 0usize;
                for (p, _) in ps {
                    s = s.saturating_add(cost(objs, p, depth + 1, memo, name)?);
                }
                s
            }
        };
        memo.insert(id.clone(), c);
        Ok(c)
    }
    fn put(objs: &HashMap<Key, Obj>, id: &Key, m: &M, out: &mut TriMesh) {
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
    let mut out = TriMesh::default();
    let mut memo = HashMap::new();
    let mut made = 0usize;
    for (id, m, (at, line)) in &items {
        let fail = |why: String| ReadError {
            at: *at,
            line: *line,
            why: format!("{path}: {why}"),
        };
        let c = cost(objs, id, 0, &mut memo, &name).map_err(fail)?;
        made = made.saturating_add(c);
        if made > MAX_TRIANGLES {
            return Err(fail(format!(
                "the build's copies make past the {MAX_TRIANGLES} triangles read"
            )));
        }
        put(objs, id, m, &mut out);
    }
    if out.triangles.is_empty() {
        return bad("the build places no triangles");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CT: &str = "<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/></Types>";

    /// An OPC package of stored members: its content types, then these.
    fn zip(members: &[(&str, &str)]) -> Vec<u8> {
        let mut all = vec![("[Content_Types].xml", CT)];
        all.extend_from_slice(members);
        bare_zip(&all)
    }

    /// A zip of stored members.
    fn bare_zip(members: &[(&str, &str)]) -> Vec<u8> {
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

    /// `zip64_eocd` of research/formats/repro/make.py: a ZIP64 end record and its locator before
    /// the classic one, whose counts and offset are then all ones, as slicers write packages.
    fn as_zip64(d: &[u8]) -> Vec<u8> {
        let e = d.windows(4).rposition(|w| w == [0x50, 0x4b, 5, 6]).unwrap();
        let n = u16::from_le_bytes([d[e + 10], d[e + 11]]) as u64;
        let size = u32::from_le_bytes(d[e + 12..e + 16].try_into().unwrap()) as u64;
        let off = u32::from_le_bytes(d[e + 16..e + 20].try_into().unwrap()) as u64;
        let mut out = d[..e].to_vec();
        out.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        out.extend_from_slice(&44u64.to_le_bytes());
        out.extend_from_slice(&[45, 0, 45, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for v in [n, n, size, off] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(e as u64).to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        let mut tail = d[e..].to_vec();
        tail[4..12].copy_from_slice(&[0xff; 8]);
        tail[12..20].copy_from_slice(&[0xff; 8]);
        out.extend_from_slice(&tail);
        out
    }

    fn one(model: &str) -> Vec<u8> {
        zip(&[("3D/3dmodel.model", model)])
    }

    #[test]
    fn a_zip64_package_reads_as_its_classic_one() {
        // research/formats/repro/make.py's tetra-zip64.3mf: what Bambu Studio and OrcaSlicer write
        let plain = one(&cube("millimeter", "<item objectid=\"1\"/>"));
        let z = as_zip64(&plain);
        let (a, b) = (read(&plain).unwrap(), read(&z).unwrap());
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn an_unknown_required_extension_is_refused_by_name() {
        // make.py's tetra-required-unknown.3mf (3MF core 1.4, 2.3.1; the consortium's N_XXX_0428)
        let m = cube("millimeter", "<item objectid=\"1\"/>").replace(
            "<model unit",
            "<model xmlns:q=\"http://example.com/forge/unknown/2026\" requiredextensions=\"q\" unit",
        );
        let e = read(&one(&m)).unwrap_err();
        assert!(
            e.why.contains("requires extension \"q\"") && e.why.contains("example.com"),
            "{e}"
        );
        // a prefix with no namespace
        let m = cube("millimeter", "<item objectid=\"1\"/>")
            .replace("<model unit", "<model requiredextensions=\"z\" unit");
        assert!(read(&one(&m)).unwrap_err().why.contains("no namespace"));
        // materials, required, are read: they change no geometry
        let m = cube("millimeter", "<item objectid=\"1\"/>").replace(
            "<model unit",
            "<model xmlns:m=\"http://schemas.microsoft.com/3dmanufacturing/material/2015/02\" requiredextensions=\"m\" unit",
        );
        assert!(read(&one(&m)).is_ok());
    }

    #[test]
    fn build_items_and_components_in_other_model_parts_are_read() {
        // make.py's build-item-path.3mf: the root's build places an object of another part, as
        // Bambu Studio's packages do (their components each name a part in 3D/Objects)
        let p = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
        let root = format!(
            "<?xml version=\"1.0\"?><model unit=\"millimeter\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" xmlns:p=\"{p}\" requiredextensions=\"p\"><resources/><build><item objectid=\"1\" p:path=\"/3D/other.model\" transform=\"1 0 0 0 1 0 0 0 1 0 0 3\"/></build></model>"
        );
        let other = cube("centimeter", "");
        let m = read(&zip(&[("3D/3dmodel.model", &root), ("3D/other.model", &other)])).unwrap();
        let s = forge_mesh::read::summary(&m);
        // the other part's unit is its own: a 10 mm cube, lifted 3 mm in the root's
        assert!((s.volume.unwrap() - 1000.0).abs() < 1e-9, "{s:?}");
        assert_eq!(s.bbox, ([0.0, 0.0, 3.0], [10.0, 10.0, 13.0]));
        // a component of the root naming an object in the other part
        let root2 = format!(
            "<?xml version=\"1.0\"?><model unit=\"millimeter\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" xmlns:p=\"{p}\"><resources><object id=\"9\"><components><component objectid=\"1\" p:path=\"/3D/other.model\"/></components></object></resources><build><item objectid=\"9\"/></build></model>"
        );
        let m = read(&zip(&[("3D/3dmodel.model", &root2), ("3D/other.model", &other)])).unwrap();
        assert!((forge_mesh::read::summary(&m).volume.unwrap() - 1000.0).abs() < 1e-9);
        // a part that is not there: named
        let e = read(&zip(&[("3D/3dmodel.model", &root)])).unwrap_err();
        assert!(e.why.contains("3D/other.model"), "{e}");
    }

    #[test]
    fn core_rules_refused() {
        // a triangle with a vertex twice (core 1.3, 4.1.4)
        let m = cube("millimeter", "<item objectid=\"1\"/>")
            .replace("v1=\"0\" v2=\"2\" v3=\"3\"", "v1=\"0\" v2=\"2\" v3=\"0\"");
        assert!(read(&one(&m)).unwrap_err().why.contains("a vertex twice"));
        // no content types: not an OPC package
        let e = read(&bare_zip(&[(
            "3D/3dmodel.model",
            &cube("millimeter", "<item objectid=\"1\"/>"),
        )]))
        .unwrap_err();
        assert!(e.why.contains("Content_Types"), "{e}");
    }
}
