//! The Bend Simulation panel's pictures and names (owned by BEND-SIM): tools by their makers'
//! names, a tool library of cards each with its drawn cross-section, and each step's part drawn
//! as it stands at the tools. The pictures are the scene's own bodies (forge_brake::scene),
//! seen end on: nothing here is drawn from anything but the records and the simulation.

use super::theme::Theme;
use forge_brake as fb;
use forge_brake::catalog::{DieRec, PressRec, PunchRec};
use forge_brake::scene::{Mesh, drawn_body, extrude};
use forge_lang::bendsim::Sim;

/// A punch's name as its maker gives it, with what it is: "Trumpf OW200/K gooseneck punch,
/// 88° R0.8".
pub fn punch_title(r: &PunchRec) -> String {
    let kind = match r.kind {
        fb::tooling::PunchKind::Gooseneck => "gooseneck punch",
        fb::tooling::PunchKind::Acute => "acute punch",
        fb::tooling::PunchKind::Straight => "straight punch",
    };
    let head = format!("{} {} {kind}", r.prov.maker, r.prov.name);
    match &r.punch {
        Ok(p) => format!("{head}, {:.0}° R{}", p.tip_angle.to_degrees(), num(p.tip_radius)),
        Err(_) => head,
    }
}

/// A die's name as its maker gives it, with its V: "Trumpf OW200/K 86° die V16".
pub fn die_title(r: &DieRec) -> String {
    match &r.die {
        Ok(d) => format!(
            "{} {} {:.0}° die V{}",
            r.prov.maker,
            r.prov.name,
            d.angle.to_degrees(),
            num(d.v)
        ),
        Err(_) => format!("{} {} die", r.prov.maker, r.prov.name),
    }
}

/// A press's name: "Trumpf TruBend 5130".
pub fn press_title(r: &PressRec) -> String {
    format!("{} {}", r.prov.maker, r.model)
}

/// A tool's title by its id, from the library (the id itself when the library has none).
pub fn tool_title(cat: Option<&fb::catalog::Catalog>, id: &str, punch: bool) -> String {
    let found = cat.and_then(|c| {
        if punch {
            c.punches.iter().find(|r| r.id == id).map(punch_title)
        } else {
            c.dies.iter().find(|r| r.id == id).map(die_title)
        }
    });
    found.unwrap_or_else(|| id.to_string())
}

/// A scene body's name in words: "punch:trumpf/OW200-S/body" is "Trumpf OW200/K gooseneck
/// punch, 86° R1 (body)"; "station2:die:..." says its station; the press's own parts as named.
pub fn body_words(cat: Option<&fb::catalog::Catalog>, name: &str) -> String {
    let (station, rest) = match name.split_once(':') {
        Some((s, r)) if s.starts_with("station") => (Some(s.trim_start_matches("station")), r),
        _ => (None, name),
    };
    let (kind, id) = match rest.split_once(':') {
        Some((k @ ("punch" | "die"), id)) => (k, id),
        _ => return name.replace(['_', ':'], " "),
    };
    let (id, part) = match id.rsplit_once('/') {
        Some((i, p @ ("tip" | "body"))) => (i, Some(p)),
        _ => (id, None),
    };
    let mut out = tool_title(cat, id, kind == "punch");
    if let Some(p) = part {
        out.push_str(&format!(" ({p})"));
    }
    if let Some(k) = station {
        out.push_str(&format!(" at station {k}"));
    }
    out
}

/// A check's name as an engineer says it (its key on hover).
pub fn rule_label(r: fb::Rule) -> &'static str {
    use fb::Rule::*;
    match r {
        ToolCollision => "Tool collision",
        ClosesOnPunch => "Closes on punch",
        SelfCollision => "Part on itself",
        MinFlange => "Minimum flange",
        VTooNarrow => "V too narrow",
        VDepth => "V depth",
        DieAngle => "Die angle",
        PunchAngle => "Punch angle",
        PunchRadius => "Punch radius",
        BackgaugeReach => "Backgauge reach",
        OpenHeight => "Open height",
        Stroke => "Stroke",
        BedLength => "Bed length",
        Segments => "Stack segments",
        Tonnage => "Tonnage",
        Withdraw => "Punch withdrawal",
        Handling => "Operator handling",
    }
}

/// A check's value with its unit: angles in degrees, force in kN, the rest in mm.
pub fn rule_value(r: fb::Rule, x: f64) -> String {
    match r {
        fb::Rule::DieAngle | fb::Rule::PunchAngle => format!("{:.1}°", x.to_degrees()),
        fb::Rule::Tonnage => format!("{x:.1} kN"),
        // a clearance at zero is zero, not "-0.00"
        _ => format!("{:.2} mm", if x.abs() < 0.005 { 0.0 } else { x }),
    }
}

fn num(x: f64) -> String {
    let s = format!("{x:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// Meshes seen end on (along the bed): their (x, z) filled into `rect`, upright, one scale for
/// all, with a margin.
pub fn paint_end_on(p: &egui::Painter, rect: egui::Rect, items: &[(&Mesh, egui::Color32)]) {
    paint_end_on_around(p, rect, items, 0, 0.0);
}

/// `paint_end_on` scaled to the items from `from` on, grown by `grow` of their size each way;
/// the others drawn about them, cut at the picture's edge.
pub fn paint_end_on_around(
    p: &egui::Painter,
    rect: egui::Rect,
    items: &[(&Mesh, egui::Color32)],
    from: usize,
    grow: f32,
) {
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for (m, _) in &items[from.min(items.len())..] {
        for q in &m.pos {
            lo[0] = lo[0].min(q[0]);
            hi[0] = hi[0].max(q[0]);
            lo[1] = lo[1].min(q[2]);
            hi[1] = hi[1].max(q[2]);
        }
    }
    if lo[0] > hi[0] {
        return;
    }
    let size = (hi[0] - lo[0]).max(hi[1] - lo[1]);
    for k in 0..2 {
        lo[k] -= grow * size;
        hi[k] += grow * size;
    }
    let r = rect.shrink(4.0);
    let s = (r.width() / (hi[0] - lo[0]).max(1e-3)).min(r.height() / (hi[1] - lo[1]).max(1e-3));
    let c = [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])];
    let to = |q: [f32; 3]| egui::pos2(r.center().x + (q[0] - c[0]) * s, r.center().y - (q[2] - c[1]) * s);
    for (m, col) in items {
        let mut out = egui::Mesh::default();
        for q in &m.pos {
            out.colored_vertex(to(*q), *col);
        }
        for t in &m.tri {
            out.add_triangle(t[0], t[1], t[2]);
        }
        p.add(egui::Shape::mesh(out));
    }
}

/// A punch's drawn section (tip and body at its published width), as a thin slice.
pub fn punch_mesh(p: &fb::Punch) -> Mesh {
    let mut m = Mesh::default();
    for sec in [p.tip_section(), drawn_body(p)] {
        if let Some(x) = extrude(&sec, 0.0, 0.0, 0.0, 1.0) {
            m.append(&x);
        }
    }
    m
}

pub fn die_mesh(d: &fb::Die) -> Mesh {
    extrude(&d.section(), 0.0, 0.0, 0.0, 1.0).unwrap_or_default()
}

/// The part's bends' needs, from the program: the sharpest included angle and the smallest
/// inside radius any step makes (the PunchAngle, DieAngle and PunchRadius rules' sides).
pub fn needs(sim: &Sim) -> Option<(f64, f64)> {
    let mut inc = f64::INFINITY;
    let mut r = f64::INFINITY;
    for s in &sim.plan.steps {
        let phimax = s.angle + s.overbend;
        inc = inc.min(std::f64::consts::PI - phimax);
        r = r.min(sim.model.bends[s.bend].inside_radius(phimax));
    }
    (inc.is_finite() && r.is_finite()).then_some((inc, r))
}

/// One card of the library: the tool's cross-section, its name, its working values and its
/// rated load; the id on hover. Returns the click.
#[allow(clippy::too_many_arguments)]
pub fn card(
    ui: &mut egui::Ui,
    t: &Theme,
    id: &str,
    title: &str,
    values: &str,
    load: &str,
    mesh: Option<&Mesh>,
    on: bool,
    width: f32,
) -> egui::Response {
    let pic_w = 46.0;
    let wrap = width - pic_w - 18.0;
    let g = ui
        .painter()
        .layout(title.to_string(), super::theme::semi(11.5), t.ink, wrap);
    let th = g.size().y;
    let h = (th + 38.0).max(64.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::click());
    let p = ui.painter_at(rect);
    let hovered = resp.hovered();
    let fill = if on {
        t.selection
    } else if hovered {
        t.hover
    } else {
        t.raised
    };
    p.rect_filled(rect, 5.0, fill);
    p.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(if on { 1.5 } else { 1.0 }, if on { t.accent } else { t.line }),
        egui::StrokeKind::Inside,
    );
    let pic = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(pic_w, h - 8.0));
    p.rect_filled(pic, 3.0, t.field);
    match mesh {
        Some(m) => paint_end_on(&p, pic, &[(m, egui::Color32::from_rgb(0x9A, 0xA0, 0xA8))]),
        None => {
            p.text(
                pic.center(),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(18.0),
                t.ink2,
            );
        }
    }
    let x = pic.right() + 8.0;
    p.galley(egui::pos2(x, rect.top() + 5.0), g, t.ink);
    let small = egui::FontId::proportional(10.5);
    p.text(
        egui::pos2(x, rect.top() + 8.0 + th),
        egui::Align2::LEFT_TOP,
        values,
        small.clone(),
        t.ink2,
    );
    p.text(
        egui::pos2(x, rect.top() + 21.0 + th),
        egui::Align2::LEFT_TOP,
        load,
        small,
        t.ink2,
    );
    resp.on_hover_text(id)
}

/// A picture: meshes seen end on, each in its colour (the first the tools about the part).
pub type Picture = Vec<(Mesh, egui::Color32)>;

/// Each step's part as it stands bent at its tools (just sprung back, the punch still on it),
/// seen end on with the punch tip and die about it: the program's rows' pictures.
pub fn step_pictures(sim: &Sim) -> Vec<Picture> {
    let mut sc = fb::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
    (0..sim.plan.steps.len())
        .map(|k| {
            let ctx_name = |n: &str| n.starts_with("punch:") || n.starts_with("die:");
            let mut tools = Mesh::default();
            let mut part = Mesh::default();
            let mut zone = Mesh::default();
            for b in sc.bodies(k, 0.8) {
                // the part's own pieces, and the working punch's tip and the die near it
                match b.look {
                    fb::scene::Look::Sheet => part.append(&b.mesh),
                    fb::scene::Look::BendZone => zone.append(&b.mesh),
                    fb::scene::Look::Tool if ctx_name(&b.name) && !b.name.ends_with("/body") => {
                        tools.append(&b.mesh)
                    }
                    _ => {}
                }
            }
            vec![
                (tools, egui::Color32::from_rgb(0x70, 0x76, 0x7E)),
                (part, egui::Color32::from_rgb(0x4A, 0x9C, 0xE8)),
                (zone, egui::Color32::from_rgb(0xF5, 0x9A, 0x1A)),
            ]
        })
        .collect()
}
