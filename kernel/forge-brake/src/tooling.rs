//! Press-brake tooling as data: punches, dies, the press with its beams and backgauge, and a
//! tooling set's rules. Lengths mm, angles radians (files hold degrees; the host converts).
//!
//! Every tool is an extrusion along the bed (y) of a section of lines and arcs in the press's
//! (x, z) plane: x across the bed, +x towards the backgauge, z up. A punch's section has its tip
//! at the origin; a die's has its V centred on x = 0 with its top on z = 0.
use crate::geo::{Region, Seg};
use forge_math::{Point2, dmath};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Style {
    /// Promecam-style tang (European).
    Eu,
    /// American-style tang.
    Us,
}

impl Style {
    pub fn word(self) -> &'static str {
        match self {
            Style::Eu => "eu",
            Style::Us => "us",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PunchKind {
    Straight,
    Gooseneck,
    Acute,
}

impl PunchKind {
    pub fn word(self) -> &'static str {
        match self {
            PunchKind::Straight => "straight",
            PunchKind::Gooseneck => "gooseneck",
            PunchKind::Acute => "acute",
        }
    }
}

/// Segmented lengths: `count` pieces `length` long.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub length: f64,
    pub count: u32,
}

/// A gooseneck's throat, open towards +x: it runs back to `depth` behind the tip line, from the
/// nose's top at `z0` (its floor rising to the back wall) up to `z1` above the tip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relief {
    pub depth: f64,
    pub z0: f64,
    pub z1: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Punch {
    pub id: String,
    pub style: Style,
    pub kind: PunchKind,
    /// Tip to the clamping face.
    pub height: f64,
    pub tip_radius: f64,
    /// Included angle at the tip.
    pub tip_angle: f64,
    /// Height of the straight flanks above the tip: the working part, whose contact with the
    /// bend being made is decided by the angle and radius rules.
    pub tip_height: f64,
    /// Body width at the clamp.
    pub width: f64,
    pub relief: Option<Relief>,
    /// An explicit section (one loop) in place of the generated one; its working part is
    /// what lies below `tip_height`.
    pub profile: Option<Vec<Seg>>,
    pub segments: Vec<Segment>,
    /// kN per metre of bend.
    pub max_load: Option<f64>,
    /// The body's published width, for drawing only, where `width` is an envelope wider than it
    /// (the checks read `width`: it errs large).
    pub drawn_width: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Die {
    pub id: String,
    pub style: Style,
    /// V opening at the top, between the sharp edges.
    pub v: f64,
    /// Included angle of the V.
    pub angle: f64,
    pub shoulder_radius: f64,
    pub height: f64,
    pub width: f64,
    /// The V's depth below the top where a flat floor (a relief channel) cuts it off, when
    /// it stops short of the sharp bottom.
    pub floor: Option<f64>,
    pub profile: Option<Vec<Seg>>,
    pub segments: Vec<Segment>,
    pub max_load: Option<f64>,
}

/// A beam's envelope across the bed: from `front` before the tool line (-x) to `back` behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub front: f64,
    pub back: f64,
}

/// A backgauge finger: a box `depth` deep behind its stop face, `height` tall, `width` along the bed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finger {
    pub depth: f64,
    pub height: f64,
    pub width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Backgauge {
    /// How many fingers.
    pub fingers: u32,
    /// The stop face's range behind the V centre line.
    pub x: (f64, f64),
    /// The finger's bottom above the die top.
    pub r: (f64, f64),
    pub finger: Finger,
    /// How far the fingers back off once the punch holds the part.
    pub retract: f64,
    /// The fingers' range along the bed (Z1/Z2 axes), from the bed's centre; None: not stated.
    pub z: Option<(f64, f64)>,
}

/// The press frame's side housings, one each end of the bed past the housing gap: plates
/// `thickness` thick along the bed, open towards the operator (-x) to `throat` behind the tool
/// line, solid from there `depth` further back, from far below the die seat to far above the
/// ram (the throat's height is the daylight the beams already bound).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub throat: f64,
    pub thickness: f64,
    pub depth: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Press {
    pub id: String,
    /// Daylight at top dead centre: ram clamp face to the bed's die seat.
    pub open_height: f64,
    pub stroke: f64,
    /// Working length of the beams.
    pub bed_length: f64,
    /// Clear width between the side frames.
    pub housing_gap: f64,
    pub ram: Beam,
    pub bed: Beam,
    /// kN.
    pub max_force: Option<f64>,
    pub backgauge: Backgauge,
    /// The side housings, when their throat is known: then a part may run past the housing gap
    /// through the throat, checked by distance against them.
    pub frame: Option<Frame>,
    /// Values a real press's record does not know (`open_height`, `stroke`, ..): the rules that
    /// need them are reported as not checked, never passed.
    pub unchecked: Vec<String>,
    /// Bodies whose envelope is an estimate that errs large (`ram`, `bed`, `frame`,
    /// `backgauge`): a collision with one is reported as against an estimate.
    pub estimated: Vec<String>,
}

/// Overbend at a target angle (the bend's turn) for a material (`*`: any).
#[derive(Clone, Debug, PartialEq)]
pub struct Springback {
    pub material: String,
    pub angle: f64,
    pub overbend: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tooling {
    pub id: String,
    pub punches: Vec<Punch>,
    pub dies: Vec<Die>,
    pub press: Press,
    /// The least distance allowed between the part and any tool.
    pub clearance: f64,
    /// The least gap allowed between the part's own pieces (0: touching is the limit).
    pub self_clearance: f64,
    /// How far each end of a bend may run past the tool covering it.
    pub short_end: Option<f64>,
    pub springback: Vec<Springback>,
    /// The die the planner tries first: V nearest this many thicknesses.
    pub v_rule: f64,
    /// Stations along the bed. None: one station, centred on the bed, tooled for each step by
    /// the segment rule (and only its tools are on the press).
    pub stations: Vec<Station>,
}

/// A station: a punch and a die (indices into the set), their stacks' lengths, and the stack's
/// centre along the bed from the bed's centre. Every station's tools are on the press in every
/// step; a step works at the station holding its punch and die.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub punch: usize,
    pub die: usize,
    pub y: f64,
    pub punch_length: f64,
    pub die_length: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolingError {
    /// A number out of its range: `what` of tool `id`.
    Value {
        id: String,
        what: &'static str,
        value: f64,
        lo: f64,
        hi: f64,
    },
    /// Two tools share an id.
    Duplicate {
        id: String,
    },
    /// An explicit section that is not one closed loop.
    Profile {
        id: String,
    },
    NoPunches,
    NoDies,
    /// A station names a punch or die the set does not hold.
    Station {
        y: f64,
    },
    /// A tool's working geometry is not known (its record holds no published value for these
    /// fields): it cannot be simulated.
    Unknown {
        id: String,
        fields: Vec<String>,
    },
}

fn check(id: &str, what: &'static str, value: f64, lo: f64, hi: f64) -> Result<(), ToolingError> {
    if value.is_finite() && value >= lo && value <= hi {
        Ok(())
    } else {
        Err(ToolingError::Value {
            id: id.into(),
            what,
            value,
            lo,
            hi,
        })
    }
}

fn closed(id: &str, p: &[Seg]) -> Result<(), ToolingError> {
    let n = p.len();
    if n < 2 || (0..n).any(|i| p[i].end().distance(p[(i + 1) % n].start()) > 1e-6) {
        return Err(ToolingError::Profile { id: id.into() });
    }
    Ok(())
}

fn tan(x: f64) -> f64 {
    let (s, c) = dmath::sin_cos(x);
    s / c
}

fn poly(pts: &[(f64, f64)]) -> Vec<Seg> {
    let n = pts.len();
    (0..n).map(|i| Seg::line(pts[i], pts[(i + 1) % n])).collect()
}

impl Punch {
    pub fn validate(&self) -> Result<(), ToolingError> {
        let id = &self.id;
        check(id, "height", self.height, 1.0, 2000.0)?;
        check(id, "tip_radius", self.tip_radius, 0.0, 100.0)?;
        check(id, "tip_angle", self.tip_angle, 0.1, dmath::PI - 0.01)?;
        check(id, "tip_height", self.tip_height, 0.5, self.height)?;
        check(id, "width", self.width, 1.0, 1000.0)?;
        if let Some(r) = self.relief {
            check(id, "relief.depth", r.depth, 0.0, 1000.0)?;
            check(id, "relief.z0", r.z0, self.tip_height, self.height)?;
            check(
                id,
                "relief.z1",
                r.z1,
                r.z0.max(self.flank_z(r.depth) + 2.0) + 1.0,
                self.height,
            )?;
        }
        for s in &self.segments {
            check(id, "segments.length", s.length, 1.0, 10000.0)?;
        }
        if let Some(p) = &self.profile {
            closed(id, p)?;
        }
        Ok(())
    }

    /// Half the width of the flanks at height `z` above the tip: the flanks are tangent to the
    /// tip circle (centre (0, rp)) at half the tip angle from vertical.
    fn flank_x(&self, z: f64) -> f64 {
        let h = self.tip_angle / 2.0;
        let rp = self.tip_radius;
        (z - rp) * tan(h) + rp / dmath::cos(h)
    }

    /// The height at which a flank is `x` out from the tip line.
    pub fn flank_z(&self, x: f64) -> f64 {
        let h = self.tip_angle / 2.0;
        let rp = self.tip_radius;
        rp + (x - rp / dmath::cos(h)) / tan(h)
    }

    /// The working part: the tip arc and both flanks up to `tip_height` (one loop).
    pub fn tip_section(&self) -> Region {
        let h = self.tip_angle / 2.0;
        let rp = self.tip_radius;
        let ht = self.tip_height;
        let xf = self.flank_x(ht);
        let (sh, ch) = dmath::sin_cos(h);
        let rt = Point2::new(rp * ch, rp - rp * sh);
        let lt = Point2::new(-rt.x, rt.y);
        let mut l: Vec<Seg> = Vec::new();
        if rp > 0.0 {
            // counter-clockwise under the tip, from the left tangent point to the right one
            l.push(Seg::Arc {
                c: Point2::new(0.0, rp),
                r: rp,
                start: dmath::PI + h,
                sweep: dmath::PI - 2.0 * h,
            });
        }
        l.push(Seg::Line {
            a: rt,
            b: Point2::new(xf, ht),
        });
        l.push(Seg::Line {
            a: Point2::new(xf, ht),
            b: Point2::new(-xf, ht),
        });
        l.push(Seg::Line {
            a: Point2::new(-xf, ht),
            b: lt,
        });
        Region::of(vec![l])
    }

    /// The rest of the punch above `tip_height`, to the clamping face.
    pub fn body_section(&self) -> Region {
        if let Some(p) = &self.profile {
            // an explicit section: all of it is checked as body above the tip height; the
            // working part below it is decided by the rules
            return Region::of(vec![p.clone()]);
        }
        let ht = self.tip_height;
        let xf = self.flank_x(ht);
        let w = self.width / 2.0;
        let top = self.height;
        // widen from the flanks' tops at 45 degrees to the body
        let hb = (ht + (w - xf).max(0.0)).min(top - 0.5);
        let wb = w.max(xf);
        let pts: Vec<(f64, f64)> = match (self.kind, self.relief) {
            (PunchKind::Gooseneck, Some(r)) => {
                // the nose rises straight from the flanks' top on the throat's side (+x) to z0,
                // then the throat runs back to x = -depth up to z1; on the far side the flank
                // line runs on up to the neck's back, then straight up to the clamp
                let neck = (self.width * 0.75).max(10.0);
                let wl = wb.max(r.depth + neck);
                let zf = self.flank_z(wl).min(top - 0.5);
                // the throat's floor rises from the nose (z0) to its back wall, which starts above
                // where the far flank has passed the throat's depth
                let zb = r.z0.max(self.flank_z(r.depth) + 2.0);
                vec![
                    (-xf, ht),
                    (xf, ht),
                    (xf, r.z0),
                    (-r.depth, zb),
                    (-r.depth, r.z1),
                    (wb, r.z1),
                    (wb, top),
                    (-wl, top),
                    (-wl, zf),
                ]
            }
            _ => vec![(-xf, ht), (xf, ht), (wb, hb), (wb, top), (-wb, top), (-wb, hb)],
        };
        let mut pts = pts;
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        Region::of(vec![poly(&pts)])
    }
}

impl Die {
    pub fn validate(&self) -> Result<(), ToolingError> {
        let id = &self.id;
        check(id, "v", self.v, 1.0, 1000.0)?;
        check(id, "angle", self.angle, 0.1, dmath::PI - 0.01)?;
        check(id, "shoulder_radius", self.shoulder_radius, 0.0, self.v)?;
        check(
            id,
            "width",
            self.width,
            self.v + 2.0 * self.shoulder_radius,
            2000.0,
        )?;
        check(id, "height", self.height, self.depth() + 1.0, 2000.0)?;
        for s in &self.segments {
            check(id, "segments.length", s.length, 1.0, 10000.0)?;
        }
        if let Some(p) = &self.profile {
            closed(id, p)?;
        }
        Ok(())
    }

    /// The V's depth below the top: to its sharp bottom, or to its floor.
    pub fn depth(&self) -> f64 {
        let sharp = self.sharp_depth();
        self.floor.map_or(sharp, |f| f.min(sharp))
    }

    fn sharp_depth(&self) -> f64 {
        (self.v / 2.0) / tan(self.angle / 2.0)
    }

    /// The right shoulder's centre (the left one mirrors it).
    pub fn shoulder(&self) -> Point2 {
        let a = self.angle / 2.0;
        let g = dmath::FRAC_PI_2 + a;
        let rs = self.shoulder_radius;
        Point2::new(self.v / 2.0 + rs / tan(g / 2.0), -rs)
    }

    pub fn section(&self) -> Region {
        if let Some(p) = &self.profile {
            return Region::of(vec![p.clone()]);
        }
        let a = self.angle / 2.0;
        let rs = self.shoulder_radius;
        let s = self.shoulder();
        let (w, h) = (self.width / 2.0, self.height);
        let bottom = Point2::new(0.0, -self.sharp_depth());
        // a floored V: the flanks meet the floor at +-xf
        let floored = self.floor.filter(|f| *f < self.sharp_depth());
        let xf = floored.map(|f| (self.sharp_depth() - f) * tan(a));
        // the flank's tangent point on the right shoulder: the circle's normal towards the V,
        // perpendicular to the flank (direction (-sin a, -cos a)), pointing up-left
        let n_ang = dmath::PI - a; // (-cos a, sin a)
        let (ns, nc) = dmath::sin_cos(n_ang);
        let ft = Point2::new(s.x + rs * nc, s.y + rs * ns);
        let tt = Point2::new(s.x, 0.0);
        let mirror = |p: Point2| Point2::new(-p.x, p.y);
        let mut l: Vec<Seg> = vec![
            Seg::line((-w, -h), (w, -h)),
            Seg::line((w, -h), (w, 0.0)),
            Seg::Line {
                a: Point2::new(w, 0.0),
                b: tt,
            },
        ];
        if rs > 0.0 {
            l.push(Seg::Arc {
                c: s,
                r: rs,
                start: dmath::FRAC_PI_2,
                sweep: n_ang - dmath::FRAC_PI_2,
            });
        }
        match (floored, xf) {
            (Some(f), Some(xf)) => {
                let (r0, l0) = (Point2::new(xf, -f), Point2::new(-xf, -f));
                l.push(Seg::Line {
                    a: if rs > 0.0 { ft } else { tt },
                    b: r0,
                });
                l.push(Seg::Line { a: r0, b: l0 });
                l.push(Seg::Line {
                    a: l0,
                    b: if rs > 0.0 { mirror(ft) } else { mirror(tt) },
                });
            }
            _ => {
                l.push(Seg::Line {
                    a: if rs > 0.0 { ft } else { tt },
                    b: bottom,
                });
                l.push(Seg::Line {
                    a: bottom,
                    b: if rs > 0.0 { mirror(ft) } else { mirror(tt) },
                });
            }
        }
        if rs > 0.0 {
            l.push(Seg::Arc {
                c: mirror(s),
                r: rs,
                start: a,
                sweep: dmath::FRAC_PI_2 - a,
            });
        }
        l.push(Seg::Line {
            a: mirror(tt),
            b: Point2::new(-w, 0.0),
        });
        l.push(Seg::line((-w, 0.0), (-w, -h)));
        Region::of(vec![l])
    }
}

impl Tooling {
    pub fn validate(&self) -> Result<(), ToolingError> {
        if self.punches.is_empty() {
            return Err(ToolingError::NoPunches);
        }
        if self.dies.is_empty() {
            return Err(ToolingError::NoDies);
        }
        let mut ids: Vec<&str> = Vec::new();
        for p in &self.punches {
            p.validate()?;
            ids.push(&p.id);
        }
        for d in &self.dies {
            d.validate()?;
            ids.push(&d.id);
        }
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        for w in sorted.windows(2) {
            if w[0] == w[1] {
                return Err(ToolingError::Duplicate { id: w[0].into() });
            }
        }
        let p = &self.press;
        check(&p.id, "open_height", p.open_height, 1.0, 10000.0)?;
        check(&p.id, "stroke", p.stroke, 0.0, p.open_height)?;
        check(&p.id, "bed_length", p.bed_length, 1.0, 100000.0)?;
        check(&p.id, "housing_gap", p.housing_gap, 1.0, 100000.0)?;
        check(&p.id, "backgauge.retract", p.backgauge.retract, 0.0, 1000.0)?;
        if let Some(f) = p.frame {
            check(&p.id, "frame.throat", f.throat, 1.0, 10000.0)?;
            check(&p.id, "frame.thickness", f.thickness, 1.0, 10000.0)?;
            check(&p.id, "frame.depth", f.depth, 1.0, 100000.0)?;
        }
        check(&self.id, "clearance", self.clearance, 0.01, 100.0)?;
        check(&self.id, "self_clearance", self.self_clearance, 0.0, 100.0)?;
        for st in &self.stations {
            if st.punch >= self.punches.len() || st.die >= self.dies.len() {
                return Err(ToolingError::Station { y: st.y });
            }
            check(
                &self.id,
                "station.punch_length",
                st.punch_length,
                1.0,
                p.bed_length + 1e-6,
            )?;
            check(
                &self.id,
                "station.die_length",
                st.die_length,
                1.0,
                p.bed_length + 1e-6,
            )?;
            let h = 0.5 * st.punch_length.max(st.die_length);
            check(
                &self.id,
                "station.y",
                st.y,
                -0.5 * p.bed_length + h - 1e-6,
                0.5 * p.bed_length - h + 1e-6,
            )?;
        }
        Ok(())
    }

    /// The station a step with this punch and die works at.
    pub fn station(&self, punch: usize, die: usize) -> Option<&Station> {
        self.stations.iter().find(|s| s.punch == punch && s.die == die)
    }

    pub fn punch(&self, id: &str) -> Option<&Punch> {
        self.punches.iter().find(|p| p.id == id)
    }

    pub fn die(&self, id: &str) -> Option<&Die> {
        self.dies.iter().find(|d| d.id == id)
    }

    /// The overbend for a bend of turn `angle` in `material`: linear between the rows stated for
    /// that material (else `*`), held flat past the ends; none stated, none.
    pub fn overbend(&self, material: Option<&str>, angle: f64) -> f64 {
        let norm = |s: &str| s.trim().to_ascii_lowercase();
        let mat = material.map(norm);
        let mut rows: Vec<(f64, f64)> = self
            .springback
            .iter()
            .filter(|s| mat.as_deref() == Some(norm(&s.material).as_str()))
            .map(|s| (s.angle, s.overbend))
            .collect();
        if rows.is_empty() {
            rows = self
                .springback
                .iter()
                .filter(|s| s.material.trim() == "*")
                .map(|s| (s.angle, s.overbend))
                .collect();
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        match rows.as_slice() {
            [] => 0.0,
            [(_, o)] => *o,
            _ => {
                if angle <= rows[0].0 {
                    return rows[0].1;
                }
                for w in rows.windows(2) {
                    if angle <= w[1].0 {
                        let f = (angle - w[0].0) / (w[1].0 - w[0].0);
                        return w[0].1 + f * (w[1].1 - w[0].1);
                    }
                }
                rows[rows.len() - 1].1
            }
        }
    }
}

/// The shortest total of the pieces (each used at most its count) that is at least `need`; None
/// if all of them together fall short. Lengths are counted in tenths of a millimetre.
pub fn cover(segments: &[Segment], need: f64) -> Option<(f64, Vec<(f64, u32)>)> {
    cover_within(segments, need, need)
}

/// The longest total of the pieces in [`need`, `room`], else the shortest at least `need`.
pub fn cover_within(segments: &[Segment], need: f64, room: f64) -> Option<(f64, Vec<(f64, u32)>)> {
    let unit = 0.1;
    let need_u = (need.max(0.0) / unit - 1e-9).ceil() as usize;
    let room_u = ((room.max(need) / unit) + 1e-9).floor() as usize;
    // every piece on its own, longest first so fewer pieces are preferred
    let mut items: Vec<(usize, f64)> = segments
        .iter()
        .flat_map(|s| {
            std::iter::repeat_n(
                (((s.length / unit).round() as usize).max(1), s.length),
                s.count as usize,
            )
        })
        .collect();
    items.sort_by_key(|i| std::cmp::Reverse(i.0));
    let total: usize = items.iter().map(|i| i.0).sum();
    if items.is_empty() || total < need_u {
        return None;
    }
    let longest = items[0].0;
    let cap = (room_u.max(need_u) + longest).min(total);
    // from[s]: the item that first reached sum s
    let mut from: Vec<u16> = vec![0; cap + 1];
    let mut reached = vec![false; cap + 1];
    reached[0] = true;
    for (k, &(l, _)) in items.iter().enumerate() {
        for s in (0..=cap.saturating_sub(l)).rev() {
            if reached[s] && !reached[s + l] {
                reached[s + l] = true;
                from[s + l] = (k + 1) as u16;
            }
        }
    }
    let best = (need_u..=room_u.min(cap))
        .rev()
        .find(|&s| reached[s])
        .or_else(|| (need_u..=cap).find(|&s| reached[s]))?;
    // walk back: each sum's item was added to a sum reached before it (by an earlier item)
    let mut used: Vec<(f64, u32)> = Vec::new();
    let mut s = best;
    while s > 0 {
        let k = from[s] as usize - 1;
        let len = items[k].1;
        match used.iter_mut().find(|u| u.0 == len) {
            Some(u) => u.1 += 1,
            None => used.push((len, 1)),
        }
        s -= items[k].0;
    }
    Some((best as f64 * unit, used))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    #[test]
    fn library_validates() {
        for t in library::sets() {
            t.validate().unwrap_or_else(|e| panic!("{}: {e:?}", t.id));
        }
    }

    #[test]
    fn sections_are_closed_and_tip_at_origin() {
        for t in library::sets() {
            for p in &t.punches {
                for r in [p.tip_section(), p.body_section()] {
                    let l = &r.loops[0];
                    for i in 0..l.len() {
                        let gap = l[i].end().distance(l[(i + 1) % l.len()].start());
                        assert!(gap < 1e-9, "{} gap {gap}", p.id);
                    }
                }
                let tip = p.tip_section();
                let (lo, _) = tip.bbox();
                assert!(lo.y.abs() < 1e-9, "{} tip at {}", p.id, lo.y);
                assert!(tip.contains(Point2::new(0.0, p.tip_height / 2.0)), "{}", p.id);
                assert!(
                    p.body_section().contains(Point2::new(0.0, p.height - 1.0)),
                    "{}",
                    p.id
                );
            }
            for d in &t.dies {
                let s = d.section();
                let l = &s.loops[0];
                for i in 0..l.len() {
                    assert!(
                        l[i].end().distance(l[(i + 1) % l.len()].start()) < 1e-9,
                        "{}",
                        d.id
                    );
                }
                assert!(s.contains(Point2::new(d.width / 2.0 - 0.5, -1.0)));
                assert!(!s.contains(Point2::new(0.0, -0.5)));
                // the shoulder is tangent to the top
                assert!((s.sd(Point2::new(d.shoulder().x, 0.5)) - 0.5).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn segments_cover() {
        let s = [
            Segment {
                length: 100.0,
                count: 2,
            },
            Segment {
                length: 15.0,
                count: 1,
            },
            Segment {
                length: 10.0,
                count: 2,
            },
        ];
        assert_eq!(cover(&s, 120.0).unwrap().0, 120.0);
        assert_eq!(cover(&s, 121.0).unwrap().0, 125.0);
        assert_eq!(cover(&s, 200.0).unwrap().0, 200.0);
        assert_eq!(cover(&s, 236.0), None);
        assert_eq!(cover(&s, 235.0).unwrap().0, 235.0);
        // the longest within the room, else the shortest past the need
        assert_eq!(cover_within(&s, 100.0, 124.0).unwrap().0, 120.0);
        assert_eq!(cover_within(&s, 101.0, 104.0).unwrap().0, 110.0);
    }
}
