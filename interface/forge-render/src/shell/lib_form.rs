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

/// A record's heading: its product name ("Trumpf TruBend 5085") with the id and kind under it,
/// or, with no name, the id. A long name is cut with "…" and shown whole on hover.
pub fn title(ui: &mut Ui, t: &Theme, name: Option<&str>, id: &str, kind: &str) {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => {
            ui.add(egui::Label::new(RichText::new(n).font(semi(14.0)).color(t.ink)).truncate())
                .on_hover_text(n);
            ui.label(
                RichText::new(format!("{id} · {kind}"))
                    .font(sans(11.0))
                    .color(t.ink2),
            );
        }
        None => {
            ui.horizontal(|ui| {
                ui.label(RichText::new(id).font(semi(14.0)).color(t.ink));
                ui.add_space(6.0);
                ui.label(RichText::new(kind).font(sans(11.0)).color(t.ink2));
            });
        }
    }
}

/// "Maker product": the product's words up to its first parenthesis (the rest is detail).
pub fn product_name(maker: &str, product: &str) -> Option<String> {
    let p = product.split(" (").next().unwrap_or(product).trim();
    let s = format!("{} {p}", maker.trim());
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// A value the record does not publish but the view draws (a conflict's first cited value, an
/// estimate by its rule): shown as drawn, in its chip's colour, the reason on hover; a click takes
/// it as the user's own value. Lengths in the document's units.
pub fn drawn_cell(ui: &mut Ui, t: &Theme, d: &crate::library_mesh::Drawn, unit: &str, w: f32) -> bool {
    let p = match d.basis {
        crate::library_mesh::Basis::Conflict => Prov::Conflict,
        _ => Prov::Estimate,
    };
    let doc = DOC_UNITS.with(|c| c.get());
    // an estimate to a tenth (its rule is no finer); a cited value as cited
    let near = |x: f64| match d.basis {
        crate::library_mesh::Basis::Estimate => (x * 10.0).round() / 10.0,
        _ => x,
    };
    let shown = match forge_units::lookup(doc) {
        Some((_, top, bottom)) if unit == "mm" && doc != "mm" => {
            value_text(near(d.value / (top / bottom)), doc)
        }
        _ => value_text(near(d.value), unit),
    };
    let text = match d.basis {
        crate::library_mesh::Basis::Estimate => format!("≈ {shown}"),
        _ => shown,
    };
    let col = p.colour(t);
    let b = egui::Button::new(RichText::new(text).font(sans(12.0)).color(t.ink))
        .fill(col.gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, col.gamma_multiply(0.45)))
        .min_size(vec2(w, 18.0));
    ui.add(b)
        .on_hover_text(format!("{} Click to take it as your own value.", d.why))
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
        "segments" => "Segments",
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
    let u = unit.to_string();
    ui.add(
        egui::DragValue::new(x)
            .speed(0.1)
            .custom_formatter(move |v, _| value_text(v, &u))
            .custom_parser(leading_number),
    )
    .changed()
}

/// A value as the sheet shows it: its number to the precision it carries (forge_units::trim),
/// then its unit ("850 kN", "10 mm", "86°", "220 mm/s").
pub fn value_text(x: f64, unit: &str) -> String {
    match unit {
        "" => forge_units::trim(x),
        "deg" | "°" => format!("{}°", forge_units::trim(x)),
        u => format!("{} {u}", forge_units::trim(x)),
    }
}

/// The number a person typed, its unit word ignored ("850 kN" → 850).
pub fn leading_number(t: &str) -> Option<f64> {
    let t = t.trim();
    let end = t
        .char_indices()
        .find(|(i, c)| {
            !(c.is_ascii_digit()
                || *c == '.'
                || (*i == 0 && (*c == '-' || *c == '+'))
                || *c == 'e'
                || *c == 'E')
        })
        .map_or(t.len(), |(i, _)| i);
    t[..end].parse().ok()
}

/// A level's heading in a library list: "Built-in · 236".
pub fn level_head(ui: &mut Ui, t: &Theme, level: &str, n: usize) {
    let mut c = level.chars();
    let name = c.next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    });
    ui.add_space(2.0);
    ui.label(
        RichText::new(format!("{name} · {n}"))
            .font(semi(11.0))
            .color(t.ink2),
    );
}

/// The document's length unit word ("mm", "in"), for a table's column heading.
pub fn doc_units() -> &'static str {
    DOC_UNITS.with(|c| c.get())
}

/// A library list's level filter (docs/ux/STANDARD.md §8): All · Project · User · Built-in as one
/// segmented row with each level's count, kept per list; a level with nothing in it is greyed.
/// Returns whether each level ([Project, User, Built-in]) is shown.
pub fn level_filter(ui: &mut Ui, t: &Theme, salt: &str, counts: [usize; 3]) -> [bool; 3] {
    let id = ui.make_persistent_id(("level-filter", salt));
    let mut on: u8 = ui.data(|d| d.get_temp(id)).unwrap_or(0);
    // a level emptied since it was chosen: back to all
    if on > 0 && counts[usize::from(on) - 1] == 0 {
        on = 0;
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let total: usize = counts.iter().sum();
        for (k, (w, n)) in [
            ("All", total),
            ("Project", counts[0]),
            ("User", counts[1]),
            ("Built-in", counts[2]),
        ]
        .into_iter()
        .enumerate()
        {
            let k = k as u8;
            let text = RichText::new(format!("{w} {n}"))
                .font(sans(11.0))
                .color(if n == 0 { t.ink2.gamma_multiply(0.6) } else { t.ink });
            let r = ui.add_enabled(n > 0 || k == 0, egui::Button::selectable(on == k, text));
            if r.clicked() {
                on = k;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(id, on));
    match on {
        0 => [true; 3],
        k => std::array::from_fn(|i| i + 1 == usize::from(k)),
    }
}

/// Whether a level is shown by `level_filter`'s answer.
pub fn level_shown(shown: [bool; 3], lv: forge_library::Level) -> bool {
    match lv {
        forge_library::Level::Project => shown[0],
        forge_library::Level::User => shown[1],
        forge_library::Level::Builtin => shown[2],
    }
}

/// The count of entries per level ([Project, User, Built-in]).
pub fn level_counts(levels: impl Iterator<Item = forge_library::Level>) -> [usize; 3] {
    let mut c = [0; 3];
    for l in levels {
        c[match l {
            forge_library::Level::Project => 0,
            forge_library::Level::User => 1,
            forge_library::Level::Builtin => 2,
        }] += 1;
    }
    c
}

/// Above a library list with no records of the user's own yet: what to do next.
pub fn empty_levels(ui: &mut Ui, t: &Theme, any_own: bool) {
    if !any_own {
        ui.label(
            RichText::new("No records of your own yet · derive one from a built-in record, then Save")
                .font(sans(11.0))
                .color(t.ink2),
        );
    }
}

/// A count (flutes, jaws, faces): a whole number.
pub fn int_edit(ui: &mut Ui, x: &mut f64) -> bool {
    let mut n = x.round() as i64;
    let ch = ui
        .add(egui::DragValue::new(&mut n).speed(0.1).range(0..=1000))
        .changed();
    if ch {
        *x = n as f64;
    }
    ch
}

/// Workholding's keys (forge-workholding's schema) as people say them.
pub fn workholding_label(key: &str) -> String {
    match key {
        "body_d" => "Body diameter",
        "body_len" => "Body length",
        "jaw_len" => "Jaw length",
        "jaw_height" => "Jaw height",
        "grip_min" => "Smallest grip",
        "grip_max" => "Largest grip",
        "through_hole" => "Through hole",
        "max_rpm" => "Max spindle speed",
        "grip_force" => "Grip force",
        "clamp_force" => "Clamp force",
        "max_opening" => "Max opening",
        "min_opening" => "Min opening",
        "stud_d" => "Stud diameter",
        "hole_d" => "Hole diameter",
        "dowel_d" => "Dowel diameter",
        "dowel_h" => "Dowel height",
        "module_d" => "Module diameter",
        "module_h" => "Module height",
        "range_min" => "Smallest size held",
        "range_max" => "Largest size held",
        "quill_d" => "Quill diameter",
        "frame_r" => "Frame radius",
        "nose_d" => "Nose diameter",
        "nose_len" => "Nose length",
        "collet_len" => "Collet length",
        "collet_d" => "Collet diameter",
        "point_len" => "Point length",
        "d" => "Diameter",
        _ => return humanize(key),
    }
    .to_string()
}

/// A text value in a sheet (STANDARD §5): shown as a label in `w`, cut with "…" and whole on
/// hover, a click to edit it; a web address shown as a link with its domain, edited by its
/// pencil. Some(new text) when an edit is committed.
pub fn text_cell(
    ui: &mut Ui,
    t: &Theme,
    salt: impl std::hash::Hash + std::fmt::Debug,
    value: &str,
    w: f32,
) -> Option<String> {
    text_cell_as(ui, t, salt, value, None, w)
}

/// A maker's coating code in words, the code kept ("Uncoated (UN)"); a symbol the maker does not
/// name says so. None: shown as written.
pub fn coating_word(code: &str) -> Option<String> {
    let w = match code.to_ascii_uppercase().as_str() {
        "UN" => "Uncoated",
        "BRIGHT" => "Bright (uncoated)",
        "TIALN" => "TiAlN",
        "ALTIN" => "AlTiN",
        "TICN" => "TiCN",
        "TIN" => "TiN",
        "APLUS" => "Aplus",
        "F" | "S" => return Some(format!("\"{code}\" (the maker's symbol, not named)")),
        _ => return None,
    };
    (w != code).then(|| format!("{w} ({code})"))
}

/// `text_cell` showing `shown` in place of the value while not editing (the value on hover).
pub fn text_cell_as(
    ui: &mut Ui,
    t: &Theme,
    salt: impl std::hash::Hash + std::fmt::Debug,
    value: &str,
    shown: Option<&str>,
    w: f32,
) -> Option<String> {
    let id = ui.make_persistent_id(salt);
    let editing = ui.data(|d| d.get_temp::<String>(id));
    if let Some(mut buf) = editing {
        let r = ui.add_sized(vec2(w, 20.0), egui::TextEdit::singleline(&mut buf));
        if !r.has_focus() && !r.lost_focus() {
            r.request_focus();
        }
        if r.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
            return (buf != value).then_some(buf);
        }
        ui.data_mut(|d| d.insert_temp(id, buf));
        return None;
    }
    let url = value.starts_with("http://") || value.starts_with("https://");
    let start = |ui: &mut Ui| ui.data_mut(|d| d.insert_temp(id, value.to_string()));
    ui.allocate_ui_with_layout(
        vec2(w, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(w);
            if url {
                let rest = value.split("://").nth(1).unwrap_or(value);
                let domain = rest.split('/').next().unwrap_or(rest).trim_start_matches("www.");
                ui.add(egui::Hyperlink::from_label_and_url(
                    RichText::new(format!("{domain} (link)")).font(sans(12.0)),
                    value,
                ))
                .on_hover_text(value);
                if ui
                    .small_button("edit")
                    .on_hover_text("change the address")
                    .clicked()
                {
                    start(ui);
                }
            } else {
                let shown = match shown {
                    Some(s) => s,
                    None if value.is_empty() => "(none)",
                    None => value,
                };
                let r = ui
                    .add(
                        egui::Label::new(RichText::new(shown).font(sans(12.0)).color(t.ink))
                            .truncate()
                            .sense(Sense::click()),
                    )
                    .on_hover_text(format!("{value}\n(click to edit)"));
                if r.clicked() {
                    start(ui);
                }
            }
        },
    );
    None
}

#[cfg(test)]
mod words_tests {
    use super::*;

    #[test]
    fn makers_codes_and_names_read_as_words() {
        assert_eq!(coating_word("UN").as_deref(), Some("Uncoated (UN)"));
        assert_eq!(coating_word("TIALN").as_deref(), Some("TiAlN (TIALN)"));
        assert_eq!(coating_word("TiAlN"), None);
        assert!(coating_word("F").unwrap().contains("not named"));
        assert_eq!(coating_word("C3"), None);
        assert_eq!(
            product_name(
                "Haas",
                "6 in single-station CrossOver vise, 9 in opening (PN 05-0404, Kurt)"
            )
            .as_deref(),
            Some("Haas 6 in single-station CrossOver vise, 9 in opening")
        );
        assert_eq!(
            product_name("Trumpf", "TruBend 5085").as_deref(),
            Some("Trumpf TruBend 5085")
        );
        assert_eq!(product_name("", ""), None);
    }
}
