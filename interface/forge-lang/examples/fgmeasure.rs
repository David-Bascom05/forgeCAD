//! A part's exact measures at full precision, for the OCCT checks of sheets and thin walls
//! (tools/occt/sheets.py): `fgmeasure F.fg` prints one JSON line, volume, area and sheet_area
//! (the solids' volume and area, the sheets' area apart), whether it is valid, and its errors.
fn main() {
    let path = std::env::args().nth(1).expect("fgmeasure F.fg");
    let src = std::fs::read_to_string(&path).expect("readable");
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    let errors: Vec<String> = run
        .errors
        .iter()
        .map(|e| format!("{e:?}").replace('"', "'"))
        .collect();
    let sheet_area = forge_lang::report::sheet_area(&run).unwrap_or(0.0);
    match run.measure {
        Some(m) => println!(
            "{{\"volume\":{:e},\"area\":{:e},\"sheet_area\":{:e},\"valid\":{},\"errors\":{:?}}}",
            m.volume,
            m.area,
            sheet_area,
            m.problems == 0,
            errors
        ),
        None => println!("{{\"errors\":{errors:?}}}"),
    }
}
