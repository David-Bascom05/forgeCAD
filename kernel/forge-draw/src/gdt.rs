//! Geometric dimensioning and tolerancing (ASME Y14.5-2018, ISO 1101 / 5459): feature control
//! frames, datum feature symbols, datum targets, surface texture (ASME Y14.36 / ISO 1302),
//! welding (AWS A2.4 / ISO 2553) and edge (ISO 13715) symbols, drawn to the standards'
//! proportions: frame height twice the text height. Every symbol and modifier glyph is drawn as
//! lines and arcs, so no font has to hold it.

use crate::annot::{arrow, stroke};
use crate::geom2::Geom2;
use crate::page::{Align, Page, Pen, Stroke, Text, text_width};
use forge_math::{Point2, Vec2, dmath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standard {
    Asme,
    Iso,
}

/// The fourteen geometric characteristics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Char {
    Straightness,
    Flatness,
    Circularity,
    Cylindricity,
    ProfileLine,
    ProfileSurface,
    Angularity,
    Perpendicularity,
    Parallelism,
    Position,
    Concentricity,
    Symmetry,
    CircularRunout,
    TotalRunout,
}

/// Whether a characteristic takes a datum reference frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Datums {
    Never,
    Optional,
    Required,
}

pub const CHARS: [(&str, Char); 14] = [
    ("straightness", Char::Straightness),
    ("flatness", Char::Flatness),
    ("circularity", Char::Circularity),
    ("cylindricity", Char::Cylindricity),
    ("profile_line", Char::ProfileLine),
    ("profile_surface", Char::ProfileSurface),
    ("angularity", Char::Angularity),
    ("perpendicularity", Char::Perpendicularity),
    ("parallelism", Char::Parallelism),
    ("position", Char::Position),
    ("concentricity", Char::Concentricity),
    ("symmetry", Char::Symmetry),
    ("circular_runout", Char::CircularRunout),
    ("total_runout", Char::TotalRunout),
];

impl Char {
    pub fn from_word(w: &str) -> Option<Char> {
        CHARS.iter().find(|c| c.0 == w).map(|c| c.1)
    }
    pub fn word(self) -> &'static str {
        CHARS.iter().find(|c| c.1 == self).map_or("", |c| c.0)
    }
    pub fn datums(self) -> Datums {
        match self {
            Char::Straightness | Char::Flatness | Char::Circularity | Char::Cylindricity => Datums::Never,
            Char::ProfileLine | Char::ProfileSurface | Char::Position => Datums::Optional,
            _ => Datums::Required,
        }
    }
    /// Whether a diameter zone makes sense for it.
    pub fn may_be_diameter(self) -> bool {
        matches!(
            self,
            Char::Straightness
                | Char::Position
                | Char::Concentricity
                | Char::Perpendicularity
                | Char::Parallelism
                | Char::Angularity
        )
    }
    /// Whether a standard has it: Y14.5-2018 dropped concentricity and symmetry.
    pub fn in_standard(self, s: Standard) -> bool {
        !(s == Standard::Asme && matches!(self, Char::Concentricity | Char::Symmetry))
    }
}

/// A modifier in a tolerance or datum cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modifier {
    Mmc,
    Lmc,
    /// Regardless of feature size: written only under ISO or older ASME, implied by Y14.5-2018.
    Rfs,
    /// A projected zone, and its height when given.
    Projected(Option<f64>),
    Free,
    Tangent,
    /// Unequally disposed profile: how far the zone lies outside.
    Unequal(f64),
    Statistical,
}

impl Modifier {
    /// Whether it may only be applied to a feature of size.
    pub fn needs_size(self) -> bool {
        matches!(self, Modifier::Mmc | Modifier::Lmc)
    }
    fn letters(self) -> &'static str {
        match self {
            Modifier::Mmc => "M",
            Modifier::Lmc => "L",
            Modifier::Rfs => "S",
            Modifier::Projected(_) => "P",
            Modifier::Free => "F",
            Modifier::Tangent => "T",
            Modifier::Unequal(_) => "U",
            Modifier::Statistical => "ST",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tol {
    pub dia: bool,
    /// A spherical zone (S⌀).
    pub sphere: bool,
    pub value: f64,
    pub mods: Vec<Modifier>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatumRef {
    /// One letter, or a common datum `A-B`.
    pub letters: String,
    pub mods: Vec<Modifier>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub ch: Char,
    pub tol: Tol,
    pub datums: Vec<DatumRef>,
}

fn modifier(word: &str) -> Option<Modifier> {
    Some(match word {
        "M" => Modifier::Mmc,
        "L" => Modifier::Lmc,
        "S" => Modifier::Rfs,
        "P" => Modifier::Projected(None),
        "F" => Modifier::Free,
        "T" => Modifier::Tangent,
        "ST" => Modifier::Statistical,
        w if w.starts_with('P') => Modifier::Projected(Some(w[1..].parse().ok()?)),
        w if w.starts_with('U') => Modifier::Unequal(w[1..].parse().ok()?),
        _ => return None,
    })
}

/// A tolerance cell from its words: `dia 0.2 M`, `0.05`, `Sdia 0.1`, `0.4 U0.1`, `dia 0.3 M P12`.
pub fn parse_tol(s: &str) -> Result<Tol, String> {
    let mut t = Tol {
        dia: false,
        sphere: false,
        value: f64::NAN,
        mods: vec![],
    };
    for w in s.split_whitespace() {
        match w {
            "dia" | "\u{2300}" | "\u{d8}" => t.dia = true,
            "Sdia" => {
                t.dia = true;
                t.sphere = true;
            }
            _ => match w.parse::<f64>() {
                Ok(v) if t.value.is_nan() && v >= 0.0 => t.value = v,
                _ => t.mods.push(
                    modifier(w)
                        .ok_or_else(|| format!("`{w}`: modifiers are M L S P F T U<n> ST, P<height>"))?,
                ),
            },
        }
    }
    if t.value.is_nan() {
        return Err("a tolerance needs its value".into());
    }
    Ok(t)
}

/// A datum cell: `A`, `B M`, `A-B`.
pub fn parse_datum(s: &str) -> Result<DatumRef, String> {
    let mut w = s.split_whitespace();
    let letters = w.next().ok_or("an empty datum")?.to_string();
    if !letters
        .split('-')
        .all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_uppercase()))
    {
        return Err(format!(
            "`{letters}`: a datum is a capital letter, or two joined: A-B"
        ));
    }
    let mods = w
        .map(|x| match modifier(x) {
            Some(m @ (Modifier::Mmc | Modifier::Lmc | Modifier::Rfs)) => Ok(m),
            _ => Err(format!("`{x}`: a datum takes M, L or S")),
        })
        .collect::<Result<_, _>>()?;
    Ok(DatumRef { letters, mods })
}

/// The letters a datum may be: not I, O or Q.
pub fn datum_letter_ok(l: &str) -> bool {
    !l.is_empty() && l.chars().all(|c| c.is_ascii_uppercase() && !"IOQ".contains(c))
}

fn line(page: &mut Page, a: Point2, b: Point2) {
    stroke(page, Pen::Thin, a, b);
}

fn circle(page: &mut Page, c: Point2, r: f64, t0: f64, t1: f64) {
    page.strokes.push(Stroke {
        pen: Pen::Thin,
        geom: Geom2::Conic {
            c,
            u: Vec2::new(r, 0.0),
            v: Vec2::new(0.0, r),
            t0,
            t1,
        },
    });
}

fn poly(page: &mut Page, o: Point2, s: f64, pts: &[(f64, f64)]) {
    for w in pts.windows(2) {
        line(
            page,
            o + Vec2::new(w[0].0, w[0].1) * s,
            o + Vec2::new(w[1].0, w[1].1) * s,
        );
    }
}

/// A characteristic's symbol in a cell of half size `r` centred at `c` (ASME Y14.5 Fig. 3-1's
/// shapes, scaled to the text height).
pub fn glyph(page: &mut Page, ch: Char, c: Point2, r: f64) {
    let k = 0.6 * r;
    match ch {
        Char::Straightness => poly(page, c, k, &[(-1.0, 0.0), (1.0, 0.0)]),
        Char::Flatness => poly(
            page,
            c,
            k,
            &[
                (-1.0, -0.45),
                (0.5, -0.45),
                (1.0, 0.45),
                (-0.5, 0.45),
                (-1.0, -0.45),
            ],
        ),
        Char::Circularity => circle(page, c, 0.8 * k, 0.0, dmath::TAU),
        Char::Cylindricity => {
            circle(page, c, 0.6 * k, 0.0, dmath::TAU);
            poly(page, c, k, &[(-1.0, -0.75), (-0.15, 0.75)]);
            poly(page, c, k, &[(0.15, -0.75), (1.0, 0.75)]);
        }
        Char::ProfileLine => circle(page, c - Vec2::new(0.0, 0.4 * k), k, 0.0, dmath::PI),
        Char::ProfileSurface => {
            circle(page, c - Vec2::new(0.0, 0.4 * k), k, 0.0, dmath::PI);
            poly(page, c, k, &[(-1.0, -0.4), (1.0, -0.4)]);
        }
        Char::Angularity => poly(page, c, k, &[(1.0, -0.6), (-1.0, -0.6), (0.6, 0.7)]),
        Char::Perpendicularity => {
            poly(page, c, k, &[(-1.0, -0.7), (1.0, -0.7)]);
            poly(page, c, k, &[(0.0, -0.7), (0.0, 0.8)]);
        }
        Char::Parallelism => {
            poly(page, c, k, &[(-0.7, -0.8), (-0.1, 0.8)]);
            poly(page, c, k, &[(0.1, -0.8), (0.7, 0.8)]);
        }
        Char::Position => {
            circle(page, c, 0.6 * k, 0.0, dmath::TAU);
            poly(page, c, k, &[(-1.0, 0.0), (1.0, 0.0)]);
            poly(page, c, k, &[(0.0, -1.0), (0.0, 1.0)]);
        }
        Char::Concentricity => {
            circle(page, c, 0.85 * k, 0.0, dmath::TAU);
            circle(page, c, 0.45 * k, 0.0, dmath::TAU);
        }
        Char::Symmetry => {
            poly(page, c, k, &[(-0.6, 0.55), (0.6, 0.55)]);
            poly(page, c, k, &[(-1.0, 0.0), (1.0, 0.0)]);
            poly(page, c, k, &[(-0.6, -0.55), (0.6, -0.55)]);
        }
        Char::CircularRunout => {
            let (a, b) = (c + Vec2::new(-0.5, -0.9) * k, c + Vec2::new(0.5, 0.9) * k);
            line(page, a, b);
            arrow(page, Pen::Thin, b, b - a, 0.6 * k);
        }
        Char::TotalRunout => {
            for dx in [-0.75, 0.15] {
                let (a, b) = (c + Vec2::new(dx, -0.9) * k, c + Vec2::new(dx + 0.6, 0.9) * k);
                line(page, a, b);
                arrow(page, Pen::Thin, b, b - a, 0.6 * k);
            }
            poly(page, c, k, &[(-0.75, -0.9), (0.15, -0.9)]);
        }
    }
}

/// A capital letter (of the few modifiers use) drawn as strokes in a box `w` x `h` from `o`.
fn stroke_letter(page: &mut Page, ch: char, o: Point2, h: f64) {
    let w = 0.7 * h;
    let p = |x: f64, y: f64| o + Vec2::new(x * w, y * h);
    let ls: &[&[(f64, f64)]] = match ch {
        'M' => &[&[(0.0, 0.0), (0.0, 1.0), (0.5, 0.35), (1.0, 1.0), (1.0, 0.0)]],
        'L' => &[&[(0.1, 1.0), (0.1, 0.0), (0.95, 0.0)]],
        'P' => &[&[
            (0.1, 0.0),
            (0.1, 1.0),
            (0.75, 1.0),
            (0.95, 0.85),
            (0.95, 0.65),
            (0.75, 0.5),
            (0.1, 0.5),
        ]],
        'F' => &[
            &[(0.95, 1.0), (0.1, 1.0), (0.1, 0.0)],
            &[(0.1, 0.52), (0.75, 0.52)],
        ],
        'T' => &[&[(0.0, 1.0), (1.0, 1.0)], &[(0.5, 1.0), (0.5, 0.0)]],
        'U' => &[&[
            (0.1, 1.0),
            (0.1, 0.25),
            (0.3, 0.0),
            (0.7, 0.0),
            (0.9, 0.25),
            (0.9, 1.0),
        ]],
        'S' => &[&[
            (0.95, 0.85),
            (0.75, 1.0),
            (0.25, 1.0),
            (0.05, 0.85),
            (0.05, 0.65),
            (0.25, 0.5),
            (0.75, 0.5),
            (0.95, 0.35),
            (0.95, 0.15),
            (0.75, 0.0),
            (0.25, 0.0),
            (0.05, 0.15),
        ]],
        _ => &[],
    };
    for l in ls {
        for k in 0..l.len() - 1 {
            line(page, p(l[k].0, l[k].1), p(l[k + 1].0, l[k + 1].1));
        }
    }
}

/// A circled modifier at `c` for text height `h`; returns its width.
fn circled(page: &mut Page, m: Modifier, c: Point2, h: f64) -> f64 {
    let letters = m.letters();
    let r = 0.62 * h;
    if letters.len() == 1 {
        circle(page, c, r, 0.0, dmath::TAU);
        let lh = 0.75 * h;
        stroke_letter(
            page,
            letters.chars().next().unwrap_or(' '),
            c - Vec2::new(0.35 * lh, 0.5 * lh),
            lh,
        );
        2.0 * r
    } else {
        // ST in a hexagon, as Y14.5 draws the statistical symbol
        let w = 1.4 * h;
        let pts: Vec<Point2> = [
            (-1.0, 0.0),
            (-0.7, 0.6),
            (0.7, 0.6),
            (1.0, 0.0),
            (0.7, -0.6),
            (-0.7, -0.6),
            (-1.0, 0.0),
        ]
        .iter()
        .map(|&(x, y)| c + Vec2::new(x * 0.5 * w, y * h))
        .collect();
        for k in 0..pts.len() - 1 {
            line(page, pts[k], pts[k + 1]);
        }
        let lh = 0.6 * h;
        stroke_letter(page, 'S', c - Vec2::new(0.45 * w * 0.9, 0.5 * lh), lh);
        stroke_letter(page, 'T', c + Vec2::new(0.02 * w, -0.5 * lh), lh);
        w
    }
}

/// The diameter sign as strokes: a circle and its slash.
fn diameter(page: &mut Page, c: Point2, h: f64) -> f64 {
    let r = 0.38 * h;
    circle(page, c, r, 0.0, dmath::TAU);
    line(page, c + Vec2::new(-0.55, -0.6) * h, c + Vec2::new(0.55, 0.6) * h);
    1.1 * h
}

fn text(page: &mut Page, s: &str, left_mid: Point2, h: f64) -> f64 {
    page.texts.push(Text {
        at: left_mid - Vec2::new(0.0, 0.5 * h),
        h,
        angle: 0.0,
        align: Align::Left,
        text: s.to_string(),
        pen: Pen::Thin,
    });
    text_width(s, h)
}

fn number(v: f64) -> String {
    crate::annot::number(v, 4)
}

/// How wide a tolerance cell's content is.
fn tol_width(t: &Tol, h: f64, std: Standard) -> f64 {
    let mut w = text_width(&number(t.value), h);
    if t.dia {
        w += 1.3 * h + if t.sphere { text_width("S", h) } else { 0.0 };
    }
    for m in &t.mods {
        if *m == Modifier::Rfs && std == Standard::Asme {
            continue;
        }
        w += 1.4 * h + 0.4 * h;
        match m {
            Modifier::Projected(Some(x)) | Modifier::Unequal(x) => w += text_width(&number(*x), h) + 0.4 * h,
            _ => {}
        }
    }
    w
}

/// Draws a tolerance cell's content from `left` (its middle line), returns its width.
fn tol_cell(page: &mut Page, t: &Tol, left: Point2, h: f64, std: Standard) -> f64 {
    let mut x = left.x;
    let y = left.y;
    if t.dia {
        if t.sphere {
            x += text(page, "S", Point2::new(x, y), h);
        }
        x += diameter(page, Point2::new(x + 0.55 * h, y), h) + 0.2 * h;
    }
    x += text(page, &number(t.value), Point2::new(x, y), h);
    for m in &t.mods {
        if *m == Modifier::Rfs && std == Standard::Asme {
            continue;
        }
        x += 0.4 * h;
        let w = circled(page, *m, Point2::new(x + 0.7 * h, y), h);
        x += w.max(1.4 * h);
        if let Modifier::Projected(Some(v)) | Modifier::Unequal(v) = m {
            x += 0.4 * h;
            x += text(page, &number(*v), Point2::new(x, y), h);
        }
    }
    x - left.x
}

fn datum_width(d: &DatumRef, h: f64) -> f64 {
    text_width(&d.letters, h) + d.mods.len() as f64 * 1.8 * h
}

fn datum_cell(page: &mut Page, d: &DatumRef, left: Point2, h: f64, std: Standard) -> f64 {
    let mut x = left.x + text(page, &d.letters, left, h);
    for m in &d.mods {
        if *m == Modifier::Rfs && std == Standard::Asme {
            continue;
        }
        x += 0.4 * h;
        x += circled(page, *m, Point2::new(x + 0.7 * h, left.y), h);
    }
    x - left.x
}

/// A feature control frame with its lower left corner at `at`, rows top to bottom; a composite
/// frame's rows share one symbol cell. Returns the frame's upper right corner.
pub fn frame(page: &mut Page, at: Point2, h: f64, rows: &[Row], composite: bool, std: Standard) -> Point2 {
    let rh = 2.0 * h;
    let pad = 0.5 * h;
    let sym = rh;
    let tol_w = rows.iter().map(|r| tol_width(&r.tol, h, std)).fold(0.0, f64::max) + 2.0 * pad;
    let ncols = rows.iter().map(|r| r.datums.len()).max().unwrap_or(0);
    let col_w: Vec<f64> = (0..ncols)
        .map(|c| {
            rows.iter()
                .filter_map(|r| r.datums.get(c))
                .map(|d| datum_width(d, h) + 2.0 * pad)
                .fold(rh, f64::max)
        })
        .collect();
    let mut right = at.x;
    let n = rows.len();
    for (k, row) in rows.iter().enumerate() {
        let y0 = at.y + rh * (n - 1 - k) as f64;
        let ym = y0 + 0.5 * rh;
        let mut xs = vec![at.x, at.x + sym, at.x + sym + tol_w];
        for w in col_w.iter().take(row.datums.len()) {
            let last = *xs.last().expect("cells");
            xs.push(last + w);
        }
        let x1 = *xs.last().expect("cells");
        right = right.max(x1);
        // the row's box and cell walls; a composite frame's symbol cell is one
        line(
            page,
            Point2::new(at.x + if composite && k > 0 { sym } else { 0.0 }, y0 + rh),
            Point2::new(x1, y0 + rh),
        );
        line(page, Point2::new(at.x, y0), Point2::new(x1, y0));
        for (i, x) in xs.iter().enumerate() {
            if composite && i == 0 && k > 0 {
                continue;
            }
            line(page, Point2::new(*x, y0), Point2::new(*x, y0 + rh));
        }
        if !composite || k == 0 {
            let c = if composite {
                Point2::new(at.x + 0.5 * sym, at.y + 0.5 * rh * n as f64)
            } else {
                Point2::new(at.x + 0.5 * sym, ym)
            };
            glyph(page, row.ch, c, 0.5 * rh);
        }
        tol_cell(page, &row.tol, Point2::new(xs[1] + pad, ym), h, std);
        for (i, d) in row.datums.iter().enumerate() {
            let w = datum_width(d, h);
            let cx = 0.5 * (xs[2 + i] + xs[3 + i]);
            datum_cell(page, d, Point2::new(cx - 0.5 * w, ym), h, std);
        }
    }
    if composite {
        line(
            page,
            Point2::new(at.x, at.y),
            Point2::new(at.x, at.y + rh * n as f64),
        );
        line(
            page,
            Point2::new(at.x, at.y + rh * n as f64),
            Point2::new(at.x + sym, at.y + rh * n as f64),
        );
    }
    Point2::new(right, at.y + rh * n as f64)
}

/// A datum feature symbol: the letter in a square box of side `2h` centred at `at`, joined to a
/// filled triangle standing on the feature at `foot`.
pub fn datum_symbol(page: &mut Page, letter: &str, at: Point2, foot: Point2, h: f64) {
    let s = (2.0 * h).max(text_width(letter, h) + h);
    let lo = at - Vec2::new(0.5 * s, 0.5 * s);
    let c = [
        lo,
        lo + Vec2::new(s, 0.0),
        lo + Vec2::new(s, s),
        lo + Vec2::new(0.0, s),
    ];
    for k in 0..4 {
        line(page, c[k], c[(k + 1) % 4]);
    }
    page.texts.push(Text {
        at: at - Vec2::new(0.0, 0.5 * h),
        h,
        angle: 0.0,
        align: Align::Centre,
        text: letter.to_string(),
        pen: Pen::Thin,
    });
    let Some(d) = (at - foot).normalize() else { return };
    // the leader leaves the box's nearest side
    let leave = if d.x.abs() > d.y.abs() {
        at - Vec2::new(d.x.signum() * 0.5 * s, 0.0)
    } else {
        at - Vec2::new(0.0, d.y.signum() * 0.5 * s)
    };
    let th = 0.9 * h;
    let base = foot + d * th;
    line(page, leave, base);
    let n = d.perp() * (th / 3f64.sqrt());
    page.fills.push((Pen::Thin, vec![foot + n, foot - n, base]));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Point,
    Line,
    Area,
}

/// A datum target: the divided circle at `at` (size above, label below), a leader to `on`, and
/// there an X (point, line) or a phantom circle of `area_r` sheet mm (area).
#[allow(clippy::too_many_arguments)]
pub fn datum_target(
    page: &mut Page,
    at: Point2,
    on: Point2,
    label: &str,
    size: &str,
    kind: Target,
    area_r: f64,
    h: f64,
) {
    let r = (1.75 * h)
        .max(0.5 * text_width(label, h) + 1.0)
        .max(0.5 * text_width(size, 0.8 * h) + 1.5);
    circle(page, at, r, 0.0, dmath::TAU);
    line(page, at - Vec2::new(r, 0.0), at + Vec2::new(r, 0.0));
    page.texts.push(Text {
        at: at - Vec2::new(0.0, 0.5 * r + 0.5 * h),
        h,
        angle: 0.0,
        align: Align::Centre,
        text: label.to_string(),
        pen: Pen::Thin,
    });
    if !size.is_empty() {
        page.texts.push(Text {
            at: at + Vec2::new(0.0, 0.5 * r - 0.4 * h),
            h: 0.8 * h,
            angle: 0.0,
            align: Align::Centre,
            text: size.to_string(),
            pen: Pen::Thin,
        });
    }
    if let Some(d) = (on - at).normalize() {
        line(page, at + d * r, on);
    }
    match kind {
        Target::Point | Target::Line => {
            let k = 0.7 * h;
            line(page, on + Vec2::new(-k, -k), on + Vec2::new(k, k));
            line(page, on + Vec2::new(-k, k), on + Vec2::new(k, -k));
        }
        Target::Area => {
            page.strokes.push(Stroke {
                pen: Pen::Phantom,
                geom: Geom2::Conic {
                    c: on,
                    u: Vec2::new(area_r, 0.0),
                    v: Vec2::new(0.0, area_r),
                    t0: 0.0,
                    t1: dmath::TAU,
                },
            });
            crate::annot::hatch(
                page,
                &[(0..32)
                    .map(|i| {
                        let (s, c) = dmath::sin_cos(dmath::TAU * i as f64 / 32.0);
                        on + Vec2::new(c, s) * area_r
                    })
                    .collect()],
                45.0,
                (0.6 * area_r).clamp(0.6, 1.5),
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Process {
    Any,
    Required,
    Forbidden,
}

/// A surface texture symbol with its vertex on the surface at `on`, standing on the side `up`
/// points to: the value (Ra, Rz..) by the long leg, the lay at its foot.
#[allow(clippy::too_many_arguments)]
pub fn surface_texture(
    page: &mut Page,
    on: Point2,
    up: Vec2,
    value: &str,
    process: Process,
    lay: &str,
    h: f64,
    std: Standard,
) {
    let Some(u) = up.normalize() else { return };
    let side = u.perp() * -1.0;
    let (s60, c60) = (dmath::sin(dmath::PI / 3.0), dmath::cos(dmath::PI / 3.0));
    let h1 = 1.4 * h;
    let h2 = 3.0 * h;
    let short = on + (side * -c60 + u * s60) * (h1 / s60);
    let long = on + (side * c60 + u * s60) * (h2 / s60);
    line(page, on, short);
    line(page, on, long);
    match process {
        Process::Any => {}
        Process::Required => line(page, short, on + (side * c60 + u * s60) * (h1 / s60)),
        Process::Forbidden => {
            let r = 0.45 * h1;
            circle(
                page,
                on + u * (r / dmath::sin(dmath::PI / 6.0)).min(h1 * 0.85),
                r,
                0.0,
                dmath::TAU,
            );
        }
    }
    // the value: ISO on the long leg's horizontal extension, ASME over the V
    let ext = long + side * (text_width(value, h).max(2.0 * h) + h);
    if std == Standard::Iso && !value.is_empty() {
        line(page, long, ext);
        page.texts.push(Text {
            at: long.lerp(ext, 0.5) - u * (h + 0.6),
            h,
            angle: 0.0,
            align: Align::Centre,
            text: value.to_string(),
            pen: Pen::Thin,
        });
    } else if !value.is_empty() {
        // ASME: the value beside the long leg's top, clear of the V
        page.texts.push(Text {
            at: long + side * (0.5 * h) - Vec2::new(0.0, 0.5 * h),
            h,
            angle: 0.0,
            align: if side.x >= 0.0 { Align::Left } else { Align::Right },
            text: value.to_string(),
            pen: Pen::Thin,
        });
    }
    if !lay.is_empty() {
        page.texts.push(Text {
            at: on + side * (h2 * 0.8) + u * (0.5 * h) - Vec2::new(0.0, 0.5 * h),
            h,
            angle: 0.0,
            align: Align::Left,
            text: lay.to_string(),
            pen: Pen::Thin,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeldKind {
    Fillet,
    Square,
    V,
    Bevel,
    Plug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeldSide {
    Arrow,
    Other,
    Both,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Weld {
    pub kind: WeldKind,
    pub side: WeldSide,
    pub size: String,
    /// Length, or length-pitch.
    pub length: String,
    pub all_around: bool,
    pub field: bool,
    pub tail: String,
}

/// A welding symbol: the arrow from the reference line's near end at `at` to the joint at
/// `tip`, the weld symbol on the arrow side (below the line) or other side (above), size to its
/// left, length-pitch to its right, all-around circle and field flag at the bend, the tail at the
/// far end. ISO 2553 adds a dashed identification line on the other side.
pub fn weld(page: &mut Page, tip: Point2, at: Point2, w: &Weld, h: f64, std: Standard) {
    let s = if tip.x <= at.x { 1.0 } else { -1.0 };
    let len = 10.0 * h;
    let far = at + Vec2::new(s * len, 0.0);
    line(page, at, far);
    line(page, at, tip);
    arrow(page, Pen::Thin, tip, tip - at, 0.9 * h);
    if w.all_around {
        circle(page, at, 0.6 * h, 0.0, dmath::TAU);
    }
    if w.field {
        let top = at + Vec2::new(0.0, 2.2 * h);
        line(page, at, top);
        page.fills.push((
            Pen::Thin,
            vec![
                top,
                top + Vec2::new(s * 1.4 * h, -0.5 * h),
                top - Vec2::new(0.0, h),
            ],
        ));
    }
    if std == Standard::Iso {
        page.strokes.push(Stroke {
            pen: Pen::Hidden,
            geom: Geom2::Line {
                a: at + Vec2::new(s * 1.5 * h, 0.8 * h),
                b: far + Vec2::new(0.0, 0.8 * h),
            },
        });
    }
    let mid = at + Vec2::new(s * 0.45 * len, 0.0);
    let k = 1.2 * h;
    let mut draw = |dir: f64| {
        let o = mid;
        let p = |x: f64, y: f64| o + Vec2::new(x * k, y * k * dir);
        match w.kind {
            WeldKind::Fillet => {
                line(page, p(-0.5, 0.0), p(-0.5, 1.0));
                line(page, p(-0.5, 1.0), p(0.5, 0.0));
            }
            WeldKind::Square => {
                line(page, p(-0.25, 0.0), p(-0.25, 0.9));
                line(page, p(0.25, 0.0), p(0.25, 0.9));
            }
            WeldKind::V => {
                line(page, p(0.0, 0.0), p(-0.6, 1.0));
                line(page, p(0.0, 0.0), p(0.6, 1.0));
            }
            WeldKind::Bevel => {
                line(page, p(-0.3, 0.0), p(-0.3, 1.0));
                line(page, p(-0.3, 0.0), p(0.4, 1.0));
            }
            WeldKind::Plug => {
                line(page, p(-0.6, 0.0), p(-0.6, 0.7));
                line(page, p(-0.6, 0.7), p(0.6, 0.7));
                line(page, p(0.6, 0.7), p(0.6, 0.0));
            }
        }
        let ty = if dir < 0.0 {
            -0.55 * k - 0.5 * h
        } else {
            0.55 * k - 0.5 * h
        };
        if !w.size.is_empty() {
            page.texts.push(Text {
                at: mid + Vec2::new(-0.8 * k, ty),
                h,
                angle: 0.0,
                align: Align::Right,
                text: w.size.clone(),
                pen: Pen::Thin,
            });
        }
        if !w.length.is_empty() {
            page.texts.push(Text {
                at: mid + Vec2::new(0.8 * k, ty),
                h,
                angle: 0.0,
                align: Align::Left,
                text: w.length.clone(),
                pen: Pen::Thin,
            });
        }
    };
    match w.side {
        WeldSide::Arrow => draw(-1.0),
        WeldSide::Other => draw(1.0),
        WeldSide::Both => {
            draw(-1.0);
            draw(1.0);
        }
    }
    if !w.tail.is_empty() {
        line(page, far, far + Vec2::new(s * 1.2 * h, 1.2 * h));
        line(page, far, far + Vec2::new(s * 1.2 * h, -1.2 * h));
        page.texts.push(Text {
            at: far + Vec2::new(s * 1.6 * h, -0.5 * h),
            h,
            angle: 0.0,
            align: if s > 0.0 { Align::Left } else { Align::Right },
            text: w.tail.clone(),
            pen: Pen::Thin,
        });
    }
}

/// An edge condition symbol (ISO 13715): the corner mark at `at` with its limits, joined to `on`.
pub fn edge(page: &mut Page, at: Point2, on: Point2, upper: &str, lower: &str, h: f64) {
    let s = 2.0 * h;
    // the corner, opening down-left as the standard draws it
    line(page, at, at + Vec2::new(0.0, s));
    line(page, at, at + Vec2::new(s, 0.0));
    let sp = |p: Point2, t: &str, page: &mut Page| {
        page.texts.push(Text {
            at: p,
            h: 0.8 * h,
            angle: 0.0,
            align: Align::Left,
            text: t.to_string(),
            pen: Pen::Thin,
        });
    };
    sp(at + Vec2::new(0.3 * h, s - 0.9 * h), upper, page);
    if !lower.is_empty() {
        sp(at + Vec2::new(0.3 * h, 0.3 * h), lower, page);
    }
    line(page, at, on);
    arrow(page, Pen::Thin, on, on - at, 0.9 * h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_parse() {
        let t = parse_tol("dia 0.2 M P12").unwrap();
        assert!(t.dia && t.value == 0.2);
        assert_eq!(t.mods, vec![Modifier::Mmc, Modifier::Projected(Some(12.0))]);
        assert!(parse_tol("dia").is_err());
        assert!(parse_tol("0.1 Q").is_err());
        let d = parse_datum("B M").unwrap();
        assert_eq!(
            (d.letters.as_str(), d.mods.as_slice()),
            ("B", &[Modifier::Mmc][..])
        );
        assert!(parse_datum("A-B").is_ok());
        assert!(parse_datum("b").is_err());
        assert!(parse_datum("A P").is_err());
        assert!(!datum_letter_ok("I") && datum_letter_ok("A"));
        assert_eq!(Char::from_word("position"), Some(Char::Position));
        assert!(!Char::Concentricity.in_standard(Standard::Asme));
        assert!(Char::Concentricity.in_standard(Standard::Iso));
    }

    #[test]
    fn a_frame_is_twice_the_text_high() {
        let mut page = Page::default();
        let rows = vec![Row {
            ch: Char::Position,
            tol: parse_tol("dia 0.2 M").unwrap(),
            datums: vec![
                parse_datum("A").unwrap(),
                parse_datum("B").unwrap(),
                parse_datum("C").unwrap(),
            ],
        }];
        let top = frame(&mut page, Point2::ORIGIN, 3.5, &rows, false, Standard::Asme);
        assert!((top.y - 7.0).abs() < 1e-12);
        // symbol, tolerance and three datum cells: six walls
        let walls = page
            .strokes
            .iter()
            .filter(|s| matches!(s.geom, Geom2::Line { a, b } if a.x == b.x && (b.y - a.y).abs() == 7.0))
            .count();
        assert_eq!(walls, 6);
        // the modifier and diameter are strokes, not text
        assert!(
            page.texts
                .iter()
                .all(|t| !t.text.contains('M') && !t.text.contains('\u{2300}'))
        );
    }
}
