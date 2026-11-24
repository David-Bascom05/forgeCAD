//! The turning view: a lathe setup in section, in (ρ, z) — z along the spindle (the chuck to the
//! left, the bar's front to the right), ρ up from the axis. The part's exact half-profile, the
//! bar, the chuck's envelope as it turns, each op's path (the selected op strong, the others
//! faint; feeds, rapids and thread passes apart), the refusals' witnesses, and the tool's insert
//! and holder at a step of the selected op's path. Mirrored about the axis for the full section
//! when asked. All of it read from forge-turn-lang's run (`TurnCam::parts/setups/paths`); the
//! material removal is CAM-SIM's, later, through the same interface as milling.

use super::CamWs;
use crate::shell::theme::{Theme, mono, sans};
use egui::{Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};
use forge_cam_geom::region::{Region, Seg};
use forge_turn::ops::Step;

/// The view's own state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurnView {
    pub open: bool,
    /// Both halves (the full section).
    pub mirror: bool,
    /// The step of the selected op's path the tool is drawn at.
    pub step: usize,
    /// The op that step is of (a new op starts at its first step).
    pub step_of: Option<String>,
}

/// Points along an arc from `a` to `b` about `c` (counter-clockwise when `ccw`), after `a`.
fn arc(a: [f64; 2], b: [f64; 2], c: [f64; 2], ccw: bool) -> Vec<[f64; 2]> {
    let (dx, dy) = (a[0] - c[0], a[1] - c[1]);
    let r = (dx * dx + dy * dy).sqrt();
    let t0 = forge_math::dmath::atan2(dy, dx);
    let mut t1 = forge_math::dmath::atan2(b[1] - c[1], b[0] - c[0]);
    let tau = std::f64::consts::TAU;
    let full = (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12;
    if ccw {
        while t1 <= t0 {
            t1 += tau;
        }
        if full {
            t1 = t0 + tau;
        }
    } else {
        while t1 >= t0 {
            t1 -= tau;
        }
        if full {
            t1 = t0 - tau;
        }
    }
    let n = ((t1 - t0).abs() * 16.0).ceil().clamp(2.0, 128.0) as usize;
    (1..=n)
        .map(|k| {
            let t = t0 + (t1 - t0) * k as f64 / n as f64;
            let (s, c2) = forge_math::dmath::sin_cos(t);
            [c[0] + r * c2, c[1] + r * s]
        })
        .collect()
}

/// A region's loops as polylines in (ρ, z).
fn loops(rg: &Region) -> Vec<Vec<[f64; 2]>> {
    rg.loops
        .iter()
        .map(|l| {
            let mut pts: Vec<[f64; 2]> = vec![];
            for s in &l.segs {
                match s {
                    Seg::Line { a, b } => {
                        if pts.is_empty() {
                            pts.push([a.x, a.y]);
                        }
                        pts.push([b.x, b.y]);
                    }
                    Seg::Arc { a, b, c, ccw } => {
                        if pts.is_empty() {
                            pts.push([a.x, a.y]);
                        }
                        pts.extend(arc([a.x, a.y], [b.x, b.y], [c.x, c.y], *ccw));
                    }
                }
            }
            pts
        })
        .collect()
}

/// Where (ρ, z) is on screen.
struct Frame {
    r: Rect,
    z0: f64,
    rho0: f64,
    k: f64,
    /// ρ = 0 on screen.
    axis_y: f32,
}

impl Frame {
    fn at(&self, p: [f64; 2], flip: bool) -> Pos2 {
        let rho = if flip { -p[0] } else { p[0] };
        pos2(
            self.r.left() + ((p[1] - self.z0) * self.k) as f32,
            self.axis_y - ((rho - self.rho0) * self.k) as f32 + (self.rho0 * self.k) as f32,
        )
    }
}

/// A region filled (scanlines, even–odd) and outlined.
fn fill(p: &egui::Painter, f: &Frame, rg: &Region, flip: bool, fill: Color32, line: Stroke) {
    let ls: Vec<Vec<Pos2>> = loops(rg)
        .iter()
        .map(|l| l.iter().map(|q| f.at(*q, flip)).collect())
        .collect();
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for l in &ls {
        for q in l {
            lo = lo.min(q.y);
            hi = hi.max(q.y);
        }
    }
    let (lo, hi) = (lo.max(f.r.top()), hi.min(f.r.bottom()));
    let mut y = lo.floor() + 0.5;
    while y < hi {
        let mut xs: Vec<f32> = vec![];
        for l in &ls {
            for w in l.windows(2) {
                let (a, b) = (w[0], w[1]);
                if (a.y <= y) != (b.y <= y) {
                    xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks(2) {
            if let [x0, x1] = pair {
                p.line_segment([pos2(*x0, y), pos2(*x1, y)], Stroke::new(1.0, fill));
            }
        }
        y += 1.0;
    }
    for l in ls {
        p.add(Shape::line(l, line));
    }
}

/// The view's body: its switches, the section, the step scrubber.
pub fn body(ui: &mut Ui, t: &Theme, w: &mut CamWs, width: f32, places: &mut Vec<(String, Rect)>) {
    let Some(tc) = w.run.as_ref().and_then(|r| r.turn.as_ref()) else {
        ui.label(egui::RichText::new("Generate to see the section.").color(t.ink2));
        return;
    };
    let sel = w.focus.op.clone();
    let setup_name = w.focus.setup.clone();
    let setups = tc.setups();
    let Some(si) = setups
        .iter()
        .position(|(n, _)| Some(n) == setup_name.as_ref())
        .or(if setups.is_empty() { None } else { Some(0) })
    else {
        ui.label(egui::RichText::new("No lathe setup was made: see the errors.").color(t.error));
        return;
    };
    let setup = &setups[si].1;
    let paths: Vec<&(String, usize, forge_turn::ops::OpPath)> =
        tc.paths().iter().filter(|p| p.1 == si).collect();
    // the selected op's path, and the step the tool is drawn at
    let selp = paths.iter().find(|p| Some(&p.0) == sel.as_ref());
    if w.turnview.step_of != sel {
        w.turnview.step_of = sel.clone();
        w.turnview.step = 0;
    }
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let r = ui.checkbox(&mut w.turnview.mirror, "Full section");
        places.push(("cam:turn:mirror".into(), r.rect));
        if let Some(p) = selp {
            let n = p.2.steps.len();
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("{} · step", p.0))
                    .font(sans(12.0))
                    .color(t.ink2),
            );
            let r = ui.add(egui::Slider::new(&mut w.turnview.step, 0..=n.saturating_sub(1)).show_value(true));
            places.push(("cam:turn:step".into(), r.rect));
        }
    });
    // half as tall as wide, within reason: a small view keeps room for a window beside it
    let h = (width * 0.5).clamp(160.0, 300.0);
    let (r, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    places.push(("cam:turn:view".into(), r));
    let p = ui.painter_at(r);
    p.rect_filled(r, 4.0, t.field);
    // what is drawn: the bar, the chuck's envelope, the part, every path
    let chuck = setup.chuck_region();
    let bar = setup.bar();
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut grow = |q: [f64; 2]| {
        b = [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])];
    };
    // framed on the bar, the part and the paths; the chuck's body runs off the left edge
    for l in loops(&bar) {
        l.into_iter().for_each(&mut grow);
    }
    if let Some(part) = tc.parts().first() {
        grow([part.bbox[0], part.bbox[1]]);
        grow([part.bbox[2], part.bbox[3]]);
    }
    for pp in &paths {
        grow(pp.2.start);
        for s in &pp.2.steps {
            grow(s.to());
        }
    }
    // and the jaws' face, a little of the chuck
    let jaws = [b[2], setup.chuck.z_face - 0.15 * (b[3] - b[1]).max(1.0)];
    b = [
        b[0].min(jaws[0]),
        b[1].min(jaws[1]),
        b[2].max(jaws[0]),
        b[3].max(jaws[1]),
    ];
    let mirror = w.turnview.mirror;
    let (rho_lo, rho_hi) = (if mirror { -b[2] } else { 0.0 }, b[2]);
    let pad = 14.0;
    let inner = r.shrink(pad);
    let k = (inner.width() as f64 / (b[3] - b[1]).max(1e-9))
        .min(inner.height() as f64 / (rho_hi - rho_lo).max(1e-9));
    let axis_y = if mirror {
        inner.center().y
    } else {
        inner.bottom() - 6.0
    };
    let f = Frame {
        r: inner,
        z0: b[1],
        rho0: 0.0,
        k,
        axis_y,
    };
    let halves: &[bool] = if mirror { &[false, true] } else { &[false] };
    let grey = Color32::from_rgba_unmultiplied(140, 146, 156, 70);
    let bar_c = Color32::from_rgba_unmultiplied(170, 180, 196, 40);
    let part_c = Color32::from_rgba_unmultiplied(120, 150, 200, 110);
    for &flip in halves {
        fill(&p, &f, &chuck, flip, grey, Stroke::new(1.0, t.ink2));
        fill(
            &p,
            &f,
            &bar,
            flip,
            bar_c,
            Stroke::new(1.0, t.ink2.gamma_multiply(0.7)),
        );
        for (_, fx) in &setup.fixtures {
            fill(&p, &f, fx, flip, grey, Stroke::new(1.0, t.ink2));
        }
        if let Some(part) = tc.parts().first() {
            fill(&p, &f, &part.region, flip, part_c, Stroke::new(1.5, t.ink));
        }
    }
    // the axis
    p.line_segment(
        [pos2(inner.left(), axis_y), pos2(inner.right(), axis_y)],
        Stroke::new(1.0, t.ink2.gamma_multiply(0.6)),
    );
    p.text(
        pos2(inner.right(), axis_y - 2.0),
        Align2::RIGHT_BOTTOM,
        "z",
        mono(11.0),
        t.ink2,
    );
    // Z0
    let z0 = f.at([0.0, 0.0], false).x;
    if inner.x_range().contains(z0) {
        p.line_segment(
            [pos2(z0, inner.top()), pos2(z0, inner.bottom())],
            Stroke::new(1.0, t.accent.gamma_multiply(0.4)),
        );
        p.text(
            pos2(z0 + 3.0, inner.top()),
            Align2::LEFT_TOP,
            "Z0",
            mono(11.0),
            t.accent,
        );
    }
    // the paths
    let feed = Color32::from_rgb(64, 140, 255);
    let rapid = Color32::from_rgb(250, 200, 64);
    let thread = Color32::from_rgb(200, 110, 230);
    for pp in &paths {
        let strong = Some(&pp.0) == sel.as_ref() || sel.is_none();
        let a = if strong { 1.0 } else { 0.25 };
        let mut at = pp.2.start;
        for s in &pp.2.steps {
            let (pts, col, wdt) = match *s {
                Step::Rapid(q) => (vec![at, q], rapid, 1.0),
                Step::Feed(q) => (vec![at, q], feed, 1.6),
                Step::Thread { to, .. } => (vec![at, to], thread, 1.6),
                Step::Arc { to, c, ccw } => {
                    let mut v = vec![at];
                    v.extend(arc(at, to, c, ccw));
                    (v, feed, 1.6)
                }
            };
            for &flip in halves {
                let line: Vec<Pos2> = pts.iter().map(|q| f.at(*q, flip)).collect();
                p.add(Shape::line(line, Stroke::new(wdt, col.gamma_multiply(a))));
            }
            at = s.to();
        }
    }
    // the tool at the step: each body's convex core about the nose centre, grown by its r
    if let Some(pp) = selp {
        let k_step = w.turnview.step.min(pp.2.steps.len().saturating_sub(1));
        let c = if pp.2.steps.is_empty() {
            pp.2.start
        } else {
            pp.2.steps[k_step].to()
        };
        for body in &pp.2.tool.bodies {
            if body.core.is_empty() {
                continue;
            }
            let poly: Vec<Pos2> = body
                .core
                .iter()
                .map(|q| f.at([c[0] + q[0], c[1] + q[1]], false))
                .collect();
            let col = if body.cutter {
                Color32::from_rgba_unmultiplied(230, 120, 60, 170)
            } else {
                Color32::from_rgba_unmultiplied(150, 156, 168, 120)
            };
            let wdt = ((2.0 * body.r * k) as f32).max(1.0);
            p.add(Shape::convex_polygon(poly.clone(), col, Stroke::new(wdt, col)));
        }
        p.circle_filled(f.at(c, false), 3.0, Color32::from_rgb(230, 120, 60));
    }
    // the refusals' witnesses, and a located one
    if let Some(rep) = &tc.report {
        for rf in &rep.refusals {
            p.circle_filled(f.at(rf.at, false), 4.5, t.error);
        }
    }
    if let Some(l) = &w.focus.locate {
        let q = f.at([l.at[0], l.at[2]], false);
        p.circle_stroke(q, 8.0, Stroke::new(2.0, t.error));
    }
    // the scale
    p.text(
        pos2(inner.left(), inner.top()),
        Align2::LEFT_TOP,
        format!(
            "bar D {} · front z {}",
            super::ui::num(2.0 * setup.bar_r),
            super::ui::num(setup.bar_front)
        ),
        mono(11.0),
        t.ink2,
    );
}
