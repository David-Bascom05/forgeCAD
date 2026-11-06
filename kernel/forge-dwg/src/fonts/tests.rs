use super::*;
use crate::scene::Fonts;

/// A shapes font (cap 6) with a capital A of two strokes, two caps wide, a space and the old
/// fonts' degree sign (shape 127), nothing else.
fn tiny_shx() -> Vec<u8> {
    shx::build::shapes(
        "1.0",
        "tiny",
        6,
        2,
        &[
            (
                65,
                &[0x62, 0x6E, 2, 8, (-9i8) as u8, 3, 1, 0x60, 2, 8, 3, (-3i8) as u8],
            ),
            (32, &[2, 0x60]),
            (127, &[10, 1, 0x00, 2, 0x40]),
        ],
    )
}

fn bbox(s: &Shaped) -> ([f64; 2], [f64; 2]) {
    let g = Glyph {
        strokes: s.strokes.clone(),
        fills: s.fills.clone(),
        advance: 0.0,
    };
    g.bbox().unwrap()
}

#[test]
fn stand_ins_for_autodesk_names() {
    let fs = FontSet::new();
    let h = fs.shape("txt", "", "H");
    assert_eq!(h.strokes.len(), 3);
    let (lo, hi) = bbox(&h);
    assert!((hi[1] - 1.0).abs() < 1e-12 && lo[1].abs() < 1e-12);
    // romans, simplex and a path with capitals all set the same.
    let a = fs.shape("romans", "", "Text 12");
    assert_eq!(
        fs.shape("C:\\Program Files\\Fonts\\SIMPLEX.SHX", "", "Text 12"),
        a
    );
    assert_eq!(fs.shape("simplex.shx", "", "Text 12"), a);
    // Simplex's own advances: H is 22/21, the space 16/21.
    assert!((fs.shape("romans", "", "H H").advance - 60.0 / 21.0).abs() < 1e-12);
    // Complex is a different face.
    assert_ne!(fs.shape("romanc.shx", "", "Text 12"), a);
    let m = fs.missing();
    assert!(m.iter().any(|m| m.font == "txt" && m.using.contains("Simplex")));
    assert!(
        m.iter()
            .any(|m| m.font == "romanc.shx" && m.using.contains("Complex"))
    );
}

#[test]
fn unknown_fonts_fall_back_to_simplex() {
    let fs = FontSet::new();
    let s = fs.shape("whatever.shx", "", "AB");
    assert_eq!(s, fs.shape("romans", "", "AB"));
    assert_eq!(fs.shape("Arial", "", "AB"), s);
    assert_eq!(fs.shape("", "", "AB"), fs.shape("txt", "", "AB"));
    let m = fs.missing();
    assert!(m.iter().any(|m| m.font == "Arial"), "{m:?}");
    assert_eq!(
        m.iter().find(|m| m.font == "whatever.shx").unwrap().to_string(),
        "font whatever.shx not found, using Hershey Roman Simplex (built-in stand-in)"
    );
}

#[test]
fn loaded_shx_wins_and_lacks_fall_back() {
    let mut fs = FontSet::new();
    fs.add("ROMANS.SHX", tiny_shx());
    assert!(fs.errors().is_empty());
    let a = fs.shape("romans", "", "A");
    assert_eq!(a.strokes.len(), 2);
    assert_eq!(a.advance, 2.0);
    assert_eq!(fs.shape("c:/x/Romans.shx", "", "A"), a);
    // B is not in the font: the stand-in's (Simplex for romans) B.
    let b = fs.shape("romans", "", "B");
    assert_eq!(b, FontSet::new().shape("romans", "", "B"));
    // %%d: the old fonts' shape 127.
    let d = fs.shape("romans", "", "%%d");
    assert!((d.advance - 4.0 / 6.0).abs() < 1e-12);
    // %%127 is the same shape.
    assert_eq!(fs.shape("romans", "", "%%127"), d);
    assert!(fs.missing().iter().all(|m| m.font != "romans"));
}

#[test]
fn truetype_by_file_or_family_and_kerning() {
    let mut fs = FontSet::new();
    let font = ttf::tests::with_tables(
        &ttf::tests::test_font(),
        &[(b"name", ttf::tests::name_table("Test Sans"))],
    );
    fs.add("testsans.ttf", font);
    let vo = fs.shape("Test Sans", "", "VO");
    assert!(vo.strokes.is_empty());
    assert_eq!(vo.fills.len(), 3);
    assert!((vo.advance - (600.0 + 700.0 - 70.0) / 700.0).abs() < 1e-12);
    assert_eq!(fs.shape("TESTSANS.TTF", "", "VO"), vo);
    assert_eq!(fs.shape("testsans", "", "VO"), vo);
    // A character the font lacks: Simplex strokes.
    let q = fs.shape("testsans.ttf", "", "Q");
    assert!(!q.strokes.is_empty() && q.fills.is_empty());
}

#[test]
fn default_truetype_for_unknown_names() {
    let mut fs = FontSet::new();
    fs.add("fallback.ttf", ttf::tests::test_font());
    fs.set_default_ttf(Some("fallback.ttf"));
    assert_eq!(fs.shape("arial.ttf", "", "H").fills.len(), 1);
    // SHX names still get their stand-ins.
    assert_eq!(fs.shape("romans", "", "H").fills.len(), 0);
    let m = fs.missing();
    assert!(
        m.iter()
            .any(|m| m.font == "arial.ttf" && m.using.contains("fallback.ttf"))
    );
}

#[test]
fn shx_over_ttf_of_the_same_stem() {
    let mut fs = FontSet::new();
    fs.add("tiny.ttf", ttf::tests::test_font());
    fs.add("tiny.shx", tiny_shx());
    assert_eq!(fs.shape("tiny", "", "A").strokes.len(), 2);
    assert_eq!(fs.shape("tiny.ttf", "", "H").fills.len(), 1);
}

#[test]
fn control_codes_and_lines() {
    let fs = FontSet::new();
    let d = fs.shape("romans", "", "%%c");
    assert_eq!(d, fs.shape("romans", "", "\u{2300}"));
    assert_eq!(d, fs.shape("romans", "", "\\U+2300"));
    assert_eq!(fs.shape("romans", "", "%%d"), fs.shape("romans", "", "°"));
    let plain = fs.shape("romans", "", "AB");
    let u = fs.shape("romans", "", "%%uAB");
    assert_eq!(u.strokes.len(), plain.strokes.len() + 1);
    assert_eq!(
        *u.strokes.last().unwrap(),
        vec![[0.0, UNDERLINE], [plain.advance, UNDERLINE]]
    );
    let o = fs.shape("romans", "", "A%%oB%%o");
    let a = fs.shape("romans", "", "A").advance;
    assert_eq!(
        *o.strokes.last().unwrap(),
        vec![[a, OVERLINE], [plain.advance, OVERLINE]]
    );
    assert_eq!(o.advance, plain.advance);
}

#[test]
fn big_fonts() {
    // Lead bytes 0x81..0x9F; character 0x8140 an 8-high box's right side, height 8.
    let big = shx::build::bigfont("big", &[8, 0, 0, 0], &[(0x81, 0x9f)], &[(0x8140, &[0x80, 0x84])]);
    let mut fs = FontSet::new();
    fs.add("bigfont.shx", big);
    // As the drawing's code-page bytes read as Windows-1252: 0x81 then '@'.
    let s = fs.shape("txt", "bigfont.shx", "A\u{81}@B");
    let esc = fs.shape("txt", "bigfont.shx", "A\\M+18140B");
    assert_eq!(s, esc);
    let a = fs.shape("txt", "", "A").advance;
    let b = fs.shape("txt", "", "B").advance;
    assert!((s.advance - (a + 1.0 + b)).abs() < 1e-12);
    assert!(
        s.strokes
            .iter()
            .any(|l| l == &vec![[a, 0.0], [a + 1.0, 0.0], [a + 1.0, 1.0]])
    );
    // Bytes outside the ranges are ordinary characters.
    assert_eq!(fs.shape("txt", "bigfont.shx", "é"), fs.shape("txt", "", "é"));
    // A big font named but not loaded: a box per pair, and a note.
    let m = fs.shape("txt", "gbcbig.shx", "\u{81}@");
    assert_eq!(m.advance, 1.0);
    assert!(fs.missing().iter().any(|m| m.font == "gbcbig.shx"));
}

#[test]
fn vertical_text() {
    let mut fs = FontSet::new();
    // A font whose A moves right 6 horizontally, but down 8 when vertical (code 14).
    fs.add(
        "v.shx",
        shx::build::shapes("1.0", "v", 6, 2, &[(65, &[0x64, 2, 14, 0x8C, 14, 0x6C, 0x60])]),
    );
    let h = fs.shape("v", "", "AA");
    assert_eq!(h.advance, 2.0);
    let v = fs.shape_vertical("v", "", "AA");
    assert!(v.advance > 0.0);
    let (lo, _) = bbox(&v);
    assert!(lo[1] < -1.0);
}

#[test]
fn caches_give_the_same_answer() {
    let fs = FontSet::new();
    let t = "Ø25 ±0.1 Größe ÀÉÎõü αβγ Привет";
    let a = fs.shape("romand", "", t);
    let b = fs.shape("romand", "", t);
    assert_eq!(a, b);
    assert!(a.strokes.len() > 50);
}

#[test]
fn bad_files_are_reported() {
    let mut fs = FontSet::new();
    fs.add("junk.ttf", vec![1, 2, 3]);
    fs.add("junk.shx", b"AutoCAD-86 shapes 1.0\r\n\x1a\x01".to_vec());
    assert_eq!(fs.errors().len(), 2);
    assert_eq!(
        fs.shape("junk", "", "A"),
        FontSet::new().shape("simplex", "", "A")
    );
}

/// The shapes' bits, pinned: native and wasm builds must agree (arcs, circles and bulges go
/// through forge-math's trig).
#[test]
fn shaping_is_pinned() {
    let mut fs = FontSet::new();
    let arcs: &[u8] = &[
        10, 3, 0x14, // octant arc
        11, 62, 54, 0, 3, 0x92, // fractional arc, clockwise
        12, 5, 2, 70, // bulge
        13, 3, 3, 90, 2, 0xff, 40, 0, 0, // bulges
    ];
    fs.add("arcs.shx", shx::build::shapes("1.0", "arcs", 6, 2, &[(65, arcs)]));
    fs.add("t.ttf", ttf::tests::test_font());
    let mut bytes = Vec::new();
    for (font, text) in [
        ("arcs", "AAA"),
        ("t.ttf", "HOVO"),
        ("romans", "%%c25 %%d %%p Größe Å ® © • αΩ Привет"),
        ("romand", "Ø12 ½ ¿?"),
        ("scriptc", "Script"),
    ] {
        let s = fs.shape(font, "", text);
        for p in s.strokes.iter().chain(&s.fills).flatten() {
            bytes.extend(p[0].to_bits().to_le_bytes());
            bytes.extend(p[1].to_bits().to_le_bytes());
        }
        bytes.extend(s.advance.to_bits().to_le_bytes());
    }
    assert_eq!(
        format!("{:032x}", forge_math::hash::hash128(&bytes)),
        "9c7b14eb60cf252336dd9f3efb23812b"
    );
}
