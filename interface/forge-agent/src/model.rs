//! A part file built line by line, every stage kept, so an edit rebuilds only from the first
//! line that changed.

use crate::facemesh::{self, FaceMesh};
use crate::select::{self, Ent, View};
use crate::text::{self, Before, Line};
use forge_doc::{Document, Op};
use forge_math::Box3;
use forge_mesh::TriMesh;
use forge_model::{Feature, History, RegenError, Solid, Source, step_solids};
use forge_topo::mass::MassProps;
use forge_topo::{Body, Name};
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

pub struct Model {
    src: String,
    /// The part as a document: what the lines are printed from, and what edits change.
    pub doc: Document,
    pub lines: Vec<Line>,
    pub features: Vec<Feature>,
    /// The feature the kernel refused at `error`, if the line read as one.
    pub refused: Option<Feature>,
    pub labels: Vec<Option<String>>,
    pub kinds: Vec<&'static str>,
    /// The part after each built feature.
    stages: Vec<Arc<Stage>>,
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
    /// Of `ms`, how much the kernel took (the rest read the lines and resolved their references).
    pub kernel_ms: f64,
    meshes: facemesh::Cache,
    /// The finished part's faces' triangles, and all of them as one mesh, made as asked for.
    faces: Option<Vec<(forge_topo::FaceId, Option<Arc<FaceMesh>>)>>,
    mesh: Option<TriMesh>,
    /// Finds the font a text line names; text finds none without it.
    pub fonts: Option<Arc<Fonts>>,
    /// The finished part's exact mass properties, worked out as asked for.
    mass: Option<Result<MassProps, String>>,
    /// The source was just written by `edit`; the next update finds it unchanged.
    edited: bool,
}

/// From a font's name in the part file to the font.
pub type Fonts = dyn Fn(&str) -> Result<Arc<forge_font::Font>, String> + Send + Sync;

/// The bodies after a line, as the next line builds on them; all of them as one body to look at
/// (finished: faces on one surface joined); and as one body unfinished, to trace faces through.
struct Stage {
    solids: Vec<Solid>,
    view: OnceLock<Joined>,
    work: OnceLock<Body>,
}

/// Bodies as one, with each body's id and where its faces, edges and vertices end in it.
type Joined = (Body, Vec<(usize, [usize; 3])>);

fn stage_body(stages: &[Arc<Stage>], n: usize) -> Option<&Body> {
    let s = stages.get(n.checked_sub(1)?)?;
    match &s.solids[..] {
        [] => None,
        _ => Some(&s.view().0),
    }
}

impl Stage {
    fn new(solids: Vec<Solid>) -> Stage {
        Stage {
            solids,
            view: OnceLock::new(),
            work: OnceLock::new(),
        }
    }

    fn view(&self) -> &Joined {
        self.view.get_or_init(|| {
            if let [one] = &self.solids[..] {
                let b = forge_model::finish(one.body.clone());
                let ends = vec![(one.id, [b.faces.len(), b.edges.len(), b.vertices.len()])];
                return (b, ends);
            }
            let mut all = Body::new();
            let mut ends = Vec::new();
            for s in &self.solids {
                all.append(&forge_model::finish(s.body.clone()));
                ends.push((s.id, [all.faces.len(), all.edges.len(), all.vertices.len()]));
            }
            (all, ends)
        })
    }

    /// The bodies as one, unfinished: the pieces each feature made, for tracing them.
    fn work(&self) -> Option<&Body> {
        match &self.solids[..] {
            [] => None,
            [one] => Some(&one.body),
            _ => Some(self.work.get_or_init(|| {
                let mut all = Body::new();
                for s in &self.solids {
                    all.append(&s.body.compact());
                }
                all
            })),
        }
    }
}

/// A model as it was: see `Model::save`.
pub struct Saved {
    src: String,
    doc: Document,
    lines: Vec<Line>,
    features: Vec<Feature>,
    labels: Vec<Option<String>>,
    kinds: Vec<&'static str>,
    stages: Vec<Arc<Stage>>,
    picks: Vec<(usize, usize)>,
    error: Option<Failure>,
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
            doc: Document::parse(""),
            lines: vec![],
            features: vec![],
            refused: None,
            labels: vec![],
            kinds: vec![],
            stages: vec![],
            origins: RefCell::new(vec![]),
            picks: RefCell::new(vec![]),
            error: None,
            ms: 0.0,
            rebuilt: 0,
            kernel_ms: 0.0,
            meshes: facemesh::Cache::default(),
            faces: None,
            mesh: None,
            fonts: None,
            mass: None,
            edited: false,
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
            // just written by an edit, which timed its own rebuild
            if std::mem::take(&mut self.edited) {
                return;
            }
            self.rebuilt = 0;
            self.ms = t0.elapsed().as_secs_f64() * 1e3;
            return;
        }
        self.src = src.to_string();
        let mut doc = Document::parse(src);
        let new = doc.lines();
        let same = |a: &Line, b: &Line| a.id == b.id && a.label == b.label && a.toks == b.toks;
        let keep = self
            .lines
            .iter()
            .zip(&new)
            .take_while(|(a, b)| same(a, b))
            .count();
        doc.adopt(&self.doc, keep);
        self.doc = doc;
        self.rebuild(new, t0);
    }

    /// Applies an op to the document and rebuilds from the first line it changed. What comes
    /// back undoes it.
    pub fn edit(&mut self, op: Op) -> Result<Vec<Op>, String> {
        let t0 = Instant::now();
        let undo = self.doc.apply(op)?;
        self.commit_doc(t0);
        Ok(undo)
    }

    /// Rebuilds after ops applied to `doc` directly: several at once cost one rebuild.
    pub fn sync_doc(&mut self) {
        self.commit_doc(Instant::now());
    }

    fn commit_doc(&mut self, t0: Instant) {
        self.src = self.doc.print();
        self.rebuild(self.doc.lines(), t0);
        self.edited = true;
    }

    /// What `restore` goes back to. Stages are shared, so this costs little beside the text.
    pub fn save(&self) -> Saved {
        Saved {
            src: self.src.clone(),
            doc: self.doc.clone(),
            lines: self.lines.clone(),
            features: self.features.clone(),
            labels: self.labels.clone(),
            kinds: self.kinds.clone(),
            stages: self.stages.clone(),
            picks: self.picks.borrow().clone(),
            error: self.error.clone(),
        }
    }

    /// Back to as it was at `save`, nothing rebuilt.
    pub fn restore(&mut self, s: Saved) {
        self.src = s.src;
        self.doc = s.doc;
        self.lines = s.lines;
        self.features = s.features;
        self.labels = s.labels;
        self.kinds = s.kinds;
        self.stages = s.stages;
        *self.picks.get_mut() = s.picks;
        self.error = s.error;
        self.origins.get_mut().clear();
        self.faces = None;
        self.mesh = None;
        self.mass = None;
    }

    /// Writes `from (verts near X Y Z)` into each built fillet line where a varying radius was
    /// left to start where the geometry says (`forge_ops::law_starts`): fixed at its first build,
    /// it never turns round after. Each start written: its line and its `from` selector.
    pub fn fill_law_starts(&mut self) -> Vec<(usize, String)> {
        let mut edits = Vec::new();
        for i in 0..self.built().min(self.features.len()) {
            let Feature::Fillet { edges, starts, .. } = &self.features[i] else {
                continue;
            };
            let Some(body) = self.body_after(i) else {
                continue;
            };
            // the line's radius groups as the parser reads them: R [from VREF] REF..
            let toks = &self.lines[i].toks;
            let end = toks.len()
                - if toks.len() > 2 && toks[toks.len() - 2] == "in" {
                    2
                } else {
                    0
                };
            let mut groups: Vec<(usize, bool, Vec<Name>)> = vec![(1, false, Vec::new())];
            let mut k = 2;
            while k < end {
                let t = toks[k].as_str();
                let last = groups.last_mut().expect("a group");
                if t == "setback" {
                    k += 2 + toks.get(k + 2).is_some_and(|t| text::is_ref(t)) as usize + 1;
                } else if t == "from" {
                    last.1 = true;
                    k += 2;
                } else if !text::is_ref(t) {
                    groups.push((k, false, Vec::new()));
                    k += 1;
                } else {
                    last.2.extend(self.refs(i, t).unwrap_or_default());
                    k += 1;
                }
            }
            for (at, from, names) in groups {
                let picks: Option<Vec<forge_ops::FilletEdge>> = names
                    .iter()
                    .map(|n| {
                        let r = edges.iter().find(|e| e.0 == *n)?.1.clone();
                        Some(forge_ops::FilletEdge {
                            edge: body.edge_by_name(*n)?,
                            radius: r,
                        })
                    })
                    .collect();
                let Some(picks) = picks else { continue };
                if from
                    || picks.is_empty()
                    || picks[0].radius.constant().is_some()
                    || names.iter().any(|n| starts.iter().any(|s| s.0 == *n))
                {
                    continue;
                }
                let Ok(found) = forge_ops::law_starts(body, &picks, &[]) else {
                    continue;
                };
                let v = found[&picks[0].edge];
                // one vertex must give the whole group's starts back (a chain, or one edge)
                if forge_ops::law_starts(body, &picks, &[(picks[0].edge, v)]).ok() != Some(found) {
                    continue;
                }
                let p = body.vertex(v).point;
                let sel = format!("(verts near {} {} {})", near(p.x), near(p.y), near(p.z));
                if self.refs(i, &sel).ok() != Some(vec![body.vertex(v).name]) {
                    continue;
                }
                if let Some((item, _)) = self.doc.at_line(self.lines[i].no) {
                    edits.push((item, at, sel, self.lines[i].no));
                }
            }
        }
        let written: Vec<(usize, String)> = edits.iter().map(|e| (e.3, e.2.clone())).collect();
        // a line's later groups first, so the earlier ones' places hold
        for (item, at, sel, _) in edits.into_iter().rev() {
            if let forge_doc::Body::Feature { args, .. } = &mut self.doc.items[item].body {
                args.splice(at..at, [forge_doc::Arg::new("from"), forge_doc::Arg::new(sel)]);
            }
        }
        if !written.is_empty() {
            let t0 = Instant::now();
            self.src = self.doc.print();
            self.rebuild(self.doc.lines(), t0);
        }
        written
    }

    /// The file as the document prints it.
    pub fn src(&self) -> &str {
        &self.src
    }

    fn rebuild(&mut self, new: Vec<Line>, t0: Instant) {
        let same = |a: &Line, b: &Line| a.id == b.id && a.label == b.label && a.toks == b.toks;
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
        self.refused = None;
        self.rebuilt = 0;
        self.kernel_ms = 0.0;
        let mut reuse = true;
        let ids: Vec<u64> = self.lines.iter().map(|l| l.id).collect();
        for i in keep..self.lines.len() {
            self.rebuilt += 1;
            let line = self.lines[i].clone();
            self.labels.push(line.label.clone());
            if let Some(k) = ids[..i].iter().position(|&d| d == line.id) {
                let msg = format!("id {} is line {}'s too", line.id, self.lines[k].no);
                self.fail(i, msg, "each line's id is its own; leave a new line's off".into());
                break;
            }
            let parsed = {
                let refs = |r: &str| self.refs(i, r);
                let face = |r: &str| self.face(i, r);
                let fonts = |n: &str| self.fonts.as_ref().map(|f| f(n)).transpose();
                let own = |k: usize, n: Name| {
                    self.body_after(i)
                        .and_then(|b| {
                            forge_model::work::own_name(
                                History::new(&self.features, Some(&ids[..i])),
                                k,
                                b,
                                n,
                            )
                        })
                        .unwrap_or(n)
                };
                let b = Before {
                    features: &self.features,
                    ids: &ids[..i],
                    labels: &self.labels[..i],
                    kinds: &self.kinds,
                    refs: &refs,
                    face: &face,
                    fonts: &fonts,
                    was: old_features.get(i - keep),
                    own: &own,
                };
                text::parse_line_refs(&line, &b)
            };
            let parsed = parsed.map(|(f, refs)| {
                self.doc.link(i, &refs);
                f
            });
            let param = line.toks.get(1).is_some_and(|t| t == "=");
            let f = match parsed {
                Ok(f) => f,
                // until a line has parsed, this may not be a part file: echo none of it
                Err(_) if i == 0 && !param && text::kind(&line.toks[0]).is_none() => {
                    let msg = "the first line is not a feature; is this a .fc part file?".into();
                    self.fail(i, msg, "`help` lists the feature kinds".into());
                    break;
                }
                Err(msg) => {
                    let hint = match text::kind(if param { "param" } else { &line.toks[0] }) {
                        _ if msg.contains("is implied by the rows above it") => {
                            "Inventor's rule: a sketch says each thing once; `ref` before a dimension only shows it".into()
                        }
                        _ if msg.starts_with("the sketch does not solve") => {
                            "change or remove one of those rows; a sketch may be under-constrained".into()
                        }
                        Some(k) => k.syntax.to_string(),
                        None => "`help` lists the feature kinds".into(),
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
            let before = if i == 0 {
                vec![]
            } else {
                self.stages[i - 1].solids.clone()
            };
            let t = Instant::now();
            let stepped = step_solids(History::new(&self.features, Some(&ids[..=i])), i, before);
            self.kernel_ms += t.elapsed().as_secs_f64() * 1e3;
            match stepped {
                Ok(solids) => self.stages.push(Arc::new(Stage::new(solids))),
                Err(e) => {
                    let (msg, hint) = explain(&e);
                    let msg = match self.body_after(i) {
                        Some(b) => with_handles(&msg, b),
                        None => msg,
                    };
                    let before = if i == 0 {
                        &[][..]
                    } else {
                        &self.stages[i - 1].solids[..]
                    };
                    let msg = msg + &size_limit(&self.features, &ids[..=i], before, &e);
                    self.fail(i, msg, hint);
                    break;
                }
            }
        }
        if self.stages.len() != old || !reuse {
            self.faces = None;
            self.mesh = None;
            self.mass = None;
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
        self.refused = self.features.get(at).cloned();
        self.features.truncate(at);
        self.labels.truncate(at);
        self.kinds.truncate(at);
        self.error = Some(Failure { at, msg, hint });
    }

    /// Bodies a line took whole: (its line number, the line number that made the body). A cut
    /// that leaves nothing of one body of several deletes it.
    pub fn removed_bodies(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in 1..self.built() {
            if matches!(self.features[i], Feature::Body(ref b) if !matches!(**b, forge_model::BodyFeature::In { .. }))
            {
                continue;
            }
            let after: Vec<usize> = self.solids_after(i + 1).iter().map(|s| s.id).collect();
            for s in self.solids_after(i) {
                if !after.contains(&s.id) {
                    out.push((self.lines[i].no, self.lines[s.id].no));
                }
            }
        }
        out
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

    /// The part after `n` lines (several bodies as one, to look at).
    pub fn body_after(&self, n: usize) -> Option<&Body> {
        stage_body(&self.stages, n)
    }

    /// The lines (their numbers) that built and left the part as it was: a tool that misses, or
    /// lies wholly inside what it adds to. Lines that only define something are not among them.
    pub fn no_effect(&self) -> Vec<usize> {
        let shape = |n: usize| {
            let mut out: Vec<(Vec<Name>, Vec<[u64; 3]>)> = self
                .solids_after(n)
                .iter()
                .map(|s| {
                    let b = &s.body;
                    let mut faces: Vec<Name> = b.face_ids().map(|f| b.face(f).name).collect();
                    let mut verts: Vec<[u64; 3]> = b
                        .vertex_ids()
                        .map(|v| {
                            let p = b.vertex(v).point;
                            [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
                        })
                        .collect();
                    faces.sort_unstable();
                    verts.sort_unstable();
                    (faces, verts)
                })
                .collect();
            out.sort();
            out
        };
        (1..self.built())
            .filter(|&i| {
                !matches!(
                    self.features[i],
                    Feature::Param(_) | Feature::Sketch(_) | Feature::Plane(_) | Feature::Work(_)
                ) && !self.solids_after(i).is_empty()
                    && shape(i) == shape(i + 1)
            })
            .map(|i| self.lines[i].no)
            .collect()
    }

    /// The bodies after `n` lines.
    pub fn solids_after(&self, n: usize) -> &[Solid] {
        n.checked_sub(1)
            .and_then(|i| self.stages.get(i))
            .map_or(&[], |s| &s.solids[..])
    }

    /// The body (the feature that made it) an entity of `body_after(n)` belongs to.
    pub fn body_of(&self, n: usize, e: Ent) -> Option<usize> {
        let s = self.stages.get(n.checked_sub(1)?)?;
        if let [one] = &s.solids[..] {
            return Some(one.id);
        }
        let at = match e {
            Ent::Face(f) => (0, f.index()),
            Ent::Edge(x) => (1, x.index()),
            Ent::Vert(v) => (2, v.index()),
        };
        s.view().1.iter().find(|(_, end)| at.1 < end[at.0]).map(|b| b.0)
    }

    /// The finished part's faces' triangles (None where a face would not mesh).
    pub fn faces(&mut self) -> &[(forge_topo::FaceId, Option<Arc<FaceMesh>>)] {
        if self.faces.is_none() {
            let fs = match stage_body(&self.stages, self.stages.len()) {
                Some(b) => self.meshes.faces(b),
                None => vec![],
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

    /// The finished part's exact mass properties (from its surfaces, not triangles).
    pub fn mass(&mut self) -> Result<&MassProps, String> {
        if self.mass.is_none() {
            let m = match self.body() {
                Some(b) => forge_topo::mass::mass(b).map_err(|e| e.to_string()),
                None => Err("no part yet".into()),
            };
            self.mass = Some(m);
        }
        self.mass
            .as_ref()
            .expect("just made")
            .as_ref()
            .map_err(|e| e.clone())
    }

    /// The finished part's volume and area (exact, or from the triangles where that fails, with
    /// why), its box (from each face's cached triangles and its edges), and how many faces
    /// would not mesh (their own, looser, boxes stand in).
    pub fn stats(&mut self) -> (f64, f64, forge_math::Box3, usize, Option<String>) {
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
                            for c in b.lp(l).coedges() {
                                let e = b.coedge(c).edge;
                                // a line's or circle's box is tight; other curves' boxes are bounds
                                bb = match b.edge(e).curve {
                                    forge_geom::Curve::Line(_) | forge_geom::Curve::Circle(_) => {
                                        bb.hull(&b.edge_box(e))
                                    }
                                    _ => b
                                        .edge_points(e, 64)
                                        .into_iter()
                                        .fold(bb, |x, p| x.hull(&Box3::from_point(p))),
                                };
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
        match self.mass() {
            Ok(m) => (m.volume, m.area, bb, bad, None),
            Err(e) => (v, a, bb, bad, Some(e)),
        }
    }

    /// Which feature made each face of the part after `n` lines.
    fn origin(&self, n: usize) -> select::Lineage {
        let mut origins = self.origins.borrow_mut();
        while origins.len() < n {
            let k = origins.len();
            let work = |n: usize| self.stages.get(n.checked_sub(1)?)?.work();
            let prev = work(k);
            let copied = match (&self.features[k], work(k + 1), prev) {
                (Feature::Pattern { of, layout }, Some(b), Some(pb)) => {
                    let from = |c: &Vec<usize>| match of {
                        Source::Body => true,
                        Source::Features(src) => c.iter().any(|x| src.contains(x)),
                    };
                    let ids: Vec<u64> = self.lines[..self.features.len()].iter().map(|l| l.id).collect();
                    let h = History::new(&self.features, Some(&ids));
                    select::copies(b, pb, &origins[k - 1], k, h, layout, &from)
                }
                _ => HashMap::new(),
            };
            let o = match work(k + 1) {
                None => HashMap::new(),
                Some(b) => select::lineage(b, prev.map(|p| (p, &origins[k - 1])), k, &copied),
            };
            origins.push(o);
        }
        n.checked_sub(1).map(|i| origins[i].clone()).unwrap_or_default()
    }

    /// Which feature made each face of the finished part, by the body's face index.
    pub fn face_makers(&self) -> Vec<Option<usize>> {
        self.face_makers_at(self.built())
    }

    /// The same of the part after `n` lines.
    pub fn face_makers_at(&self, n: usize) -> Vec<Option<usize>> {
        let origin = self.origin(n);
        let Some(b) = self.body_after(n) else {
            return Vec::new();
        };
        let mut out = vec![None; b.faces.len()];
        for f in b.face_ids() {
            out[f.index()] = origin.get(&b.face(f).name).and_then(|m| m.last().copied());
        }
        out
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

    /// How a line is referred to: its label, else its kind and count (`hole2`).
    pub fn feat_name(&self, i: usize) -> String {
        if let Some(l) = &self.labels[i] {
            return l.clone();
        }
        let k = self.kinds[i];
        format!("{k}{}", self.kinds[..=i].iter().filter(|x| **x == k).count())
    }

    /// Runs a selector on the part after `n` lines (all that built if `None`).
    pub fn select(&self, sel: &str, n: Option<usize>) -> Result<(&Body, Vec<Ent>), String> {
        let n = n.unwrap_or(self.built()).min(self.built());
        let body = self.body_after(n).ok_or("no part yet")?;
        // a joined face was made by every feature that made one of its pieces
        let mut origin = self.origin(n);
        for &(piece, whole) in &body.joins.aliases {
            if let Some(mut o) = origin.get(&piece).cloned() {
                let w = origin.entry(whole).or_default();
                o.append(w);
                o.sort_unstable();
                o.dedup();
                *w = o;
            }
        }
        let feat = |r: &str| self.feat(n, r);
        let body_of = |e: Ent| self.body_of(n, e);
        let ents = select::select(
            &View {
                body,
                origin: &origin,
                feat: &feat,
                body_of: &body_of,
            },
            sel,
        )?;
        Ok((body, ents))
    }

    /// The entities a handle or selector names on the finished part.
    pub fn select_ref(&self, r: &str) -> Result<Vec<Name>, String> {
        if let Some(n) = text::full_name(r) {
            return Ok(vec![n]);
        }
        if r.starts_with('@') {
            let body = self.body().ok_or("no part yet")?;
            return Ok(vec![select::name(body, select::by_handle(body, r)?)]);
        }
        let (body, ents) = self.select(r, None)?;
        Ok(ents.into_iter().map(|e| select::name(body, e)).collect())
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
/// For a finishing feature refused as too big for the part: the size it does build at, found by
/// building it smaller (`; it builds up to radius R`), or that no smaller one builds, so the size
/// is not what stops it. Empty for any other refusal.
fn size_limit(features: &[Feature], ids: &[u64], before: &[forge_model::Solid], e: &RegenError) -> String {
    use forge_ops::{ChamferError as C, FilletError as F, ShellError as S};
    let i = features.len() - 1;
    let sized = matches!(
        e,
        RegenError::Fillet {
            error: F::Overflow { .. } | F::NoBlend { .. } | F::Corner { .. } | F::Tangency { .. },
            ..
        } | RegenError::Chamfer {
            error: C::TooLarge { .. } | C::Overflows { .. } | C::Rebuild(_),
            ..
        } | RegenError::Shell {
            error: S::Collapses { .. } | S::SelfIntersecting { .. } | S::Rebuild(_),
            ..
        }
    );
    let Some((what, size)) = sized.then(|| sized_as(&features[i])).flatten() else {
        return String::new();
    };
    let builds = |k: f64| {
        let mut fs = features.to_vec();
        fs[i] = scaled(&features[i], k);
        let h = History::new(&fs, Some(ids));
        catch_unwind(AssertUnwindSafe(|| {
            step_solids(h, i, before.to_vec())
                .is_ok_and(|ss| ss.iter().all(|s| forge_topo::check(&s.body).is_empty()))
        }))
        .unwrap_or(false)
    };
    let mut k = 0.5;
    while k > 0.02 && !builds(k) {
        k *= 0.5;
    }
    if k <= 0.02 {
        return format!("; no smaller {what} builds either, so the size is not the reason");
    }
    let (mut lo, mut hi) = (k, 2.0 * k);
    for _ in 0..4 {
        let m = 0.5 * (lo + hi);
        if builds(m) {
            lo = m;
        } else {
            hi = m;
        }
    }
    format!("; it builds up to {what} {}", crate::short(lo * size))
}

/// What a finishing feature's size is called, and its largest value.
fn sized_as(f: &Feature) -> Option<(&'static str, f64)> {
    use forge_ops::{ChamferSize, Radius};
    let top = |r: &Radius| match r {
        Radius::Constant(v) => *v,
        Radius::Linear(a, b) => a.max(*b),
        Radius::Points(ps) => ps.iter().fold(0.0, |m, p| p.1.max(m)),
    };
    match f {
        Feature::Fillet { edges, .. } => Some(("radius", edges.iter().fold(0.0, |m, e| top(&e.1).max(m)))),
        Feature::FaceFillet { radius, .. } => Some(("radius", *radius)),
        Feature::Chamfer { size, .. } => Some((
            "distance",
            match size {
                ChamferSize::Equal(d) | ChamferSize::DistanceAngle { d, .. } => *d,
                ChamferSize::Distances { d1, d2, .. } => d1.max(*d2),
            },
        )),
        Feature::Shell { thickness, .. } => Some(("thickness", *thickness)),
        _ => None,
    }
}

/// The feature with its size times `k`.
fn scaled(f: &Feature, k: f64) -> Feature {
    use forge_ops::{ChamferSize, Radius};
    let mut f = f.clone();
    match &mut f {
        Feature::Fillet { edges, .. } => {
            for (_, r) in edges {
                *r = match &*r {
                    Radius::Constant(v) => Radius::Constant(v * k),
                    Radius::Linear(a, b) => Radius::Linear(a * k, b * k),
                    Radius::Points(ps) => Radius::Points(ps.iter().map(|p| (p.0, p.1 * k)).collect()),
                };
            }
        }
        Feature::FaceFillet { radius, .. } => *radius *= k,
        Feature::Chamfer { size, .. } => match size {
            ChamferSize::Equal(d) | ChamferSize::DistanceAngle { d, .. } => *d *= k,
            ChamferSize::Distances { d1, d2, .. } => {
                *d1 *= k;
                *d2 *= k;
            }
        },
        Feature::Shell { thickness, .. } => *thickness *= k,
        _ => {}
    }
    f
}

pub fn explain(e: &RegenError) -> (String, String) {
    use RegenError::*;
    let hint = |s: &str| s.to_string();
    match e {
        Extrude { error, .. } => (
            format!("extrude: {error:?}"),
            hint(if matches!(error, forge_ops::ExtrudeError::TooShort) {
                "the distance is (about) zero: extrude a real length"
            } else {
                "check the loops: closed, not crossing, holes inside the outer loop"
            }),
        ),
        Revolve { error, .. } => (
            format!("revolve: {error:?}"),
            hint("keep the profile on one side of the axis, off it"),
        ),
        Boolean {
            error: forge_bool::BoolError::ZeroThickness { at },
            ..
        } => (
            format!(
                "boolean: ZeroThickness: this line leaves material of no thickness at ({}, {}, {}), where the solids only touch",
                crate::short(at.x),
                crate::short(at.y),
                crate::short(at.z)
            ),
            hint("this cannot be made: move one apart, or overlap them"),
        ),
        Boolean { error, .. } => (
            format!("boolean: {error:?}"),
            hint("move or resize the tool so it does not just touch the part"),
        ),
        Fillet {
            error: forge_ops::FilletError::TooLarge { max, .. },
            ..
        } => (
            format!(
                "fillet: the round's ball is wider than the ring it rolls round; it needs a radius under {}",
                crate::short(*max)
            ),
            hint(
                "a round of this ring works up to that radius; for a full round, make the ring thinner or wider",
            ),
        ),
        Fillet { error, .. } => (
            format!("fillet: {error:?}"),
            hint("try a smaller radius, or fewer edges at once"),
        ),
        NotClosed { feature, what } => (
            format!("this leaves no closed solid (feature {feature}): {what}"),
            hint("a boolean lost a face there; a kernel bug: nudge a dimension and report the part"),
        ),
        NoReferences { .. } => (
            "names nothing to work on".to_string(),
            hint("give it @handles or a (selector)"),
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
            hint("the faces must be planar and meet the neutral plane; draft before rounding their edges"),
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
        Thread { error, .. } => (
            format!("thread: {error:?}"),
            hint(
                "a spec that fits its cylinder: ext round a rod of the major (M6: 6), int in a bore of the minor, \
                 major - 1.0825 pitch (M36x2: 33.835; a tap drill, major - pitch, is close); a positive length",
            ),
        ),
        Text { error, .. } => (
            format!("text: {error:?}"),
            hint(
                "a TrueType (glyf) font file, a positive size and depth; wrapped text shorter than the cylinder's round",
            ),
        ),
        Work { error, .. } => (
            format!("work feature: {error:?}"),
            hint("REF is a work feature, or FEAT @h: a face, edge or vertex of FEAT's own solid"),
        ),
        Body { error, .. } => (
            format!("body: {error:?}"),
            hint("`in FEAT` names the line that made a body; split and splitface need a cutter that crosses"),
        ),
        Bodies { count } => (
            format!("the part is {count} bodies, not one"),
            hint("combine them, or ask for every body"),
        ),
        NothingToCombine { .. } => (
            "nothing to combine with yet".into(),
            hint("start with a solid: extrude, revolve or ball (op new)"),
        ),
        Empty { .. } => (
            "this leaves nothing of the part".into(),
            hint("a cut or keep must leave some of the part"),
        ),
    }
}

/// A coordinate as `fill_law_starts` writes it: four decimals, trailing zeros dropped.
fn near(x: f64) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

#[cfg(test)]
mod tests {

    /// A fillet between a feature's pattern copies and the part finds its edges on the n-ary
    /// path too (pattern copies, standing flush on the part, named alike on both).
    #[test]
    fn fillet_after_a_pattern_builds_both_ways() {
        let src = "extrude 95 rect -120 -65 240 130\nextrude 44 xz at 62 87 95 loop line -42 0 42 0 arc 0 0 42 0 -180\nfeet: extrude 16 loop line -96 -55 -96 -85 arc -88 -85 8 180 90 line -88 -93 -60 -93 arc -60 -85 8 270 90 line -52 -85 -52 -55 line -52 -55 -96 -55\npattern feet lin 1 0 0 2 148 0 1 0 2 146\nfillet 5 (edges between feet f1 concave)\n";
        let m = Model::new(src);
        assert!(m.error.is_none(), "{:?}", m.error);
        let part = forge_model::Part {
            features: m.features.clone(),
        };
        part.regenerate_nary(1).expect("n-ary regen");
    }

    use super::*;
    use forge_model::{Part, fillets, suite};

    fn finished(m: &Model) -> Body {
        m.body().expect("built").compact()
    }

    /// A boss on a block, and the same moved by an inexact rotation: a boolean leaves a vertex
    /// between two pieces of one line in one and not the other; unify takes it out on every face,
    /// so both have the same vertices and edges (forgecad-gate/repros M1).
    #[test]
    fn no_vertex_left_between_pieces_of_one_line() {
        let counts = |src: &str| {
            let m = Model::new(src);
            assert!(m.error.is_none(), "{:?}", m.error);
            let b = finished(&m);
            (b.vertices.len(), b.edge_ids().count())
        };
        let placed = counts(
            "extrude new 17 rect 0 0 82 138\n\
         extrude add 15.625457658136117 axes 0.6111332437811676 0.7772515422987627 -0.14965359447638588 -0.7694654469826927 0.6277120352052737 0.11790049515662869 0.18557770406283436 0.04310035791786255 0.9816838976483822 at 76.92553480349156 114.11085208111551 9.80044547195848 rrect -7.706843451315201 -3.8327248781946053 15.413686902630403 7.665449756389211 2.6191965533564114\n",
        );
        let moved = counts(
            "extrude new 17 axes 0.49940546967895366 0.5243757431629014 0.6896551724137931 -0.8149821640903687 0.014268727705112894 0.5793103448275863 0.29393579072532705 -0.8513674197384067 0.43448275862068964 at 4 65 93 rect 0 0 82 138\n\
         extrude add 15.625457658136117 axes -0.3722314069655698 0.4589640340786829 0.8067191550289692 -0.8611941905585102 -0.49490900383953673 -0.11580001755971503 0.3461045301880708 -0.7378462531861086 0.5794778346429791 at -47.70057464716051 98.62232118358013 216.4158146199741 rrect -7.706843451315201 -3.8327248781946053 15.413686902630403 7.665449756389211 2.6191965533564114\n",
        );
        assert_eq!(placed, moved);
        assert_eq!(placed, (28, 42));
    }

    /// Two blocks joined into an L: faces on one plane are one face, as in the agents' trial
    /// (forgecad-gate/repros/union-coplanar-faces-unmerged.fc).
    #[test]
    fn an_l_of_two_blocks_has_eight_faces() {
        let src = "extrude 5 rect 0 0 60 40\nextrude 5 xz at 0 5 0 rect 0 0 60 40\n";
        let m = Model::new(src);
        let b = finished(&m);
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
        assert_eq!((b.face_ids().count(), b.edge_ids().count()), (8, 18));
    }

    /// A block flush with both faces of a disc (a conrod's bolt boss), two blind cuts ending on
    /// one plane, and two pads flush with a box: the sequential and n-ary regens give the same
    /// faces, the flush ends one face each, and a chamfer there builds (forgecad-gate/repros
    /// R36, R26, M3, R30).
    #[test]
    fn flush_ends_are_one_face_both_ways() {
        let disc = "big: extrude sym 22 xz circle 0 0 31\nb: extrude sym 22 xz rect 26 -24 16 46\n";
        let cases = [
            (disc.to_string(), 8),
            (
                "big: extrude sym 22 xz circle 0 0 31\nbx: extrude sym 22 xz rect 20 -10 30 20\n\
                 bigbore: hole 48 xz at 0 11 0 thru\nchamfer 1 (edges of bigbore circle)\n"
                    .to_string(),
                0,
            ),
            (
                "extrude new 80 rect 0 0 200 150\nextrude cut 13 yz at 6 28 45 circle 0 0 2\n\
                 extrude cut 9 yz at 6 26 45 circle 0 0 1\n"
                    .to_string(),
                11,
            ),
            // the box's bottom split twice in seq, once in n-ary: one original either way (R30)
            (
                "box: extrude 95 rect -120 -65 240 130\na: extrude 16 rect -96 -93 44 34\n\
                 b: extrude 16 rect 52 -93 44 34\n"
                    .to_string(),
                14,
            ),
        ];
        for (src, faces) in cases {
            let m = Model::new(&src);
            assert!(m.error.is_none(), "{src}: {:?}", m.error);
            let seq = finished(&m);
            let part = Part {
                features: m.features.clone(),
            };
            let nary = part.regenerate_nary(1).expect("n-ary").compact();
            assert!(forge_topo::check(&nary).is_empty(), "{src}");
            // the same faces, by name
            let names = |b: &Body| {
                let mut n: Vec<Name> = b.face_ids().map(|f| b.face(f).name).collect();
                n.sort();
                n
            };
            assert_eq!(names(&seq), names(&nary), "{src}");
            if faces > 0 {
                assert_eq!(seq.face_ids().count(), faces, "{src}");
            }
        }
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
        assert!(f.hint.starts_with("hole (D | SIZE"), "{}", f.hint);
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
