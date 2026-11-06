//! The font layer's tools.
//!
//! `dwgfonts pack JHF_DIR OUT.bin` packs the Hershey fonts (the Usenet distribution's glyph data,
//! in the `.jhf` files of e.g. github.com/kamalmostafa/hershey-fonts) into the compact table
//! `src/fonts/hershey.bin` the stand-in faces are read from: for each face its name, then for
//! each printable ASCII character the left and right bearing bytes and the coordinate-pair bytes
//! as the distribution has them (`R` is zero, `" R"` lifts the pen).
//!
//! `dwgfonts sheet OUT.svg [FONT_FILE..]` sets sample strings in every stand-in face (and in any
//! SHX or TrueType files given) and writes them as SVG, to look at.
//!
//! `dwgfonts bench` times shaping 50,000 short texts.
use forge_dwg::fonts::{FontSet, hershey};
use forge_dwg::scene::Fonts;
use std::fmt::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("pack") if a.len() == 4 => pack(&a[2], &a[3]),
        Some("sheet") if a.len() >= 3 => sheet(&a[2], &a[3..]),
        Some("bench") => bench(),
        _ => eprintln!("dwgfonts pack JHF_DIR OUT.bin | dwgfonts sheet OUT.svg [FONT_FILE..]"),
    }
}

fn bench() {
    let fs = FontSet::new();
    let t = std::time::Instant::now();
    let mut points = 0;
    for k in 0..50_000 {
        let s = fs.shape(
            if k % 2 == 0 { "romans" } else { "txt" },
            "",
            &format!("%%c{k} M{} TYP.", k % 97),
        );
        points += s.strokes.iter().map(Vec::len).sum::<usize>();
    }
    eprintln!("50000 texts, {points} points, {:?}", t.elapsed());
}

fn pack(dir: &str, out: &str) {
    let mut bin = b"HSY1".to_vec();
    bin.push(hershey::FACES.len() as u8);
    for f in hershey::FACES {
        let text = std::fs::read_to_string(format!("{dir}/{}.jhf", f.file)).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| l.len() >= 10).collect();
        assert!(lines.len() >= 95, "{}: {} glyphs", f.file, lines.len());
        bin.push(f.file.len() as u8);
        bin.extend_from_slice(f.file.as_bytes());
        for l in &lines[..95] {
            let b = l.as_bytes();
            let n: usize = l[5..8].trim().parse().unwrap();
            let pts = &b[10..];
            assert_eq!(pts.len(), 2 * (n - 1), "{}: {l}", f.file);
            bin.push(b[8]);
            bin.push(b[9]);
            bin.extend_from_slice(&(pts.len() as u16).to_le_bytes());
            bin.extend_from_slice(pts);
        }
    }
    std::fs::write(out, &bin).unwrap();
    eprintln!("{} faces, {} bytes", hershey::FACES.len(), bin.len());
}

fn sheet(out: &str, files: &[String]) {
    let mut fs = FontSet::new();
    let mut names: Vec<String> = hershey::SHX_NAMES.iter().map(|(n, _)| n.to_string()).collect();
    for f in files {
        let name = std::path::Path::new(f)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        fs.add(&name, std::fs::read(f).unwrap());
        names.push(name);
    }
    let samples = [
        "The quick brown fox jumps over the lazy dog 0123456789",
        "%%c25 H7  45%%d  %%p0.05  %%uM8x1.25%%u  R12 TYP. \u{2300}6 \u{b0}\u{b1}",
        "\u{c4}\u{d6}\u{dc} \u{e4}\u{f6}\u{fc}\u{df} \u{e9}\u{e8}\u{ea}\u{eb} \u{e7}\u{c7} \u{f1}\u{d1} \u{e5}\u{c5} \u{f8}\u{d8} \u{bf}\u{a1} \u{b2}\u{b3} \u{bd}\u{bc} \u{b5}\u{d7}\u{f7}",
        "\u{3b1}\u{3b2}\u{3b3} \u{3a9}\u{3a3}\u{394} \u{41f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{2264}\u{2265}\u{2260}\u{221e} \u{20ac}100",
    ];
    let (size, line) = (24.0, 34.0);
    let mut body = String::new();
    let mut y = 20.0;
    for n in &names {
        y += line;
        let _ = writeln!(
            body,
            "<text x='10' y='{y}' font-family='sans-serif' font-size='13' fill='#06c'>{n}</text>"
        );
        for s in samples {
            y += line;
            let sh = fs.shape(n, "", s);
            let tx = |p: &[f64; 2]| format!("{:.2},{:.2}", 20.0 + p[0] * size, y - p[1] * size);
            for st in &sh.strokes {
                let pts: Vec<String> = st.iter().map(tx).collect();
                let _ = writeln!(
                    body,
                    "<polyline points='{}' fill='none' stroke='black' stroke-width='1.2' stroke-linecap='round' stroke-linejoin='round'/>",
                    pts.join(" ")
                );
            }
            if !sh.fills.is_empty() {
                let d: String = sh
                    .fills
                    .iter()
                    .map(|r| format!("M{}Z", r.iter().map(tx).collect::<Vec<_>>().join("L")))
                    .collect();
                let _ = writeln!(body, "<path d='{d}' fill='black' fill-rule='evenodd'/>");
            }
            let _ = writeln!(
                body,
                "<line x1='20' y1='{y}' x2='{:.2}' y2='{y}' stroke='#e44' stroke-width='0.5'/>",
                20.0 + sh.advance * size
            );
        }
        y += 8.0;
    }
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='1700' height='{}'>\n<rect width='100%' height='100%' fill='white'/>\n{body}</svg>\n",
        y + 30.0
    );
    std::fs::write(out, svg).unwrap();
    for m in fs.missing() {
        eprintln!("{m}");
    }
}
