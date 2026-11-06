//! Entities' own fields (chapter 20.4). Points are as stored: WCS, or OCS for planar entities.

use super::Spec;
use crate::bits::R;
use crate::io::{Href, Io};
use crate::str::Str;
use crate::version::Version;

const UP: [f64; 3] = [0.0, 0.0, 1.0];

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub elevation: f64,
    pub insert: [f64; 2],
    /// The alignment point, when stored (not for left-aligned text).
    pub align_pt: Option<[f64; 2]>,
    pub extrusion: [f64; 3],
    pub thickness: f64,
    pub oblique: f64,
    pub rotation: f64,
    pub height: f64,
    pub width: f64,
    pub text: Str,
    pub generation: i16,
    pub halign: i16,
    pub valign: i16,
    pub style: Href,
}

impl Default for Text {
    fn default() -> Text {
        Text {
            elevation: 0.0,
            insert: [0.0; 2],
            align_pt: None,
            extrusion: UP,
            thickness: 0.0,
            oblique: 0.0,
            rotation: 0.0,
            height: 2.5,
            width: 1.0,
            text: Str::default(),
            generation: 0,
            halign: 0,
            valign: 0,
            style: Href::NULL_HARD,
        }
    }
}

/// TEXT's data, also the start of ATTRIB's and ATTDEF's.
pub fn text_io<S: Io>(t: &mut Text, s: &mut S) -> R<()> {
    let mut flags = 0u8;
    if !S::READ {
        let bits = [
            t.elevation == 0.0,
            t.align_pt.is_none(),
            t.oblique == 0.0,
            t.rotation == 0.0,
            t.width == 1.0,
            t.generation == 0,
            t.halign == 0,
            t.valign == 0,
        ];
        flags = bits.iter().enumerate().map(|(k, &b)| (b as u8) << k).sum();
    }
    s.rc(&mut flags)?;
    if flags & 0x01 == 0 {
        s.rd(&mut t.elevation)?;
    } else if S::READ {
        t.elevation = 0.0;
    }
    s.rd2(&mut t.insert)?;
    if flags & 0x02 == 0 {
        let mut a = t.align_pt.unwrap_or(t.insert);
        s.dd2(&mut a, t.insert)?;
        t.align_pt = Some(a);
    } else if S::READ {
        t.align_pt = None;
    }
    s.be(&mut t.extrusion)?;
    s.bt(&mut t.thickness)?;
    opt_rd(s, flags & 0x04 == 0, &mut t.oblique, 0.0)?;
    opt_rd(s, flags & 0x08 == 0, &mut t.rotation, 0.0)?;
    s.rd(&mut t.height)?;
    opt_rd(s, flags & 0x10 == 0, &mut t.width, 1.0)?;
    s.tv(&mut t.text)?;
    opt_bs(s, flags & 0x20 == 0, &mut t.generation)?;
    opt_bs(s, flags & 0x40 == 0, &mut t.halign)?;
    opt_bs(s, flags & 0x80 == 0, &mut t.valign)?;
    Ok(())
}

fn opt_rd<S: Io>(s: &mut S, present: bool, v: &mut f64, default: f64) -> R<()> {
    if present {
        s.rd(v)
    } else {
        if S::READ {
            *v = default;
        }
        Ok(())
    }
}

fn opt_bs<S: Io>(s: &mut S, present: bool, v: &mut i16) -> R<()> {
    if present {
        s.bs(v)
    } else {
        if S::READ {
            *v = 0;
        }
        Ok(())
    }
}

impl Spec for Text {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        text_io(self, s)?;
        s.h(&mut self.style)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Block {
    pub name: Str,
}

impl Spec for Block {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.tv(&mut self.name)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EndBlk;

impl Spec for EndBlk {
    fn io<S: Io>(&mut self, _: &mut S) -> R<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub thickness: f64,
    pub extrusion: [f64; 3],
}

impl Default for Line {
    fn default() -> Line {
        Line {
            start: [0.0; 3],
            end: [0.0; 3],
            thickness: 0.0,
            extrusion: UP,
        }
    }
}

impl Spec for Line {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut flat = self.start[2] == 0.0 && self.end[2] == 0.0;
        s.b(&mut flat)?;
        s.rd(&mut self.start[0])?;
        s.dd(&mut self.end[0], self.start[0])?;
        s.rd(&mut self.start[1])?;
        s.dd(&mut self.end[1], self.start[1])?;
        if !flat {
            s.rd(&mut self.start[2])?;
            s.dd(&mut self.end[2], self.start[2])?;
        } else if S::READ {
            self.start[2] = 0.0;
            self.end[2] = 0.0;
        }
        s.bt(&mut self.thickness)?;
        s.be(&mut self.extrusion)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: [f64; 3],
    pub radius: f64,
    pub thickness: f64,
    pub extrusion: [f64; 3],
}

impl Default for Circle {
    fn default() -> Circle {
        Circle {
            center: [0.0; 3],
            radius: 1.0,
            thickness: 0.0,
            extrusion: UP,
        }
    }
}

impl Spec for Circle {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.center)?;
        s.bd(&mut self.radius)?;
        s.bt(&mut self.thickness)?;
        s.be(&mut self.extrusion)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Arc {
    pub circle: Circle,
    /// Radians, counter-clockwise from start to end.
    pub start: f64,
    pub end: f64,
}

impl Spec for Arc {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.circle.io(s)?;
        s.bd(&mut self.start)?;
        s.bd(&mut self.end)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub at: [f64; 3],
    pub thickness: f64,
    pub extrusion: [f64; 3],
    pub x_angle: f64,
}

impl Default for Point {
    fn default() -> Point {
        Point {
            at: [0.0; 3],
            thickness: 0.0,
            extrusion: UP,
            x_angle: 0.0,
        }
    }
}

impl Spec for Point {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.at)?;
        s.bt(&mut self.thickness)?;
        s.be(&mut self.extrusion)?;
        s.bd(&mut self.x_angle)
    }
}

/// SOLID; TRACE has the same fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Solid {
    pub thickness: f64,
    pub elevation: f64,
    pub corners: [[f64; 2]; 4],
    pub extrusion: [f64; 3],
}

impl Default for Solid {
    fn default() -> Solid {
        Solid {
            thickness: 0.0,
            elevation: 0.0,
            corners: [[0.0; 2]; 4],
            extrusion: UP,
        }
    }
}

impl Spec for Solid {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bt(&mut self.thickness)?;
        s.bd(&mut self.elevation)?;
        for c in &mut self.corners {
            s.rd2(c)?;
        }
        s.be(&mut self.extrusion)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ellipse {
    pub center: [f64; 3],
    pub major: [f64; 3],
    pub extrusion: [f64; 3],
    pub ratio: f64,
    pub start: f64,
    pub end: f64,
}

impl Default for Ellipse {
    fn default() -> Ellipse {
        Ellipse {
            center: [0.0; 3],
            major: [1.0, 0.0, 0.0],
            extrusion: UP,
            ratio: 1.0,
            start: 0.0,
            end: std::f64::consts::TAU,
        }
    }
}

impl Spec for Ellipse {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.center)?;
        s.bd3(&mut self.major)?;
        s.bd3(&mut self.extrusion)?;
        s.bd(&mut self.ratio)?;
        s.bd(&mut self.start)?;
        s.bd(&mut self.end)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Spline {
    /// 1: control points and knots; 2: fit points.
    pub scenario: i32,
    /// R2013+.
    pub flags1: i32,
    pub knot_param: i32,
    pub degree: i32,
    pub fit_tol: f64,
    pub begin_tangent: [f64; 3],
    pub end_tangent: [f64; 3],
    pub rational: bool,
    pub closed: bool,
    pub periodic: bool,
    pub knot_tol: f64,
    pub ctrl_tol: f64,
    pub weighted: bool,
    pub knots: Vec<f64>,
    pub points: Vec<[f64; 3]>,
    pub weights: Vec<f64>,
    pub fit: Vec<[f64; 3]>,
}

impl Spec for Spline {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.scenario)?;
        if s.ver() >= Version::R2013 {
            s.bl(&mut self.flags1)?;
            s.bl(&mut self.knot_param)?;
        }
        s.bl(&mut self.degree)?;
        let mut nfit = self.fit.len() as i32;
        let (mut nk, mut nc) = (self.knots.len() as i32, self.points.len() as i32);
        // R2013+: the stored scenario is overruled by the flags: fit points by method 1, control
        // points by a custom (15) knot parameterization
        let mut scenario = self.scenario;
        if s.ver() >= Version::R2013 {
            if self.flags1 & 1 != 0 {
                scenario = 2;
            }
            if self.knot_param == 15 {
                scenario = 1;
            }
        }
        let fit = scenario & 2 != 0;
        let ctrl = scenario & 1 != 0;
        if fit {
            s.bd(&mut self.fit_tol)?;
            s.bd3(&mut self.begin_tangent)?;
            s.bd3(&mut self.end_tangent)?;
            s.bl(&mut nfit)?;
        }
        if ctrl {
            s.b(&mut self.rational)?;
            s.b(&mut self.closed)?;
            s.b(&mut self.periodic)?;
            s.bd(&mut self.knot_tol)?;
            s.bd(&mut self.ctrl_tol)?;
            s.bl(&mut nk)?;
            s.bl(&mut nc)?;
            s.b(&mut self.weighted)?;
        } else {
            nk = 0;
            nc = 0;
        }
        s.items(&mut self.knots, nk, &mut |s, k| s.bd(k))?;
        if S::READ {
            self.weights.clear();
        }
        let weighted = self.weighted;
        let mut ws = std::mem::take(&mut self.weights);
        let mut i = 0;
        s.items(&mut self.points, nc, &mut |s, p| {
            s.bd3(p)?;
            if weighted {
                if S::READ {
                    ws.push(0.0);
                }
                let w = &mut ws[i];
                s.bd(w)?;
            }
            i += 1;
            Ok(())
        })?;
        self.weights = ws;
        s.items(&mut self.fit, nfit, &mut |s, p| s.bd3(p))
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LwPolyline {
    /// 4 const width, 8 elevation, 2 thickness, 1 normal, 16 bulges, 32 widths, 256 plinegen,
    /// 512 closed, 1024 vertex ids (R2010+).
    pub flag: i16,
    pub const_width: f64,
    pub elevation: f64,
    pub thickness: f64,
    pub normal: [f64; 3],
    pub points: Vec<[f64; 2]>,
    pub bulges: Vec<f64>,
    pub ids: Vec<i32>,
    pub widths: Vec<[f64; 2]>,
}

impl Spec for LwPolyline {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        if !S::READ {
            self.flag &= !(4 | 8 | 2 | 1 | 16 | 32 | 1024);
            self.flag |= (self.const_width != 0.0) as i16 * 4
                | (self.elevation != 0.0) as i16 * 8
                | (self.thickness != 0.0) as i16 * 2
                | (self.normal != UP && self.normal != [0.0; 3]) as i16
                | (!self.bulges.is_empty()) as i16 * 16
                | (!self.widths.is_empty()) as i16 * 32
                | (!self.ids.is_empty() && s.ver() >= Version::R2010) as i16 * 1024;
        }
        s.bs(&mut self.flag)?;
        let f = self.flag;
        if S::READ {
            self.normal = UP;
        }
        if f & 4 != 0 {
            s.bd(&mut self.const_width)?;
        }
        if f & 8 != 0 {
            s.bd(&mut self.elevation)?;
        }
        if f & 2 != 0 {
            s.bd(&mut self.thickness)?;
        }
        if f & 1 != 0 {
            s.bd3(&mut self.normal)?;
        }
        let mut n = self.points.len() as i32;
        s.bl(&mut n)?;
        let mut nb = self.bulges.len() as i32;
        if f & 16 != 0 {
            s.bl(&mut nb)?;
        } else {
            nb = 0;
        }
        let mut ni = self.ids.len() as i32;
        if f & 1024 != 0 && s.ver() >= Version::R2010 {
            s.bl(&mut ni)?;
        } else {
            ni = 0;
        }
        let mut nw = self.widths.len() as i32;
        if f & 32 != 0 {
            s.bl(&mut nw)?;
        } else {
            nw = 0;
        }
        let mut prev = [0.0; 2];
        let mut first = true;
        s.items(&mut self.points, n, &mut |s, p| {
            if first {
                s.rd2(p)?;
                first = false;
            } else {
                s.dd2(p, prev)?;
            }
            prev = *p;
            Ok(())
        })?;
        s.items(&mut self.bulges, nb, &mut |s, b| s.bd(b))?;
        s.items(&mut self.ids, ni, &mut |s, b| s.bl(b))?;
        s.items(&mut self.widths, nw, &mut |s, w| s.bd2(w))
    }
}
