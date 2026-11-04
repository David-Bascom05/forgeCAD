//! Our stroked icon set: paths on a 24 by 24 grid in `icons.txt`, read once and drawn as egui lines.

use egui::{Color32, Painter, Pos2, Shape, Stroke, pos2};
use std::collections::HashMap;
use std::sync::OnceLock;

const DATA: &str = include_str!("icons.txt");
const SHEET: &str = include_str!("sheet_icons.txt");

type Lines = Vec<Vec<(f32, f32)>>;

fn all() -> &'static HashMap<&'static str, Lines> {
    static ICONS: OnceLock<HashMap<&'static str, Lines>> = OnceLock::new();
    ICONS.get_or_init(|| {
        DATA.lines()
            .chain(SHEET.lines())
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once(' '))
            .map(|(name, d)| (name, flatten(d)))
            .collect()
    })
}

/// A named icon with its square's top left at `at`.
pub fn draw(p: &Painter, at: Pos2, side: f32, name: &str, width: f32, colour: Color32) {
    let Some(lines) = all().get(name) else { return };
    let k = side / 24.0;
    for line in lines {
        let pts: Vec<Pos2> = line
            .iter()
            .map(|q| pos2(at.x + q.0 * k, at.y + q.1 * k))
            .collect();
        p.add(Shape::line(pts, Stroke::new(width, colour)));
    }
}

/// A path's strokes as polylines: M L H V C A Z, relative in lower case.
fn flatten(d: &str) -> Lines {
    let b = d.as_bytes();
    let mut i = 0;
    let mut out: Lines = Vec::new();
    let (mut at, mut start, mut cmd) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32), b'M');
    let skip = |i: &mut usize| {
        while *i < b.len() && (b[*i] == b' ' || b[*i] == b',') {
            *i += 1;
        }
    };
    let number = |i: &mut usize| -> f32 {
        skip(i);
        let s = *i;
        if *i < b.len() && b[*i] == b'-' {
            *i += 1;
        }
        let mut dot = false;
        while *i < b.len() && (b[*i].is_ascii_digit() || (b[*i] == b'.' && !dot)) {
            dot |= b[*i] == b'.';
            *i += 1;
        }
        d[s..*i].parse().unwrap_or(0.0)
    };
    loop {
        skip(&mut i);
        if i >= b.len() {
            return out;
        }
        if b[i].is_ascii_alphabetic() {
            cmd = b[i];
            i += 1;
        }
        let base = if cmd.is_ascii_lowercase() { at } else { (0.0, 0.0) };
        let point = |i: &mut usize| (number(i) + base.0, number(i) + base.1);
        match cmd.to_ascii_uppercase() {
            b'M' => {
                at = point(&mut i);
                start = at;
                out.push(vec![at]);
                cmd = if cmd.is_ascii_lowercase() { b'l' } else { b'L' };
            }
            b'L' => {
                at = point(&mut i);
                out.last_mut().expect("starts with M").push(at);
            }
            b'H' => {
                at.0 = number(&mut i) + base.0;
                out.last_mut().expect("starts with M").push(at);
            }
            b'V' => {
                at.1 = number(&mut i) + base.1;
                out.last_mut().expect("starts with M").push(at);
            }
            b'C' => {
                let (c1, c2, p) = (point(&mut i), point(&mut i), point(&mut i));
                let line = out.last_mut().expect("starts with M");
                for k in 1..=12 {
                    let t = k as f32 / 12.0;
                    let u = 1.0 - t;
                    let m = |a: f32, b: f32, c: f32, d: f32| {
                        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
                    };
                    line.push((m(at.0, c1.0, c2.0, p.0), m(at.1, c1.1, c2.1, p.1)));
                }
                at = p;
            }
            b'A' => {
                let (rx, ry) = (number(&mut i), number(&mut i));
                number(&mut i);
                let (large, sweep) = (number(&mut i) != 0.0, number(&mut i) != 0.0);
                let p = point(&mut i);
                arc(
                    out.last_mut().expect("starts with M"),
                    at,
                    (rx, ry),
                    large,
                    sweep,
                    p,
                );
                at = p;
            }
            b'Z' => {
                out.last_mut().expect("starts with M").push(start);
                at = start;
            }
            _ => i += 1,
        }
    }
}

// SVG's endpoint form, unrotated
fn arc(line: &mut Vec<(f32, f32)>, a: (f32, f32), r: (f32, f32), large: bool, sweep: bool, b: (f32, f32)) {
    let (mx, my) = ((a.0 - b.0) * 0.5 / r.0, (a.1 - b.1) * 0.5 / r.1);
    let d2 = mx * mx + my * my;
    let k = if d2 > 1.0 {
        0.0
    } else {
        let s = ((1.0 - d2) / d2).max(0.0).sqrt();
        if large == sweep { -s } else { s }
    };
    let (cx, cy) = (k * my, -k * mx);
    let centre = (cx * r.0 + (a.0 + b.0) * 0.5, cy * r.1 + (a.1 + b.1) * 0.5);
    let t0 = (my - cy).atan2(mx - cx);
    let mut dt = (-my - cy).atan2(-mx - cx) - t0;
    let tau = core::f32::consts::TAU;
    if sweep && dt < 0.0 {
        dt += tau;
    } else if !sweep && dt > 0.0 {
        dt -= tau;
    }
    for j in 1..=20 {
        let t = t0 + dt * j as f32 / 20.0;
        line.push((centre.0 + r.0 * t.cos(), centre.1 + r.1 * t.sin()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_parses_and_stays_near_its_grid() {
        assert!(all().len() > 50);
        for (name, lines) in all() {
            assert!(!lines.is_empty(), "{name}");
            for p in lines.iter().flatten() {
                assert!(
                    p.0 > -1.0 && p.0 < 26.0 && p.1 > -1.0 && p.1 < 26.0,
                    "{name} {p:?}"
                );
            }
        }
    }

    #[test]
    fn a_circle_is_a_circle() {
        for p in flatten("M4 12A8 8 0 1 0 20 12A8 8 0 1 0 4 12").iter().flatten() {
            assert!((((p.0 - 12.0).powi(2) + (p.1 - 12.0).powi(2)).sqrt() - 8.0).abs() < 1e-3);
        }
    }
}
