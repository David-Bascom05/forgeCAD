//! 3MF (3MF Consortium, Core Specification 1.4): an OPC package (a ZIP) of a content-types part,
//! the package relationships, and one 3D model part in millimetres. Each object is a mesh of type
//! "model" with its name, vertices shared and triangles facing out; a material, where an object
//! has one, a base material with its display colour; each placement a build item with its
//! transform. Written in one order with numbers in one form, so the same scene gives the same
//! bytes everywhere.

use crate::scene::{IDENTITY, Item, Material, Object, Scene, num};
use crate::zip;
use forge_math::Point3;
use std::fmt::Write as _;

pub const MODEL_PATH: &str = "3D/3dmodel.model";
const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const START_PART: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\
</Types>\n";

fn rels() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Target=\"/{MODEL_PATH}\" Id=\"rel0\" Type=\"{START_PART}\"/>\
</Relationships>\n"
    )
}

/// Text as an XML attribute value or element content.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o += "&amp;",
            '<' => o += "&lt;",
            '>' => o += "&gt;",
            '"' => o += "&quot;",
            '\'' => o += "&apos;",
            // XML 1.0 has no other control characters
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => o.push(' '),
            c => o.push(c),
        }
    }
    o
}

/// The 3D model part: the scene's objects as resources and its items as the build.
pub fn model_xml(scene: &Scene) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"{CORE}\">\n"
    );
    if !scene.title.is_empty() {
        let _ = writeln!(s, " <metadata name=\"Title\">{}</metadata>", esc(&scene.title));
    }
    s += " <metadata name=\"Application\">ForgeCAD</metadata>\n <resources>\n";
    // ids: the materials group 1, then the objects from 2
    let used = scene.objects.iter().any(|o| o.material.is_some());
    let mat_id = 1;
    let first = if used && !scene.materials.is_empty() { 2 } else { 1 };
    if first == 2 {
        let _ = writeln!(s, "  <basematerials id=\"{mat_id}\">");
        for m in &scene.materials {
            let _ = writeln!(
                s,
                "   <base name=\"{}\" displaycolor=\"#{:02X}{:02X}{:02X}\"/>",
                esc(&m.name),
                m.rgb[0],
                m.rgb[1],
                m.rgb[2]
            );
        }
        s += "  </basematerials>\n";
    }
    for (k, o) in scene.objects.iter().enumerate() {
        let _ = write!(s, "  <object id=\"{}\" type=\"model\"", first + k);
        if !o.name.is_empty() {
            let _ = write!(s, " name=\"{}\"", esc(&o.name));
        }
        if first == 2
            && let Some(m) = o.material.filter(|&m| m < scene.materials.len())
        {
            let _ = write!(s, " pid=\"{mat_id}\" pindex=\"{m}\"");
        }
        s += ">\n   <mesh>\n    <vertices>\n";
        for p in &o.positions {
            let _ = writeln!(
                s,
                "     <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
                num(p.x),
                num(p.y),
                num(p.z)
            );
        }
        s += "    </vertices>\n    <triangles>\n";
        for t in &o.triangles {
            let _ = writeln!(
                s,
                "     <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>",
                t[0], t[1], t[2]
            );
        }
        s += "    </triangles>\n   </mesh>\n  </object>\n";
    }
    s += " </resources>\n <build>\n";
    for it in &scene.items {
        let _ = write!(s, "  <item objectid=\"{}\"", first + it.object);
        if it.transform != IDENTITY {
            let m: Vec<String> = it.transform.iter().map(|&x| matrix_num(x)).collect();
            let _ = write!(s, " transform=\"{}\"", m.join(" "));
        }
        if !it.name.is_empty() {
            let _ = write!(s, " partnumber=\"{}\"", esc(&it.name));
        }
        s += "/>\n";
    }
    s += " </build>\n</model>\n";
    s
}

/// A matrix entry: rotations to 1e-12, finer than the positions they move.
fn matrix_num(x: f64) -> String {
    let mut s = format!("{x:.12}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// The scene as a .3mf package.
pub fn to_3mf(scene: &Scene) -> Result<Vec<u8>, zip::ZipError> {
    let model = model_xml(scene);
    let rels = rels();
    zip::write(&[
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        (MODEL_PATH, model.as_bytes()),
    ])
}

// ---- reading ---------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ThreeMfError(pub String);

/// One XML tag: its local name (prefix dropped), its attributes, whether it closes (`</x>`) and
/// whether it is empty (`<x/>`); and the text before it.
struct Tag<'a> {
    name: &'a str,
    attrs: Vec<(&'a str, String)>,
    close: bool,
    empty: bool,
    text: &'a str,
}

fn unesc(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut o = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        o += &rest[..i];
        let j = rest[i..].find(';').map_or(rest.len(), |j| i + j);
        let ent = &rest[i + 1..j.min(rest.len())];
        match ent {
            "amp" => o.push('&'),
            "lt" => o.push('<'),
            "gt" => o.push('>'),
            "quot" => o.push('"'),
            "apos" => o.push('\''),
            e if e.starts_with("#x") => {
                o.extend(u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32))
            }
            e if e.starts_with('#') => o.extend(e[1..].parse().ok().and_then(char::from_u32)),
            _ => {}
        }
        rest = &rest[(j + 1).min(rest.len())..];
    }
    o + rest
}

/// The tags of an XML document in order, comments, declarations and processing instructions
/// skipped: enough XML for a 3MF model part.
fn tags(xml: &str) -> Result<Vec<Tag<'_>>, ThreeMfError> {
    let bad = |w: &str| ThreeMfError(format!("model XML: {w}"));
    let mut out = Vec::new();
    let mut at = 0;
    let b = xml.as_bytes();
    while let Some(i) = xml[at..].find('<').map(|i| at + i) {
        let text = &xml[at..i];
        if xml[i..].starts_with("<!--") {
            at = xml[i..]
                .find("-->")
                .map(|j| i + j + 3)
                .ok_or_else(|| bad("comment"))?;
            continue;
        }
        if xml[i..].starts_with("<?") || xml[i..].starts_with("<!") {
            at = xml[i..]
                .find('>')
                .map(|j| i + j + 1)
                .ok_or_else(|| bad("declaration"))?;
            continue;
        }
        let mut j = i + 1;
        let close = b.get(j) == Some(&b'/');
        if close {
            j += 1;
        }
        let ns = j;
        while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' && b[j] != b'/' {
            j += 1;
        }
        let qname = &xml[ns..j];
        let name = qname.rsplit(':').next().unwrap_or(qname);
        let mut attrs = Vec::new();
        loop {
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            match b.get(j) {
                None => return Err(bad("unclosed tag")),
                Some(b'>') => {
                    j += 1;
                    out.push(Tag {
                        name,
                        attrs,
                        close,
                        empty: false,
                        text,
                    });
                    break;
                }
                Some(b'/') if b.get(j + 1) == Some(&b'>') => {
                    j += 2;
                    out.push(Tag {
                        name,
                        attrs,
                        close,
                        empty: true,
                        text,
                    });
                    break;
                }
                Some(_) => {
                    let k0 = j;
                    while j < b.len() && b[j] != b'=' && !b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    let key = &xml[k0..j];
                    while j < b.len() && (b[j].is_ascii_whitespace() || b[j] == b'=') {
                        j += 1;
                    }
                    let q = *b.get(j).ok_or_else(|| bad("attribute"))?;
                    if q != b'"' && q != b'\'' {
                        return Err(bad("unquoted attribute"));
                    }
                    let v0 = j + 1;
                    let v1 = xml[v0..]
                        .find(q as char)
                        .map(|e| v0 + e)
                        .ok_or_else(|| bad("attribute value"))?;
                    attrs.push((key, unesc(&xml[v0..v1])));
                    j = v1 + 1;
                }
            }
        }
        at = j;
    }
    Ok(out)
}

fn attr<'a>(t: &'a Tag, k: &str) -> Option<&'a str> {
    t.attrs.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
}

/// A 3MF package's scene, in millimetres: its objects (meshes; an object of components is taken
/// apart into its meshes' placements), base materials, and build items. Objects keep their
/// order in the file.
pub fn read_3mf(data: &[u8]) -> Result<Scene, ThreeMfError> {
    let parts = zip::read(data, 1 << 31).map_err(|e| ThreeMfError(e.to_string()))?;
    let get = |name: &str| {
        parts
            .iter()
            .find(|p| p.0.trim_start_matches('/').eq_ignore_ascii_case(name))
            .map(|p| String::from_utf8_lossy(&p.1).into_owned())
    };
    let ct = get("[Content_Types].xml").ok_or(ThreeMfError("no [Content_Types].xml".into()))?;
    if !ct.contains("application/vnd.ms-package.3dmanufacturing-3dmodel+xml") {
        return Err(ThreeMfError("no 3D model content type".into()));
    }
    let rels = get("_rels/.rels").ok_or(ThreeMfError("no _rels/.rels".into()))?;
    let start = tags(&rels)?
        .into_iter()
        .find(|t| t.name == "Relationship" && attr(t, "Type") == Some(START_PART))
        .and_then(|t| attr(&t, "Target").map(str::to_string))
        .ok_or(ThreeMfError("no StartPart relationship".into()))?;
    let model =
        get(start.trim_start_matches('/')).ok_or_else(|| ThreeMfError(format!("no model part {start}")))?;
    read_model(&model)
}

/// A 3D model part's scene.
pub fn read_model(xml: &str) -> Result<Scene, ThreeMfError> {
    let bad = |w: String| ThreeMfError(w);
    let tags = tags(xml)?;
    let mut scene = Scene::default();
    let mut scale = 1.0;
    // resource id -> material base index per group, object index
    let mut groups: Vec<(String, usize)> = Vec::new();
    let mut ids: Vec<(String, usize)> = Vec::new();
    let mut components: Vec<(usize, Placements)> = Vec::new();
    let mut cur: Option<usize> = None;
    let mut in_group = false;
    let mut in_title = false;
    let mut items = Vec::new();
    let num = |t: &Tag, k: &str| -> Result<f64, ThreeMfError> {
        attr(t, k)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .ok_or_else(|| ThreeMfError(format!("<{}> {k}", t.name)))
    };
    let idx = |t: &Tag, k: &str| -> Result<u32, ThreeMfError> {
        attr(t, k)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .ok_or_else(|| ThreeMfError(format!("<{}> {k}", t.name)))
    };
    let matrix = |s: Option<&str>| -> Result<[f64; 12], ThreeMfError> {
        let Some(s) = s else { return Ok(IDENTITY) };
        let v: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        v.try_into()
            .map_err(|_| ThreeMfError(format!("transform \"{s}\"")))
    };
    for t in &tags {
        if in_title && t.close && t.name == "metadata" {
            scene.title = unesc(t.text.trim());
            in_title = false;
        }
        if t.close {
            match t.name {
                "object" => cur = None,
                "basematerials" => in_group = false,
                _ => {}
            }
            continue;
        }
        match t.name {
            "model" => {
                scale = match attr(t, "unit").unwrap_or("millimeter") {
                    "micron" => 1e-3,
                    "millimeter" => 1.0,
                    "centimeter" => 10.0,
                    "inch" => 25.4,
                    "foot" => 304.8,
                    "meter" => 1000.0,
                    u => return Err(bad(format!("unit {u}"))),
                };
                if let Some(r) = attr(t, "requiredextensions").filter(|r| !r.trim().is_empty()) {
                    return Err(bad(format!("required extensions not read: {r}")));
                }
            }
            "metadata" if attr(t, "name") == Some("Title") && !t.empty => in_title = true,
            "basematerials" => {
                groups.push((attr(t, "id").unwrap_or("").to_string(), scene.materials.len()));
                in_group = !t.empty;
            }
            "base" if in_group => {
                let c = attr(t, "displaycolor").unwrap_or("#000000");
                let h = |i: usize| u8::from_str_radix(c.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
                scene.materials.push(Material {
                    name: attr(t, "name").unwrap_or("").to_string(),
                    rgb: [h(1), h(3), h(5)],
                });
            }
            "object" => {
                let k = scene.objects.len();
                ids.push((attr(t, "id").unwrap_or("").to_string(), k));
                let material = match (attr(t, "pid"), attr(t, "pindex")) {
                    (Some(p), Some(i)) => groups
                        .iter()
                        .find(|g| g.0 == p)
                        .and_then(|g| i.parse::<usize>().ok().map(|i| g.1 + i)),
                    _ => None,
                };
                scene.objects.push(Object {
                    name: attr(t, "name").unwrap_or("").to_string(),
                    material,
                    ..Object::default()
                });
                cur = (!t.empty).then_some(k);
            }
            "vertex" => {
                let o = cur.ok_or_else(|| bad("vertex outside an object".into()))?;
                let p = Point3::new(num(t, "x")? * scale, num(t, "y")? * scale, num(t, "z")? * scale);
                scene.objects[o].positions.push(p);
            }
            "triangle" => {
                let o = cur.ok_or_else(|| bad("triangle outside an object".into()))?;
                let tri = [idx(t, "v1")?, idx(t, "v2")?, idx(t, "v3")?];
                let n = scene.objects[o].positions.len() as u32;
                if tri.iter().any(|&v| v >= n) {
                    return Err(bad(format!("triangle {tri:?} past {n} vertices")));
                }
                scene.objects[o].triangles.push(tri);
            }
            "component" => {
                let o = cur.ok_or_else(|| bad("component outside an object".into()))?;
                let m = matrix(attr(t, "transform"))?;
                let r = attr(t, "objectid").unwrap_or("").to_string();
                match components.iter_mut().find(|c| c.0 == o) {
                    Some(c) => c.1.push((r, m)),
                    None => components.push((o, vec![(r, m)])),
                }
            }
            "item" => {
                let r = attr(t, "objectid").unwrap_or("");
                let m = matrix(attr(t, "transform"))?;
                let name = attr(t, "partnumber").unwrap_or("").to_string();
                let object = ids
                    .iter()
                    .find(|i| i.0 == r)
                    .map(|i| i.1)
                    .ok_or_else(|| bad(format!("item of no object {r}")))?;
                expand(&ids, &components, object, m, &name, &mut items, 0)?;
            }
            _ => {}
        }
    }
    scene.items = items;
    if scale != 1.0 {
        for it in &mut scene.items {
            for k in 9..12 {
                it.transform[k] *= scale;
            }
        }
    }
    Ok(scene)
}

/// An object of components' parts: each one's object id and transform.
type Placements = Vec<(String, [f64; 12])>;

/// An item's placements of meshes: the object itself, or each of its components', recursively.
fn expand(
    ids: &[(String, usize)],
    comps: &[(usize, Placements)],
    object: usize,
    m: [f64; 12],
    name: &str,
    items: &mut Vec<Item>,
    depth: usize,
) -> Result<(), ThreeMfError> {
    if depth > 32 {
        return Err(ThreeMfError("components nested too deep".into()));
    }
    let Some(c) = comps.iter().find(|c| c.0 == object) else {
        items.push(Item {
            object,
            transform: m,
            name: name.to_string(),
        });
        return Ok(());
    };
    for (r, cm) in &c.1 {
        let o = ids
            .iter()
            .find(|i| &i.0 == r)
            .map(|i| i.1)
            .ok_or_else(|| ThreeMfError(format!("component of no object {r}")))?;
        expand(ids, comps, o, compose(cm, &m), name, items, depth + 1)?;
    }
    Ok(())
}

/// p * a * b, as one matrix.
fn compose(a: &[f64; 12], b: &[f64; 12]) -> [f64; 12] {
    let mut o = [0.0; 12];
    for r in 0..4 {
        for c in 0..3 {
            let mut v = if r == 3 { b[9 + c] } else { 0.0 };
            for k in 0..3 {
                v += a[r * 3 + k] * b[k * 3 + c];
            }
            o[r * 3 + c] = v;
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra() -> Object {
        Object {
            name: "t & <1>".into(),
            positions: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
            material: Some(0),
            ..Object::default()
        }
    }

    #[test]
    fn round_trip() {
        let mut s = Scene::of_objects(
            "tet",
            vec![tetra()],
            vec![Material {
                name: "Steel".into(),
                rgb: [0x80, 0x81, 0xfe],
            }],
        );
        s.items.push(Item {
            object: 0,
            transform: [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 5.0, 0.0, 0.0],
            name: "second".into(),
        });
        assert!(s.objects[0].check().watertight());
        let bytes = to_3mf(&s).unwrap();
        assert_eq!(bytes, to_3mf(&s).unwrap());
        let back = read_3mf(&bytes).unwrap();
        assert_eq!(back.title, "tet");
        assert_eq!(back.objects.len(), 1);
        assert_eq!(back.objects[0].name, "t & <1>");
        assert_eq!(back.objects[0].positions, s.objects[0].positions);
        assert_eq!(back.objects[0].triangles, s.objects[0].triangles);
        assert_eq!(back.materials, s.materials);
        assert_eq!(back.objects[0].material, Some(0));
        assert_eq!(back.items, s.items);
    }

    #[test]
    fn components_are_taken_apart() {
        let xml = r#"<?xml version="1.0"?><m:model xmlns:m="x" unit="centimeter"><m:resources>
            <m:object id="5"><m:mesh><m:vertices><m:vertex x="0" y="0" z="0"/><m:vertex x="1" y="0" z="0"/>
            <m:vertex x="0" y="1" z="0"/></m:vertices><m:triangles><m:triangle v1="0" v2="1" v3="2"/></m:triangles></m:mesh></m:object>
            <!-- a comment <object> -->
            <m:object id="6"><m:components><m:component objectid="5" transform="1 0 0 0 1 0 0 0 1 1 2 3"/></m:components></m:object>
            </m:resources><m:build><m:item objectid="6"/></m:build></m:model>"#;
        let s = read_model(xml).unwrap();
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].object, 0);
        assert_eq!(&s.items[0].transform[9..], &[10.0, 20.0, 30.0]);
        assert_eq!(s.objects[0].positions[1], Point3::new(10.0, 0.0, 0.0));
    }
}
