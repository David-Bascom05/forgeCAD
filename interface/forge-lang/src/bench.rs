//! The real-part benchmarks, `parts/bench/*.fg`: engineered parts written in Forge, and the block
//! font their text is cut in (written here: no font files are kept with the code). Tests, the
//! speed gate and the renderer's benches build them with the font in the part's folder.

use forge_font::Seg;
use forge_font::testing::{TestGlyph, build};
use forge_math::Point2;

/// The part names, each a file `parts/bench/NAME.fg`.
pub const NAMES: [&str; 15] = [
    "bracket", "clamp", "conrod", "crank", "frame", "gearbox", "guard", "heatsink", "hub", "insert", "jack",
    "manifold", "quadarm", "valve", "volute",
];

/// The part file of that name.
pub fn src(name: &str) -> Option<&'static str> {
    Some(match name {
        "bracket" => include_str!("../parts/bench/bracket.fg"),
        "clamp" => include_str!("../parts/bench/clamp.fg"),
        "conrod" => include_str!("../parts/bench/conrod.fg"),
        "crank" => include_str!("../parts/bench/crank.fg"),
        "frame" => include_str!("../parts/bench/frame.fg"),
        "gearbox" => include_str!("../parts/bench/gearbox.fg"),
        "guard" => include_str!("../parts/bench/guard.fg"),
        "heatsink" => include_str!("../parts/bench/heatsink.fg"),
        "hub" => include_str!("../parts/bench/hub.fg"),
        "insert" => include_str!("../parts/bench/insert.fg"),
        "jack" => include_str!("../parts/bench/jack.fg"),
        "manifold" => include_str!("../parts/bench/manifold.fg"),
        "quadarm" => include_str!("../parts/bench/quadarm.fg"),
        "valve" => include_str!("../parts/bench/valve.fg"),
        "volute" => include_str!("../parts/bench/volute.fg"),
        _ => return None,
    })
}

/// The font file name the parts' text gives (`text(.., font: "block.ttf")`).
pub const FONT: &str = "block.ttf";

/// A block capital font on a grid of 100 units (cap height 700): the glyphs the bench parts'
/// text uses.
pub fn font_bytes() -> Vec<u8> {
    // each glyph's contours in grid units, x y pairs, `|` between contours, the outer first
    const O: &str =
        "1.2 0 3.8 0 5 1.2 5 5.8 3.8 7 1.2 7 0 5.8 0 1.2|1.6 1 3.4 1 4 1.6 4 5.4 3.4 6 1.6 6 1 5.4 1 1.6";
    let glyphs = [
        ('-', "0.5 3 4.5 3 4.5 4 0.5 4"),
        ('0', O),
        ('1', "2 0 2 5.6 1 5.1 1 6.1 2.6 7 3 7 3 0"),
        ('4', "3 0 3 2 0 2 0 3 3 7 4 7 4 3 5 3 5 2 4 2 4 0|1.3 3 3 3 3 5.3"),
        ('7', "0 7 5 7 5 6 2.2 0 1 0 3.8 6 0 6"),
        ('A', "0 0 2 7 3 7 5 0 3.9 0 3.4 2 1.6 2 1.1 0|1.9 3 3.1 3 2.5 5.2"),
        (
            'C',
            "5 6 4 7 1 7 0 6 0 1 1 0 4 0 5 1 5 2 4 2 4 1.4 3.6 1 1.4 1 1 1.4 1 5.6 1.4 6 3.6 6 4 5.6 4 5 5 5",
        ),
        ('D', "0 0 0 7 3.5 7 5 5.5 5 1.5 3.5 0|1 1 3 1 4 2 4 5 3 6 1 6"),
        ('E', "0 0 0 7 5 7 5 6 1 6 1 4 4 4 4 3 1 3 1 1 5 1 5 0"),
        ('F', "0 0 0 7 5 7 5 6 1 6 1 4 4 4 4 3 1 3 1 0"),
        (
            'G',
            "5 6 4 7 1 7 0 6 0 1 1 0 4 0 5 1 5 3.5 2.5 3.5 2.5 2.5 4 2.5 4 1.4 3.6 1 1.4 1 1 1.4 1 5.6 1.4 6 3.6 6 4 5.6 4 5 5 5",
        ),
        (
            'M',
            "0 0 0 7 1 7 2.5 4 4 7 5 7 5 0 4 0 4 4.6 2.9 2.4 2.1 2.4 1 4.6 1 0",
        ),
        ('O', O),
        (
            'R',
            "0 0 0 7 4 7 5 6 5 4 4 3 5 0 3.9 0 2.9 3 1 3 1 0|1 4 1 6 3.6 6 4 5.6 4 4.4 3.6 4",
        ),
        (
            'W',
            "0 7 1 7 1.5 1.8 2.2 5 2.8 5 3.5 1.8 4 7 5 7 4.2 0 3.2 0 2.5 3.4 1.8 0 0.8 0",
        ),
        (
            'X',
            "0 0 1.9 3.5 0 7 1.2 7 2.5 4.6 3.8 7 5 7 3.1 3.5 5 0 3.8 0 2.5 2.4 1.2 0",
        ),
    ];
    let glyphs: Vec<(char, TestGlyph)> = glyphs
        .iter()
        .map(|&(c, src)| {
            let contours = src
                .split('|')
                .enumerate()
                .map(|(k, pts)| {
                    let v: Vec<f64> = pts
                        .split(' ')
                        .map(|t| t.parse::<f64>().expect("a number") * 100.0)
                        .collect();
                    let mut p: Vec<Point2> = v.chunks(2).map(|q| Point2::new(q[0], q[1])).collect();
                    let n = p.len();
                    let area2: f64 = (0..n)
                        .map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y)
                        .sum();
                    // outer clockwise, holes counter-clockwise
                    if (area2 > 0.0) == (k == 0) {
                        p.reverse();
                    }
                    (0..n).map(|i| Seg::Line(p[i], p[(i + 1) % n])).collect()
                })
                .collect();
            (
                c,
                TestGlyph {
                    contours,
                    advance: 650,
                },
            )
        })
        .collect();
    build(1000, &glyphs, &[])
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_bench_part_is_named_and_the_font_reads() {
        for n in super::NAMES {
            assert!(super::src(n).is_some(), "{n}");
        }
        assert!(forge_font::Font::parse(super::font_bytes()).is_ok());
    }
}
