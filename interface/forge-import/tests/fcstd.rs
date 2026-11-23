//! Small FreeCAD documents made here, translated to Forge parts: each builds valid and lint-free
//! to the solid FreeCAD would make of it (volume and centroid worked out by hand), or says why not.

use forge_import::fcstd::translate;

/// A zip of stored members (no CRC: the reader does not check one).
fn zip(members: &[(&str, &str)]) -> Vec<u8> {
    zip_with(members, false)
}

/// A zip of stored or deflated members.
fn zip_with(members: &[(&str, &str)], deflated: bool) -> Vec<u8> {
    let (mut out, mut dir) = (vec![], vec![]);
    let le16 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u16).to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: usize| v.extend_from_slice(&(x as u32).to_le_bytes());
    for (name, data) in members {
        let off = out.len();
        let method = if deflated { 8 } else { 0 };
        let body = if deflated {
            forge_git::zlib::deflate(data.as_bytes())
        } else {
            data.as_bytes().to_vec()
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

/// The document's zip, stored; its deflated twin must read the same.
fn file(d: String) -> Vec<u8> {
    let members = [
        ("Document.xml", d.as_str()),
        ("PadShape.brp", EDGE),
        ("PocketShape.brp", EDGE),
        ("FilletShape.brp", EDGE),
    ];
    let stored = zip_with(&members, false);
    let deflated = zip_with(&members, true);
    assert!(deflated.len() < stored.len());
    let read = |z: &[u8]| {
        translate(z).map(|bs| {
            bs.iter()
                .map(|b| (b.status().to_string(), b.fg.clone(), format!("{:?}", b.fail)))
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(read(&stored), read(&deflated));
    stored
}

fn only(data: &[u8]) -> String {
    let bodies = translate(data).expect("reads");
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].status(), "translated", "{:?}", bodies[0].fail);
    bodies[0].fg.clone().expect("a part file")
}

/// The part built: valid, with no errors or lints; its measure.
fn built(src: &str) -> forge_lang::Measure {
    let (r, _) = forge_lang::run(src, forge_lang::Options::default(), Default::default());
    let report = forge_lang::report::text(&r, src);
    assert!(r.errors.is_empty() && r.warnings.is_empty(), "{src}\n{report}");
    let m = r.measure.clone().expect("measured");
    assert_eq!(m.problems, 0, "{src}\n{report}");
    m
}

fn near(a: f64, b: f64, what: &str, src: &str) {
    assert!(
        (a - b).abs() <= 1e-6 * b.abs().max(1.0),
        "{what} {a} vs {b}\n{src}"
    );
}

fn pad(name: &str, base: &str, sketch: &str, length: f64, extra: &[(&'static str, String)]) -> Obj {
    let mut p = vec![
        ("BaseFeature", format!("<Link value=\"{base}\"/>")),
        ("Profile", format!("<LinkSub value=\"{sketch}\"/>")),
        ("Length", format!("<Float value=\"{length}\"/>")),
        ("Type", "<Integer value=\"0\"/>".into()),
        ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
    ];
    p.extend(extra.iter().cloned());
    (leak(name), "PartDesign::Pad", p)
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn sketch(name: &str, placement: String, g: String) -> Obj {
    (
        leak(name),
        "Sketcher::SketchObject",
        vec![("Placement", placement), ("Geometry", g)],
    )
}

/// A placement: at (x, y, z), turned by the quaternion (qx, qy, qz, qw).
fn placed(o: (f64, f64, f64), q: (f64, f64, f64, f64)) -> String {
    format!(
        "<PropertyPlacement Px=\"{}\" Py=\"{}\" Pz=\"{}\" Q0=\"{}\" Q1=\"{}\" Q2=\"{}\" Q3=\"{}\" A=\"0\" Ox=\"0\" Oy=\"0\" Oz=\"1\"/>",
        o.0, o.1, o.2, q.0, q.1, q.2, q.3
    )
}

fn v(x: f64, y: f64, z: f64) -> String {
    format!("<PropertyVector valueX=\"{x}\" valueY=\"{y}\" valueZ=\"{z}\"/>")
}

const PI: f64 = std::f64::consts::PI;

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
    let fg = only(&file(d));
    // the pad, then the slot and the hole: two regions, a cut each
    assert!(fg.starts_with("# FreeCAD body Body: Pad, Pocket\n"), "{fg}");
    assert!(
        fg.contains("t0 = extrude(t0_sk, d)") && fg.contains("t1 = cut(t1_sk, d)"),
        "{fg}"
    );
    assert!(fg.contains("t1x1 = cut(t1x1_sk, d)"), "{fg}");
    let m = built(&fg);
    let (slot, hole) = (4.0 * (160.0 + 16.0 * PI), 4.0 * 9.0 * PI);
    let vol = 24000.0 - slot - hole;
    near(m.volume, vol, "volume", &fg);
    // the hole where the sketch has it: (20, 10), from the top down
    near(m.centroid[0], -hole * 20.0 / vol, "centroid x", &fg);
    near(m.centroid[1], -hole * 10.0 / vol, "centroid y", &fg);
    near(
        m.centroid[2],
        (24000.0 * 5.0 - (slot + hole) * 8.0) / vol,
        "centroid z",
        &fg,
    );
    assert_eq!((m.faces, m.edges, m.bodies), (6 + 5 + 2, 12 + 12 + 3, 1), "{fg}");
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
    let fg = only(&file(d));
    // the front top edge, named by the faces it lies between
    assert!(
        fg.contains("t1 = fillet(edges(between: (t0.end, t0.side(t0_sk.l0))), r)"),
        "{fg}"
    );
    let m = built(&fg);
    near(m.volume, 24000.0 - (4.0 - PI) * 60.0, "volume", &fg);
    assert_eq!((m.faces, m.edges), (7, 15), "{fg}");
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
    assert!(b.fg.is_none());
}

#[test]
fn a_pad_on_the_xz_plane_runs_along_its_normal() {
    // XZ_Plane: turned a quarter about x, so the sketch's y is the world's z and its normal -y
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let d = doc(&[
        body("Pad", &["Sketch", "Pad"]),
        sketch("Sketch", placed((0.0, 5.0, 0.0), (s, 0.0, 0.0, s)), rect()),
        pad("Pad", "", "Sketch", 10.0, &[]),
    ]);
    let fg = only(&file(d));
    let m = built(&fg);
    near(m.volume, 24000.0, "volume", &fg);
    // x -30..30, z -20..20, y from 5 back to -5
    near(m.centroid[0], 0.0, "centroid x", &fg);
    near(m.centroid[1], 0.0, "centroid y", &fg);
    near(m.centroid[2], 0.0, "centroid z", &fg);
    let d = doc(&[
        body("Pad", &["Sketch", "Pad"]),
        sketch("Sketch", placed((0.0, 5.0, 0.0), (s, 0.0, 0.0, s)), rect()),
        pad(
            "Pad",
            "",
            "Sketch",
            10.0,
            &[("Reversed", "<Bool value=\"true\"/>".into())],
        ),
    ]);
    let fg = only(&file(d));
    let m = built(&fg);
    near(m.centroid[1], 10.0, "reversed: centroid y", &fg);
}

#[test]
fn a_pad_along_a_slant_is_a_ruled_loft_of_the_same_height() {
    let d = doc(&[
        body("Pad", &["Sketch", "Pad"]),
        sketch("Sketch", place(0.0), rect()),
        pad(
            "Pad",
            "",
            "Sketch",
            10.0,
            &[
                ("UseCustomVector", "<Bool value=\"true\"/>".into()),
                ("Direction", v(0.3, 0.2, 1.0)),
                ("AlongSketchNormal", "<Bool value=\"true\"/>".into()),
            ],
        ),
    ]);
    let fg = only(&file(d));
    assert!(fg.contains("t0 = loft([t0_sk, t0_end])"), "{fg}");
    let m = built(&fg);
    near(m.volume, 24000.0, "volume", &fg);
    // the walls lean 0.3 in x and 0.2 in y for each 1 up
    near(m.centroid[0], 1.5, "centroid x", &fg);
    near(m.centroid[1], 1.0, "centroid y", &fg);
    near(m.centroid[2], 5.0, "centroid z", &fg);
    assert_eq!((m.faces, m.edges), (6, 12), "{fg}");
}

/// A pocket through all along a slant: literally through (`thru` along the slant), not a loft
/// as long as the part's box.
#[test]
fn a_pocket_through_all_along_a_slant_is_through() {
    let sq = geo(&[
        line((-5.0, -5.0), (5.0, -5.0), 0),
        line((5.0, -5.0), (5.0, 5.0), 0),
        line((5.0, 5.0), (-5.0, 5.0), 0),
        line((-5.0, 5.0), (-5.0, -5.0), 0),
    ]);
    let d = doc(&[
        body("Pocket", &["Sketch", "Pad", "Sketch001", "Pocket"]),
        sketch("Sketch", place(0.0), rect()),
        pad("Pad", "", "Sketch", 10.0, &[]),
        sketch("Sketch001", place(10.0), sq),
        (
            "Pocket",
            "PartDesign::Pocket",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch001\"/>".into()),
                ("Type", "<Integer value=\"1\"/>".into()),
                ("UseCustomVector", "<Bool value=\"true\"/>".into()),
                ("Direction", v(0.3, 0.0, -1.0)),
                ("Shape", "<Part file=\"PocketShape.brp\"/>".into()),
            ],
        ),
    ]);
    let fg = only(&file(d));
    assert!(fg.contains("cut(t1_sk, thru, along: "), "{fg}");
    assert!(!fg.contains("loft("), "{fg}");
    let m = built(&fg);
    // a slanted square prism through 10: its volume the square's area times the height
    near(m.volume, 24000.0 - 1000.0, "volume", &fg);
    // the hole leans 0.3 in x for each 1 down: its middle 1.5 off the square's, at z = 5
    near(m.centroid[0], -1000.0 * 1.5 / 23000.0, "centroid x", &fg);
    near(m.centroid[2], 5.0, "centroid z", &fg);
}

#[test]
fn a_revolve_symmetric_about_its_sketch_turns_half_each_way() {
    // a 10 square from x 10 to 20, turned 90 degrees about the sketch's y axis, half each side
    let sq = geo(&[
        line((10.0, 0.0), (20.0, 0.0), 0),
        line((20.0, 0.0), (20.0, 10.0), 0),
        line((20.0, 10.0), (10.0, 10.0), 0),
        line((10.0, 10.0), (10.0, 0.0), 0),
    ]);
    let rev = |mid: bool| {
        doc(&[
            body("Rev", &["Sketch", "Rev"]),
            sketch("Sketch", place(0.0), sq.clone()),
            (
                "Rev",
                "PartDesign::Revolution",
                vec![
                    ("BaseFeature", "<Link value=\"\"/>".into()),
                    ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                    ("Base", v(0.0, 0.0, 0.0)),
                    ("Axis", v(0.0, 1.0, 0.0)),
                    ("Angle", "<Float value=\"90\"/>".into()),
                    ("Midplane", format!("<Bool value=\"{mid}\"/>")),
                    ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
                ],
            ),
        ])
    };
    let vol = 100.0 * 15.0 * PI / 2.0;
    // the centroid of a sector of the ring: at 15 * sin(45deg) / (pi / 4) from the axis, mid-angle
    let r = (2.0 / 3.0) * (20.0f64.powi(3) - 10.0f64.powi(3)) / (20.0f64.powi(2) - 10.0f64.powi(2))
        * std::f64::consts::FRAC_1_SQRT_2
        / (PI / 4.0);
    let fg = only(&file(rev(true)));
    let m = built(&fg);
    near(m.volume, vol, "volume", &fg);
    near(m.centroid[0], r, "symmetric: centroid x", &fg);
    near(m.centroid[2], 0.0, "symmetric: centroid z", &fg);
    // one way: about +y from +x, toward -z
    let fg = only(&file(rev(false)));
    let m = built(&fg);
    near(m.volume, vol, "volume", &fg);
    near(
        m.centroid[0],
        r * std::f64::consts::FRAC_1_SQRT_2,
        "one way: centroid x",
        &fg,
    );
    near(
        m.centroid[2],
        -r * std::f64::consts::FRAC_1_SQRT_2,
        "one way: centroid z",
        &fg,
    );
}

#[test]
fn counterbored_holes_patterned_round_an_axis() {
    let d = doc(&[
        body("Polar", &["Sketch", "Pad", "Sketch001", "Hole", "Polar"]),
        sketch("Sketch", place(0.0), rect()),
        pad("Pad", "", "Sketch", 10.0, &[]),
        sketch("Sketch001", place(10.0), geo(&[circle((10.0, 0.0), 2.0)])),
        (
            "Hole",
            "PartDesign::Hole",
            vec![
                ("BaseFeature", "<Link value=\"Pad\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch001\"/>".into()),
                ("Diameter", "<Float value=\"4\"/>".into()),
                ("DepthType", "<Integer value=\"1\"/>".into()),
                ("HoleCutType", "<Integer value=\"1\"/>".into()),
                ("HoleCutDiameter", "<Float value=\"8\"/>".into()),
                ("HoleCutDepth", "<Float value=\"3\"/>".into()),
                ("Shape", "<Part file=\"PocketShape.brp\"/>".into()),
            ],
        ),
        (
            "Polar",
            "PartDesign::PolarPattern",
            vec![
                ("BaseFeature", "<Link value=\"Hole\"/>".into()),
                (
                    "Originals",
                    "<LinkList count=\"1\"><Link value=\"Hole\"/></LinkList>".into(),
                ),
                (
                    "Axis",
                    "<LinkSub value=\"Sketch001\"><Sub value=\"N_Axis\"/></LinkSub>".into(),
                ),
                ("Angle", "<Float value=\"360\"/>".into()),
                ("Occurrences", "<Integer value=\"4\"/>".into()),
                ("Shape", "<Part file=\"PocketShape.brp\"/>".into()),
            ],
        ),
    ]);
    let fg = only(&file(d));
    assert!(fg.contains("cbore: (seat_d, seat_depth)"), "{fg}");
    let m = built(&fg);
    let one = PI * (4.0 * 7.0 + 16.0 * 3.0);
    near(m.volume, 24000.0 - 4.0 * one, "volume", &fg);
    // four round the axis: the centroid stays on it
    near(m.centroid[0], 0.0, "centroid x", &fg);
    near(m.centroid[1], 0.0, "centroid y", &fg);
}

#[test]
fn a_part_cut_of_a_box_by_a_cylinder_is_bodies_combined() {
    let d = doc(&[
        (
            "Box",
            "Part::Box",
            vec![
                ("Placement", place(0.0)),
                ("Length", "<Float value=\"20\"/>".into()),
                ("Width", "<Float value=\"10\"/>".into()),
                ("Height", "<Float value=\"5\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Cyl",
            "Part::Cylinder",
            vec![
                ("Placement", placed((10.0, 5.0, -1.0), (0.0, 0.0, 0.0, 1.0))),
                ("Radius", "<Float value=\"2\"/>".into()),
                ("Height", "<Float value=\"7\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Cut",
            "Part::Cut",
            vec![
                ("Placement", place(0.0)),
                ("Base", "<Link value=\"Box\"/>".into()),
                ("Tool", "<Link value=\"Cyl\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        (
            "Pad",
            "PartDesign::Pad",
            vec![
                ("BaseFeature", "<Link value=\"Cut\"/>".into()),
                ("Profile", "<LinkSub value=\"Sketch\"/>".into()),
                ("Length", "<Float value=\"2\"/>".into()),
                ("Type", "<Integer value=\"0\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        sketch(
            "Sketch",
            place(5.0),
            geo(&[
                line((0.0, 0.0), (4.0, 0.0), 0),
                line((4.0, 0.0), (4.0, 4.0), 0),
                line((4.0, 4.0), (0.0, 4.0), 0),
                line((0.0, 4.0), (0.0, 0.0), 0),
            ]),
        ),
        body("Pad", &["Pad", "Sketch"]),
    ]);
    let bodies = translate(&file(d)).expect("reads");
    let b = bodies
        .iter()
        .find(|b| b.status() == "translated")
        .unwrap_or_else(|| panic!("{bodies:?}"));
    let fg = b.fg.clone().unwrap();
    assert!(fg.contains("combine(p2, tools: p1, op: \"cut\")"), "{fg}");
    let m = built(&fg);
    near(m.volume, 1000.0 - PI * 4.0 * 5.0 + 32.0, "volume", &fg);
}

/// A square sketch on z, (x, y) to (x + 4, y + 4).
fn square(name: &str, z: f64, x: f64, y: f64) -> Obj {
    sketch(
        name,
        place(z),
        geo(&[
            line((x, y), (x + 4.0, y), 0),
            line((x + 4.0, y), (x + 4.0, y + 4.0), 0),
            line((x + 4.0, y + 4.0), (x, y + 4.0), 0),
            line((x, y + 4.0), (x, y), 0),
        ]),
    )
}

#[test]
fn a_chain_two_shapes_of_a_part_tree_share_is_written_twice_with_ids_of_its_own() {
    // Base and Tool of the fuse both stand on Plate: each chain steps it, and the second time
    // its statements (and the next feature's) need ids no statement before them has
    let d = doc(&[
        pad("Plate", "", "SkPlate", 10.0, &[]),
        sketch("SkPlate", place(0.0), rect()),
        pad("Left", "Plate", "SkLeft", 2.0, &[]),
        square("SkLeft", 10.0, 0.0, 0.0),
        pad("Right", "Plate", "SkRight", 3.0, &[]),
        square("SkRight", 10.0, 10.0, 0.0),
        (
            "Fuse",
            "Part::Fuse",
            vec![
                ("Placement", place(0.0)),
                ("Base", "<Link value=\"Left\"/>".into()),
                ("Tool", "<Link value=\"Right\"/>".into()),
                ("Shape", "<Part file=\"PadShape.brp\"/>".into()),
            ],
        ),
        pad("Top", "Fuse", "SkTop", 1.0, &[]),
        square("SkTop", 10.0, 20.0, 0.0),
        body("Top", &["Top", "SkTop"]),
    ]);
    let bodies = translate(&file(d)).expect("reads");
    let b = bodies
        .iter()
        .find(|b| b.fg.as_deref().is_some_and(|f| f.contains("combine(")))
        .unwrap_or_else(|| panic!("{bodies:?}"));
    assert_eq!(b.status(), "translated", "{:?}", b.fail);
    let fg = b.fg.clone().unwrap();
    // the tool's chain first (Plate t0, Right t1), then the base's: Plate again, a body of its
    // own, then Left on it, whose t2 the second Plate already holds
    assert!(fg.contains("t2 = extrude(t2_sk, d, new: true)"), "{fg}");
    assert!(fg.contains("t2r1 = extrude(t2r1_sk, d)"), "{fg}");
    assert!(fg.contains("combine(t2, tools: t0, op: \"add\")"), "{fg}");
    let m = built(&fg);
    near(
        m.volume,
        60.0 * 40.0 * 10.0 + 16.0 * (2.0 + 3.0 + 1.0),
        "volume",
        &fg,
    );
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
        fg: None,
        fail: None,
    };
    let bodies = [body("../../x"), body("a/b"), body("Body"), body("Body"), body("")];
    let names = forge_import::fcstd::part_files("../evil.FCStd", &bodies);
    assert_eq!(
        names,
        [
            "___evil_FCStd.______x.fg",
            "___evil_FCStd.a_b.fg",
            "___evil_FCStd.Body.fg",
            "___evil_FCStd.Body-2.fg",
            "___evil_FCStd._.fg"
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

/// One straight edge from `a` to `b`, as a stored shape holds it.
fn edge_brep(a: (f64, f64, f64), b: (f64, f64, f64)) -> String {
    let d = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
    let l = (d.0 * d.0 + d.1 * d.1 + d.2 * d.2).sqrt();
    let u = (d.0 / l, d.1 / l, d.2 / l);
    format!(
        "DBRep_DrawableShape\n\nCASCADE Topology V1, (c) Matra-Datavision\nLocations 0\nCurve2ds 0\nCurves 1\n1 {} {} {} {} {} {}\nPolygon3D 0\nPolygonOnTriangulations 0\nSurfaces 0\nTriangulations 0\n\nTShapes 3\nVe\n1e-07\n{} {} {}\n0 0\n\n0101101\n*\nVe\n1e-07\n{} {} {}\n0 0\n\n0101101\n*\nEd\n 1e-07 1 1 0\n1  1 0 0 {l}\n0\n\n0101000\n+3 0 -2 0 *\n\n+1 0\n",
        a.0, a.1, a.2, u.0, u.1, u.2, a.0, a.1, a.2, b.0, b.1, b.2
    )
}

/// Three square bosses, the second and third a linear pattern's copies; FreeCAD rounds the front
/// top edge of the last copy alone. Twelve edges of the copies are alike in all but which copy
/// and which side: the fillet names its edge on that copy, by the copied boss's own anchors.
#[test]
fn a_fillet_on_one_copy_of_a_pattern_names_that_copys_edge() {
    let square = geo(&[
        line((-25.0, -5.0), (-15.0, -5.0), 0),
        line((-15.0, -5.0), (-15.0, 5.0), 0),
        line((-15.0, 5.0), (-25.0, 5.0), 0),
        line((-25.0, 5.0), (-25.0, -5.0), 0),
    ]);
    let d = doc(&[
        body(
            "Fillet",
            &["Sketch", "Pad", "Sketch001", "Pad001", "LinearPattern", "Fillet"],
        ),
        sketch("Sketch", place(0.0), rect()),
        pad("Pad", "", "Sketch", 10.0, &[]),
        sketch("Sketch001", place(10.0), square),
        pad("Pad001", "Pad", "Sketch001", 5.0, &[]),
        (
            "LinearPattern",
            "PartDesign::LinearPattern",
            vec![
                ("BaseFeature", "<Link value=\"Pad001\"/>".into()),
                (
                    "Originals",
                    "<LinkList count=\"1\"><Link value=\"Pad001\"/></LinkList>".into(),
                ),
                (
                    "Direction",
                    "<LinkSub value=\"Sketch001\"><Sub value=\"H_Axis\"/></LinkSub>".into(),
                ),
                ("Length", "<Float value=\"40\"/>".into()),
                ("Occurrences", "<Integer value=\"3\"/>".into()),
                ("Shape", "<Part file=\"PatternShape.brp\"/>".into()),
            ],
        ),
        (
            "Fillet",
            "PartDesign::Fillet",
            vec![
                ("BaseFeature", "<Link value=\"LinearPattern\"/>".into()),
                (
                    "Base",
                    "<LinkSub value=\"LinearPattern\"><Sub value=\"Edge1\"/></LinkSub>".into(),
                ),
                ("Radius", "<Float value=\"1\"/>".into()),
                ("Shape", "<Part file=\"FilletShape.brp\"/>".into()),
            ],
        ),
    ]);
    let front = edge_brep((15.0, -5.0, 15.0), (25.0, -5.0, 15.0));
    let data = zip(&[
        ("Document.xml", d.as_str()),
        ("PadShape.brp", EDGE),
        ("PatternShape.brp", front.as_str()),
        ("FilletShape.brp", EDGE),
    ]);
    let fg = only(&data);
    assert!(fg.contains("t2[2]."), "{fg}");
    let m = built(&fg);
    near(
        m.volume,
        24000.0 + 3.0 * 500.0 - (1.0 - PI / 4.0) * 10.0,
        "volume",
        &fg,
    );
    // the rounding on the copy at x = 20, its front: off centre toward +x and +y
    let plain = 24000.0 + 3.0 * 500.0;
    let gone = (1.0 - PI / 4.0) * 10.0;
    let want_x = (3.0 * 500.0 * 0.0 - gone * 20.0) / (plain - gone);
    near(m.centroid[0], want_x, "centroid x", &fg);
    assert!(m.centroid[1] > 0.0, "{fg}");
}

// research/formats/repro/fcstd_sidetype.rs (formats-corpus): FreeCAD 1.1 says a pad's sides with
// SideType (0 one side, 1 two sides, 2 symmetric) and leaves the deprecated Midplane false, as its
// data/examples/PartDesignExample.FCStd does.
#[test]
fn a_freecad_1_1_pad_takes_its_sides_from_side_type() {
    let side = |t: i32, extra: &[(&'static str, String)]| {
        let mut p = vec![
            ("SideType", format!("<Integer value=\"{t}\"/>")),
            ("Midplane", "<Bool value=\"false\"/>".into()),
        ];
        p.extend(extra.iter().cloned());
        doc(&[
            body("Pad", &["Sketch", "Pad"]),
            sketch("Sketch", place(0.0), rect()),
            pad("Pad", "", "Sketch", 10.0, &p),
        ])
    };
    // symmetric: 10 in all, -5..5
    let fg = only(&file(side(2, &[])));
    let m = built(&fg);
    near(m.volume, 60.0 * 40.0 * 10.0, "symmetric: volume", &fg);
    near(m.centroid[2], 0.0, "symmetric: centroid z", &fg);
    // two sides: 10 up and Length2 = 4 down, -4..10
    let fg = only(&file(side(
        1,
        &[
            ("Length2", "<Float value=\"4\"/>".into()),
            ("Type2", "<Integer value=\"0\"/>".into()),
        ],
    )));
    let m = built(&fg);
    near(m.volume, 60.0 * 40.0 * 14.0, "two sides: volume", &fg);
    near(m.centroid[2], 3.0, "two sides: centroid z", &fg);
}
