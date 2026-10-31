//! A part file built line by line, every stage kept, so an edit rebuilds only from the first
//! line that changed.

use crate::facemesh::{self, FaceMesh};
use crate::select::{self, Ent, View};
use crate::text::{self, Before, Line};
use forge_mesh::TriMesh;
use forge_model::{Feature, RegenError, Source, step};
use forge_topo::{Body, Name};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct Model {
    src: String,
    pub lines: Vec<Line>,
    pub features: Vec<Feature>,
    pub labels: Vec<Option<String>>,
    pub kinds: Vec<&'static str>,
    /// The part after each built feature.
    stages: Vec<Option<Body>>,
    /// Each stage's faces' making features, filled as asked for.
    origins: RefCell<Vec<select::Lineage>>,
    /// How many entities each inline selector picked: (line index, count).
    picks: RefCell<Vec<(usize, usize)>>,
    /// The first line that failed, if one did.
    pub error: Option<Failure>,
    /// Milliseconds the last update spent parsing and regenerating.
    pub ms: f64,
    /// Lines the last update rebuilt.
    pub rebuilt: usize,
    meshes: facemesh::Cache,
    /// The finished part's faces' triangles, and all of them as one mesh, made as asked for.
    faces: Option<Vec<(forge_topo::FaceId, Option<Arc<FaceMesh>>)>>,
    mesh: Option<TriMesh>,
}

#[derive(Clone, Debug)]
pub struct Failure {
    /// Index into `lines`.
    pub at: usize,
    pub msg: String,
    pub hint: String,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            src: String::new(),
            lines: vec![],
            features: vec![],
            labels: vec![],
            kinds: vec![],
            stages: vec![],
            origins: RefCell::new(vec![]),
            picks: RefCell::new(vec![]),
            error: None,
            ms: 0.0,
            rebuilt: 0,
            meshes: facemesh::Cache::default(),
            faces: None,
            mesh: None,
        }
    }
}

impl Model {
    pub fn new(src: &str) -> Model {
        let mut m = Model::default();
        m.update(src);
        m
    }

    /// Rebuilds from the first line whose label or tokens changed, keeping each part built
    /// after it while the line reads the same as before and nothing changed above it but
    /// parameters and sketches (which make nothing).
    pub fn update(&mut self, src: &str) {
        let t0 = Instant::now();
        if src == self.src && !self.lines.is_empty() {
            self.rebuilt = 0;
            self.ms = t0.elapsed().as_secs_f64() * 1e3;
            return;
        }
        self.src = src.to_string();
        let new = text::lines(src);
        let same = |a: &Line, b: &Line| a.label == b.label && a.toks == b.toks;
        let keep = self
            .lines
            .iter()
            .zip(&new)
            .take_while(|(a, b)| same(a, b))
            .count()
            .min(self.stages.len());
        let old = self.stages.len();
        let old_features = self.features.split_off(keep);
        let mut old_stages = self.stages.split_off(keep).into_iter();
        self.labels.truncate(keep);
        self.kinds.truncate(keep);
        self.picks.get_mut().retain(|p| p.0 < keep);
        self.lines = new;
        self.error = None;
        self.rebuilt = 0;
        let mut reuse = true;
        for i in keep..self.lines.len() {
            self.rebuilt += 1;
            let line = self.lines[i].clone();
            self.labels.push(line.label.clone());
            let parsed = {
                let refs = |r: &str| self.refs(i, r);
                let face = |r: &str| self.face(i, r);
                let b = Before {
                    features: &self.features,
                    labels: &self.labels[..i],
                    kinds: &self.kinds,
                    refs: &refs,
                    face: &face,
                };
                text::parse_line(&line, &b)
            };
            let param = line.toks.get(1).is_some_and(|t| t == "=");
            let f = match parsed {
                Ok(f) => f,
                // until a line has parsed, this may not be a part file: echo none of it
                Err(_) if i == 0 && !param => {
                    let msg = "the first line is not a feature; is this a .fc part file?".into();
                    self.fail(i, msg, "see AGENTS.md for the feature kinds".into());
                    break;
                }
                Err(msg) => {
                    let hint = match text::kind(if param { "param" } else { &line.toks[0] }) {
                        _ if msg.starts_with("the sketch does not solve") => {
                            "change or remove one of those rows; a sketch may be under-constrained".into()
                        }
                        Some(k) => k.syntax.to_string(),
                        None => "see AGENTS.md for the feature kinds".into(),
                    };
                    let hint = self.defined_below(i, &msg).unwrap_or(hint);
                    self.fail(i, msg, hint);
                    break;
                }
            };
            let neutral = |f: &Feature| matches!(f, Feature::Param(_) | Feature::Sketch(_));
            let was = old_features.get(i - keep);
            reuse &= was.is_some_and(|w| *w == f || neutral(w) && neutral(&f));
            self.kinds.push(text::kind_of(&f).name);
            self.features.push(f);
            if let (true, Some(stage)) = (reuse, old_stages.next()) {
                self.stages.push(stage);
                self.rebuilt -= 1;
                continue;
            }
            reuse = false;
            self.origins.get_mut().truncate(i);
            let before = if i == 0 { None } else { self.stages[i - 1].clone() };
            match step(&self.features, i, before) {
                Ok(b) => self.stages.push(b),
                Err(e) => {
                    let (msg, hint) = explain(&e);
                    let msg = match self.body_after(i) {
                        // a fillet works on the part compacted
                        Some(b) if b.dead > 0 && matches!(e, RegenError::Fillet { .. }) => {
                            with_handles(&msg, &b.compact())
                        }
                        Some(b) => with_handles(&msg, b),
                        None => msg,
                    };
                    self.fail(i, msg, hint);
                    break;
                }
            }
        }
        if self.stages.len() != old || !reuse {
            self.faces = None;
            self.mesh = None;
        }
        self.origins.get_mut().truncate(self.stages.len());
        self.ms = t0.elapsed().as_secs_f64() * 1e3;
    }

    /// For an unknown name in `msg`, where a line below line `i` defines it.
    fn defined_below(&self, i: usize, msg: &str) -> Option<String> {
        let name = msg.split('\'').nth(1)?;
        let l = self.lines[i + 1..]
            .iter()
            .find(|l| l.toks.len() > 1 && l.toks[0] == name && l.toks[1] == "=")?;
        Some(format!(
            "{name} is defined on line {}, below; define parameters above their use",
            l.no
        ))
    }

    fn fail(&mut self, at: usize, msg: String, hint: String) {
        self.features.truncate(at);
        self.labels.truncate(at);
        self.kinds.truncate(at);
        self.error = Some(Failure { at, msg, hint });
    }

    /// How many entities each inline selector picked: (file line number, count), in order.
    pub fn picks(&self) -> Vec<(usize, usize)> {
        self.picks
            .borrow()
            .iter()
            .map(|&(i, n)| (self.lines[i].no, n))
            .collect()
    }

    /// Lines built without error.
    pub fn built(&self) -> usize {
        self.stages.len()
    }

    /// The part as far as it built.
    pub fn body(&self) -> Option<&Body> {
        self.body_after(self.built())
    }

    /// The part after `n` lines.
    pub fn body_after(&self, n: usize) -> Option<&Body> {
        self.stages.get(n.checked_sub(1)?)?.as_ref()
    }

    /// The finished part's faces' triangles (None where a face would not mesh).
    pub fn faces(&mut self) -> &[(forge_topo::FaceId, Option<Arc<FaceMesh>>)] {
        if self.faces.is_none() {
            let fs = match self.stages.last() {
                Some(Some(b)) => self.meshes.faces(b),
                _ => vec![],
            };
            self.faces = Some(fs);
        }
        self.faces.as_deref().expect("just made")
    }

    /// The finished part's triangles.
    pub fn mesh(&mut self) -> Result<&TriMesh, String> {
        if self.mesh.is_none() {
            let m = facemesh::join(self.faces());
            if m.triangles.is_empty() {
                return Err("no part to mesh".into());
            }
            self.mesh = Some(m);
        }
        Ok(self.mesh.as_ref().expect("just made"))
    }

    /// The finished part and its triangles.
    pub fn ready(&mut self) -> Result<(&Body, &TriMesh), String> {
        self.mesh()?;
        let m = &*self;
        Ok((m.body().ok_or("no part yet")?, m.mesh.as_ref().ok_or("no mesh")?))
    }

    /// The finished part's volume, area and box (from each face's cached triangles and its
    /// edges), and how many faces would not mesh (their own, looser, boxes stand in).
    pub fn stats(&mut self) -> (f64, f64, forge_math::Box3, usize) {
        let fs = self.faces().to_vec();
        let body = self.body();
        let (mut v, mut a, mut bb, mut bad) = (0.0, 0.0, forge_math::Box3::EMPTY, 0);
        for (f, m) in &fs {
            match m {
                Some(m) => {
                    v += m.volume;
                    a += m.area;
                    // the boundary's boxes are tight; the triangles catch a dome's top
                    bb = bb.hull(&m.bbox);
                    if let Some(b) = body {
                        for &l in &b.face(*f).loops {
                            for &c in &b.lp(l).coedges {
                                bb = bb.hull(&b.edge_box(b.coedge(c).edge));
                            }
                        }
                    }
                }
                None => {
                    bad += 1;
                    if let Some(b) = body {
                        bb = bb.hull(&b.face_box(*f));
                    }
                }
            }
        }
        (v, a, bb, bad)
    }

    /// Which feature made each face of the part after `n` lines.
    fn origin(&self, n: usize) -> select::Lineage {
        let mut origins = self.origins.borrow_mut();
        while origins.len() < n {
            let k = origins.len();
            let prev = (k > 0).then(|| self.stages[k - 1].as_ref()).flatten();
            let copied = match (&self.features[k], &self.stages[k], prev) {
                (Feature::Pattern { of, layout }, Some(b), Some(pb)) => {
                    let from = |c: &Vec<usize>| match of {
                        Source::Body => true,
                        Source::Features(src) => c.iter().any(|x| src.contains(x)),
                    };
                    select::copies(b, pb, &origins[k - 1], k, layout, &from)
                }
                _ => HashMap::new(),
            };
            let o = match &self.stages[k] {
                None => HashMap::new(),
                Some(b) => select::lineage(b, prev.map(|p| (p, &origins[k - 1])), k, &copied),
            };
            origins.push(o);
        }
        n.checked_sub(1).map(|i| origins[i].clone()).unwrap_or_default()
    }

    /// The index of the feature a reference names, among the first `n`.
    pub fn feat(&self, n: usize, r: &str) -> Result<usize, String> {
        if let Some(i) = self.labels[..n.min(self.labels.len())]
            .iter()
            .position(|l| l.as_deref() == Some(r))
        {
            return Ok(i);
        }
        let digits = r.trim_end_matches(|c: char| c.is_ascii_digit());
        let k: usize = r[digits.len()..]
            .parse()
            .map_err(|_| format!("no feature '{r}'"))?;
        let found = if digits == "f" {
            k.checked_sub(1).filter(|&i| i < n)
        } else {
            self.kinds[..n.min(self.kinds.len())]
                .iter()
                .enumerate()
                .filter(|x| *x.1 == digits)
                .nth(k.wrapping_sub(1))
                .map(|x| x.0)
        };
        found.ok_or_else(|| format!("no feature '{r}'"))
    }

    /// Runs a selector on the part after `n` lines (all that built if `None`).
    pub fn select(&self, sel: &str, n: Option<usize>) -> Result<(&Body, Vec<Ent>), String> {
        let n = n.unwrap_or(self.built()).min(self.built());
        let body = self.body_after(n).ok_or("no part yet")?;
        let origin = self.origin(n);
        let feat = |r: &str| self.feat(n, r);
        let ents = select::select(
            &View {
                body,
                origin: &origin,
                feat: &feat,
            },
            sel,
        )?;
        Ok((body, ents))
    }

    /// The planar face a reference names before line `i`: its name and its plane, z outward.
    fn face(&self, i: usize, r: &str) -> Result<(Name, forge_math::Frame), String> {
        let names = self.refs(i, r)?;
        let [name] = names[..] else {
            return Err(format!("{r} names {} entities; one face is needed", names.len()));
        };
        let body = self.body_after(i).ok_or("nothing is built above this line")?;
        let f = body
            .face_by_name(name)
            .ok_or_else(|| format!("{r} is not a face"))?;
        match &body.face(f).surface {
            forge_geom::Surface::Plane(p) if body.face(f).same_sense => Ok((name, p.frame)),
            forge_geom::Surface::Plane(p) => Ok((name, p.frame.flipped())),
            _ => Err(format!("{r} is not a planar face")),
        }
    }

    /// Resolves a line's `@handle` or `(selector)` against the part before line `i`.
    fn refs(&self, i: usize, r: &str) -> Result<Vec<Name>, String> {
        if let Some(n) = text::full_name(r) {
            return Ok(vec![n]);
        }
        if r.starts_with('@') {
            let body = self.body_after(i).ok_or("nothing is built above this line")?;
            return Ok(vec![select::name(body, select::by_handle(body, r)?)]);
        }
        let (body, ents) = self.select(r, Some(i))?;
        self.picks.borrow_mut().push((i, ents.len()));
        Ok(ents.into_iter().map(|e| select::name(body, e)).collect())
    }
}

/// `EdgeId(6)` and `FaceId(2)` in a message as the handles of those entities of `body`.
fn with_handles(msg: &str, body: &Body) -> String {
    let handles = select::handles(body);
    let mut out = String::new();
    let mut rest = msg;
    while let Some(k) = rest.find("Id(") {
        let kind = &rest[..k];
        let close = rest[k..].find(')').map(|c| k + c);
        let id = close.and_then(|c| rest[k + 3..c].parse::<u32>().ok());
        let ent = match (id, kind) {
            (Some(n), _) if kind.ends_with("Edge") && (n as usize) < body.edges.len() => {
                Some(Ent::Edge(forge_topo::EdgeId(n)))
            }
            (Some(n), _) if kind.ends_with("Face") && (n as usize) < body.faces.len() => {
                Some(Ent::Face(forge_topo::FaceId(n)))
            }
            _ => None,
        };
        match (ent, close) {
            (Some(e), Some(c)) => {
                let word = if kind.ends_with("Edge") { "Edge" } else { "Face" };
                out.push_str(&kind[..kind.len() - word.len()]);
                out.push_str(&handles[&select::name(body, e)]);
                rest = &rest[c + 1..];
            }
            _ => {
                out.push_str(&rest[..k + 3]);
                rest = &rest[k + 3..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// What went wrong, and what to try.
pub fn explain(e: &RegenError) -> (String, String) {
    use RegenError::*;
    let hint = |s: &str| s.to_string();
    match e {
        Extrude { error, .. } => (
            format!("extrude: {error:?}"),
            hint("check the loops: closed, not crossing, holes inside the outer loop"),
        ),
        Revolve { error, .. } => (
            format!("revolve: {error:?}"),
            hint("keep the profile on one side of the axis, off it"),
        ),
        Boolean { error, .. } => (
            format!("boolean: {error:?}"),
            hint("move or resize the tool so it does not just touch the part"),
        ),
        Fillet { error, .. } => (
            format!("fillet: {error:?}"),
            hint("try a smaller radius, or fewer edges at once"),
        ),
        MissingEdge { name, .. } => (
            format!("no edge {name:?} on the part here"),
            hint("an upstream edit renamed it; use a (selector) or re-run query for a handle"),
        ),
        Chamfer { error, .. } => (
            format!("chamfer: {error:?}"),
            hint("try a smaller size; the face must bound every edge"),
        ),
        Shell { error, .. } => (format!("shell: {error:?}"), hint("try a thinner wall")),
        Draft { error, .. } => (
            format!("draft: {error:?}"),
            hint("the faces must be planar and meet the neutral plane"),
        ),
        Hole { error, .. } => (
            format!("hole: {error:?}"),
            hint("hole D FRAME [cb D2 DEPTH | cs D2 A] (N | thru | to FEAT): head wider than the hole"),
        ),
        Extent { error, .. } => (
            format!("extent: {error:?}"),
            hint("`to` needs an earlier plane or planar face square to the travel; thru only cuts"),
        ),
        Pattern { error, .. } => (
            format!("pattern: {error:?}"),
            hint("counts > 0, nonzero directions; copy earlier solid features"),
        ),
        Sweep { error, .. } => (
            format!("sweep: {error:?}"),
            hint(
                "start the path on the profile's plane, square to it; join lines and arcs without corners; ease tight bends",
            ),
        ),
        Coil { error, .. } => (
            format!("coil: {error:?}"),
            hint("keep the profile off the axis; pitch more than the profile's height; less taper"),
        ),
        Loft { error, .. } => (
            format!("loft: {error:?}"),
            hint("give every profile the same loops and segment counts, in one plane each, apart"),
        ),
        Edit { error, .. } => (
            format!("edit: {error:?}"),
            hint(match error {
                forge_model::EditError::Rib(_) => {
                    "the rib must meet the part on its fill side (try flip) and run into it at both ends"
                }
                forge_model::EditError::Direct(forge_ops::DirectError::CannotHeal { .. }) => {
                    "delete whole holes or bosses, or single chamfer and fillet strips"
                }
                _ => "try a smaller distance or angle; the faces' neighbours must still meet",
            }),
        ),
        NothingToCombine { .. } => (
            "nothing to combine with yet".into(),
            hint("start with a solid: extrude, revolve or ball (op new)"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_model::{Part, fillets, suite};

    fn finished(m: &Model) -> Body {
        m.body().expect("built").compact()
    }

    /// The part file's ribs, offsets, thickenings and direct edits, picked by selectors.
    #[test]
    fn edits_from_text() {
        let src = "extrude 20 rect 0 0 40 30 circle 20 15 4
offset 1 (faces cyl)
move (faces normal +x) by 2 0 0
top: plane at 0 0 24
replace (faces normal +z) with top
delete (faces cyl)
shell 2 (faces normal +z)
rib 2 web draft 1 at 0 0 24 line 10 15 30 15
thicken 1 (faces normal -z)
";
        let m = Model::new(src);
        assert!(m.error.is_none(), "{:?}", m.error);
        let gusset = "extrude 5 rect 0 0 30 40
extrude -5 xz rect 0 0 30 40
rib 2 flip yz at 15 0 0 line 5 25 25 5
";
        let m = Model::new(gusset);
        assert!(m.error.is_none(), "{:?}", m.error);
        let m = Model::new("extrude 5 rect 0 0 10 10\ndelete (faces normal +z)\n");
        assert!(m.error.is_some());
    }

    /// Built line by line, a part is the part `regenerate` makes.
    #[test]
    fn matches_regenerate() {
        let mut parts = vec![suite::tour(), suite::fuzz(3, 25), suite::fuzz(7, 25)];
        parts.extend(fillets::cases().into_iter().take(12).map(|c| c.1));
        for part in parts {
            let m = Model::new(&text::print(&part));
            assert!(m.error.is_none(), "{:?}", m.error);
            assert_eq!(finished(&m), part.regenerate().expect("regenerates"));
        }
    }

    /// An edit rebuilds from its line on, and ends where a fresh build does.
    #[test]
    fn edits_rebuild_from_the_first_change() {
        let src = text::print(&suite::fuzz(5, 20));
        let mut m = Model::new(&src);
        let n = m.lines.len();
        for k in [n - 1, n / 2, 1] {
            let mut lines: Vec<String> = src.lines().map(String::from).collect();
            lines[k] = lines[k].replacen(" cut ", " add ", 1).replacen(" 1", " 1.5", 1);
            let edited = lines.join("\n");
            m.update(&edited);
            let fresh = Model::new(&edited);
            // the lines from the edit to the end, or to the one that fails; none if the edit
            // reads the same (a frame's x hint scaled, say)
            let same = Model::new(&src).features.get(k) == fresh.features.get(k);
            let want = if same {
                0
            } else {
                fresh.error.as_ref().map_or(n, |f| f.at + 1) - k
            };
            assert_eq!(m.rebuilt, want, "edit at line {k}");
            assert_eq!(m.error.as_ref().map(|f| f.at), fresh.error.as_ref().map(|f| f.at));
            assert_eq!(m.built(), fresh.built());
            assert_eq!(finished(&m), finished(&fresh), "edit at line {k}");
            m.update(&src);
            assert_eq!(finished(&m), finished(&Model::new(&src)));
        }
        m.update(&src);
        assert_eq!(m.rebuilt, 0);
    }

    /// Pattern copies count as made by what they copy, however booleans split them.
    #[test]
    fn copies_trace_back_to_their_source() {
        let m = Model::new(
            "body: extrude 30 rrect -50 -30 100 60 6
shell 2 (faces normal +z top)
boss: extrude 25 at 0 0 2 circle 42 22 3.5
bhole: hole 2.5 -xy at 42 22 27 15
pattern boss bhole mirror yz
pattern boss bhole pattern1 mirror xz
",
        );
        assert!(m.error.is_none(), "{:?}", m.error);
        let count = |sel: &str| m.select(sel, None).expect("selects").1.len();
        assert_eq!(count("edges of boss concave"), 4);
        assert_eq!(count("edges between boss shell1 concave"), 4);
        assert_eq!(count("faces cyl r=1.25 of bhole"), count("faces cyl r=1.25"));
        assert_eq!(count("faces cyl r=3.5 of bhole"), 0);
        assert!(count("faces of pattern1") < count("faces of boss bhole"));
    }

    #[test]
    fn errors_name_the_line_and_hint() {
        let m = Model::new("# a plate\nextrude 5 rect 0 0 10 10\n\nhole 3 -xy at 5 5 5 thrue\n");
        let f = m.error.expect("fails");
        assert_eq!(m.lines[f.at].no, 4);
        assert!(f.msg.contains("thrue"), "{}", f.msg);
        assert!(f.hint.starts_with("hole D FRAME"), "{}", f.hint);
        let m = Model::new("extrude 5 rect 0 0 10 10\nfillet 50 (edges along +z)\n");
        let f = m.error.expect("too big a radius");
        assert!(f.msg.starts_with("fillet:"), "{}", f.msg);
        assert!(!f.msg.contains("EdgeId("), "{}", f.msg);
        assert_eq!(
            Part::default(),
            Part {
                features: Model::new("").features
            }
        );
    }
}
