//! A vendor record in forge-tooling's library and the same tool imported from a Fusion tool
//! library look alike: the same type and the same dc, lcf, dcon, oal and flutes. The Fusion JSON
//! here is written from the Harvey Tool page's own figures (inch library, as Harvey ships it).
use forge_tooling::Library;

#[test]
fn harvey_792208_imported_equals_the_vendor_record() {
    let lib = Library::builtin();
    let ours = lib.tool("harvey-792208").expect("the Harvey record");
    let fusion = r#"{"version": 22, "data": [{"type": "flat end mill", "unit": "inches",
        "description": "Harvey 792208", "vendor": "Harvey Tool", "product-id": "792208",
        "geometry": {"DC": 0.125, "LCF": 0.375, "OAL": 1.5, "SFDM": 0.125, "NOF": 2}}]}"#;
    let imp = forge_cam_import::fusion(fusion, "align.json", "2026-11-23").unwrap();
    let mut l = Library::default();
    l.add("align.toml", &imp.toml).unwrap();
    let t = &l.tools[0];
    assert_eq!(t.kind, ours.kind);
    for (a, b, what) in [
        (t.dc, ours.dc, "dc"),
        (t.lcf, ours.lcf, "lcf"),
        (t.dcon, ours.dcon, "dcon"),
        (t.oal, ours.oal, "oal"),
    ] {
        assert!((a - b).abs() < 1e-9, "{what}: imported {a} vs vendor {b}");
    }
    assert_eq!(t.nof, ours.nof);
    // ours carries what an import cannot: who says so
    assert!(
        ours.real
            && ours
                .cites
                .iter()
                .any(|c| c.path == "dc" && c.source.contains("harveytool.com"))
    );
}
