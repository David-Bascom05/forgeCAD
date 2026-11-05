//! A feature's dialog: floats over the view, drags by its title, remembers where it was left,
//! scrolls, and groups its fields in sections, in tabs where a kind has many. It says what the fields
//! are; the host turns them into a tool call, previews it live and commits it on OK.

use super::theme::{mono, sans, semi};
use super::{Action, Shell, panel};
use crate::shell::placed::Placed;
use egui::{Align2, Color32, Rect, Sense, Stroke, pos2, vec2};
use forge_doc::ItemId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogTarget {
    New(&'static str),
    Edit(ItemId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dialog {
    pub target: DialogTarget,
    pub kind: String,
    pub title: String,
    /// Label and text, in the line's order.
    pub fields: Vec<(String, String)>,
    /// The field that takes what's clicked in the view, and what to say till it has some.
    pub pick: Option<(String, &'static str)>,
    /// The line last previewed, and that preview's number.
    pub sent: Option<String>,
    pub seq: u64,
    pub error: Option<String>,
    /// OK was pressed: the answer's on its way.
    pub awaiting: bool,
    /// The unit a bare number's in, and the part's parameter names.
    pub units: &'static str,
    pub params: Vec<String>,
    /// Each number field as first shown and as the file had it: what's untouched goes back as it was.
    pub first: Vec<(String, String, String)>,
    /// How an extrude's distance is taken, which the file writes with it.
    pub extent: Extent,
    pub toward: Toward,
    /// The face an extrude goes to, as a line names it.
    pub surface: String,
    /// + was pressed: OK, then another of these.
    pub again: bool,
    /// A thread's choice from the catalogue.
    pub thread: ThreadChoice,
    /// A hole's choices; its fields are made from them.
    pub hole: Option<HoleChoice>,
    /// A rib's other side has been tried by itself; or the person has chosen.
    pub side_settled: bool,
    /// The selections still to come after the one taking clicks.
    pub queue: Vec<(String, &'static str)>,
    /// A revolve goes all the way round, or by its angle this way.
    pub full: bool,
    pub way: Way,
}

/// How far an extrude goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    Distance,
    Through,
    Next,
    /// To a face picked.
    Surface,
}

/// Which way it goes from its sketch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Forward,
    Back,
    Both,
}

/// What a revolve waits for, till it has its axis.
pub const AXIS_HINT: &str = "Pick a sketch line, a straight edge, or an axis to turn about";

/// A line in the world, as an axis in a sketch plane's own terms: `axis OX OY DX DY`, if it lies
/// in the plane.
pub fn axis_in_plane(plane: &forge_math::Frame, at: [f64; 3], dir: [f64; 3]) -> Result<String, String> {
    let p = plane.point_to_local(forge_math::Point3::new(at[0], at[1], at[2]));
    let d = plane.vec_to_local(forge_math::Vec3::new(dir[0], dir[1], dir[2]));
    let len = d.length();
    if len == 0.0 || p.z.abs() > 1e-6 || d.z.abs() > 1e-9 * len {
        return Err("The axis must lie in the sketch's plane: pick one that does".to_string());
    }
    let f = |x: f64| {
        let r = (x * 1e9).round() / 1e9;
        if r == 0.0 { "0".to_string() } else { format!("{r}") }
    };
    Ok(format!(
        "axis {} {} {} {}",
        f(p.x),
        f(p.y),
        f(d.x / len),
        f(d.y / len)
    ))
}

/// The sketch a revolve's profile is on, by name.
pub fn profile_sketch(d: &Dialog) -> Option<String> {
    let f = d.fields.iter().find(|f| f.0 == "Profile")?;
    f.1.split_whitespace().next().map(str::to_string)
}

/// Which way a revolve turns from its sketch: one way by its angle, the other, half each way, or
/// an angle each way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Forward,
    Back,
    Symmetric,
    Two,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleKind {
    Simple,
    Clearance,
    Tapped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    None,
    Counterbore,
    Spotface,
    Countersink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    Distance,
    Through,
    Next,
    /// To a face picked.
    Surface,
}

/// How a hole's placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placing {
    Sketch,
    /// On a face, measured from two of its edges.
    Linear,
    /// Round a circular edge.
    Concentric,
}

/// A hole placed by distances from two edges of a face: what's been picked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Linear {
    /// A point of the face, and its normal outward.
    pub face: Option<([f64; 3], [f64; 3])>,
    /// Where on the face it was clicked: which side of an edge is in.
    pub clicked: [f64; 3],
    /// Each edge's ends.
    pub edges: Vec<([f64; 3], [f64; 3])>,
    pub first: String,
    pub second: String,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit3(a: [f64; 3]) -> [f64; 3] {
    let l = dot3(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

impl Linear {
    /// Where the hole's centre is: this far in from each edge, on the face. None if the edges don't
    /// fix a place.
    pub fn place(&self, d1: f64, d2: f64) -> Option<[f64; 3]> {
        let (o, n) = self.face?;
        let [(a1, b1), (a2, b2)] = self.edges[..] else {
            return None;
        };
        let inward = |a: [f64; 3], b: [f64; 3]| {
            let v = unit3(cross3(n, sub(b, a)));
            if dot3(v, sub(self.clicked, a)) >= 0.0 {
                v
            } else {
                v.map(|x| -x)
            }
        };
        let (v1, v2) = (inward(a1, b1), inward(a2, b2));
        // three planes: the face, and each edge's offset
        let (r1, r2, r3) = (dot3(n, o), dot3(v1, a1) + d1, dot3(v2, a2) + d2);
        let (c23, c31, c12) = (cross3(v1, v2), cross3(v2, n), cross3(n, v1));
        let det = dot3(n, c23);
        if det.abs() < 1e-9 {
            return None;
        }
        Some([0, 1, 2].map(|k| (c23[k] * r1 + c31[k] * r2 + c12[k] * r3) / det))
    }
}

/// What a hole is, as the dialog asks it.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleChoice {
    pub kind: HoleKind,
    /// A simple hole's diameter, as typed.
    pub diameter: String,
    /// A clearance hole's screw and how it fits.
    pub screw: String,
    pub fit: &'static str,
    pub seat: Seat,
    pub seat_diameter: String,
    pub seat_depth: String,
    pub seat_angle: String,
    pub termination: Termination,
    pub depth: String,
    /// How it's placed, and what that's picked.
    pub placing: Placing,
    pub linear: Linear,
    /// Round an edge: its centre and axis.
    pub concentric: Option<([f64; 3], [f64; 3])>,
    /// Which way it goes, for a frame of its own: against the outward normal unless turned.
    pub turned: bool,
    /// The face it goes to, as a line names it.
    pub surface: String,
    /// The drill's point: none is flat.
    pub point: Option<String>,
    /// What the seat's sizes were last filled from: another screw or seat fills them again.
    pub seat_from: String,
}

impl Default for HoleChoice {
    fn default() -> HoleChoice {
        HoleChoice {
            kind: HoleKind::Simple,
            diameter: "5".to_string(),
            screw: "M6".to_string(),
            fit: "normal",
            seat: Seat::None,
            seat_diameter: String::new(),
            seat_depth: String::new(),
            seat_angle: String::new(),
            termination: Termination::Through,
            depth: "10".to_string(),
            placing: Placing::Sketch,
            linear: Linear::default(),
            concentric: None,
            turned: false,
            surface: String::new(),
            point: None,
            seat_from: String::new(),
        }
    }
}

/// The kernel's screw row with this size and pitch, if it has one, and whose tables it follows.
fn screw_of(major: f64, pitch: f64) -> Option<&'static forge_model::standard::Screw> {
    forge_model::standard::SCREWS
        .iter()
        .find(|s| (s.size - major).abs() < 1e-9 && (s.pitch - pitch).abs() < 1e-9)
}

/// A tapped hole's drill: the table's where the size is in it; else worked out, and it says so.
fn tap_drill(t: &forge_doc::threads::Thread) -> (f64, &'static str) {
    match screw_of(t.major, t.pitch) {
        Some(s) => (s.tap[s.default_convention() as usize], "from the table"),
        None => ((t.major - t.pitch).max(0.1), "calculated: major less pitch"),
    }
}

/// A hole's fields, in the order its line reads them.
fn hole_fields(place: String) -> Vec<(String, String)> {
    ["Diameter", "Points", "Seat", "Depth", "Point", "Thread", "Class"]
        .into_iter()
        .map(|l| {
            (
                l.to_string(),
                if l == "Points" {
                    place.clone()
                } else {
                    String::new()
                },
            )
        })
        .collect()
}

/// Screw sizes a clearance hole can be for, from the kernel's table.
fn screws() -> Vec<&'static str> {
    forge_model::standard::SCREWS.iter().map(|s| s.name).collect()
}

/// A thread from the catalogue: type, size, designation, class; and which way it's gauged.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreadChoice {
    pub family: String,
    pub size: String,
    pub designation: String,
    pub class: String,
    /// Cut in a bore, not on a rod: which classes it has.
    pub internal: bool,
}

impl Default for ThreadChoice {
    fn default() -> ThreadChoice {
        ThreadChoice {
            family: "ISO Metric".to_string(),
            size: String::new(),
            designation: String::new(),
            class: String::new(),
            internal: false,
        }
    }
}

impl ThreadChoice {
    fn entry(&self) -> Option<&'static forge_doc::threads::Thread> {
        forge_doc::threads::designations(&self.family, &self.size)
            .into_iter()
            .find(|t| t.designation == self.designation)
    }

    fn classes(&self) -> &'static [&'static str] {
        self.entry()
            .map_or(&[], |t| if self.internal { t.internal } else { t.external })
    }
}

/// Does this kind's line start with how it combines with the part?
fn has_operation(kind: &str) -> bool {
    matches!(kind, "extrude" | "revolve" | "sweep" | "coil" | "loft")
}

impl Dialog {
    pub fn new_feature(kind: &'static str) -> Dialog {
        let (line, pick) = panel::template(kind);
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut fields = panel::fields(kind, &words);
        if let Some((label, _)) = pick
            && !fields.iter().any(|f| f.0 == label)
        {
            fields.push((label.to_string(), String::new()));
        }
        let mut queue: Vec<(String, &'static str)> = Vec::new();
        // the line reads in their order: size, where, how far
        match kind {
            "sweep" => {
                fields = vec![("Profile".into(), String::new()), ("Path".into(), String::new())];
                queue.push(("Path".to_string(), "Pick the sketch to sweep along"));
            }
            "loft" => fields = vec![("Profiles".into(), String::new())],
            // what it turns and about what are the person's to pick; all the way round to start
            "revolve" => {
                fields = vec![
                    ("Angle".into(), "360".into()),
                    ("Angle 2".into(), "90".into()),
                    ("Axis".into(), String::new()),
                ];
            }
            // exact by default: only what's picked, unless told to follow the tangent chain
            "fillet" => {
                fields = vec![
                    ("Radius".into(), "2".into()),
                    ("Only".into(), "only".into()),
                    ("Edges".into(), String::new()),
                ];
            }
            "rib" => {
                fields = vec![
                    ("Thickness".into(), "2".into()),
                    ("Flip".into(), String::new()),
                    ("Path".into(), String::new()),
                ];
            }
            "coil" => {
                fields = vec![
                    ("Profile".into(), String::new()),
                    ("Axis".into(), String::new()),
                    ("Pitch".into(), "8".into()),
                    ("Turns".into(), "4".into()),
                ];
                queue.push(("Axis".to_string(), "Pick a line of its sketch to turn about"));
            }
            "pattern" => {
                fields = vec![
                    ("Features".into(), String::new()),
                    ("Direction".into(), "lin 1 0 0".into()),
                    ("Count".into(), "3".into()),
                    ("Spacing".into(), "20".into()),
                ];
            }
            "combine" => {
                fields = vec![
                    ("Operation".into(), "add".into()),
                    ("Target".into(), String::new()),
                    ("Tools".into(), String::new()),
                ];
                queue.push(("Tools".to_string(), "Pick the bodies to combine with it"));
            }
            "plane" => {
                fields = vec![
                    ("Reference".into(), String::new()),
                    ("Offset".into(), "10".into()),
                ];
            }
            "axis" | "point" => fields = vec![("Reference".into(), String::new())],
            "hole" => {
                fields = hole_fields(String::new());
                fields[0].1 = "5".to_string();
                fields[3].1 = "thru".to_string();
            }
            "thread" => {
                fields = vec![
                    ("Size".to_string(), String::new()),
                    ("Cylinder".to_string(), String::new()),
                    ("Length".to_string(), "full".to_string()),
                    ("Hand".to_string(), String::new()),
                    ("Class".to_string(), String::new()),
                ];
            }
            _ => {}
        }
        Dialog {
            target: DialogTarget::New(kind),
            kind: kind.to_string(),
            title: panel::kind_word(kind),
            fields,
            pick: pick.map(|(l, why)| (l.to_string(), why)),
            queue,
            side_settled: false,
            thread: ThreadChoice::default(),
            hole: None,
            sent: None,
            seq: 0,
            error: None,
            awaiting: false,
            units: "mm",
            params: Vec::new(),
            first: Vec::new(),
            extent: Extent::Distance,
            toward: Toward::Forward,
            surface: String::new(),
            again: false,
            full: true,
            way: Way::Forward,
        }
    }

    pub fn edit(id: ItemId, kind: &str, title: String, fields: Vec<(String, String)>) -> Dialog {
        Dialog {
            target: DialogTarget::Edit(id),
            kind: kind.to_string(),
            title,
            fields,
            pick: None,
            sent: None,
            seq: 0,
            error: None,
            awaiting: false,
            units: "mm",
            params: Vec::new(),
            first: Vec::new(),
            extent: Extent::Distance,
            toward: Toward::Forward,
            surface: String::new(),
            again: false,
            queue: Vec::new(),
            side_settled: false,
            thread: ThreadChoice::default(),
            hole: None,
            full: true,
            way: Way::Forward,
        }
    }

    /// Its numbers as a person reads them in the document's unit.
    pub fn in_units(mut self, units: &'static str, params: Vec<String>) -> Dialog {
        self.units = units;
        self.params = params;
        if has_operation(&self.kind) && !self.fields.iter().any(|f| f.0 == "Operation") {
            self.fields.insert(0, ("Operation".to_string(), String::new()));
        }
        self.read_extent();
        if self.kind == "revolve" && matches!(self.target, DialogTarget::Edit(_)) {
            self.read_revolve();
        }
        // an existing fillet reads back whether it follows the chain
        if self.kind == "fillet" && matches!(self.target, DialogTarget::Edit(_)) {
            let at = self.fields.iter().position(|f| f.0 == "Edges");
            let only = at.is_some_and(|k| self.fields[k].1.starts_with("only "));
            if let Some(k) = at {
                if only {
                    self.fields[k].1 = self.fields[k].1["only ".len()..].to_string();
                }
                self.fields.insert(
                    k,
                    ("Only".to_string(), if only { "only" } else { "" }.to_string()),
                );
            }
        }
        if self.kind == "hole" {
            self.first.clear();
            self.thread.internal = true;
            match self.target {
                DialogTarget::Edit(_) => self.read_hole(),
                DialogTarget::New(_) => {
                    self.choose_thread(Some("ISO Metric"), Some("M6"), None, None);
                    self.hole = Some(HoleChoice::default());
                    self.settle_hole();
                }
            }
            return self;
        }
        if self.kind == "thread" && matches!(self.target, DialogTarget::Edit(_)) {
            self.read_thread();
        }
        for (label, text) in &mut self.fields {
            if let Some(kind) = panel::field_kind(label) {
                let file = std::mem::take(text);
                *text = forge_doc::units::field_show(&file, kind, units);
                self.first.push((label.clone(), text.clone(), file));
            }
        }
        self
    }

    /// An extrude read from its file: how its distance is taken, apart from the number.
    fn read_extent(&mut self) {
        if self.kind != "extrude" {
            return;
        }
        let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Distance") else {
            return;
        };
        let w: Vec<&str> = f.1.split_whitespace().collect();
        let (extent, toward, rest) = match w[..] {
            ["sym", n] => (Extent::Distance, Toward::Both, n.to_string()),
            ["thru"] => (Extent::Through, Toward::Forward, "10".to_string()),
            ["thru", "back"] => (Extent::Through, Toward::Back, "10".to_string()),
            ["thru", "both"] => (Extent::Through, Toward::Both, "10".to_string()),
            ["next"] => (Extent::Next, Toward::Forward, "10".to_string()),
            ["next", "back"] => (Extent::Next, Toward::Back, "10".to_string()),
            [n] if n.starts_with('-') => (Extent::Distance, Toward::Back, n[1..].to_string()),
            _ => return,
        };
        f.1 = rest;
        self.extent = extent;
        self.toward = toward;
    }

    /// A rib refused on the side it started on, with nobody having chosen: the other side's
    /// where it belongs, if either. True if it turned.
    pub fn try_other_side(&mut self) -> bool {
        if self.kind != "rib" || self.side_settled {
            return false;
        }
        self.side_settled = true;
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Flip") {
            f.1 = if f.1.is_empty() {
                "flip".to_string()
            } else {
                String::new()
            };
            self.error = None;
            return true;
        }
        false
    }

    /// Sets what a selection has, and goes on to the next one wanted.
    pub fn take_pick(&mut self, label: &str, text: String) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == label) {
            f.1 = text;
        }
        self.next_pick();
    }

    fn next_pick(&mut self) {
        let done = self
            .pick
            .as_ref()
            .is_some_and(|(l, _)| self.fields.iter().any(|f| f.0 == *l && !f.1.trim().is_empty()));
        if done && !self.queue.is_empty() {
            self.pick = Some(self.queue.remove(0));
        }
    }

    /// The selection a row of it says should take clicks now.
    pub fn activate(&mut self, label: &str) {
        let active = self.pick.as_ref().is_some_and(|(l, _)| l == label);
        if let (false, Some(k)) = (active, self.queue.iter().position(|q| q.0 == label)) {
            let next = self.queue.remove(k);
            if let Some(was) = self.pick.replace(next) {
                self.queue.push(was);
            }
        }
    }

    /// A field's text, as the file would have it.
    fn file_text(&self, label: &str, text: &str) -> String {
        match (self.kind.as_str(), label) {
            ("hole", _) if self.hole.is_some() => return text.to_string(),
            // a rib names its sketch with no word before it
            ("rib", "Path") => return text.trim_start_matches("path ").to_string(),
            ("plane", "Reference") if !text.trim().is_empty() => return format!("off {}", text.trim()),
            ("coil", "Pitch") if !text.trim_start().starts_with("pitch") => {
                return format!("pitch {}", self.number_text(label, text));
            }
            ("coil", "Turns") if !text.trim_start().starts_with("turns") => {
                return format!("turns {}", text.trim());
            }
            _ => {}
        }
        if label == "Distance" && self.kind == "extrude" {
            let number = self.number_text(label, text);
            if self.extent == Extent::Surface {
                return format!("to {}", self.surface);
            }
            return match (self.extent, self.toward) {
                (Extent::Distance, Toward::Forward) => number,
                (Extent::Distance, Toward::Back) => format!("-{number}"),
                (Extent::Distance, Toward::Both) => format!("sym {number}"),
                (Extent::Through, Toward::Forward) => "thru".to_string(),
                (Extent::Through, Toward::Back) => "thru back".to_string(),
                (Extent::Through, Toward::Both) => "thru both".to_string(),
                (Extent::Next, Toward::Back) => "next back".to_string(),
                (Extent::Next, _) => "next".to_string(),
                (Extent::Surface, _) => String::new(),
            };
        }
        self.number_text(label, text)
    }

    fn number_text(&self, label: &str, text: &str) -> String {
        let Some(kind) = panel::field_kind(label) else {
            return text.to_string();
        };
        if let Some(f) = self.first.iter().find(|f| f.0 == label && f.1 == text) {
            return f.2.clone();
        }
        let is_param = |w: &str| self.params.iter().any(|p| p == w);
        forge_doc::units::field_file(text, kind, self.units, &is_param)
    }

    /// It reads this sketch, with no plane or profile of its own.
    pub fn on_sketch(&mut self, name: &str) {
        if self.kind == "revolve" {
            self.fields.retain(|f| f.0 != "Plane" && f.0 != "Profile");
            let at = self.fields.iter().position(|f| f.0 == "Axis").unwrap_or_else(|| {
                self.fields.push(("Axis".to_string(), String::new()));
                self.fields.len() - 1
            });
            self.fields.insert(at, ("Profile".to_string(), name.to_string()));
            let axis_empty = self.fields.iter().any(|f| f.0 == "Axis" && f.1.is_empty());
            let taking = self.pick.as_ref().map(|p| p.0.clone());
            if axis_empty && taking.as_deref() != Some("Axis") && !self.queue.iter().any(|q| q.0 == "Axis") {
                let ask = ("Axis".to_string(), AXIS_HINT);
                match taking.as_deref() {
                    Some("Profile") if name.is_empty() => self.queue.push(ask),
                    _ => self.pick = Some(ask),
                }
            }
            return;
        }
        self.fields
            .retain(|f| f.0 != "Plane" && f.0 != "Profile" && f.0 != "Axis");
        self.fields.push(("Profile".to_string(), name.to_string()));
    }

    /// A revolve on a sketch, as its line reads: `revolve [op] [TURN] SKETCH [axis ..] [REGION]`.
    fn revolve_line(&self) -> Option<String> {
        if self.kind != "revolve" || self.fields.iter().any(|f| f.0 == "Plane") {
            return None;
        }
        let get = |l: &str| self.fields.iter().find(|f| f.0 == l).map_or("", |f| f.1.trim());
        let profile = get("Profile");
        let (sketch, region) = profile.split_once(' ').unwrap_or((profile, ""));
        let a = self.number_text("Angle", get("Angle"));
        let turn = match (self.full, self.way) {
            (true, _) => String::new(),
            (false, Way::Forward) => a,
            (false, Way::Back) => format!("two 0 {a}"),
            (false, Way::Symmetric) => format!("sym {a}"),
            (false, Way::Two) => format!("two {a} {}", self.number_text("Angle 2", get("Angle 2"))),
        };
        let words = [get("Operation"), turn.as_str(), sketch, get("Axis"), region];
        Some(
            std::iter::once("revolve")
                .chain(words.into_iter().filter(|w| !w.is_empty()))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    /// An existing revolve on a sketch: its turn, profile and axis read back into their fields.
    fn read_revolve(&mut self) {
        let words: Vec<String> = self
            .fields
            .iter()
            .flat_map(|f| f.1.split_whitespace().map(str::to_string).collect::<Vec<_>>())
            .collect();
        let mut k = 0;
        let op = if words
            .first()
            .is_some_and(|w| ["add", "cut", "keep", "new"].contains(&w.as_str()))
        {
            k += 1;
            words[0].clone()
        } else {
            String::new()
        };
        let num = |w: Option<&String>| {
            w.is_some_and(|w| w.parse::<f64>().is_ok() || !w.chars().next().is_some_and(char::is_alphabetic))
        };
        let (mut full, mut way, mut a, mut b) = (true, Way::Forward, "360".to_string(), "90".to_string());
        match words.get(k).map(String::as_str) {
            Some("sym") => {
                (full, way, a) = (
                    false,
                    Way::Symmetric,
                    words.get(k + 1).cloned().unwrap_or_default(),
                );
                k += 2;
            }
            Some("two") => {
                let (x, y) = (
                    words.get(k + 1).cloned().unwrap_or_default(),
                    words.get(k + 2).cloned().unwrap_or_default(),
                );
                if x == "0" {
                    (full, way, a) = (false, Way::Back, y);
                } else {
                    (full, way, a, b) = (false, Way::Two, x, y);
                }
                k += 3;
            }
            _ if num(words.get(k)) => {
                (full, a) = (false, words[k].clone());
                k += 1;
            }
            _ => {}
        }
        let Some(sketch) = words.get(k).cloned() else {
            return;
        };
        if panel::fields("revolve", &[sketch.as_str()])
            .iter()
            .any(|f| f.0 == "Plane" || f.0 == "Profile")
        {
            return;
        }
        k += 1;
        let mut axis = String::new();
        if words.get(k).is_some_and(|w| w == "axis") {
            let n = if words.get(k + 1).is_some_and(|w| w.parse::<f64>().is_ok()) {
                5
            } else {
                2
            };
            axis = words[k..(k + n).min(words.len())].join(" ");
            k += n;
        }
        let region = words.get(k..).map_or(String::new(), |w| w.join(" "));
        let profile = if region.is_empty() {
            sketch
        } else {
            format!("{sketch} {region}")
        };
        self.full = full;
        self.way = way;
        self.fields = vec![
            ("Operation".to_string(), op),
            ("Angle".to_string(), a),
            ("Angle 2".to_string(), b),
            ("Profile".to_string(), profile),
            ("Axis".to_string(), axis),
        ];
    }

    /// The feature's line, from its fields.
    pub fn line(&self) -> String {
        if let Some(l) = self.revolve_line() {
            return l;
        }
        let mut out = self.kind.clone();
        for (label, text) in &self.fields {
            let file = self.file_text(label, text);
            let t = file.trim();
            if !t.is_empty() {
                out.push(' ');
                out.push_str(t);
            }
        }
        out
    }

    /// A field that must come from the view is still empty.
    pub fn waiting_for_pick(&self) -> bool {
        self.thread_blocked().is_some()
            || self.seat_wanting()
            || self
                .hole
                .as_ref()
                .is_some_and(|h| h.termination == Termination::Surface && h.surface.is_empty())
            || (self.kind == "extrude" && self.extent == Extent::Surface && self.surface.is_empty())
            || (self.kind == "revolve" && self.fields.iter().any(|f| f.0 == "Axis" && f.1.trim().is_empty()))
            || (self.kind == "thread"
                && self.fields.iter().any(|f| f.0 == "Size" && f.1.is_empty())
                && matches!(self.target, DialogTarget::New(_)))
            || self
                .pick
                .as_ref()
                .is_some_and(|(l, _)| self.fields.iter().any(|f| f.0 == *l && f.1.trim().is_empty()))
            || self
                .queue
                .iter()
                .any(|(l, _)| self.fields.iter().any(|f| f.0 == *l && f.1.trim().is_empty()))
            || (self.kind == "loft"
                && self
                    .fields
                    .iter()
                    .any(|f| f.0 == "Profiles" && f.1.matches("profile").count() < 2))
    }

    /// The feature's profile as a person reads it: its sketch's dimensions, if it's written inline.
    pub fn sketch_dims(&self) -> Vec<panel::Shown> {
        let words: Vec<String> = self
            .fields
            .iter()
            .flat_map(|f| f.1.split_whitespace().map(str::to_string))
            .collect();
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        panel::inline_profile(&self.kind, &words).map_or(Vec::new(), |p| p.shown)
    }

    /// What's selected in the view, as the part file names it, goes to the field taking picks.
    /// A hole's point: on or off the list; another sketch's starts the list again. `on` says which
    /// way the sketch faces into the part.
    pub fn toggle_point(&mut self, sketch: &str, point: &str, on: &str) {
        let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Points") else {
            return;
        };
        // once flipped, it stays flipped
        let on = if f.1.starts_with("-on ") {
            "-on"
        } else if f.1.starts_with("on ") {
            "on"
        } else {
            on
        };
        let head = format!("{on} {sketch} pts");
        let mut points: Vec<String> = match f.1.strip_prefix(&head) {
            Some(rest) => rest.split_whitespace().map(str::to_string).collect(),
            None => Vec::new(),
        };
        if let Some(k) = points.iter().position(|p| p == point) {
            points.remove(k);
        } else {
            points.push(point.to_string());
        }
        f.1 = if points.is_empty() {
            String::new()
        } else {
            format!("{head} {}", points.join(" "))
        };
    }

    /// A seat's sizes from the fastener's row, when there's a fastener in the table and the seat or
    /// the fastener has just changed. With none, they're the person's to type.
    fn fill_seat(&mut self) {
        let screw = match self.hole.as_ref().map(|h| h.kind) {
            Some(HoleKind::Clearance) => self
                .hole
                .as_ref()
                .and_then(|h| forge_model::standard::screw(&h.screw)),
            Some(HoleKind::Tapped) => self.thread.entry().and_then(|t| screw_of(t.major, t.pitch)),
            _ => None,
        };
        let Some(h) = &mut self.hole else { return };
        let key = format!("{:?} {:?} {}", h.seat, h.kind, screw.map_or("", |s| s.name));
        if h.seat_from == key {
            return;
        }
        h.seat_from = key;
        let (mut d, mut deep, mut angle) = (String::new(), String::new(), String::new());
        if let Some(s) = screw {
            let at = s.default_convention() as usize;
            match h.seat {
                Seat::Counterbore => {
                    d = s.counterbore[at].0.to_string();
                    deep = s.counterbore[at].1.to_string();
                }
                // a spotface's as wide as the screw's counterbore; how deep is the person's
                Seat::Spotface => d = s.counterbore[at].0.to_string(),
                Seat::Countersink => {
                    if let Some((a, b)) = s.countersink {
                        d = a.to_string();
                        angle = (b * 180.0 / core::f64::consts::PI).round().to_string();
                    }
                }
                Seat::None => {}
            }
        }
        (h.seat_diameter, h.seat_depth, h.seat_angle) = (d, deep, angle);
    }

    /// Everything's been said that a line needs?
    fn seat_wanting(&self) -> bool {
        self.fields
            .iter()
            .any(|f| f.0 == "Seat" && f.1 == "needs-seat-size")
    }

    /// The hole's fields, from what's chosen. A typed length reads in the part's unit.
    pub fn settle_hole(&mut self) {
        self.fill_seat();
        let Some(h) = self.hole.clone() else { return };
        let is_param = |w: &str| self.params.iter().any(|p| p == w);
        let len =
            |t: &str| forge_doc::units::field_file(t, forge_doc::units::Kind::Length, self.units, &is_param);
        let tapped = self.thread.entry().filter(|_| h.kind == HoleKind::Tapped);
        let size = match h.kind {
            HoleKind::Simple => len(&h.diameter),
            HoleKind::Clearance => format!("{} {}", h.screw, h.fit),
            // the drill's the major less the pitch
            HoleKind::Tapped => tapped.map_or(String::new(), |t| tap_drill(t).0.to_string()),
        };
        let seat = match h.seat {
            Seat::None => String::new(),
            Seat::Counterbore | Seat::Spotface
                if h.seat_diameter.trim().is_empty() || h.seat_depth.trim().is_empty() =>
            {
                "needs-seat-size".to_string()
            }
            Seat::Countersink if h.seat_diameter.trim().is_empty() || h.seat_angle.trim().is_empty() => {
                "needs-seat-size".to_string()
            }
            Seat::Counterbore | Seat::Spotface => {
                format!("cb {} {}", len(&h.seat_diameter), len(&h.seat_depth))
            }
            Seat::Countersink => format!("cs {} {}", len(&h.seat_diameter), h.seat_angle.trim()),
        };
        let end = match h.termination {
            Termination::Distance => len(&h.depth),
            Termination::Through => "thru".to_string(),
            Termination::Next => "next".to_string(),
            Termination::Surface => format!("to {}", h.surface),
        };
        let frame = |at: [f64; 3], n: [f64; 3]| {
            let s = if h.turned { 1.0 } else { -1.0 };
            let z = n.map(|v| {
                let v = v * s;
                if v == 0.0 { 0.0 } else { v }
            });
            format!("z {} {} {} at {} {} {}", z[0], z[1], z[2], at[0], at[1], at[2])
        };
        let placed = match h.placing {
            Placing::Sketch => None,
            Placing::Concentric => Some(h.concentric.map_or(String::new(), |(c, n)| frame(c, n))),
            Placing::Linear => {
                let number = |t: &str| forge_doc::expr::eval(&len(t), &|_| None).map(|v| v.0).ok();
                let at = number(&h.linear.first)
                    .zip(number(&h.linear.second))
                    .and_then(|(a, b)| h.linear.place(a, b));
                let n = h.linear.face.map(|f| f.1);
                Some(at.zip(n).map_or(String::new(), |(at, n)| frame(at, n)))
            }
        };
        if let Some(p) = placed
            && let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Points")
        {
            f.1 = p;
        }
        let point = match (&h.point, h.termination) {
            (Some(a), Termination::Distance) => format!("pt {}", a.trim()),
            _ => String::new(),
        };
        let thread = tapped.map_or(String::new(), |t| match &t.spec {
            Some(s) => format!("thread {s}"),
            None => format!("thread {} {}", t.major, t.pitch),
        });
        let class = match (h.kind, self.thread.class.as_str()) {
            (HoleKind::Tapped, c) if !c.is_empty() && c != "-" => format!("class {c}"),
            _ => String::new(),
        };
        for f in &mut self.fields {
            f.1 = match f.0.as_str() {
                "Diameter" => size.clone(),
                "Seat" => seat.clone(),
                "Depth" => end.clone(),
                "Point" => point.clone(),
                "Thread" => thread.clone(),
                "Class" => class.clone(),
                _ => continue,
            };
        }
    }

    /// A hole's line read back into its choices, to edit it.
    fn read_hole(&mut self) {
        let line: Vec<String> = self
            .fields
            .iter()
            .flat_map(|f| forge_doc::text::tokens(&f.1))
            .collect();
        let mut h = HoleChoice::default();
        let mut k = 0;
        let word = |k: usize| line.get(k).map_or("", |w| w.as_str());
        if forge_model::standard::screw(word(0)).is_some() {
            h.kind = HoleKind::Clearance;
            h.screw = word(0).to_string();
            k = 1;
            if matches!(word(k), "iso" | "ansi") {
                k += 1;
            }
            match word(k) {
                "close" => (h.fit, k) = ("close", k + 1),
                "normal" => (h.fit, k) = ("normal", k + 1),
                "loose" => (h.fit, k) = ("loose", k + 1),
                _ => {}
            }
        } else if !word(0).is_empty() {
            h.diameter = word(0).to_string();
            k = 1;
        }
        // where: a frame or a sketch's points, up to the seat or the end
        let from = k;
        while k < line.len()
            && !matches!(word(k), "cb" | "cs" | "thru" | "next" | "to")
            && !(k > from
                && word(from) != "on"
                && word(from) != "-on"
                && word(k - 1) != "at"
                && k >= from + if word(from + 1) == "at" { 5 } else { 1 })
        {
            if (word(from) == "on" || word(from) == "-on") && k > from + 2 && word(k).parse::<f64>().is_ok() {
                break;
            }
            k += 1;
        }
        let place = line[from..k].join(" ");
        match word(k) {
            "cb" => {
                h.seat = Seat::Counterbore;
                h.seat_diameter = word(k + 1).to_string();
                h.seat_depth = word(k + 2).to_string();
                k += 3;
            }
            "cs" => {
                h.seat = Seat::Countersink;
                h.seat_diameter = word(k + 1).to_string();
                h.seat_angle = word(k + 2).to_string();
                k += 3;
            }
            _ => {}
        }
        match word(k) {
            "thru" => {
                h.termination = Termination::Through;
                k += 1;
            }
            "next" => {
                h.termination = Termination::Next;
                k += 1;
            }
            "" => {}
            w => {
                h.termination = Termination::Distance;
                h.depth = w.to_string();
                k += 1;
            }
        }
        if word(k) == "pt" {
            h.point = Some(word(k + 1).to_string());
            k += 2;
        }
        if word(k) == "thread" {
            h.kind = HoleKind::Tapped;
            let spec = word(k + 1).to_string();
            let major = word(k + 1).parse::<f64>().ok();
            let pitch = word(k + 2).parse::<f64>().ok();
            let found = forge_doc::threads::catalogue().iter().find(|t| {
                t.spec.as_deref() == Some(spec.as_str())
                    || major
                        .zip(pitch)
                        .is_some_and(|(a, b)| t.major == a && t.pitch == b)
            });
            k += if major.is_some() { 3 } else { 2 };
            if let Some(t) = found {
                self.thread.family = t.family.to_string();
                self.thread.size.clone_from(&t.size);
                self.thread.designation.clone_from(&t.designation);
            }
        }
        self.thread.internal = true;
        if word(k) == "class" {
            self.thread.class = word(k + 1).to_string();
        }
        self.fields = hole_fields(place);
        self.hole = Some(h);
        self.settle_hole();
    }

    /// A thread's cylinder, and the size of the chosen type that fits it.
    pub fn thread_on(&mut self, on: String, diameter: f64, internal: bool) {
        self.thread.internal = internal;
        let fit = forge_doc::threads::fits(&self.thread.family, diameter, internal);
        if let Some(t) = fit {
            self.thread.size.clone_from(&t.size);
            self.thread.designation.clone_from(&t.designation);
        }
        self.settle_thread();
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == "Cylinder") {
            f.1 = on;
        }
    }

    /// Choose from the catalogue; what's below what changed comes to the first it has.
    pub fn choose_thread(
        &mut self,
        family: Option<&str>,
        size: Option<&str>,
        designation: Option<&str>,
        class: Option<&str>,
    ) {
        if let Some(f) = family {
            // the nearest it has to what was chosen, or its first
            let was = self.thread.entry().map(|t| t.major);
            self.thread.family = f.to_string();
            let near = was
                .and_then(|d| forge_doc::threads::fits(f, d, self.thread.internal))
                .or_else(|| forge_doc::threads::catalogue().iter().find(|t| t.family == f));
            self.thread.size = near.map_or(String::new(), |t| t.size.clone());
            self.thread.designation = near.map_or(String::new(), |t| t.designation.clone());
        }
        if let Some(s) = size {
            self.thread.size = s.to_string();
            self.thread.designation = forge_doc::threads::designations(&self.thread.family, s)
                .first()
                .map_or(String::new(), |t| t.designation.clone());
        }
        if let Some(d) = designation {
            self.thread.designation = d.to_string();
        }
        if let Some(c) = class {
            self.thread.class = c.to_string();
        }
        self.settle_thread();
    }

    /// The fields the line reads, from what's chosen.
    fn settle_thread(&mut self) {
        if !self.thread.classes().iter().any(|c| *c == self.thread.class) {
            self.thread.class = self
                .thread
                .classes()
                .first()
                .map_or(String::new(), |c| c.to_string());
        }
        let spec = self
            .thread
            .entry()
            .and_then(|t| t.spec.clone())
            .unwrap_or_default();
        let class = match self.thread.class.as_str() {
            "" | "-" => String::new(),
            c => format!("class {c}"),
        };
        for f in &mut self.fields {
            match f.0.as_str() {
                "Size" => f.1.clone_from(&spec),
                "Class" => f.1.clone_from(&class),
                _ => {}
            }
        }
    }

    /// What's chosen can't be cut yet: why.
    pub fn thread_blocked(&self) -> Option<String> {
        if self.kind != "thread" {
            return None;
        }
        self.thread.entry().filter(|t| !t.modeled).map(|t| {
            format!(
                "{} has its own form, which can't be cut yet: only 60 degree V threads can",
                t.family
            )
        })
    }

    /// A thread's line read back into its choice, to edit it.
    fn read_thread(&mut self) {
        let line: Vec<String> = self
            .fields
            .iter()
            .flat_map(|f| f.1.split_whitespace().map(str::to_string).collect::<Vec<_>>())
            .collect();
        let Some(first) = line.first() else { return };
        let (mut cylinder, mut length, mut hand, mut class) =
            (String::new(), String::new(), String::new(), String::new());
        let found = forge_doc::threads::catalogue().iter().find(|t| {
            t.spec.as_deref() == Some(first.as_str())
                || t.spec
                    .as_deref()
                    .is_some_and(|s| s.split('x').next() == Some(first.as_str()) && t.pitch > 0.0)
        });
        let mut k = 1;
        if first == "hole" {
            cylinder = line[..2.min(line.len())].join(" ");
            k = 2;
        } else if line.get(1).is_some_and(|w| w == "face") {
            cylinder = line[1..4.min(line.len())].join(" ");
            k = 4;
        }
        while k < line.len() {
            match line[k].as_str() {
                "left" => hand = "left".to_string(),
                "class" => {
                    class = line.get(k + 1).cloned().unwrap_or_default();
                    k += 1;
                }
                w if length.is_empty() => length = w.to_string(),
                _ => {}
            }
            k += 1;
        }
        if let Some(t) = found {
            self.thread = ThreadChoice {
                family: t.family.to_string(),
                size: t.size.clone(),
                designation: t.designation.clone(),
                class: class.clone(),
                internal: false,
            };
        }
        let size = if found.is_some() {
            String::new()
        } else {
            first.clone()
        };
        self.fields = vec![
            ("Size".to_string(), size),
            ("Cylinder".to_string(), cylinder),
            (
                "Length".to_string(),
                if length.is_empty() {
                    "full".to_string()
                } else {
                    length
                },
            ),
            ("Hand".to_string(), hand),
            (
                "Class".to_string(),
                if class.is_empty() {
                    String::new()
                } else {
                    format!("class {class}")
                },
            ),
        ];
        if found.is_some() {
            if self.thread.internal || !self.thread.classes().iter().any(|c| *c == class) {
                self.thread.internal = true;
            }
            self.settle_thread();
        }
    }

    pub fn picked(&mut self, refs: &[String]) {
        let Some((label, _)) = &self.pick else { return };
        let label = label.clone();
        if let Some(f) = self.fields.iter_mut().find(|f| f.0 == label) {
            f.1 = refs.join(" ");
        }
    }
}

/// How wide a dialog's body is.
pub(super) const DIALOG_W: f32 = 284.0;

pub(super) fn heading(ui: &mut egui::Ui, text: &str, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 18.0), Sense::hover());
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), semi(12.0), t.ink2));
    let right = r.left() + g.size().x + 8.0;
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y * 0.5), g, t.ink2);
    ui.painter()
        .hline(right..=r.right(), r.center().y, Stroke::new(1.0, t.line));
}

/// A row of big toggles with their words under them; which was clicked.
pub(super) fn toggle_row(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    items: &[(&str, &str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 52.0), Sense::hover());
    let w = (DIALOG_W - 4.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
    let mut chose = None;
    for (k, (icon, word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(row.left_top() + vec2(k as f32 * (w + 4.0), 0.0), vec2(w, 52.0));
        let resp = ui.interact(r, ui.scope_id().with((name, k)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, *on, *word));
        let fill = if *on {
            t.on
        } else if resp.hovered() {
            t.hover
        } else {
            t.raised
        };
        let p = ui.painter();
        p.rect_filled(r, 4.0, fill);
        let edge = if *on { t.on_ink } else { t.line };
        p.rect_stroke(r, 4.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let ink = if *on { t.on_ink } else { t.ink };
        super::icons::draw(p, pos2(r.center().x - 11.0, r.top() + 6.0), 22.0, icon, 1.5, ink);
        p.text(
            pos2(r.center().x, r.bottom() - 11.0),
            Align2::CENTER_CENTER,
            *word,
            sans(11.0),
            ink,
        );
        places.push((format!("{name}:{word}"), r));
        if resp.clicked() {
            chose = Some(k);
        }
    }
    chose
}

type ThreadChange = (Option<String>, Option<String>, Option<String>, Option<String>);

/// One of a thread's dropdowns.
pub(super) fn dropdown(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    label: &str,
    now: &str,
    items: &[(String, String, bool)],
) -> Option<String> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 28.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let at = Rect::from_min_size(
        pos2(row.left() + FIELD_AT, row.top() + 1.0),
        vec2(DIALOG_W - FIELD_AT, 26.0),
    );
    let mut chose = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
        egui::ComboBox::from_id_salt(("thread", label))
            .selected_text(now)
            .width(DIALOG_W - FIELD_AT)
            .height(320.0)
            .show_ui(ui, |ui| {
                for (value, words, greyed) in items {
                    let text = egui::RichText::new(words);
                    let text = if *greyed { text.color(t.ink2) } else { text };
                    if ui.selectable_label(value == now, text).clicked() {
                        chose = Some(value.clone());
                    }
                }
            });
    });
    chose
}

/// Type, Size, Designation, Class, in that order.
fn thread_rows(ui: &mut egui::Ui, c: &ThreadChoice, t: &super::theme::Theme, out: &mut ThreadChange) {
    use forge_doc::threads as th;
    let families: Vec<(String, String, bool)> = th::families()
        .into_iter()
        .map(|f| {
            let cut = th::catalogue().iter().any(|x| x.family == f && x.modeled);
            let words = match (cut, th::checked(f)) {
                (true, true) => f.to_string(),
                (true, false) => format!("{f} (sizes not all checked)"),
                (false, true) => format!("{f} (can't be cut yet)"),
                (false, false) => format!("{f} (can't be cut yet; sizes not all checked)"),
            };
            (f.to_string(), words, !cut)
        })
        .collect();
    out.0 = dropdown(ui, t, "Type", &c.family, &families);
    let sizes: Vec<(String, String, bool)> = th::sizes(&c.family)
        .into_iter()
        .map(|s| (s.to_string(), s.to_string(), false))
        .collect();
    out.1 = dropdown(ui, t, "Size", &c.size, &sizes);
    let names: Vec<(String, String, bool)> = th::designations(&c.family, &c.size)
        .into_iter()
        .map(|x| (x.designation.clone(), x.designation.clone(), false))
        .collect();
    out.2 = dropdown(ui, t, "Designation", &c.designation, &names);
    let classes: Vec<(String, String, bool)> = c
        .classes()
        .iter()
        .map(|k| (k.to_string(), k.to_string(), false))
        .collect();
    out.3 = dropdown(ui, t, "Class", &c.class, &classes);
}

/// A row of worded toggles, side by side.
pub(super) fn text_toggles(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    items: &[(&str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 26.0), Sense::hover());
    let w = (DIALOG_W - 3.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
    let mut chose = None;
    for (k, (word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(row.left_top() + vec2(k as f32 * (w + 3.0), 0.0), vec2(w, 26.0));
        let resp = ui.interact(r, ui.scope_id().with((name, k)), Sense::click());
        let fill = if *on {
            t.on
        } else if resp.hovered() {
            t.hover
        } else {
            t.raised
        };
        let p = ui.painter();
        p.rect_filled(r, 3.0, fill);
        p.rect_stroke(
            r,
            3.0,
            Stroke::new(1.0, if *on { t.on_ink } else { t.line }),
            egui::StrokeKind::Inside,
        );
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            *word,
            sans(11.0),
            if *on { t.on_ink } else { t.ink },
        );
        places.push((format!("{name}:{word}"), r));
        if resp.clicked() {
            chose = Some(k);
        }
    }
    chose
}

/// The hole in section, as a sketch of what's chosen: not to scale.
fn hole_section(ui: &mut egui::Ui, h: &HoleChoice, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 92.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 3.0, t.field);
    let (top, bottom, mid) = (r.top() + 10.0, r.bottom() - 6.0, r.center().x);
    let half = 12.0;
    let end = if h.termination == Termination::Distance {
        bottom - 22.0
    } else {
        bottom
    };
    let metal = t.ink2.gamma_multiply(0.45);
    let (left, right) = (r.left() + 60.0, r.right() - 60.0);
    p.rect_filled(
        Rect::from_min_max(pos2(left, top), pos2(right, bottom)),
        0.0,
        metal,
    );
    let hole = t.field;
    p.rect_filled(
        Rect::from_min_max(pos2(mid - half, top), pos2(mid + half, end)),
        0.0,
        hole,
    );
    match h.seat {
        Seat::Counterbore | Seat::Spotface => {
            let deep = if h.seat == Seat::Spotface { 5.0 } else { 18.0 };
            p.rect_filled(
                Rect::from_min_max(pos2(mid - 24.0, top), pos2(mid + 24.0, top + deep)),
                0.0,
                hole,
            );
        }
        Seat::Countersink => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(mid - 24.0, top),
                    pos2(mid + 24.0, top),
                    pos2(mid + half, top + 12.0),
                    pos2(mid - half, top + 12.0),
                ],
                hole,
                Stroke::NONE,
            ));
        }
        Seat::None => {}
    }
    if h.termination == Termination::Distance && h.point.is_some() {
        p.add(egui::Shape::convex_polygon(
            vec![
                pos2(mid - half, end),
                pos2(mid + half, end),
                pos2(mid, end + 12.0),
            ],
            hole,
            Stroke::NONE,
        ));
    }
    if h.kind == HoleKind::Tapped {
        let mut y = top + 20.0;
        while y < end - 2.0 {
            for x in [mid - half - 3.0, mid + half + 3.0] {
                p.line_segment([pos2(x - 3.0, y), pos2(x + 3.0, y)], Stroke::new(1.0, t.ink));
            }
            y += 6.0;
        }
    }
}

/// A row of small icon buttons after its word: for options that don't earn a big toggle.
fn small_row(
    ui: &mut egui::Ui,
    t: &super::theme::Theme,
    label: &str,
    items: &[(&str, &str, bool)],
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 28.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let mut chose = None;
    for (k, (icon, word, on)) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(row.left() + FIELD_AT + k as f32 * 30.0, row.top()),
            vec2(28.0, 28.0),
        );
        let resp = ui.interact(r, ui.scope_id().with((name, k)), Sense::click());
        let fill = if *on {
            t.on
        } else if resp.hovered() {
            t.hover
        } else {
            t.raised
        };
        let p = ui.painter();
        p.rect_filled(r, 3.0, fill);
        let edge = if *on { t.on_ink } else { t.line };
        p.rect_stroke(r, 3.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let ink = if *on { t.on_ink } else { t.ink };
        super::icons::draw(p, r.min + vec2(5.0, 5.0), 18.0, icon, 1.5, ink);
        places.push((format!("{name}:{word}"), r));
        if resp.on_hover_text(*word).clicked() {
            chose = Some(k);
        }
    }
    chose
}

/// A selection: what it wants, how many it has, and whether it's what the view's clicks go to.
pub(super) fn pick_row(
    ui: &mut egui::Ui,
    label: &str,
    n: usize,
    waiting: bool,
    hint: &str,
    t: &super::theme::Theme,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(DIALOG_W, 34.0), Sense::click());
    let p = ui.painter();
    let (edge, fill) = if waiting {
        (t.focus, t.raised)
    } else if n > 0 {
        (t.accent, t.raised)
    } else {
        (t.line, t.raised)
    };
    p.rect_filled(r, 4.0, fill);
    p.rect_stroke(
        r,
        4.0,
        Stroke::new(if waiting { 2.0 } else { 1.0 }, edge),
        egui::StrokeKind::Inside,
    );
    super::icons::draw(p, r.left_top() + vec2(8.0, 9.0), 16.0, "select", 1.5, edge);
    p.text(
        pos2(r.left() + 32.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let state = if n == 0 {
        "None selected".to_string()
    } else {
        format!("{n} selected")
    };
    p.text(
        pos2(r.right() - 10.0, r.center().y),
        Align2::RIGHT_CENTER,
        state,
        sans(12.0),
        if n > 0 { t.ink } else { t.ink2 },
    );
    let clicked = resp.clicked();
    if !hint.is_empty() {
        let _ = resp.on_hover_text(hint);
    }
    places.push((format!("dialog:pick:{label}"), r));
    clicked
}

fn value_row(ui: &mut egui::Ui, name: &str, value: &str, t: &super::theme::Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, 22.0), Sense::hover());
    let p = ui.painter();
    p.text(
        pos2(r.left(), r.center().y),
        Align2::LEFT_CENTER,
        name,
        sans(12.0),
        t.ink2,
    );
    p.text(
        pos2(r.left() + FIELD_AT, r.center().y),
        Align2::LEFT_CENTER,
        value,
        mono(12.0),
        t.ink,
    );
}

pub(super) const FIELD_AT: f32 = 92.0;

/// Where a field's Measure... leaves the field's label for the shell to take.
pub(super) const MEASURE_INTO: &str = "measure-into-field";

/// A named value: its word, its field, its unit when what's typed has none.
fn field_row(
    ui: &mut egui::Ui,
    label: &str,
    text: &mut String,
    units: &str,
    t: &super::theme::Theme,
    places: &mut Vec<(String, Rect)>,
) {
    let (row, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
    ui.painter().text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.0),
        t.ink,
    );
    let field = Rect::from_min_size(
        pos2(row.left() + FIELD_AT, row.top() + 2.0),
        vec2(DIALOG_W - FIELD_AT, 26.0),
    );
    let edit = ui.put(
        field,
        egui::TextEdit::singleline(text)
            .font(mono(13.0))
            .desired_width(field.width() - 14.0)
            .margin(vec2(7.0, 5.0)),
    );
    // as Inventor's value fields: Measure, the value measured goes in
    if panel::field_kind(label).is_some() {
        edit.context_menu(|ui| {
            if ui
                .button(egui::RichText::new("Measure...").font(sans(12.0)))
                .clicked()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::unique(MEASURE_INTO), label.to_string()));
                ui.close();
            }
        });
    }
    if let Some(kind) = panel::field_kind(label)
        && forge_doc::units::is_bare(text)
    {
        ui.painter().text(
            pos2(field.right() - 7.0, field.center().y),
            Align2::RIGHT_CENTER,
            forge_doc::units::unit_of(kind, units),
            mono(11.0),
            t.ink2,
        );
    }
    places.push((format!("dialog:field:{label}"), field));
}

impl Shell {
    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context) {
        // a field's Measure... last frame
        if let Some(label) = ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::unique(MEASURE_INTO))) {
            self.measure_into(&label);
        }
        let t = self.theme;
        let Some(mut d) = self.dialog.take() else { return };
        let frame = egui::Frame::new()
            .fill(t.surface)
            .stroke(Stroke::new(1.0, t.line))
            .corner_radius(6)
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(90),
            })
            .inner_margin(12);
        let v = self.view_rect;
        let mut out = None;
        let mut edit_sketch = false;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let shown = egui::Window::new(egui::RichText::new(d.title.clone()).font(semi(14.0)))
            .id(egui::Id::unique("feature-dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_pos(pos2(v.left() + 16.0, v.top() + 16.0))
            .fixed_size(vec2(DIALOG_W, 0.0))
            .frame(frame)
            .constrain_to(super::dialog_area(ctx))
            .show_placed(ctx, &mut places, |ui, places| {
                ui.spacing_mut().item_spacing.y = 6.0;
                // the feature, by its icon and name
                let (head, _) = ui.allocate_exact_size(vec2(DIALOG_W, 30.0), Sense::hover());
                super::icons::draw(
                    ui.painter(),
                    head.left_top() + vec2(0.0, 3.0),
                    24.0,
                    panel::icon(&d.kind),
                    1.5,
                    t.ink,
                );
                ui.painter().text(
                    pos2(head.left() + 32.0, head.center().y),
                    Align2::LEFT_CENTER,
                    panel::kind_word(&d.kind),
                    semi(15.0),
                    t.ink,
                );
                let pick = d.pick.clone();
                let dims = d.sketch_dims();
                let units = d.units;
                let waiting = d.waiting_for_pick();
                let extrude = d.kind == "extrude";
                let combine = d.kind == "combine";
                let mut activate = None;
                let d_kind = d.kind.clone();
                let mut settled_now = false;
                let thread = d.thread.clone();
                let mut thread_change: ThreadChange = (None, None, None, None);
                let mut extent = d.extent;
                let mut toward = d.toward;
                let (mut full, mut way) = (d.full, d.way);
                let revolve = d.kind == "revolve" && !d.fields.iter().any(|f| f.0 == "Plane");
                if let Some(mut h) = d.hole.clone() {
                    let thread = d.thread.clone();
                    let mut change: ThreadChange = (None, None, None, None);
                    let units = d.units;
                    ui.add_space(4.0);
                    heading(ui, "Placement", &t);
                    if let Some(k) = text_toggles(
                        ui,
                        &t,
                        &[
                            ("From Sketch", h.placing == Placing::Sketch),
                            ("Linear", h.placing == Placing::Linear),
                            ("Concentric", h.placing == Placing::Concentric),
                        ],
                        "dialog:place",
                        places,
                    ) {
                        h.placing = [Placing::Sketch, Placing::Linear, Placing::Concentric][k];
                        if let Some(f) = d.fields.iter_mut().find(|f| f.0 == "Points") {
                            f.1.clear();
                        }
                    }
                    match h.placing {
                        Placing::Concentric => {
                            let n = h.concentric.is_some() as usize;
                            pick_row(ui, "Round edge", n, n == 0, "Pick a circular edge", &t, places);
                        }
                        Placing::Linear => {
                            let face = h.linear.face.is_some() as usize;
                            pick_row(
                                ui,
                                "Face",
                                face,
                                face == 0,
                                "Pick the face it goes in",
                                &t,
                                places,
                            );
                            let n = h.linear.edges.len();
                            pick_row(
                                ui,
                                "Edges",
                                n,
                                face == 1 && n < 2,
                                "Pick two straight edges of it",
                                &t,
                                places,
                            );
                            field_row(ui, "Distance 1", &mut h.linear.first, units, &t, places);
                            field_row(ui, "Distance 2", &mut h.linear.second, units, &t, places);
                        }
                        Placing::Sketch => {}
                    }
                    if h.placing != Placing::Sketch
                        && let Some(k) = small_row(
                            ui,
                            &t,
                            "Direction",
                            &[
                                ("backward", "Into the face", !h.turned),
                                ("forward", "Out of it", h.turned),
                            ],
                            "dialog:turn",
                            places,
                        )
                    {
                        h.turned = k == 1;
                    }
                    let points = d
                        .fields
                        .iter()
                        .find(|f| f.0 == "Points")
                        .map_or(String::new(), |f| f.1.clone());
                    let n = if points.starts_with("on ") || points.starts_with("-on ") {
                        points.split_whitespace().count().saturating_sub(3)
                    } else {
                        (!points.is_empty()) as usize
                    };
                    if h.placing == Placing::Sketch {
                        pick_row(
                            ui,
                            "Points",
                            n,
                            n == 0,
                            "Pick sketch points or circle centres",
                            &t,
                            places,
                        );
                    }
                    let flipped = points.starts_with("-on ");
                    if points.starts_with("on ") || flipped {
                        let way = small_row(
                            ui,
                            &t,
                            "Direction",
                            &[
                                ("forward", "Along the normal", !flipped),
                                ("backward", "Against the normal", flipped),
                            ],
                            "dialog:flip",
                            places,
                        );
                        if let Some(k) = way {
                            let rest = points.split_once(' ').map_or("", |p| p.1);
                            let now = if k == 1 {
                                format!("-on {rest}")
                            } else {
                                format!("on {rest}")
                            };
                            if let Some(f) = d.fields.iter_mut().find(|f| f.0 == "Points") {
                                f.1 = now;
                            }
                        }
                    }
                    ui.add_space(4.0);
                    heading(ui, "Type", &t);
                    if let Some(k) = text_toggles(
                        ui,
                        &t,
                        &[
                            ("Simple", h.kind == HoleKind::Simple),
                            ("Clearance", h.kind == HoleKind::Clearance),
                            ("Tapped", h.kind == HoleKind::Tapped),
                        ],
                        "dialog:holekind",
                        places,
                    ) {
                        h.kind = [HoleKind::Simple, HoleKind::Clearance, HoleKind::Tapped][k];
                    }
                    match h.kind {
                        HoleKind::Simple => {
                            field_row(ui, "Diameter", &mut h.diameter, units, &t, places);
                        }
                        HoleKind::Clearance => {
                            let items: Vec<(String, String, bool)> = screws()
                                .into_iter()
                                .map(|s| (s.to_string(), s.to_string(), false))
                                .collect();
                            if let Some(s) = dropdown(ui, &t, "Fastener", &h.screw, &items) {
                                h.screw = s;
                            }
                            let fits: Vec<(String, String, bool)> = ["close", "normal", "loose"]
                                .into_iter()
                                .map(|f| (f.to_string(), f.to_string(), false))
                                .collect();
                            if let Some(f) = dropdown(ui, &t, "Fit", h.fit, &fits) {
                                h.fit = match f.as_str() {
                                    "close" => "close",
                                    "loose" => "loose",
                                    _ => "normal",
                                };
                            }
                        }
                        HoleKind::Tapped => {
                            thread_rows(ui, &thread, &t, &mut change);
                            let drill = thread.entry().map_or(String::new(), |e| {
                                let (d, how) = tap_drill(e);
                                format!("{} mm, {how}", forge_doc::units::trim(d))
                            });
                            value_row(ui, "Tap drill", &drill, &t);
                            let g = ui.fonts_mut(|f| {
                                f.layout(
                                    "Records the thread; it isn't cut. A Thread feature on this hole cuts it."
                                        .to_string(),
                                    sans(12.0),
                                    t.ink2,
                                    DIALOG_W,
                                )
                            });
                            let (r, _) =
                                ui.allocate_exact_size(vec2(DIALOG_W, g.size().y + 2.0), Sense::hover());
                            ui.painter().galley(r.min, g, t.ink2);
                        }
                    }
                    ui.add_space(4.0);
                    heading(ui, "Seat", &t);
                    if let Some(k) = text_toggles(
                        ui,
                        &t,
                        &[
                            ("None", h.seat == Seat::None),
                            ("Counterbore", h.seat == Seat::Counterbore),
                            ("Spotface", h.seat == Seat::Spotface),
                            ("Countersink", h.seat == Seat::Countersink),
                        ],
                        "dialog:seat",
                        places,
                    ) {
                        h.seat = [Seat::None, Seat::Counterbore, Seat::Spotface, Seat::Countersink][k];
                    }
                    if h.seat != Seat::None {
                        field_row(ui, "Seat diameter", &mut h.seat_diameter, units, &t, places);
                        if h.seat == Seat::Countersink {
                            field_row(ui, "Angle", &mut h.seat_angle, units, &t, places);
                        } else {
                            field_row(ui, "Seat depth", &mut h.seat_depth, units, &t, places);
                        }
                    }
                    ui.add_space(4.0);
                    heading(ui, "Termination", &t);
                    if let Some(k) = text_toggles(
                        ui,
                        &t,
                        &[
                            ("Distance", h.termination == Termination::Distance),
                            ("Through All", h.termination == Termination::Through),
                            ("To Next", h.termination == Termination::Next),
                            ("To", h.termination == Termination::Surface),
                        ],
                        "dialog:end",
                        places,
                    ) {
                        h.termination = [
                            Termination::Distance,
                            Termination::Through,
                            Termination::Next,
                            Termination::Surface,
                        ][k];
                    }
                    if h.termination == Termination::Surface {
                        let n = (!h.surface.is_empty()) as usize;
                        pick_row(ui, "To face", n, n == 0, "Pick the face it stops at", &t, places);
                    }
                    if h.termination == Termination::Distance {
                        field_row(ui, "Depth", &mut h.depth, units, &t, places);
                        if let Some(k) = text_toggles(
                            ui,
                            &t,
                            &[("Flat", h.point.is_none()), ("Angled 118", h.point.is_some())],
                            "dialog:point",
                            places,
                        ) {
                            h.point = (k == 1).then(|| "118".to_string());
                        }
                    }
                    hole_section(ui, &h, &t);
                    d.hole = Some(h);
                    let (a, b, c, e) = change;
                    if a.is_some() || b.is_some() || c.is_some() || e.is_some() {
                        d.choose_thread(a.as_deref(), b.as_deref(), c.as_deref(), e.as_deref());
                    }
                    d.settle_hole();
                }
                let hole_up = d.hole.is_some();
                for section in ["Input Geometry", "Behavior", "Output"] {
                    if hole_up {
                        break;
                    }
                    let there = d.fields.iter().any(|f| panel::section(&f.0) == section);
                    if !there {
                        continue;
                    }
                    ui.add_space(4.0);
                    heading(ui, section, &t);
                    for (label, text) in d.fields.iter_mut().filter(|f| panel::section(&f.0) == section) {
                        if label == "Operation" {
                            let now = text.clone();
                            let three = [
                                ("join", "Join", now == "add"),
                                ("opcut", "Cut", now == "cut"),
                                ("intersect", "Intersect", now == "keep"),
                            ];
                            let four = [
                                ("join", "Join", now.is_empty() || now == "add"),
                                ("opcut", "Cut", now == "cut"),
                                ("intersect", "Intersect", now == "keep"),
                                ("newsolid", "New Solid", now == "new"),
                            ];
                            let row = toggle_row(
                                ui,
                                &t,
                                if combine { &three[..] } else { &four[..] },
                                "dialog:op",
                                places,
                            );
                            if let Some(k) = row {
                                *text = if combine {
                                    ["add", "cut", "keep"][k].to_string()
                                } else {
                                    ["", "cut", "keep", "new"][k].to_string()
                                };
                            }
                            continue;
                        }
                        if label == "Hand" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Hand",
                                &[
                                    ("forward", "Right-handed", text.is_empty()),
                                    ("backward", "Left-handed", text == "left"),
                                ],
                                "dialog:hand",
                                places,
                            ) {
                                *text = ["", "left"][k].to_string();
                            }
                            continue;
                        }
                        if label == "Only" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Tangent chain",
                                &[
                                    ("edge1", "Only what's picked", !text.is_empty()),
                                    ("chain", "All that's tangent to it", text.is_empty()),
                                ],
                                "dialog:chain",
                                places,
                            ) {
                                *text = ["only", ""][k].to_string();
                            }
                            continue;
                        }
                        if label == "Flip" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Fills",
                                &[
                                    ("forward", "This side", text.is_empty()),
                                    ("backward", "The other side", text == "flip"),
                                ],
                                "dialog:flip",
                                places,
                            ) {
                                *text = ["", "flip"][k].to_string();
                                settled_now = true;
                            }
                            continue;
                        }
                        if label == "Class" {
                            continue;
                        }
                        if label == "Size" && d_kind == "thread" {
                            thread_rows(ui, &thread, &t, &mut thread_change);
                            continue;
                        }
                        if label == "Direction" {
                            let now = text.clone();
                            let row = small_row(
                                ui,
                                &t,
                                "Direction",
                                &[
                                    ("axisx", "X", now == "lin 1 0 0"),
                                    ("axisy", "Y", now == "lin 0 1 0"),
                                    ("axisz", "Z", now == "lin 0 0 1"),
                                ],
                                "dialog:way",
                                places,
                            );
                            if let Some(k) = row {
                                *text = ["lin 1 0 0", "lin 0 1 0", "lin 0 0 1"][k].to_string();
                            }
                            continue;
                        }
                        if label == "Profile" && !dims.is_empty() {
                            for (name, value) in &dims {
                                value_row(ui, name, value, &t);
                            }
                            if ui.button("Edit Sketch").clicked() {
                                edit_sketch = true;
                            }
                            continue;
                        }
                        let is_pick = (matches!(
                            label.as_str(),
                            "Edges"
                                | "Faces"
                                | "Reference"
                                | "Features"
                                | "Bodies"
                                | "Points"
                                | "Cylinder"
                                | "Profile"
                                | "Profiles"
                                | "Path"
                                | "Target"
                                | "Tools"
                        )) || (label == "Axis" && (d_kind == "coil" || d_kind == "revolve"));
                        if is_pick {
                            let wanted = pick.as_ref().is_some_and(|(l, _)| l == label);
                            let n = match label.as_str() {
                                "Points" => text.split_whitespace().count().saturating_sub(3),
                                "Cylinder" => (!text.trim().is_empty()) as usize,
                                "Profile" | "Path" | "Axis" | "Reference" | "Target" => {
                                    (!text.trim().is_empty()) as usize
                                }
                                "Profiles" => text.matches("profile").count(),
                                _ => forge_doc::text::tokens(text).len(),
                            };
                            let hint = pick.as_ref().filter(|(l, _)| l == label).map_or("", |(_, w)| *w);
                            if pick_row(ui, label, n, wanted && waiting, hint, &t, places) {
                                activate = Some(label.clone());
                            }
                            continue;
                        }
                        // how far: the kind of extent and the way it goes, then the number if it has one
                        // how far round: all of it, or an angle one way, the other, both, or each its own
                        if revolve && label == "Angle 2" && (full || way != Way::Two) {
                            continue;
                        }
                        if revolve && label == "Angle" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Extents",
                                &[("angle", "Angle", !full), ("full", "Full", full)],
                                "dialog:extents",
                                places,
                            ) {
                                full = k == 1;
                            }
                            if full {
                                continue;
                            }
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Direction",
                                &[
                                    ("forward", "Forward", way == Way::Forward),
                                    ("backward", "Back", way == Way::Back),
                                    ("symmetric", "Symmetric", way == Way::Symmetric),
                                    ("twoway", "Two directions", way == Way::Two),
                                ],
                                "dialog:way",
                                places,
                            ) {
                                way = [Way::Forward, Way::Back, Way::Symmetric, Way::Two][k];
                            }
                        }
                        if extrude && label == "Distance" {
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Extent",
                                &[
                                    ("distance", "Distance", extent == Extent::Distance),
                                    ("thruall", "Through All", extent == Extent::Through),
                                    ("tonext", "To Next", extent == Extent::Next),
                                    ("tosurface", "To Surface", extent == Extent::Surface),
                                ],
                                "dialog:extent",
                                places,
                            ) {
                                extent =
                                    [Extent::Distance, Extent::Through, Extent::Next, Extent::Surface][k];
                            }
                            if let Some(k) = small_row(
                                ui,
                                &t,
                                "Direction",
                                &[
                                    ("forward", "Forward", toward == Toward::Forward),
                                    ("backward", "Back", toward == Toward::Back),
                                    ("symmetric", "Symmetric", toward == Toward::Both),
                                ],
                                "dialog:dir",
                                places,
                            ) {
                                toward = [Toward::Forward, Toward::Back, Toward::Both][k];
                            }
                            if extent != Extent::Distance {
                                continue;
                            }
                        }
                        field_row(ui, label, text, units, &t, places);
                    }
                }
                d.extent = extent;
                d.toward = toward;
                d.full = full;
                d.way = way;
                if let Some(l) = activate {
                    d.activate(&l);
                }
                d.side_settled |= settled_now;
                let (a, b, c, e) = thread_change;
                if a.is_some() || b.is_some() || c.is_some() || e.is_some() {
                    d.choose_thread(a.as_deref(), b.as_deref(), c.as_deref(), e.as_deref());
                }
                if let Some(why) = d.thread_blocked() {
                    let g = ui.fonts_mut(|f| f.layout(why, sans(12.0), t.ink, DIALOG_W - 14.0));
                    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, g.size().y + 8.0), Sense::hover());
                    ui.painter()
                        .rect_stroke(r, 4.0, Stroke::new(1.0, t.focus), egui::StrokeKind::Inside);
                    ui.painter().galley(r.min + vec2(7.0, 4.0), g, t.ink);
                }
                if let Some(why) = &d.error {
                    ui.add_space(2.0);
                    let g =
                        ui.fonts_mut(|f| f.layout(why.clone(), sans(12.0), Color32::WHITE, DIALOG_W - 14.0));
                    let (r, _) = ui.allocate_exact_size(vec2(DIALOG_W, g.size().y + 8.0), Sense::hover());
                    ui.painter().rect_filled(r, 4.0, t.error);
                    ui.painter().galley(r.min + vec2(7.0, 4.0), g, Color32::WHITE);
                }
                ui.add_space(4.0);
                let can_ok = d.error.is_none() && !d.waiting_for_pick() && !d.awaiting;
                ui.horizontal(|ui| {
                    let new = matches!(d.target, DialogTarget::New(_));
                    if new {
                        let r = ui.add_enabled(
                            can_ok,
                            egui::Button::new(egui::RichText::new("+").font(semi(15.0)))
                                .min_size(vec2(30.0, 28.0)),
                        );
                        places.push(("dialog:apply".to_string(), r.rect));
                        if r.on_hover_text("Apply and make another").clicked() {
                            d.again = true;
                            out = Some(Action::DialogOk);
                        }
                    }
                    let gap = DIALOG_W - if new { 30.0 + 8.0 } else { 0.0 } - 64.0 - 8.0 - 64.0;
                    ui.add_space(gap.max(0.0));
                    let c = ui.add_sized(vec2(64.0, 28.0), egui::Button::new("Cancel"));
                    places.push(("dialog:cancel".to_string(), c.rect));
                    if c.clicked() {
                        out = Some(Action::DialogCancel);
                    }
                    let label = egui::RichText::new("OK").font(semi(13.0)).color(Color32::WHITE);
                    let ok = ui.add_enabled(
                        can_ok,
                        egui::Button::new(label).fill(t.accent).min_size(vec2(64.0, 28.0)),
                    );
                    places.push(("dialog:ok".to_string(), ok.rect));
                    if ok.clicked() {
                        out = Some(Action::DialogOk);
                    }
                });
                if can_ok && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    out = Some(Action::DialogOk);
                }
            });
        self.places.extend(places);
        if let Some(s) = shown {
            self.note("dialog", s.response.rect);
        }
        if edit_sketch && let DialogTarget::Edit(id) = d.target {
            out = Some(Action::DialogCancel);
            self.out.push(Action::EditInline(id));
        }
        self.dialog = Some(d);
        self.out.extend(out);
    }
}
