//! Small FreeCAD documents made here, translated; the expected text is what the Python importer
//! (tools/find-parts/recreate/freecad.py) wrote for the same documents.

use forge_import::fcstd::translate;

/// A zip of stored members (no CRC: the reader does not check one).
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

fn object(name: &str, props: &[(&str, String)]) -> String {
    let ps: String = props
        .iter()
        .map(|(k, v)| format!("<Property name=\"{k}\" type=\"x\">{v}</Property>"))
        .collect();
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

fn arc(c: (f64, f64), r: f64, a0: f64, a1: f64) -> String {
    format!(
        "<Geometry type=\"Part::GeomArcOfCircle\"><ArcOfCircle CenterX=\"{}\" CenterY=\"{}\" CenterZ=\"0\" NormalX=\"0\" NormalY=\"0\" NormalZ=\"1\" AngleXU=\"0\" Radius=\"{r}\" StartAngle=\"{a0:?}\" EndAngle=\"{a1:?}\"/><Construction value=\"0\"/></Geometry>",
        c.0, c.1
    )
}

fn circle(c: (f64, f64), r: f64) -> String {
    format!(
        "<Geometry type=\"Part::GeomCircle\"><Circle CenterX=\"{}\" CenterY=\"{}\" CenterZ=\"0\" NormalX=\"0\" NormalY=\"0\" NormalZ=\"1\" Radius=\"{r}\"/><Construction value=\"0\"/></Geometry>",
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

/// An object: name, type, properties.
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

/// One straight edge, (-30, -20, 10) to (30, -20, 10): the pad's front top edge.
const EDGE: &str = "DBRep_DrawableShape\n\nCASCADE Topology V1, (c) Matra-Datavision\nLocations 0\nCurve2ds 0\nCurves 1\n1 0 -20 10 1 0 0\nPolygon3D 0\nPolygonOnTriangulations 0\nSurfaces 0\nTriangulations 0\n\nTShapes 3\nVe\n1e-07\n-30 -20 10\n0 0\n\n0101101\n*\nVe\n1e-07\n30 -20 10\n0 0\n\n0101101\n*\nEd\n 1e-07 1 1 0\n1  1 0 -30 30\n0\n\n0101000\n+3 0 -2 0 *\n\n+1 0\n";

fn rect() -> String {
    geo(&[
        line((-30.0, -20.0), (30.0, -20.0), 0),
        line((30.0, -20.0), (30.0, 20.0), 0),
        line((30.0, 20.0), (-30.0, 20.0), 0),
        line((-30.0, 20.0), (-30.0, -20.0), 0),
        line((0.0, -30.0), (0.0, 30.0), 1),
    ])
}

fn body(tip: &str, group: &[&str]) -> Obj {
    let links: String = group.iter().map(|g| format!("<Link value=\"{g}\"/>")).collect();
    (
        "Body",
        "PartDesign::Body",
        vec![
            ("Tip", format!("<Link value=\"{tip}\"/>")),
            ("Group", format!("<LinkList count=\"1\">{links}</LinkList>")),
        ],
    )
}

fn file(d: String) -> Vec<u8> {
    zip(&[
        ("Document.xml", &d),
        ("PadShape.brp", EDGE),
        ("PocketShape.brp", EDGE),
        ("FilletShape.brp", EDGE),
    ])
}

fn only(data: &[u8]) -> String {
    let bodies = translate(data).expect("reads");
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].status(), "translated", "{:?}", bodies[0].fail);
    bodies[0].fc.clone().expect("a part file")
}

#[test]
fn pad_and_pocket_of_a_slot_and_a_hole() {
    let h = std::f64::consts::FRAC_PI_2;
    let slot = geo(&[
        line((-10.0, -4.0), (10.0, -4.0), 0),
        arc((10.0, 0.0), 4.0, -h, h),
        line((10.0, 4.0), (-10.0, 4.0), 0),
        arc((-10.0, 0.0), 4.0, h, 3.0 * h),
        circle((20.0, 10.0), 3.0),
    ]);
    let d = doc(&[
        body("Pocket", &["Sketch", "Pad", "Sketch001", "Pocket"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", rect())],
        ),
        (
            "Pad",
            "PartDesign::Pad",
            vec![
                ("BaseFeature", "<Link value=\"\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Length", "<Float value=\"10\"/>".into()),
                ("Type", "<Integer value=\"0\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Sketch001",
            "Sketcher::SketchObject",
            vec![("Placement", place(10.0)), ("Geometry", slot)],
        ),
        (
            "Pocket",
            "PartDesign::Pocket",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch001\"/>".into()),
                ("Length", "<Float value=\"4\"/>".into()),
                ("Type", "<Integer value=\"0\"/>".into()),
                ("Shape", "<Part file=\"PocketShape.brp\"/>".into()),
            ],
        ),
    ]);
    assert_eq!(
        only(&file(d)),
        "t0: extrude 10 z 0 0 1 x 1 0 0 at 0 0 0 loop line -30 -20 30 -20 line 30 -20 30 20 line 30 20 -30 20 line -30 20 -30 -20\n\
         t1: extrude cut 4 z 0 0 -1 x 1 0 0 at 0 0 10 loop line -10 4 10 4 arc 10 0 4 90.0000000000 -180.0000000000 line 10 -4 -10 -4 arc -10 0 4 -90.0000000000 -180.0000000000\n\
         t1x1: extrude cut 4 z 0 0 -1 x 1 0 0 at 0 0 10 circle 20 -10 3\n"
    );
}

#[test]
fn a_fillet_finds_its_edge_where_the_stored_shape_has_it() {
    let d = doc(&[
        body("Fillet", &["Sketch", "Pad", "Fillet"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", rect())],
        ),
        (
            "Pad",
            "PartDesign::Pad",
            vec![
                ("BaseFeature", "<Link value=\"\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Length", "<Float value=\"10\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Fillet",
            "PartDesign::Fillet",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                (
                    "Base",
                    "<LinkSub value=\"Pad\"><Sub value=\"Edge1\"/></LinkSub>".into(),
                ),
                ("Radius", "<Float value=\"2\"/>".into()),
                ("Shape", "<Part file=\"FilletShape.brp\"/>".into()),
            ],
        ),
    ]);
    assert_eq!(
        only(&file(d)),
        "t0: extrude 10 z 0 0 1 x 1 0 0 at 0 0 0 loop line -30 -20 30 -20 line 30 -20 30 20 line 30 20 -30 20 line -30 20 -30 -20\n\
         t1: fillet 2 @e6bf1773d\n"
    );
}

#[test]
fn a_fillet_on_an_edge_the_stored_shape_lacks_is_unresolved() {
    let d = doc(&[
        body("Fillet", &["Sketch", "Pad", "Fillet"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", rect())],
        ),
        (
            "Pad",
            "PartDesign::Pad",
            vec![
                ("BaseFeature", "<Link value=\"\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Length", "<Float value=\"10\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Fillet",
            "PartDesign::Fillet",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                (
                    "Base",
                    "<LinkSub value=\"Pad\"><Sub value=\"Edge2\"/></LinkSub>".into(),
                ),
                ("Radius", "<Float value=\"2\"/>".into()),
                ("Shape", "<Part file=\"FilletShape.brp\"/>".into()),
            ],
        ),
    ]);
    let b = &translate(&file(d)).expect("reads")[0];
    assert_eq!(b.status(), "untranslated");
    assert!(b.fc.is_none());
}

#[test]
fn not_a_zip_is_refused() {
    assert!(translate(b"PK nothing").is_err());
}

#[test]
fn part_file_names_stay_in_their_directory_and_apart() {
    let body = |n: &str| forge_import::Body {
        name: n.into(),
        features: vec![],
        approx: false,
        fc: None,
        fail: None,
    };
    let bodies = [body("../../x"), body("a/b"), body("Body"), body("Body"), body("")];
    let names = forge_import::fcstd::part_files("../evil.FCStd", &bodies);
    assert_eq!(
        names,
        [
            "___evil_FCStd.______x.fc",
            "___evil_FCStd.a_b.fc",
            "___evil_FCStd.Body.fc",
            "___evil_FCStd.Body-2.fc",
            "___evil_FCStd._.fc"
        ]
    );
    assert!(names.iter().all(|n| !n.contains('/') && !n.contains("..")));
}

/// A zip whose one member says it inflates to `size` from `csize` bytes.
fn claiming(size: u32, csize: u32) -> Vec<u8> {
    let mut z = zip(&[("Document.xml", "x")]);
    // the central directory entry follows the 31-byte local header and the member's byte
    let cd = 30 + "Document.xml".len() + 1;
    z[cd + 20..cd + 24].copy_from_slice(&csize.to_le_bytes());
    z[cd + 24..cd + 28].copy_from_slice(&size.to_le_bytes());
    z
}

#[test]
fn a_zip_that_would_inflate_past_its_caps_is_refused_unread() {
    assert!(translate(&claiming(300 << 20, 1)).is_err());
    // a megabyte from a hundred bytes: past the ratio a document ever needs
    assert!(translate(&claiming(1 << 20, 100)).is_err());
}

#[test]
fn a_spline_of_absurd_multiplicity_is_a_clean_refusal() {
    let spline = "<GeometryList count=\"1\"><Geometry type=\"Part::GeomBSplineCurve\"><BSplineCurve PolesCount=\"2\" KnotsCount=\"2\" Degree=\"1\" IsPeriodic=\"0\"><Pole X=\"0\" Y=\"0\" Z=\"0\" Weight=\"1\"/><Pole X=\"1\" Y=\"0\" Z=\"0\" Weight=\"1\"/><Knot Value=\"0\" Mult=\"4000000000000\"/><Knot Value=\"1\" Mult=\"2\"/></BSplineCurve><Construction value=\"0\"/></Geometry></GeometryList>";
    let d = doc(&[
        body("Pad", &["Sketch", "Pad"]),
        (
            "Sketch",
            "Sketcher::SketchObject",
            vec![("Placement", place(0.0)), ("Geometry", spline.into())],
        ),
        (
            "Pad",
            "PartDesign::Pad",
            vec![
                ("BaseFeature", "<Link value=\"\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Length", "<Float value=\"10\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
    ]);
    let b = &translate(&file(d)).expect("reads")[0];
    assert_eq!(b.status(), "untranslated");
    assert!(b.fail.as_ref().is_some_and(|f| f.why().contains("knots")));
}
