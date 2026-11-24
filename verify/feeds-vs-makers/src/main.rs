//! Writes verify/feeds-vs-makers/REPORT.md and rows.tsv from the makers' raw rows; with
//! `--generic`, kernel/forge-tooling/data/generic.toml (the generic classes) first.
//! usage: cargo run -p feeds-vs-makers [--print | --generic]
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = root.join("../../research/cam/tooling/raw");
    let (rows, skips) = feeds_vs_makers::rows(&raw);
    if std::env::args().any(|a| a == "--generic") {
        let t = feeds_vs_makers::generic_table(&rows, None);
        std::fs::write(
            root.join("../../kernel/forge-tooling/data/generic.toml"),
            feeds_vs_makers::generic_toml(&t),
        )
        .unwrap();
        println!(
            "{} generic classes (rerun without --generic for the report)",
            t.len()
        );
        return;
    }
    let report = feeds_vs_makers::report::markdown(&rows, &skips);
    if std::env::args().any(|a| a == "--print") {
        println!("{report}");
        return;
    }
    std::fs::write(root.join("REPORT.md"), report).unwrap();
    let mut tsv = String::from(
        "vendor\ttool\tkind\td\tz\tsubstrate\tgroup\top\tmaterial\tae\tap\ttheirs_rpm\tours_rpm\ttheirs_f\tours_f\tr_rpm\tr_f\tr_vf\tours_src\tgeneric\tcircular\turl\n",
    );
    let o = |x: Option<f64>| x.map_or("-".to_string(), |v| format!("{v:.5}"));
    for r in &rows {
        tsv.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\t{:.1}\t{}\t{:.5}\t{:.4}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.vendor, r.tool, r.kind, r.d, r.z, r.substrate, r.group, r.op, r.material, o(r.ae), o(r.ap),
            r.theirs_rpm, r.ours_rpm, o(r.theirs_f), r.ours_f, r.r_rpm(), o(r.r_f()), o(r.r_vf()), r.ours_src, r.generic, r.circular, r.url
        ));
    }
    std::fs::write(root.join("rows.tsv"), tsv).unwrap();
    println!("{} rows", rows.len());
}
