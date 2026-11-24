//! Writes verify/feeds-vs-makers/REPORT.md and rows.tsv from the makers' raw rows.
//! usage: cargo run -p feeds-vs-makers [--print]
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = root.join("../../research/cam/tooling/raw");
    let (rows, skips) = feeds_vs_makers::rows(&raw);
    let report = feeds_vs_makers::report::markdown(&rows, &skips);
    if std::env::args().any(|a| a == "--print") {
        println!("{report}");
        return;
    }
    std::fs::write(root.join("REPORT.md"), report).unwrap();
    let mut tsv = String::from(
        "vendor\ttool\tkind\td\tz\tsubstrate\tgroup\top\tmaterial\tae\tap\ttheirs_rpm\tours_rpm\ttheirs_f\tours_f\tr_rpm\tr_f\tr_vf\tours_src\tcircular\turl\n",
    );
    let o = |x: Option<f64>| x.map_or("-".to_string(), |v| format!("{v:.5}"));
    for r in &rows {
        tsv.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\t{:.1}\t{}\t{:.5}\t{:.4}\t{}\t{}\t{}\t{}\t{}\n",
            r.vendor, r.tool, r.kind, r.d, r.z, r.substrate, r.group, r.op, r.material, o(r.ae), o(r.ap),
            r.theirs_rpm, r.ours_rpm, o(r.theirs_f), r.ours_f, r.r_rpm(), o(r.r_f()), o(r.r_vf()), r.ours_src, r.circular, r.url
        ));
    }
    std::fs::write(root.join("rows.tsv"), tsv).unwrap();
    println!("{} rows", rows.len());
}
