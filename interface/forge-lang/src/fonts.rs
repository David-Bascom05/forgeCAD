//! The fonts shipped with ForgeCAD, by family and style: Open Sans 1.10 Regular and Bold (TrueType outlines; the
//! version Onshape sets sketch text in; Apache License 2.0) and Source Sans 3 (CFF outlines; SIL
//! Open Font License 1.1). Both licences allow them to be embedded and redistributed with
//! software (fonts/README.md records where each came from, its licence and its hash).
//! A part names one by its family (`font: "Open Sans"`); a font file beside the part
//! (`font: "block.ttf"`) is read as `media::font` reads it.

use forge_font::Font;
use std::sync::{Arc, OnceLock};

/// A shipped face: its family, its style as its name table gives it, its bytes.
struct Face {
    family: &'static str,
    style: &'static str,
    bytes: &'static [u8],
}

const FACES: &[Face] = &[
    Face {
        family: "Open Sans",
        style: "Regular",
        bytes: include_bytes!("../fonts/OpenSans-Regular.ttf"),
    },
    Face {
        family: "Open Sans",
        style: "Bold",
        bytes: include_bytes!("../fonts/OpenSans-Bold.ttf"),
    },
    Face {
        family: "Source Sans 3",
        style: "Regular",
        bytes: include_bytes!("../fonts/SourceSans3-Regular.otf"),
    },
];

/// The family a sketch text takes when it names none.
pub const DEFAULT: &str = "Open Sans";

/// The style a bold and italic choice names.
pub fn style(bold: bool, italic: bool) -> &'static str {
    match (bold, italic) {
        (false, false) => "Regular",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (true, true) => "Bold Italic",
    }
}

/// Whether a family is shipped (in any style), its name matched without regard to case.
pub fn is_family(family: &str) -> bool {
    FACES.iter().any(|f| f.family.eq_ignore_ascii_case(family))
}

/// Every shipped family and style.
pub fn faces() -> impl Iterator<Item = (&'static str, &'static str)> {
    FACES.iter().map(|f| (f.family, f.style))
}

/// The shipped face of the family in that style, parsed once; None if the family has no such
/// face.
pub fn shipped(family: &str, bold: bool, italic: bool) -> Option<Arc<Font>> {
    static PARSED: OnceLock<Vec<OnceLock<Arc<Font>>>> = OnceLock::new();
    let parsed = PARSED.get_or_init(|| FACES.iter().map(|_| OnceLock::new()).collect());
    let want = style(bold, italic);
    let k = FACES
        .iter()
        .position(|f| f.family.eq_ignore_ascii_case(family) && f.style == want)?;
    Some(
        parsed[k]
            .get_or_init(|| {
                Arc::new(Font::parse_cff_too(FACES[k].bytes.to_vec()).expect("a shipped font reads"))
            })
            .clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped face reads, says the family and style it is filed under, and holds an
    /// outline for every printable character of Latin-1.
    #[test]
    fn shipped_faces_read() {
        for (family, style) in faces() {
            let (bold, italic) = (style.contains("Bold"), style.contains("Italic"));
            let f = shipped(family, bold, italic).expect("shipped");
            assert_eq!(f.names(), (family.to_string(), style.to_string()));
            for c in (0x21u32..0x7f).chain(0xa1..0x100).filter_map(char::from_u32) {
                if c == '\u{ad}' {
                    continue;
                }
                let g = f.glyph(c);
                assert!(g != 0, "{family} {style}: no glyph for {c:?}");
                f.outline(g)
                    .unwrap_or_else(|e| panic!("{family} {style} {c:?}: {e:?}"));
            }
        }
        assert!(shipped("open sans", false, false).is_some());
    }

    /// Every shipped face, cut short and scrambled: refused or read, never a panic.
    #[test]
    fn shipped_faces_survive_corruption() {
        let mut r = forge_math::rng::Rng::new(2026);
        let mut files: Vec<Vec<u8>> = Vec::new();
        for f in FACES {
            let b = f.bytes;
            files.extend((1..48).map(|k| b[..b.len() * k / 48].to_vec()));
            for _ in 0..60 {
                let mut d = b.to_vec();
                for _ in 0..1 + r.below(16) {
                    let k = r.below(d.len());
                    d[k] = r.below(256) as u8;
                }
                files.push(d);
            }
        }
        forge_math::par::map(files.len(), |i| {
            if let Ok(f) = Font::parse_cff_too(files[i].clone()) {
                for c in ('!'..='~').chain(['\u{e9}', '\u{c5}', '\u{2026}']) {
                    let g = f.glyph(c);
                    let _ = f.outline(g);
                    let _ = f.advance(g);
                }
                let _ =
                    forge_font::layout::lay_out(&f, "Fuzz 0\nSecond", forge_font::layout::Align::Center, 7.0);
            }
        });
        assert!(shipped("Source Sans 3", true, false).is_none());
        assert!(shipped("Open Sans", false, true).is_none());
    }
}

/// Every contour of the shipped faces over Latin-1 is simple in winding2d's sense
/// (verify/forge-text, `winding_is_topological`): no two pieces that are not neighbours meet,
/// and neighbours meet only at their shared end. Pieces are split at their middle until their
/// control boxes are apart by more than the splits' rounding; at a shared end the two end pieces'
/// control points must lie in cones from that end that share no direction (exact orient2d), the
/// curves lying in their hulls. Any contour that does not come apart within the splits allowed is
/// named.
#[cfg(test)]
mod simple {
    use super::*;
    use forge_font::Seg;
    use forge_math::Point2;
    use forge_math::predicates::orient2d;

    const DEPTH: usize = 28;

    fn split(p: &[Point2]) -> (Vec<Point2>, Vec<Point2>) {
        let mut rows = p.to_vec();
        let n = p.len();
        let (mut l, mut r) = (vec![rows[0]], vec![rows[n - 1]]);
        for k in 1..n {
            for i in 0..n - k {
                rows[i] = Point2::new(
                    (rows[i].x + rows[i + 1].x) / 2.0,
                    (rows[i].y + rows[i + 1].y) / 2.0,
                );
            }
            l.push(rows[0]);
            r.push(rows[n - 1 - k]);
        }
        r.reverse();
        (l, r)
    }

    fn bbox(p: &[Point2]) -> (Point2, Point2) {
        p.iter().fold(
            (
                Point2::new(f64::INFINITY, f64::INFINITY),
                Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(lo, hi), q| {
                (
                    Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                    Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                )
            },
        )
    }

    /// The two boxes are apart by more than `m`.
    fn apart(a: &[Point2], b: &[Point2], m: f64) -> bool {
        let ((al, ah), (bl, bh)) = (bbox(a), bbox(b));
        ah.x + m < bl.x || bh.x + m < al.x || ah.y + m < bl.y || bh.y + m < al.y
    }

    /// The control points of `a` and of `b` (all but `j`, their shared end) lie in cones from `j`
    /// sharing no direction: a line through `j` along one of the rays has all of one strictly on
    /// one side (or on that ray), all of the other strictly on the other.
    fn cones_apart(j: Point2, a: &[Point2], b: &[Point2]) -> bool {
        let (ua, ub): (Vec<Point2>, Vec<Point2>) = (
            a.iter().copied().filter(|&p| p != j).collect(),
            b.iter().copied().filter(|&p| p != j).collect(),
        );
        if ua.is_empty() || ub.is_empty() {
            return false;
        }
        let side = |r: Point2, p: Point2| orient2d(j, r, p);
        for (mine, other) in [(&ua, &ub), (&ub, &ua)] {
            for &r in mine.iter() {
                // the ray j -> r: one cone on it or left (or right), the other strictly the other way
                for s in [1.0, -1.0] {
                    let mine_ok = mine
                        .iter()
                        .all(|&p| s * side(r, p) >= 0.0 && (side(r, p) != 0.0 || (p - j).dot(r - j) > 0.0));
                    // strictly the other side, or on the opposite ray (a straight continuation)
                    let other_ok = other
                        .iter()
                        .all(|&p| s * side(r, p) < 0.0 || (side(r, p) == 0.0 && (p - j).dot(r - j) < 0.0));
                    if mine_ok && other_ok {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether pieces `a` and `b` (control points) meet only at the shared ends `joints`.
    fn only_at(a: &[Point2], b: &[Point2], joints: &[Point2], m: f64, depth: usize) -> bool {
        let ja: Vec<Point2> = joints
            .iter()
            .copied()
            .filter(|j| a.first() == Some(j) || a.last() == Some(j))
            .collect();
        let shared: Vec<Point2> = ja
            .iter()
            .copied()
            .filter(|j| b.first() == Some(j) || b.last() == Some(j))
            .collect();
        if shared.is_empty() && apart(a, b, m) {
            return true;
        }
        if let [j] = shared[..]
            && cones_apart(j, a, b)
        {
            return true;
        }
        if depth == 0 {
            return false;
        }
        let ((a0, a1), (b0, b1)) = (split(a), split(b));
        [(&a0, &b0), (&a0, &b1), (&a1, &b0), (&a1, &b1)]
            .iter()
            .all(|(x, y)| only_at(x, y, joints, m, depth - 1))
    }

    fn pts(s: &Seg) -> Vec<Point2> {
        s.points()
    }

    /// One contour: at least three pieces, none a point, non-neighbours apart, neighbours meeting
    /// only at their shared end.
    fn simple_contour(contour: &[Seg]) -> bool {
        let n = contour.len();
        let ps: Vec<Vec<Point2>> = contour.iter().map(pts).collect();
        let big = ps
            .iter()
            .flatten()
            .fold(1.0f64, |m, p| m.max(p.x.abs()).max(p.y.abs()));
        // the splits' rounding: a few units in the last place of the largest coordinate
        let m = 64.0 * f64::EPSILON * big;
        n >= 3
            && (0..n).all(|i| ps[i].first() != ps[i].last())
            && (0..n).all(|i| {
                (i + 1..n).all(|j| {
                    let mut joints = Vec::new();
                    if j == i + 1 {
                        joints.push(contour[i].end());
                    }
                    if i == 0 && j == n - 1 {
                        joints.push(contour[0].start());
                    }
                    only_at(&ps[i], &ps[j], &joints, m, DEPTH)
                })
            })
    }

    /// The check fails where it should: a contour pinched at a point two non-neighbouring sides
    /// share, one with a side touching another's interior, one doubling back on itself.
    #[test]
    fn the_check_finds_touches() {
        let p = Point2::new;
        let poly = |v: &[(f64, f64)]| -> Vec<Seg> {
            (0..v.len())
                .map(|k| {
                    Seg::Line(
                        p(v[k].0, v[k].1),
                        p(v[(k + 1) % v.len()].0, v[(k + 1) % v.len()].1),
                    )
                })
                .collect()
        };
        assert!(simple_contour(&poly(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0)
        ])));
        assert!(simple_contour(&poly(&[
            (0.0, 0.0),
            (2.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0)
        ])));
        assert!(!simple_contour(&poly(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (2.0, 2.0),
            (4.0, 4.0),
            (0.0, 4.0),
            (2.0, 2.0)
        ])));
        assert!(!simple_contour(&poly(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (2.0, 0.0),
            (0.0, 4.0)
        ])));
        assert!(!simple_contour(&poly(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (2.0, 0.0),
            (2.0, 4.0)
        ])));
        // a quadratic touching a side
        let q = vec![
            Seg::Line(p(0.0, 0.0), p(4.0, 0.0)),
            Seg::Quad(p(4.0, 0.0), p(2.0, -4.0), p(0.0, 4.0)),
            Seg::Line(p(0.0, 4.0), p(0.0, 0.0)),
        ];
        assert!(!simple_contour(&q));
    }

    #[test]
    fn shipped_contours_are_simple() {
        let mut bad = Vec::new();
        for (family, style) in faces() {
            let f = shipped(family, style.contains("Bold"), false).expect("shipped");
            for c in (0x21u32..0x7f).chain(0xa1..0x100).filter_map(char::from_u32) {
                if c == '\u{ad}' {
                    continue;
                }
                for (k, contour) in f.outline(f.glyph(c)).expect("outline").iter().enumerate() {
                    if !simple_contour(contour) {
                        bad.push(format!("{family} {style} {c:?} contour {k}"));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "contours not simple: {bad:?}");
    }
}
