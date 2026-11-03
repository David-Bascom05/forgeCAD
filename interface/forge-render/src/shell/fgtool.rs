//! The ribbon's tools on a Forge (.fg) part: each a form of picks and params that writes one
//! Forge statement (and its params above it), applied as an `AddFeature` op. Picks are anchor
//! text (`base.end`, `edges(between: (a, b))`, `plan.outline[1]`): what the evaluator itself
//! resolves, never a coordinate or a hash. A pick with no stable Forge name is refused, typed.

use forge_lang::error::Literal;
use forge_lang::values::Dim;

/// What a slot takes, and where from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    /// Edges of the part, picked in the view; several joined.
    Edges,
    /// Faces of the part, picked in the view; several joined.
    Faces,
    /// One face of the part.
    Face,
    /// One edge of the part.
    Edge,
    /// A plane: a planar face, an origin plane, a sketch (from the tree).
    Plane,
    /// A direction: a planar face's normal, or a sketch line's.
    Dir,
    /// A closed sketch entity or loop, or a whole sketch.
    Profile,
    Profiles,
    /// A sketch line.
    Line,
    /// Sketch lines, arcs, splines or loops, in order.
    Path,
    /// A sketch's points.
    Points,
    /// A feature (from the tree).
    Feature,
    Features,
    /// A sheet metal edge a flange stands on.
    SheetEdge,
}

impl SlotKind {
    pub fn many(self) -> bool {
        matches!(
            self,
            SlotKind::Edges | SlotKind::Faces | SlotKind::Profiles | SlotKind::Path | SlotKind::Features
        )
    }

    /// It takes what's clicked in the view.
    pub fn from_view(self) -> bool {
        matches!(
            self,
            SlotKind::Edges
                | SlotKind::Faces
                | SlotKind::Face
                | SlotKind::Edge
                | SlotKind::Plane
                | SlotKind::Dir
                | SlotKind::SheetEdge
        )
    }

    /// It takes a feature or sketch clicked in the tree.
    pub fn from_tree(self) -> bool {
        matches!(
            self,
            SlotKind::Feature | SlotKind::Features | SlotKind::Profile | SlotKind::Profiles | SlotKind::Plane
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub label: &'static str,
    pub kind: SlotKind,
    pub picks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolParam {
    pub name: &'static str,
    pub text: String,
    pub dim: Dim,
}

/// Why a tool can't write its statement: typed, never a guess.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolError {
    MissingPick { slot: &'static str },
    TooFew { slot: &'static str, want: usize },
    BadNumber { param: &'static str, text: String },
    /// The picked entity has no anchor: no Forge text names it stably.
    NoStableName { what: &'static str },
    /// The pick is not what the slot takes.
    WrongPick { slot: &'static str, what: &'static str },
    BadName { name: String },
    /// The tool has no Forge form (yet), or the act none in Forge's ops.
    NoForgeForm { tool: &'static str, why: String },
}

impl ToolError {
    pub fn kind(&self) -> &'static str {
        match self {
            ToolError::MissingPick { .. } => "MissingPick",
            ToolError::TooFew { .. } => "TooFew",
            ToolError::BadNumber { .. } => "BadNumber",
            ToolError::NoStableName { .. } => "NoStableName",
            ToolError::WrongPick { .. } => "WrongPick",
            ToolError::BadName { .. } => "BadName",
            ToolError::NoForgeForm { .. } => "NoForgeForm",
        }
    }

    pub fn text(&self) -> String {
        match self {
            ToolError::MissingPick { slot } => format!("{slot}: nothing picked yet"),
            ToolError::TooFew { slot, want } => format!("{slot}: pick at least {want}"),
            ToolError::BadNumber { param, text } => format!("{param}: {text:?} is not a number"),
            ToolError::NoStableName { what } => format!(
                "that {what} has no stable Forge name (no anchor of the part names it); pick another, or name it in the source"
            ),
            ToolError::WrongPick { slot, what } => format!("{slot} takes no {what}"),
            ToolError::BadName { name } => format!("{name:?} is not a feature name"),
            ToolError::NoForgeForm { tool, why } => format!("{tool}: {why}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FgTool {
    pub kind: &'static str,
    /// The feature's id: what it's assigned to.
    pub name: String,
    pub slots: Vec<Slot>,
    pub params: Vec<ToolParam>,
    /// Words: a thread's spec and length, a combine's operation.
    pub words: Vec<(&'static str, String)>,
    pub flags: Vec<(&'static str, bool)>,
    /// The slot that takes the next pick.
    pub active: usize,
    pub error: Option<ToolError>,
}

fn slot(label: &'static str, kind: SlotKind) -> Slot {
    Slot {
        label,
        kind,
        picks: vec![],
    }
}

fn param(name: &'static str, text: &str, dim: Dim) -> ToolParam {
    ToolParam {
        name,
        text: text.into(),
        dim,
    }
}

/// The tools there are a Forge form for, by the ribbon's kind.
pub const KINDS: [&str; 18] = [
    "extrude", "revolve", "hole", "fillet", "chamfer", "shell", "draft", "thread", "combine", "plane", "axis",
    "point", "pattern", "sweep", "loft", "coil", "rib", "flange",
];

impl FgTool {
    /// The form for a ribbon kind, named the kind and the first free count (`fillet1`).
    pub fn new(kind: &'static str, taken: &[String]) -> Option<FgTool> {
        use SlotKind as K;
        let (slots, params, words, flags): (Vec<Slot>, Vec<ToolParam>, Vec<(&str, String)>, Vec<(&str, bool)>) =
            match kind {
                "extrude" => (
                    vec![slot("Profile", K::Profile)],
                    vec![param("d", "10", Dim::Length)],
                    vec![],
                    vec![("cut", false)],
                ),
                "revolve" => (
                    vec![slot("Profile", K::Profile), slot("Axis", K::Line)],
                    vec![param("ang", "360", Dim::Angle)],
                    vec![],
                    vec![("cut", false)],
                ),
                "hole" => (
                    vec![slot("Points", K::Points)],
                    vec![param("d", "5", Dim::Length), param("depth", "10", Dim::Length)],
                    vec![],
                    vec![("thru", true)],
                ),
                "fillet" => (vec![slot("Edges", K::Edges)], vec![param("r", "2", Dim::Length)], vec![], vec![]),
                "chamfer" => (vec![slot("Edges", K::Edges)], vec![param("d", "1", Dim::Length)], vec![], vec![]),
                "shell" => (vec![slot("Open", K::Faces)], vec![param("t", "2", Dim::Length)], vec![], vec![]),
                "draft" => (
                    vec![slot("Faces", K::Faces), slot("Neutral", K::Plane)],
                    vec![param("ang", "3", Dim::Angle)],
                    vec![],
                    vec![],
                ),
                "thread" => (
                    vec![slot("Cylinder", K::Face)],
                    vec![],
                    vec![("spec", "M6".into()), ("length", "full".into())],
                    vec![],
                ),
                "combine" => (
                    vec![slot("Target", K::Feature), slot("Tools", K::Features)],
                    vec![],
                    vec![("op", "add".into())],
                    vec![],
                ),
                "plane" => (vec![slot("Face", K::Face)], vec![param("d", "10", Dim::Length)], vec![], vec![]),
                "axis" => (vec![slot("Along", K::Face)], vec![], vec![], vec![]),
                "point" => (vec![slot("Edge", K::Edge)], vec![], vec![], vec![]),
                "pattern" => (
                    vec![slot("Features", K::Features), slot("Direction", K::Dir)],
                    vec![param("n", "3", Dim::Count), param("step", "20", Dim::Length)],
                    vec![],
                    vec![],
                ),
                "sweep" => (vec![slot("Profile", K::Profile), slot("Path", K::Path)], vec![], vec![], vec![]),
                "loft" => (vec![slot("Profiles", K::Profiles)], vec![], vec![], vec![]),
                "coil" => (
                    vec![slot("Profile", K::Profile), slot("Axis", K::Line)],
                    vec![param("pitch", "8", Dim::Length), param("turns", "4", Dim::Count)],
                    vec![],
                    vec![],
                ),
                "rib" => (vec![slot("Path", K::Path)], vec![param("t", "2", Dim::Length)], vec![], vec![]),
                "flange" => (
                    vec![slot("Edge", K::SheetEdge)],
                    vec![param("height", "20", Dim::Length), param("ang", "90", Dim::Angle)],
                    vec![],
                    vec![],
                ),
                _ => return None,
            };
        let base = if kind == "point" { "wpoint" } else if kind == "axis" { "waxis" } else if kind == "plane" { "wplane" } else { kind };
        let name = (1..)
            .map(|k| format!("{base}{k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        Some(FgTool {
            kind,
            name,
            slots,
            params,
            words: words.into_iter().map(|(k, v)| (k, v)).collect(),
            flags,
            active: 0,
            error: None,
        })
    }

    pub fn flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x.0 == f && x.1)
    }

    fn word(&self, w: &str) -> String {
        self.words.iter().find(|x| x.0 == w).map_or(String::new(), |x| x.1.clone())
    }

    /// A pick for the active slot: taken (a second pick of the same thing takes it back out), or
    /// refused, typed.
    pub fn take(&mut self, text: Result<String, ToolError>) {
        let Some(s) = self.slots.get_mut(self.active) else { return };
        match text {
            Ok(t) => {
                self.error = None;
                if let Some(k) = s.picks.iter().position(|p| *p == t) {
                    s.picks.remove(k);
                } else if s.kind.many() {
                    s.picks.push(t);
                } else {
                    s.picks = vec![t];
                    // a one-pick slot filled: the next slot waiting takes the next pick
                    if let Some(k) = self.slots.iter().position(|x| x.picks.is_empty()) {
                        self.active = k;
                    }
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn one(&self, label: &'static str) -> Result<String, ToolError> {
        let s = self.slots.iter().find(|s| s.label == label).expect("the tool's own slot");
        s.picks.first().cloned().ok_or(ToolError::MissingPick { slot: label })
    }

    fn all(&self, label: &'static str, sep: &str) -> Result<String, ToolError> {
        let s = self.slots.iter().find(|s| s.label == label).expect("the tool's own slot");
        if s.picks.is_empty() {
            return Err(ToolError::MissingPick { slot: label });
        }
        Ok(s.picks.join(sep))
    }

    /// The params it writes above its statement: `param NAME.x = VALUE`.
    fn param_lines(&self, skip: &[&str]) -> Result<String, ToolError> {
        let mut out = String::new();
        for p in self.params.iter().filter(|p| !skip.contains(&p.name)) {
            let v = super::fg::parse_value(&p.text, p.dim)
                .filter(|l| l.dim == p.dim)
                .ok_or(ToolError::BadNumber {
                    param: p.name,
                    text: p.text.clone(),
                })?;
            let lit = Literal { value: v.value, dim: p.dim };
            out.push_str(&format!("param {}.{} = {}\n", self.name, p.name, lit.text()));
        }
        Ok(out)
    }

    /// The Forge text it adds: its params, then its statement.
    pub fn text(&self) -> Result<String, ToolError> {
        let n = &self.name;
        let ok_name = !n.is_empty()
            && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !n.starts_with(|c: char| c.is_ascii_digit());
        if !ok_name {
            return Err(ToolError::BadName { name: n.clone() });
        }
        let list = |s: String, many: bool| if many { format!("[{s}]") } else { s };
        let mut skip: Vec<&str> = vec![];
        let stmt = match self.kind {
            "extrude" => {
                let f = if self.flag("cut") { "cut" } else { "extrude" };
                format!("{n} = {f}({}, d)", self.one("Profile")?)
            }
            "revolve" => format!(
                "{n} = revolve({}, axis: {}, angle: ang{})",
                self.one("Profile")?,
                self.one("Axis")?,
                if self.flag("cut") { ", cut: true" } else { "" }
            ),
            "hole" => {
                let depth = if self.flag("thru") {
                    skip.push("depth");
                    "thru"
                } else {
                    "depth"
                };
                format!("{n} = hole(at: {}, d: d, depth: {depth})", self.one("Points")?)
            }
            "fillet" => format!("{n} = fillet({}, r)", self.all("Edges", " + ")?),
            "chamfer" => format!("{n} = chamfer({}, d)", self.all("Edges", " + ")?),
            "shell" => format!("{n} = shell(t, open: {})", self.all("Open", " + ")?),
            "draft" => format!(
                "{n} = draft({}, angle: ang, neutral: {})",
                self.all("Faces", " + ")?,
                self.one("Neutral")?
            ),
            "thread" => {
                let len = self.word("length");
                let len = if len.parse::<f64>().is_ok() {
                    len
                } else {
                    format!("{len:?}")
                };
                format!(
                    "{n} = thread(face: {}, spec: {:?}, length: {len})",
                    self.one("Cylinder")?,
                    self.word("spec")
                )
            }
            "combine" => {
                let tools = self.slots.iter().find(|s| s.label == "Tools").expect("its slot");
                if tools.picks.is_empty() {
                    return Err(ToolError::MissingPick { slot: "Tools" });
                }
                format!(
                    "{n} = combine({}, tools: {}, op: {:?})",
                    self.one("Target")?,
                    list(tools.picks.join(", "), tools.picks.len() > 1),
                    self.word("op")
                )
            }
            "plane" => format!("{n} = workplane(offset: {}, d: d)", self.one("Face")?),
            "axis" => format!("{n} = workaxis(along: {})", self.one("Along")?),
            "point" => format!("{n} = workpoint(mid: {})", self.one("Edge")?),
            "pattern" => {
                let of = self.slots.iter().find(|s| s.label == "Features").expect("its slot");
                if of.picks.is_empty() {
                    return Err(ToolError::MissingPick { slot: "Features" });
                }
                format!(
                    "{n} = pattern(of: {}, layout: lin(along: {}, n: n, step: step))",
                    list(of.picks.join(", "), of.picks.len() > 1),
                    self.one("Direction")?
                )
            }
            "sweep" => {
                let path = self.slots.iter().find(|s| s.label == "Path").expect("its slot");
                if path.picks.is_empty() {
                    return Err(ToolError::MissingPick { slot: "Path" });
                }
                format!(
                    "{n} = sweep({}, along: {})",
                    self.one("Profile")?,
                    list(path.picks.join(", "), path.picks.len() > 1)
                )
            }
            "loft" => {
                let ps = self.slots.iter().find(|s| s.label == "Profiles").expect("its slot");
                if ps.picks.len() < 2 {
                    return Err(ToolError::TooFew {
                        slot: "Profiles",
                        want: 2,
                    });
                }
                format!("{n} = loft([{}])", ps.picks.join(", "))
            }
            "coil" => format!(
                "{n} = coil({}, axis: {}, pitch: pitch, turns: turns)",
                self.one("Profile")?,
                self.one("Axis")?
            ),
            "rib" => {
                let path = self.slots.iter().find(|s| s.label == "Path").expect("its slot");
                if path.picks.is_empty() {
                    return Err(ToolError::MissingPick { slot: "Path" });
                }
                format!("{n} = rib({}, t)", list(path.picks.join(", "), path.picks.len() > 1))
            }
            "flange" => format!("{n} = flange({}, height: height, angle: ang)", self.one("Edge")?),
            _ => {
                return Err(ToolError::NoForgeForm {
                    tool: self.kind,
                    why: "no Forge form".into(),
                });
            }
        };
        Ok(format!("{}{stmt}\n", self.param_lines(&skip)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fillet_writes_its_param_and_its_edges_by_anchor() {
        let mut t = FgTool::new("fillet", &["fillet1".into()]).unwrap();
        assert_eq!(t.name, "fillet2");
        assert_eq!(t.text(), Err(ToolError::MissingPick { slot: "Edges" }));
        t.take(Ok("edges(between: (a.end, a.side(s.c)))".into()));
        t.take(Ok("edges(between: (b.end, b.side(s.c)))".into()));
        assert_eq!(
            t.text().unwrap(),
            "param fillet2.r = 2\nfillet2 = fillet(edges(between: (a.end, a.side(s.c))) + edges(between: (b.end, b.side(s.c))), r)\n"
        );
        // the same pick again takes it back
        t.take(Ok("edges(between: (b.end, b.side(s.c)))".into()));
        assert_eq!(t.slots[0].picks.len(), 1);
        t.take(Err(ToolError::NoStableName { what: "edge" }));
        assert_eq!(t.error.as_ref().map(ToolError::kind), Some("NoStableName"));
        t.params[0].text = "x".into();
        assert!(matches!(t.text(), Err(ToolError::BadNumber { .. })));
    }

    #[test]
    fn every_kind_has_a_form_and_writes_typed_params() {
        for k in KINDS {
            let mut t = FgTool::new(k, &[]).unwrap();
            for s in 0..t.slots.len() {
                t.active = s;
                t.take(Ok(format!("p{s}a")));
                if t.slots[s].kind.many() {
                    t.active = s;
                    t.take(Ok(format!("p{s}b")));
                }
            }
            let text = t.text().unwrap_or_else(|e| panic!("{k}: {}", e.text()));
            assert!(text.contains(&format!("{} = ", t.name)), "{k}: {text}");
            assert!(forge_lang::syntax::parse(&text).is_ok(), "{k}: {text}");
        }
        let r = FgTool::new("revolve", &[]).unwrap();
        assert!(r.param_lines(&[]).unwrap().contains("param revolve1.ang = 360deg"));
        let p = FgTool::new("pattern", &[]).unwrap();
        assert!(p.param_lines(&[]).unwrap().contains("param pattern1.n = count(3)"));
    }
}
