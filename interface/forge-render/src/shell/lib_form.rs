//! What the library builder's forms share (research/cam/library-builder.md §3): human labels
//! for the record keys (the raw key on hover), the sections they sit in, provenance chips with
//! a one-line legend (published / estimate / unknown / yours / conflict / uncited), an unknown
//! value drawn as a fillable slot, and the small outline thumbnails of the library lists.
use super::theme::{Theme, mono, sans, semi};
use egui::{Color32, CornerRadius, Response, RichText, Sense, Stroke, Ui, vec2};

/// Where a value stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prov {
    Published,
    Estimate,
    Unknown,
    Conflict,
    /// The user's own value.
    Yours,
    /// Copied from another record, not yet confirmed.
    Copied,
    /// A forgecad stand-in number.
    StandIn,
    Uncited,
}

impl Prov {
    /// From a cite's status word (the tooling and machine formats' words, plus "user").
    pub fn of(word: Option<&str>) -> Prov {
        match word {
            // cited to a source (a URL or a catalogue)
            Some("published") | Some("cited") => Prov::Published,
            Some("estimate") => Prov::Estimate,
            Some("unknown") | Some("UNKNOWN") => Prov::Unknown,
            Some("conflict") => Prov::Conflict,
            Some("user") | Some("yours") => Prov::Yours,
            Some("copied") => Prov::Copied,
            Some(w) if w.starts_with("stand-in") || w.starts_with("estimate (stand-in") => Prov::StandIn,
            Some(w) if w.starts_with("copied") => Prov::Copied,
            _ => Prov::Uncited,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Prov::Published => "published",
            Prov::Estimate => "estimate",
            Prov::Unknown => "unknown",
            Prov::Conflict => "conflict",
            Prov::Yours => "yours",
            Prov::Copied => "copied",
            Prov::StandIn => "stand-in",
            Prov::Uncited => "uncited",
        }
    }

    pub fn meaning(self) -> &'static str {
        match self {
            Prov::Published => "the maker's published number, cited",
            Prov::Estimate => "an estimate, with its basis",
            Prov::Unknown => "not known: fill it in from the maker's data",
            Prov::Conflict => "sources disagree: see the note",
            Prov::Yours => "your own value",
            Prov::Copied => "copied from another record: confirm or replace",
            Prov::StandIn => "forgecad's stand-in number: replace with the maker's",
            Prov::Uncited => "no cite",
        }
    }

    pub fn colour(self, t: &Theme) -> Color32 {
        match self {
            Prov::Published => t.accent,
            Prov::Estimate | Prov::StandIn | Prov::Copied => t.warn,
            Prov::Unknown | Prov::Conflict => t.error,
            Prov::Yours => t.success,
            Prov::Uncited => t.ink2,
        }
    }
}

/// A small coloured badge; clicking it opens a menu with what it means and "mark unknown".
/// Returns true when the user chose to mark the value unknown.
pub fn chip(ui: &mut Ui, t: &Theme, p: Prov, detail: &str, can_unknown: bool) -> bool {
    let col = p.colour(t);
    let font = semi(9.5);
    let galley = ui.painter().layout_no_wrap(p.word().to_uppercase(), font, col);
    let size = vec2(galley.size().x + 10.0, 15.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = col.gamma_multiply(if p == Prov::Unknown { 0.28 } else { 0.16 });
    ui.painter().rect(
        rect,
        CornerRadius::same(7),
        fill,
        Stroke::new(1.0, col.gamma_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, col);
    let resp = resp.on_hover_text(format!("{}: {}", p.word(), p.meaning()));
    let mut unknown = false;
    egui::Popup::menu(&resp).show(|ui| {
        ui.label(RichText::new(p.meaning()).font(sans(11.0)).color(t.ink));
        if !detail.is_empty() {
            ui.label(RichText::new(detail).font(sans(10.5)).color(t.ink2));
        }
        if can_unknown
            && p != Prov::Unknown
            && ui.button("I don't know this value: mark it unknown").clicked()
        {
            unknown = true;
        }
    });
    unknown
}

/// The chips' one-line legend.
pub fn legend(ui: &mut Ui, t: &Theme) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for p in [Prov::Published, Prov::Estimate, Prov::Unknown, Prov::Yours] {
            chip(ui, t, p, "", false);
            ui.label(RichText::new(short(p)).font(sans(10.0)).color(t.ink2));
            ui.add_space(4.0);
        }
        ui.label(
            RichText::new("· click a chip for its source")
                .font(sans(10.0))
                .color(t.ink2),
        );
    });
}

fn short(p: Prov) -> &'static str {
    match p {
        Prov::Published => "maker's data",
        Prov::Estimate => "estimated",
        Prov::Unknown => "fill in",
        Prov::Yours => "you entered",
        _ => "",
    }
}

/// A field's name: its human label, the raw key on hover.
pub fn name(ui: &mut Ui, t: &Theme, label: &str, key: &str, help: &str, w: f32) -> Response {
    let r = ui
        .allocate_ui_with_layout(
            vec2(w, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(w);
                ui.add(egui::Label::new(RichText::new(label).font(sans(12.0)).color(t.ink2)).truncate())
            },
        )
        .inner;
    let tip = if help.is_empty() {
        format!("key: {key}")
    } else {
        format!("{help}\nkey: {key}")
    };
    r.on_hover_ui(|ui| {
        ui.label(RichText::new(tip).font(mono(11.0)));
    })
}

/// A value not known: a highlighted slot with "enter value". True when clicked.
pub fn unknown_slot(ui: &mut Ui, t: &Theme, w: f32) -> bool {
    let b = egui::Button::new(
        RichText::new("unknown · enter value")
            .font(sans(11.0))
            .color(t.error),
    )
    .fill(t.error.gamma_multiply(0.10))
    .stroke(Stroke::new(1.0, t.error.gamma_multiply(0.6)))
    .min_size(vec2(w, 18.0));
    ui.add(b)
        .on_hover_text("not known: enter the maker's value (it becomes yours, cited as such)")
        .clicked()
}

/// A section heading.
pub fn section(ui: &mut Ui, t: &Theme, s: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(s.to_uppercase()).font(semi(10.5)).color(t.ink2));
}

/// "tip_radius" → "Tip radius"; "relief.depth" → "Relief depth".
pub fn humanize(key: &str) -> String {
    let s = key.replace(['_', '.'], " ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The tooling formats' keys: (label, section). Sections: Geometry, Cutting, Holder interface,
/// Source.
pub fn tooling_label(key: &str) -> (String, &'static str) {
    let (l, g) = match key {
        "dc" => ("Cutting diameter", "Geometry"),
        "dcon" => ("Shank diameter", "Geometry"),
        "lcf" => ("Flute length", "Geometry"),
        "oal" => ("Overall length", "Geometry"),
        "re" => ("Corner radius", "Geometry"),
        "angle" => ("Included angle", "Geometry"),
        "sig" => ("Point angle", "Geometry"),
        "tip_d" => ("Tip diameter", "Geometry"),
        "tp" => ("Thread pitch", "Geometry"),
        "neck_d" => ("Neck diameter", "Geometry"),
        "reach" => ("Reach", "Geometry"),
        "nof" => ("Flutes", "Cutting"),
        "helix" => ("Helix angle", "Cutting"),
        "hand" => ("Hand", "Cutting"),
        "type" | "kind" => ("Type", "Cutting"),
        "material" => ("Tool material", "Cutting"),
        "coating" => ("Coating", "Cutting"),
        "chipbreaker" => ("Chip breaker", "Cutting"),
        "max_rpm" => ("Max spindle speed", "Cutting"),
        "presets" => ("Feeds and speeds", "Cutting"),
        "gauge_length" => ("Gauge length", "Geometry"),
        "profile" => ("Profile (z, diameter)", "Geometry"),
        "bore" => ("Bore", "Geometry"),
        "od" => ("Outside diameter", "Geometry"),
        "length" => ("Length", "Geometry"),
        "ic" => ("Inscribed circle", "Geometry"),
        "s" => ("Thickness", "Geometry"),
        "l" => ("Edge length", "Geometry"),
        "clearance" => ("Clearance angle", "Geometry"),
        "kr" => ("Entering angle", "Geometry"),
        "h" => ("Shank height", "Geometry"),
        "b" => ("Shank width", "Geometry"),
        "f" => ("Functional width", "Geometry"),
        "dmm" => ("Shank diameter", "Geometry"),
        "min_bore" => ("Minimum bore", "Geometry"),
        "interface" => ("Spindle interface", "Holder interface"),
        "clamp" => ("Clamping", "Holder interface"),
        "system" => ("Collet system", "Holder interface"),
        "thread" => ("Thread", "Holder interface"),
        "coolant" => ("Coolant through", "Holder interface"),
        "insert" => ("Insert", "Holder interface"),
        "vendor" => ("Vendor", "Source"),
        "maker" => ("Maker", "Source"),
        "product_id" => ("Product number", "Source"),
        "code" => ("ISO code", "Source"),
        "source" => ("Source", "Source"),
        "taken" => ("Read on", "Source"),
        "note" => ("Note", "Source"),
        _ => return (humanize(key), "Other"),
    };
    (l.to_string(), g)
}

/// Press-brake keys (forge_brake::schema paths): (label, section).
pub fn brake_label(key: &str, group: &str) -> (String, &'static str) {
    let l = match key {
        "maker" => "Maker",
        "name" => "Name",
        "model" => "Model",
        "real" => "Real product",
        "note" => "Note",
        "clamp" => "Clamping system",
        "kind" => "Form",
        "height" => "Height",
        "tip_radius" => "Tip radius",
        "tip_angle" => "Tip angle",
        "tip_height" => "Tip height",
        "width" => "Body width",
        "relief.depth" => "Relief depth",
        "relief.z0" => "Relief from",
        "relief.z1" => "Relief to",
        "max_load" => "Max load",
        "segments" => "Segments (length, count)",
        "outline" => "Section outline (x, z)",
        "v" => "V opening",
        "angle" => "V angle",
        "shoulder_radius" => "Shoulder radius",
        "floor" => "Floor",
        "force" => "Press force",
        "bed_length" => "Bending length",
        "housing_gap" => "Width between housings",
        "open_height" => "Open height",
        "stroke" => "Stroke",
        "clamps_included" => "Clamps included",
        "throat" => "Throat depth",
        "frame.thickness" => "Housing thickness",
        "frame.depth" => "Housing depth",
        "ram.front" => "Ram front",
        "ram.back" => "Ram back",
        "bed.front" => "Bed front",
        "bed.back" => "Bed back",
        "table_height" => "Table height",
        "speeds.approach" => "Approach speed",
        "speeds.work" => "Working speed",
        "speeds.return" => "Return speed",
        "backgauge.axes" => "Backgauge axes",
        "backgauge.x" => "Backgauge X travel",
        "backgauge.r" => "Backgauge R travel",
        "backgauge.z" => "Backgauge Z travel",
        "backgauge.fingers" => "Fingers",
        "backgauge.finger.depth" => "Finger depth",
        "backgauge.finger.height" => "Finger height",
        "backgauge.finger.width" => "Finger width",
        "backgauge.retract" => "Retract",
        _ => return (humanize(key), section_of_brake(key, group)),
    };
    (l.to_string(), section_of_brake(key, group))
}

fn section_of_brake(key: &str, group: &str) -> &'static str {
    if group == "Working geometry" {
        return "Working geometry";
    }
    match key {
        "maker" | "name" | "model" | "real" | "note" => "Source",
        "clamp" | "kind" | "clamps_included" => "Tooling interface",
        "max_load" | "force" | "segments" => "Rating and lengths",
        k if k.starts_with("speeds.") => "Speeds",
        k if k.starts_with("backgauge.") => "Backgauge",
        _ => "Frame",
    }
}

/// A thumbnail of closed outlines (x, y up), fitted to a `size` square.
pub fn thumb(ui: &mut Ui, t: &Theme, polys: &[Vec<[f64; 2]>], size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(3), t.field);
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for q in polys.iter().flatten() {
        for k in 0..2 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    if !(lo[0].is_finite() && (hi[0] > lo[0] || hi[1] > lo[1])) {
        return;
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
    let s = (size as f64 - 6.0) / span;
    let c = rect.center();
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    for poly in polys {
        let pts: Vec<egui::Pos2> = poly
            .iter()
            .map(|q| {
                egui::pos2(
                    c.x + ((q[0] - mid[0]) * s) as f32,
                    c.y - ((q[1] - mid[1]) * s) as f32,
                )
            })
            .collect();
        if pts.len() >= 3 {
            p.add(egui::Shape::closed_line(pts, Stroke::new(1.0, t.ink)));
        }
    }
}

/// A turned outline (z along the axis, r) as the closed silhouette (axis across, z up).
pub fn silhouette(outline: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut v: Vec<[f64; 2]> = outline.iter().map(|[z, r]| [*r, *z]).collect();
    v.extend(outline.iter().rev().map(|[z, r]| [-*r, *z]));
    v
}

/// The front view (x across, z up) of points: their bounding rectangle.
pub fn rect_xz(pts: &[[f64; 3]]) -> Vec<[f64; 2]> {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in pts {
        lo = [lo[0].min(p[0]), lo[1].min(p[2])];
        hi = [hi[0].max(p[0]), hi[1].max(p[2])];
    }
    vec![[lo[0], lo[1]], [hi[0], lo[1]], [hi[0], hi[1]], [lo[0], hi[1]]]
}

/// A cylinder's extreme points (its two end centres pushed out by the radius on every axis).
pub fn cyl_pts(base: [f64; 3], axis: [f64; 3], r: f64, h: f64) -> Vec<[f64; 3]> {
    let top = [
        base[0] + axis[0] * h,
        base[1] + axis[1] * h,
        base[2] + axis[2] * h,
    ];
    let mut v = Vec::new();
    for c in [base, top] {
        for k in 0..3 {
            // no extent along the axis itself
            let e = r * (1.0 - axis[k] * axis[k]).max(0.0).sqrt();
            let mut a = c;
            a[k] += e;
            v.push(a);
            let mut b = c;
            b[k] -= e;
            v.push(b);
        }
    }
    v
}

/// A machine's thumbnail: each body's primitives (placed at home) seen from the front.
pub fn machine_thumb(def: &forge_machine::def::MachineDef) -> Vec<Vec<[f64; 2]>> {
    use forge_machine::def::Prim;
    def.bodies
        .iter()
        .flat_map(|b| b.prims.iter())
        .map(|p| match p {
            Prim::Box { min, max } => rect_xz(&[*min, *max]),
            Prim::Cyl { base, axis, r, h } => rect_xz(&cyl_pts(*base, *axis, *r, *h)),
        })
        .collect()
}

thread_local! {
    static DOC_UNITS: std::cell::Cell<&'static str> = const { std::cell::Cell::new("mm") };
}

/// The length unit the sheets show lengths in this frame (the window's mm/in).
pub fn set_doc_units(u: &'static str) {
    DOC_UNITS.with(|c| c.set(u));
}

/// A number kept in `unit` edited in place: a length shown in the window's unit (inches in an
/// inch document), anything else in the unit the record keeps it in. True when changed.
pub fn num_edit(ui: &mut Ui, x: &mut f64, unit: &str) -> bool {
    let doc = DOC_UNITS.with(|c| c.get());
    if unit == "mm"
        && doc != "mm"
        && let Some((_, top, bottom)) = forge_units::lookup(doc)
    {
        let per = top / bottom;
        let mut v = *x / per;
        let ch = ui
            .add(
                egui::DragValue::new(&mut v)
                    .speed(0.01)
                    .max_decimals(4)
                    .suffix(format!(" {doc}")),
            )
            .changed();
        if ch {
            *x = v * per;
        }
        return ch;
    }
    let sfx = match unit {
        "" => String::new(),
        "deg" | "°" => "°".to_string(),
        u => format!(" {u}"),
    };
    ui.add(egui::DragValue::new(x).speed(0.1).max_decimals(4).suffix(sfx))
        .changed()
}
