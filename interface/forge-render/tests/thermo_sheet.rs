//! The thermoform process sheet as the shop gets it: its PDF text in WinAnsi, its vent table.

#[test]
fn the_process_sheet_prints_its_signs_and_no_negative_zero() {
    let src = include_str!("../../../examples/thermoform/cover.fg");
    let opts = forge_lang::Options {
        dir: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/thermoform").into()),
        ..Default::default()
    };
    let r = forge_lang::run(src, opts, Default::default()).0;
    let o = r.thermos.first().expect("the cover's thermoform");
    let (_, gfx) = forge_render::thermo_view::view(o, 0.25, 3.0);
    let sheet = &gfx.sheet;
    assert!(
        sheet.iter().any(|l| l.starts_with("Blank") && l.contains('×')),
        "{sheet:?}"
    );
    assert!(!sheet.iter().any(|l| l.contains("-0.00")), "{sheet:?}");
    assert!(!sheet.last().unwrap().is_empty(), "{sheet:?}");
    let pdf = String::from_utf8_lossy(&forge_render::thermo_view::process_pdf(sheet)).to_string();
    assert!(pdf.contains("/Encoding /WinAnsiEncoding"));
    // × is WinAnsi 0xD7
    assert!(pdf.contains(" \\327 ") && !pdf.contains(" ? "), "{pdf}");
}
