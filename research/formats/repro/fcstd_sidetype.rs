// A test for interface/forge-import/tests/fcstd.rs (its helpers), for the owner to add with the fix:
// FreeCAD 1.1 says a pad's sides with SideType (0 one side, 1 two sides, 2 symmetric) and leaves
// the deprecated Midplane false (FreeCAD's data/examples/PartDesignExample.FCStd writes so).
// Fails today: SideType is not read, so both pads come out one-sided, 0..10.
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
    let fg = only(&file(side(1, &[("Length2", "<Float value=\"4\"/>".into()), ("Type2", "<Integer value=\"0\"/>".into())])));
    let m = built(&fg);
    near(m.volume, 60.0 * 40.0 * 14.0, "two sides: volume", &fg);
    near(m.centroid[2], 3.0, "two sides: centroid z", &fg);
}
