//! Zips taken apart and packed again, so a FreeCAD file's members can be made wrong one at a time
//! while the archive around them stays readable; and small FreeCAD documents as seeds.

/// One member: name, whether it was deflated, its bytes.
pub type Member = (String, bool, Vec<u8>);

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// The members of a zip, by its central directory; None where it does not read as one.
pub fn unpack(b: &[u8]) -> Option<Vec<Member>> {
    let e = (0..b.len().saturating_sub(21))
        .rev()
        .find(|&i| b[i..].starts_with(b"PK\x05\x06"))?;
    let n = u16_at(b, e + 10)?;
    let mut at = u32_at(b, e + 16)?;
    let mut out = vec![];
    for _ in 0..n.min(4096) {
        if !b.get(at..)?.starts_with(b"PK\x01\x02") {
            return None;
        }
        let method = u16_at(b, at + 10)?;
        let csize = u32_at(b, at + 20)?;
        let (nl, xl, cl) = (u16_at(b, at + 28)?, u16_at(b, at + 30)?, u16_at(b, at + 32)?);
        let lho = u32_at(b, at + 42)?;
        let name = String::from_utf8_lossy(b.get(at + 46..at + 46 + nl)?).into_owned();
        let (lnl, lxl) = (u16_at(b, lho + 26)?, u16_at(b, lho + 28)?);
        let data = b.get(lho + 30 + lnl + lxl..lho + 30 + lnl + lxl + csize)?;
        let raw = match method {
            0 => data.to_vec(),
            8 => forge_git::zlib::inflate(data, 256 << 20).ok()?,
            _ => return None,
        };
        out.push((name, method == 8, raw));
        at += 46 + nl + xl + cl;
    }
    Some(out)
}

/// A zip of the members, each stored or deflated as marked. No CRC: the reader does not check one.
pub fn pack(members: &[Member]) -> Vec<u8> {
    let (mut out, mut dir) = (vec![], vec![]);
    let le16 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u16).to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u32).to_le_bytes());
    for (name, deflated, data) in members {
        let off = out.len();
        let method = if *deflated { 8 } else { 0 };
        let body = if *deflated {
            forge_git::zlib::deflate(data)
        } else {
            data.clone()
        };
        le32(&mut out, 0x0403_4b50);
        out.extend_from_slice(&[20, 0, 0, 0, method, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        le32(&mut out, body.len());
        le32(&mut out, data.len());
        le16(&mut out, name.len());
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&body);
        le32(&mut dir, 0x0201_4b50);
        dir.extend_from_slice(&[20, 0, 20, 0, 0, 0, method, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        le32(&mut dir, body.len());
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

fn prop(k: &str, v: &str) -> String {
    format!("<Property name=\"{k}\" type=\"x\">{v}</Property>")
}

fn object(name: &str, props: &[(&str, String)]) -> String {
    let ps: String = props.iter().map(|(k, v)| prop(k, v)).collect();
    format!(
        "<Object name=\"{name}\"><Properties Count=\"{}\">{ps}</Properties></Object>",
        props.len()
    )
}

fn line(a: (f64, f64), b: (f64, f64), con: u8) -> String {
    format!(
        "<Geometry type=\"Part::GeomLineSegment\"><LineSegment StartX=\"{}\" StartY=\"{}\" StartZ=\"0\" EndX=\"{}\" EndY=\"{}\" EndZ=\"0\"/><Construction value=\"{con}\"/></Geometry>",
        a.0, a.1, b.0, b.1
    )
}

fn circle(c: (f64, f64), r: f64) -> String {
    format!(
        "<Geometry type=\"Part::GeomCircle\"><Circle CenterX=\"{}\" CenterY=\"{}\" CenterZ=\"0\" NormalX=\"0\" NormalY=\"0\" NormalZ=\"1\" Radius=\"{r}\"/><Construction value=\"0\"/></Geometry>",
        c.0, c.1
    )
}

fn arc(c: (f64, f64), r: f64, a0: f64, a1: f64) -> String {
    format!(
        "<Geometry type=\"Part::GeomArcOfCircle\"><ArcOfCircle CenterX=\"{}\" CenterY=\"{}\" CenterZ=\"0\" NormalX=\"0\" NormalY=\"0\" NormalZ=\"1\" AngleXU=\"0\" Radius=\"{r}\" StartAngle=\"{a0:?}\" EndAngle=\"{a1:?}\"/><Construction value=\"0\"/></Geometry>",
        c.0, c.1
    )
}

fn geo(g: &[String]) -> String {
    format!(
        "<GeometryList count=\"{}\">{}</GeometryList>",
        g.len(),
        g.concat()
    )
}

fn place(z: f64) -> String {
    format!(
        "<PropertyPlacement Px=\"0\" Py=\"0\" Pz=\"{z}\" Q0=\"0\" Q1=\"0\" Q2=\"0\" Q3=\"1\" A=\"0\" Ox=\"0\" Oy=\"0\" Oz=\"1\"/>"
    )
}

type Obj = (&'static str, &'static str, Vec<(&'static str, String)>);

fn doc(objs: &[Obj]) -> String {
    let types: String = objs
        .iter()
        .map(|(n, t, _)| format!("<Object type=\"{t}\" name=\"{n}\"/>"))
        .collect();
    let data: String = objs.iter().map(|(n, _, p)| object(n, p)).collect();
    format!(
        "<?xml version='1.0' encoding='utf-8'?>\n<Document SchemaVersion=\"4\" FileVersion=\"1\"><Objects Count=\"{0}\">{types}</Objects><ObjectData Count=\"{0}\">{data}</ObjectData></Document>\n",
        objs.len()
    )
}

/// One straight edge as OpenCascade writes it.
const EDGE: &str = "DBRep_DrawableShape\n\nCASCADE Topology V1, (c) Matra-Datavision\nLocations 0\nCurve2ds 0\nCurves 1\n1 0 -20 10 1 0 0\nPolygon3D 0\nPolygonOnTriangulations 0\nSurfaces 0\nTriangulations 0\n\nTShapes 3\nVe\n1e-07\n-30 -20 10\n0 0\n\n0101101\n*\nVe\n1e-07\n30 -20 10\n0 0\n\n0101101\n*\nEd\n 1e-07 1 1 0\n1  1 0 -30 30\n0\n\n0101000\n+3 0 -2 0 *\n\n+1 0\n";

fn body(tip: &str, group: &[&str]) -> Obj {
    let links: String = group.iter().map(|g| format!("<Link value=\"{g}\"/>")).collect();
    (
        "Body",
        "PartDesign::Body",
        vec![
            ("Tip", format!("<Link value=\"{tip}\"/>")),
            (
                "Group",
                format!("<LinkList count=\"{}\">{links}</LinkList>", group.len()),
            ),
        ],
    )
}

/// A pad, a pocket through it and a fillet; a revolved groove; a pad of a slot: as FreeCAD stores
/// them, stored and deflated.
pub fn fcstd_seeds() -> Vec<(String, Vec<u8>)> {
    let h = std::f64::consts::FRAC_PI_2;
    let rect = geo(&[
        line((-30.0, -20.0), (30.0, -20.0), 0),
        line((30.0, -20.0), (30.0, 20.0), 0),
        line((30.0, 20.0), (-30.0, 20.0), 0),
        line((-30.0, 20.0), (-30.0, -20.0), 0),
        line((0.0, -30.0), (0.0, 30.0), 1),
    ]);
    let slot = geo(&[
        line((-10.0, -4.0), (10.0, -4.0), 0),
        arc((10.0, 0.0), 4.0, -h, h),
        line((10.0, 4.0), (-10.0, 4.0), 0),
        arc((-10.0, 0.0), 4.0, h, 3.0 * h),
        circle((20.0, 10.0), 3.0),
    ]);
    let pad = |base: &str, sk: &str, len: f64| -> Vec<(&'static str, String)> {
        vec![
            ("BaseFeature", format!("<Link value=\"{base}\"/>")),
            ("Profile", format!("<LinkSub value=\"{sk}\"/>")),
            ("Length", format!("<Float value=\"{len}\"/>")),
            ("Type", "<Integer value=\"0\"/>".into()),
            ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
        ]
    };
    let one = doc(&[
        body("Fillet", &["Sketch", "Pad", "Sketch001", "Pocket", "Fillet"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", rect)],
        ),
        ("Pad", "PartDesign::Pad", pad("", "Sketch", 10.0)),
        (
            "Sketch001",
            "Sketcher::SketchObject",
            vec![
                ("Placement", place(10.0)),
                ("Geometry", geo(&[circle((0.0, 0.0), 5.0)])),
            ],
        ),
        (
            "Pocket",
            "PartDesign::Pocket",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch001\"/>".into()),
                ("Length", "<Float value=\"5\"/>".into()),
                ("Type", "<Integer value=\"1\"/>".into()),
                ("Shape", "<Part file=\"PocketShape.brp\"/>".into()),
            ],
        ),
        (
            "Fillet",
            "PartDesign::Fillet",
            vec![
                ("BaseFeature", "<Link value=\"Pocket\"/>".into()),
                (
                    "Base",
                    "<LinkSub value=\"Pocket\" count=\"1\"><Sub value=\"Edge1\"/></LinkSub>".into(),
                ),
                ("Radius", "<Float value=\"2\"/>".into()),
                ("Shape", "<Part file=\"FilletShape.brp\"/>".into()),
            ],
        ),
    ]);
    let two = doc(&[
        body("Pad", &["Sketch", "Pad"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", slot)],
        ),
        ("Pad", "PartDesign::Pad", pad("", "Sketch", 6.0)),
    ]);
    let three = doc(&[
        body("Revolution", &["Sketch", "Revolution"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![
                ("Placement", place(0.0)),
                (
                    "Geometry",
                    geo(&[
                        line((5.0, 0.0), (10.0, 0.0), 0),
                        line((10.0, 0.0), (10.0, 8.0), 0),
                        line((10.0, 8.0), (5.0, 8.0), 0),
                        line((5.0, 8.0), (5.0, 0.0), 0),
                    ]),
                ),
            ],
        ),
        (
            "Revolution",
            "PartDesign::Revolution",
            vec![
                ("BaseFeature", "<Link value=\"\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Angle", "<Float value=\"360\"/>".into()),
                (
                    "ReferenceAxis",
                    "<LinkSub value=\"Sketch\" count=\"1\"><Sub value=\"V_Axis\"/></LinkSub>".into(),
                ),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
    ]);
    let mut out = vec![];
    for (i, d) in [one, two, three].into_iter().enumerate() {
        for deflated in [false, true] {
            let members: Vec<Member> = [
                ("Document.xml", d.as_str()),
                ("PadShape.brp", EDGE),
                ("PocketShape.brp", EDGE),
                ("FilletShape.brp", EDGE),
            ]
            .iter()
            .map(|(n, s)| (n.to_string(), deflated, s.as_bytes().to_vec()))
            .collect();
            out.push((format!("made/doc{i}-{deflated}.fcstd"), pack(&members)));
        }
    }
    out
}

/// A cube and a ring of cubes as STL (binary and ASCII), OBJ and 3MF (with a component placed
/// twice, one mirrored).
pub fn mesh_seeds(t: crate::Target) -> Vec<(String, Vec<u8>)> {
    let mut obj = String::from("# cubes\n");
    for k in 0..4 {
        let o = 3 * k;
        for i in 0..8 {
            obj += &format!("v {} {} {}\n", o + (i & 1), (i >> 1) & 1, (i >> 2) & 1);
        }
        let b = 8 * k + 1;
        for f in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            obj += &format!("f {} {} {} {}\n", b + f[0], b + f[1], b + f[2], b + f[3]);
        }
    }
    let mesh = forge_mesh::read::obj(obj.as_bytes()).expect("the seed reads");
    match t {
        crate::Target::Obj => vec![("made/cubes.obj".into(), obj.into_bytes())],
        crate::Target::Stl => {
            let mut ascii = String::from("solid cubes\n");
            for tr in &mesh.triangles {
                ascii += "facet normal 0 0 0\n outer loop\n";
                for &i in tr {
                    let p = mesh.positions[i as usize];
                    ascii += &format!("  vertex {} {} {}\n", p.x, p.y, p.z);
                }
                ascii += " endloop\nendfacet\n";
            }
            ascii += "endsolid cubes\n";
            vec![
                ("made/cubes.stl".into(), forge_mesh::to_binary_stl(&mesh, "cubes")),
                ("made/cubes-ascii.stl".into(), ascii.into_bytes()),
            ]
        }
        _ => {
            let vs: String = mesh
                .positions
                .iter()
                .map(|p| format!("<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>", p.x, p.y, p.z))
                .collect();
            let ts: String = mesh
                .triangles
                .iter()
                .map(|t| format!("<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]))
                .collect();
            let model = format!(
                "<?xml version=\"1.0\"?><model unit=\"millimeter\"><resources><object id=\"1\" type=\"model\"><mesh><vertices>{vs}</vertices><triangles>{ts}</triangles></mesh></object><object id=\"2\"><components><component objectid=\"1\" transform=\"1 0 0 0 1 0 0 0 1 5 0 0\"/><component objectid=\"1\" transform=\"-1 0 0 0 1 0 0 0 1 0 0 0\"/></components></object></resources><build><item objectid=\"2\" transform=\"1 0 0 0 1 0 0 0 1 0 0 2\"/><item objectid=\"1\"/></build></model>"
            );
            let rels = "<Relationships><Relationship Target=\"/3D/3dmodel.model\" Id=\"r\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/></Relationships>";
            [false, true]
                .iter()
                .map(|&d| {
                    let ms: Vec<Member> = vec![
                        ("_rels/.rels".into(), d, rels.as_bytes().to_vec()),
                        ("3D/3dmodel.model".into(), d, model.as_bytes().to_vec()),
                    ];
                    (format!("made/cubes-{d}.3mf"), pack(&ms))
                })
                .collect()
        }
    }
}
