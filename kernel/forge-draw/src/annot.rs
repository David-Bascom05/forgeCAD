//! Annotations drawn on a sheet (millimetres): dimensions, leaders, balloons, centre marks and
//! centrelines, hatching, tables. ANSI style: text horizontal (unidirectional), 3.5 mm, the
//! dimension line broken round it; arrowheads 3 mm long, 1:3, filled.

use crate::geom2::Geom2;
use crate::page::{Align, Page, Pen, Stroke, Text, text_width};
use forge_math::{Point2, Vec2, dmath};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub text_h: f64,
    pub arrow: f64,
    /// Between an extension line and the object it comes from.
    pub gap: f64,
    /// How far an extension line runs past its dimension line.
    pub over: f64,
    /// Decimal places at most; trailing zeros are not written.
    pub places: usize,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            text_h: 3.5,
            arrow: 3.0,
            gap: 1.0,
            over: 2.0,
            places: 2,
        }
    }
}

/// A length or angle as written: at most `places` decimals, no trailing zeros.
pub fn number(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if s == "-0" { "0".into() } else { s }
}

pub fn stroke(page: &mut Page, pen: Pen, a: Point2, b: Point2) {
    page.strokes.push(Stroke {
        pen,
        geom: Geom2::Line { a, b },
    });
}

/// A filled arrowhead with its tip at `tip`, pointing along `dir`.
pub fn arrow(page: &mut Page, pen: Pen, tip: Point2, dir: Vec2, len: f64) {
    let Some(d) = dir.normalize() else { return };
    let back = tip - d * len;
    let n = d.perp() * (len / 6.0);
    page.fills.push((pen, vec![tip, back + n, back - n]));
}

/// Marks a dimension text boxed (a basic dimension) when it starts the text.
pub const BOXED: char = '\u{1}';
/// Splits a dimension text into its value and a stacked pair: `value STACK upper UNDER lower`.
/// With no value the pair is the text, full size (limits); with one, small beside it (deviations).
pub const STACK: char = '\u{2}';
pub const UNDER: char = '\u{3}';

/// A dimension text's parts: boxed, the value, and the stacked pair.
fn parts(text: &str) -> (bool, &str, Option<(&str, &str)>) {
    let boxed = text.starts_with(BOXED);
    let t = text.trim_start_matches(BOXED);
    match t.split_once(STACK) {
        Some((main, pair)) => (boxed, main, Some(pair.split_once(UNDER).unwrap_or((pair, "")))),
        None => (boxed, t, None),
    }
}

/// The stacked pair's text height for a main text height `h`.
fn stack_h(main: &str, h: f64) -> f64 {
    if main.is_empty() { h } else { 0.7 * h }
}

/// How wide and high a dimension text is drawn at height `h`.
pub fn text_size(text: &str, h: f64) -> Vec2 {
    let (boxed, main, pair) = parts(text);
    let mut w = text_width(main, h);
    let mut ht = h;
    if let Some((u, l)) = pair {
        let sh = stack_h(main, h);
        w += text_width(u, sh).max(text_width(l, sh)) + if main.is_empty() { 0.0 } else { 0.8 };
        ht = ht.max(if main.is_empty() {
            2.0 * h + 1.5
        } else {
            2.0 * sh + 1.0
        });
    }
    if boxed {
        w += 2.0;
        ht += 2.0;
    }
    Vec2::new(w, ht)
}

/// The text's box half-sizes, for breaking lines round it.
fn half_box(text: &str, h: f64) -> Vec2 {
    let s = text_size(text, h);
    Vec2::new(0.5 * s.x + 1.0, 0.5 * s.y + 1.0)
}

/// A dimension text with its left, middle or right at `at` (on its middle line): its value, its
/// stacked limits or deviations, its basic box.
pub fn dim_text(page: &mut Page, pen: Pen, at: Point2, align: Align, h: f64, text: &str) {
    let size = text_size(text, h);
    let x0 = match align {
        Align::Left => at.x,
        Align::Centre => at.x - 0.5 * size.x,
        Align::Right => at.x - size.x,
    };
    let (boxed, main, pair) = parts(text);
    let mut x = x0 + if boxed { 1.0 } else { 0.0 };
    let put = |page: &mut Page, x: f64, y: f64, h: f64, s: &str| {
        page.texts.push(Text {
            at: Point2::new(x, y),
            h,
            angle: 0.0,
            align: Align::Left,
            text: s.to_string(),
            pen,
        });
    };
    if !main.is_empty() {
        put(page, x, at.y - 0.5 * h, h, main);
        x += text_width(main, h) + 0.8;
    }
    if let Some((u, l)) = pair {
        let sh = stack_h(main, h);
        let gap = if main.is_empty() { 0.75 } else { 0.5 };
        put(page, x, at.y + gap, sh, u);
        put(page, x, at.y - gap - sh, sh, l);
    }
    if boxed {
        let (lo, hi) = (
            Point2::new(x0, at.y - 0.5 * size.y),
            Point2::new(x0 + size.x, at.y + 0.5 * size.y),
        );
        let c = [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)];
        for k in 0..4 {
            stroke(page, pen, c[k], c[(k + 1) % 4]);
        }
    }
}

fn label(page: &mut Page, pen: Pen, centre: Point2, h: f64, text: &str) {
    dim_text(page, pen, centre, Align::Centre, h, text);
}

/// Draws `a`..`b` less the part inside the box round `c` (half sizes `hb`).
fn broken(page: &mut Page, pen: Pen, a: Point2, b: Point2, c: Point2, hb: Vec2) {
    let d = b - a;
    // the parameters where the segment is inside the box
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for (p, q, cl, ch) in [
        (a.x, d.x, c.x - hb.x, c.x + hb.x),
        (a.y, d.y, c.y - hb.y, c.y + hb.y),
    ] {
        if q.abs() < 1e-12 {
            if p < cl || p > ch {
                lo = 1.0;
                hi = 0.0;
            }
            continue;
        }
        let (t1, t2) = ((cl - p) / q, (ch - p) / q);
        lo = lo.max(t1.min(t2));
        hi = hi.min(t1.max(t2));
    }
    if lo >= hi {
        stroke(page, pen, a, b);
        return;
    }
    if lo > 0.0 {
        stroke(page, pen, a, a + d * lo);
    }
    if hi < 1.0 {
        stroke(page, pen, a + d * hi, b);
    }
}

/// A linear dimension between `a` and `b` measured along `dir`, its dimension line through `at`.
#[allow(clippy::too_many_arguments)]
pub fn linear(
    page: &mut Page,
    pen: Pen,
    st: &Style,
    a: Point2,
    b: Point2,
    dir: Vec2,
    at: Point2,
    text: &str,
) {
    let Some(u) = dir.normalize() else { return };
    let n = u.perp();
    // the feet of the extension lines on the dimension line
    let off = |p: Point2| (at - p).dot(n);
    let (fa, fb) = (a + n * off(a), b + n * off(b));
    for (p, f) in [(a, fa), (b, fb)] {
        let d = f - p;
        let l = d.length();
        if l > st.gap {
            let s = d * (1.0 / l);
            stroke(page, pen, p + s * st.gap, f + s * st.over);
        }
    }
    let mid = fa.lerp(fb, 0.5);
    let span = fa.distance(fb);
    let hb = half_box(text, st.text_h);
    let room = span > 2.0 * st.arrow + 2.0 * hb.x.max(hb.y) + 2.0;
    let along = (fb - fa).normalize().unwrap_or(u);
    if room {
        broken(page, pen, fa, fb, mid, hb);
        arrow(page, pen, fa, along * -1.0, st.arrow);
        arrow(page, pen, fb, along, st.arrow);
        label(page, pen, mid, st.text_h, text);
    } else {
        // arrows outside pointing in, the text past the second arrow
        stroke(
            page,
            pen,
            fa - along * (2.0 * st.arrow),
            fb + along * (2.0 * st.arrow),
        );
        arrow(page, pen, fa, along, st.arrow);
        arrow(page, pen, fb, along * -1.0, st.arrow);
        let reach = if along.x.abs() < 0.5 { hb.y } else { hb.x };
        let c = fb + along * (2.0 * st.arrow + reach + 1.0);
        label(page, pen, c, st.text_h, text);
    }
}

/// A leader: an arrow at `tip`, a line to `at`, a short shoulder and the text lines beside it.
pub fn leader(page: &mut Page, pen: Pen, st: &Style, tip: Point2, at: Point2, lines: &[String]) {
    let right = at.x >= tip.x;
    let s = if right { 1.0 } else { -1.0 };
    let shoulder = at + Vec2::new(s * 4.0, 0.0);
    stroke(page, pen, tip, at);
    stroke(page, pen, at, shoulder);
    arrow(page, pen, tip, tip - at, st.arrow);
    let x = shoulder.x + s * 1.0;
    let heights: Vec<f64> = lines
        .iter()
        .map(|l| text_size(l, st.text_h).y.max(st.text_h) + 0.6 * st.text_h)
        .collect();
    let total: f64 = heights.iter().sum();
    let mut y = at.y + 0.5 * total;
    for (l, lh) in lines.iter().zip(&heights) {
        y -= lh;
        dim_text(
            page,
            pen,
            Point2::new(x, y + 0.5 * lh),
            if right { Align::Left } else { Align::Right },
            st.text_h,
            l,
        );
    }
}

/// A radius or diameter: a leader from `at` through the circle at `c` (radius `r`, sheet mm)
/// to its edge, the arrow on the edge pointing at the centre.
pub fn radial(page: &mut Page, pen: Pen, st: &Style, c: Point2, r: f64, at: Point2, text: &str) {
    let Some(d) = (at - c).normalize() else { return };
    let tip = c + d * r;
    leader(page, pen, st, tip, at, &[text.to_string()]);
}

/// An angle between the lines from `v` along `da` and `db`, its arc through `at`.
#[allow(clippy::too_many_arguments)]
pub fn angular(page: &mut Page, pen: Pen, st: &Style, v: Point2, da: Vec2, db: Vec2, at: Point2, text: &str) {
    let (Some(da), Some(db)) = (da.normalize(), db.normalize()) else {
        return;
    };
    let r = (at - v).length();
    let (a0, a1) = (dmath::atan2(da.y, da.x), dmath::atan2(db.y, db.x));
    let mut sweep = a1 - a0;
    while sweep <= -dmath::PI {
        sweep += dmath::TAU;
    }
    while sweep > dmath::PI {
        sweep -= dmath::TAU;
    }
    let (t0, t1) = if sweep >= 0.0 {
        (a0, a0 + sweep)
    } else {
        (a1, a1 - sweep)
    };
    page.strokes.push(Stroke {
        pen,
        geom: Geom2::Conic {
            c: v,
            u: Vec2::new(r, 0.0),
            v: Vec2::new(0.0, r),
            t0,
            t1,
        },
    });
    let p = |t: f64| {
        let (s, c) = dmath::sin_cos(t);
        v + Vec2::new(c, s) * r
    };
    let tan = |t: f64| {
        let (s, c) = dmath::sin_cos(t);
        Vec2::new(-s, c)
    };
    arrow(page, pen, p(t0), tan(t0) * -1.0, st.arrow);
    arrow(page, pen, p(t1), tan(t1), st.arrow);
    let mid = p(0.5 * (t0 + t1));
    let out = (mid - v).normalize().unwrap_or(Vec2::Y);
    label(page, pen, mid + out * (0.5 * st.text_h + 2.0), st.text_h, text);
}

/// A balloon: the item number in a circle at `at`, a leader to `tip` ending in a dot.
pub fn balloon(page: &mut Page, pen: Pen, st: &Style, tip: Point2, at: Point2, n: &str) {
    let r = 0.5 * text_width(n, st.text_h).max(st.text_h) + 2.0;
    page.strokes.push(Stroke {
        pen,
        geom: Geom2::Conic {
            c: at,
            u: Vec2::new(r, 0.0),
            v: Vec2::new(0.0, r),
            t0: 0.0,
            t1: dmath::TAU,
        },
    });
    if let Some(d) = (tip - at).normalize() {
        stroke(page, pen, at + d * r, tip);
        let k = 0.6;
        let ring: Vec<Point2> = (0..8)
            .map(|i| {
                let (s, c) = dmath::sin_cos(dmath::TAU * i as f64 / 8.0);
                tip + Vec2::new(c, s) * k
            })
            .collect();
        page.fills.push((pen, ring));
    }
    label(page, pen, at, st.text_h, n);
}

/// A centre mark: a short cross at `c` and centrelines running `over` past the circle of radius `r`.
pub fn center_mark(page: &mut Page, c: Point2, r: f64, over: f64) {
    let e = r + over;
    let s = 1.5f64.min(0.4 * r);
    for d in [Vec2::X, Vec2::Y] {
        stroke(page, Pen::Thin, c - d * s, c + d * s);
        if r > 2.0 * s {
            stroke(page, Pen::Center, c + d * (s + 1.5), c + d * e);
            stroke(page, Pen::Center, c - d * (s + 1.5), c - d * e);
        }
    }
}

/// A centreline from `a` to `b`, run `over` past both.
pub fn centerline(page: &mut Page, a: Point2, b: Point2, over: f64) {
    let Some(d) = (b - a).normalize() else { return };
    stroke(page, Pen::Center, a - d * over, b + d * over);
}

/// Hatch lines at `angle` degrees, `spacing` apart, over the region the loops bound (even-odd).
pub fn hatch(page: &mut Page, loops: &[Vec<Point2>], angle: f64, spacing: f64) {
    let (s, c) = dmath::sin_cos(angle.to_radians());
    let (u, n) = (Vec2::new(c, s), Vec2::new(-s, c));
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in loops.iter().flatten() {
        let h = p.to_vec().dot(n);
        lo = lo.min(h);
        hi = hi.max(h);
    }
    if lo > hi || spacing <= 0.0 {
        return;
    }
    let mut h = (lo / spacing).floor() * spacing + 0.5 * spacing;
    while h < hi {
        let mut xs: Vec<f64> = Vec::new();
        for l in loops {
            for k in 0..l.len() {
                let (a, b) = (l[k], l[(k + 1) % l.len()]);
                let (ha, hb) = (a.to_vec().dot(n), b.to_vec().dot(n));
                if (ha <= h) != (hb <= h) {
                    let t = (h - ha) / (hb - ha);
                    xs.push(a.lerp(b, t).to_vec().dot(u));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for w in xs.chunks(2) {
            if let [x0, x1] = w {
                let p = |x: f64| Point2::ORIGIN + u * x + n * h;
                stroke(page, Pen::Hatch, p(*x0), p(*x1));
            }
        }
        h += spacing;
    }
}

/// A table whose lower right corner is at `at`: the header row on top, then the rows. Returns
/// the table's height.
pub fn table(page: &mut Page, at: Point2, header: &[&str], widths: &[f64], rows: &[Vec<String>]) -> f64 {
    let rh = 6.0;
    let w: f64 = widths.iter().sum();
    let n = rows.len() + 1;
    let x0 = at.x - w;
    let height = rh * n as f64;
    for k in 0..=n {
        let y = at.y + rh * k as f64;
        let pen = if k == 0 || k == n || k == n - 1 {
            Pen::Border
        } else {
            Pen::Thin
        };
        stroke(page, pen, Point2::new(x0, y), Point2::new(at.x, y));
    }
    let mut x = x0;
    for (i, cw) in widths.iter().enumerate() {
        stroke(
            page,
            if i == 0 { Pen::Border } else { Pen::Thin },
            Point2::new(x, at.y),
            Point2::new(x, at.y + height),
        );
        x += cw;
    }
    stroke(
        page,
        Pen::Border,
        Point2::new(at.x, at.y),
        Point2::new(at.x, at.y + height),
    );
    let put = |page: &mut Page, row: usize, cells: Vec<String>| {
        let y = at.y + height - rh * (row + 1) as f64 + 1.6;
        let mut x = x0;
        for (i, c) in cells.iter().enumerate() {
            let cw = widths.get(i).copied().unwrap_or(20.0);
            let mut t = c.clone();
            while !t.is_empty() && text_width(&t, 2.5) > cw - 2.0 {
                t.pop();
            }
            page.texts.push(Text {
                at: Point2::new(x + cw / 2.0, y),
                h: 2.5,
                angle: 0.0,
                align: Align::Centre,
                text: t,
                pen: Pen::Thin,
            });
            x += cw;
        }
    };
    put(page, 0, header.iter().map(|s| s.to_string()).collect());
    for (k, r) in rows.iter().enumerate() {
        put(page, k + 1, r.clone());
    }
    height
}

/// A view's label under it: `SECTION A-A`, `DETAIL B`, with the scale on the next line.
pub fn view_label(page: &mut Page, at: Point2, title: &str, scale: &str) {
    page.texts.push(Text {
        at,
        h: 5.0,
        angle: 0.0,
        align: Align::Centre,
        text: title.to_string(),
        pen: Pen::Thin,
    });
    if !scale.is_empty() {
        page.texts.push(Text {
            at: at - Vec2::new(0.0, 6.5),
            h: 3.5,
            angle: 0.0,
            align: Align::Centre,
            text: format!("SCALE {scale}"),
            pen: Pen::Thin,
        });
    }
}

/// A section's cutting-plane line along `pts` (sheet mm), arrows at both ends pointing along
/// `look` (where the section looks), and its letter at each end.
pub fn cutting_line(page: &mut Page, st: &Style, pts: &[Point2], look: Vec2, letter: &str) {
    for w in pts.windows(2) {
        stroke(page, Pen::Cut, w[0], w[1]);
    }
    let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
        return;
    };
    let Some(l) = look.normalize() else { return };
    for (p, out) in [
        (first, pts.get(1).map(|q| first - *q)),
        (last, pts.len().checked_sub(2).map(|i| last - pts[i])),
    ] {
        let Some(out) = out.and_then(|o| o.normalize()) else {
            continue;
        };
        let end = p + out * 3.0;
        stroke(page, Pen::Visible, p, end);
        let tip = end + l * 8.0;
        stroke(page, Pen::Visible, end, tip);
        arrow(page, Pen::Visible, tip, l, st.arrow + 1.0);
        label(page, Pen::Thin, tip + out * 4.0, 5.0, letter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_drop_trailing_zeros() {
        assert_eq!(number(906.0, 2), "906");
        assert_eq!(number(3.02, 2), "3.02");
        assert_eq!(number(2.5, 2), "2.5");
        assert_eq!(number(-0.001, 2), "0");
    }

    #[test]
    fn hatch_fills_a_square_with_a_hole() {
        let mut page = Page::default();
        let sq = |c: f64, r: f64| {
            vec![
                Point2::new(c - r, c - r),
                Point2::new(c + r, c - r),
                Point2::new(c + r, c + r),
                Point2::new(c - r, c + r),
            ]
        };
        hatch(&mut page, &[sq(10.0, 10.0), sq(10.0, 5.0)], 0.0, 1.0);
        let len: f64 = page.strokes.iter().map(|s| s.geom.length()).sum();
        // 20 lines of 20 less 10 lines of 10
        assert!((len - (400.0 - 100.0)).abs() < 1e-9, "{len}");
    }
}
