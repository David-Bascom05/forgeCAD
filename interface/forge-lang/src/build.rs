//! Building as the program evaluates: each feature goes to the kernel as it is made, so later
//! anchors and selections read the part as it stands. Stages are kept, and a session reuses
//! the stages of a run whose features (and ids) are the same up to that point.

use forge_agent::select::{self, Lineage};
use forge_model::{Feature, History, RegenError, Solid, step_solids};
use forge_topo::{Body, FaceId, Name};
use std::any::Any;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

pub struct Stage {
    pub solids: Vec<Solid>,
    view: OnceCell<Body>,
    spans: OnceCell<Vec<Span>>,
}

/// Where one solid's faces and edges are in the stage's joined body.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub solid: usize,
    pub faces: std::ops::Range<u32>,
    pub edges: std::ops::Range<u32>,
}

impl Stage {
    pub fn new(solids: Vec<Solid>) -> Stage {
        Stage {
            solids,
            view: OnceCell::new(),
            spans: OnceCell::new(),
        }
    }

    /// The bodies as one, faces on one surface joined: what selections look at.
    pub fn view(&self) -> Option<&Body> {
        if self.solids.is_empty() {
            return None;
        }
        Some(self.view.get_or_init(|| {
            let mut all = Body::new();
            let mut spans = vec![];
            for s in &self.solids {
                let one = forge_model::finish(s.body.clone());
                let (f, e) = (all.faces.len() as u32, all.edges.len() as u32);
                if self.solids.len() == 1 {
                    all = one;
                } else {
                    all.append(&one);
                }
                spans.push(Span {
                    solid: s.id,
                    faces: f..all.faces.len() as u32,
                    edges: e..all.edges.len() as u32,
                });
            }
            let _ = self.spans.set(spans);
            all
        }))
    }

    /// Each solid's faces and edges in `view()`'s body.
    pub fn spans(&self) -> &[Span] {
        let _ = self.view();
        self.spans.get().map_or(&[], |s| &s[..])
    }

    /// The bodies as one, unfinished: the pieces each feature made, for lineage.
    fn work(&self) -> Option<Body> {
        match &self.solids[..] {
            [] => None,
            [one] => Some(one.body.clone()),
            many => {
                let mut all = Body::new();
                for s in many {
                    all.append(&s.body.compact());
                }
                Some(all)
            }
        }
    }
}

/// The last run's history and stages, for reuse.
#[derive(Default)]
pub struct Cache {
    features: Vec<Feature>,
    ids: Vec<u64>,
    stages: Vec<Rc<Stage>>,
    /// The finished part's measures, with the stage they were taken of.
    pub measured: Option<(Rc<Stage>, crate::Measure)>,
    /// An assembly's loaded parts, solver and checker, kept between runs (`asm.rs`).
    pub asm: Option<Box<crate::asm::AsmCache>>,
    /// Work done outside the kernel's history (a sheet's builds and shop checks), by a key
    /// that holds everything it read.
    memo: HashMap<String, Rc<dyn Any>>,
    /// Stages built before, by the whole history up to them (`Step`): an edit made and taken
    /// back, or tried again, builds nothing.
    kept: Kept,
    /// The steps of the last run's history, by position (as `stages`).
    steps: Vec<Option<Rc<Step>>>,
}

/// One step of a history: the step before it (by identity), and this one's feature as text with
/// every literal (`{:?}`: shortest round-trip numbers, enum names) and its id. Two steps are
/// equal exactly when they are the same object: a lookup compares the full key, no hash trusted.
pub struct Step {
    before: Option<Rc<Step>>,
    text: String,
    id: u64,
}

/// Stages by step, the least lately used dropped past `KEPT`.
#[derive(Default)]
struct Kept {
    /// By a hash of the key (`bucket`), each entry the full key and its stage.
    stages: HashMap<u128, Vec<(Rc<Step>, Rc<Stage>, u64)>>,
    len: usize,
    tick: u64,
}

/// How many stages a session keeps beyond its last run's.
const KEPT: usize = 512;
/// How many memo entries (`Builder::memo`) a session keeps beyond its last run's.
const MEMO_KEPT: usize = 128;

/// A step's text, None when it can't be keyed: its feature reads what its text does not hold (an
/// imported file's body, a font), or a step before it could not be.
fn keyed(steps: &[Option<Rc<Step>>], f: &Feature) -> Option<String> {
    let reads_a_file = |f: &Feature| matches!(f, Feature::Import(_) | Feature::Text(_));
    let inner = match f {
        Feature::Body(b) => match &**b {
            forge_model::BodyFeature::In { feature, .. } => reads_a_file(feature),
            _ => false,
        },
        _ => false,
    };
    if steps.last().is_some_and(|s| s.is_none()) || reads_a_file(f) || inner {
        return None;
    }
    Some(format!("{f:?}"))
}

fn bucket(before: &Option<Rc<Step>>, text: &str, id: u64) -> u128 {
    let mut h = forge_math::hash::Hash128::new();
    h.word(before.as_ref().map_or(0, |b| Rc::as_ptr(b) as usize as u64));
    h.word(id);
    h.str(text);
    h.finish()
}

impl Kept {
    /// The step and stage kept for exactly this key.
    fn get(&mut self, before: &Option<Rc<Step>>, text: &str, id: u64) -> Option<(Rc<Step>, Rc<Stage>)> {
        self.tick += 1;
        let t = self.tick;
        let e = self
            .stages
            .get_mut(&bucket(before, text, id))?
            .iter_mut()
            .find(|e| {
                e.0.id == id
                    && e.0.text == text
                    && match (&e.0.before, before) {
                        (None, None) => true,
                        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                        _ => false,
                    }
            })?;
        e.2 = t;
        Some((e.0.clone(), e.1.clone()))
    }

    fn put(&mut self, step: Rc<Step>, st: Rc<Stage>) {
        self.tick += 1;
        if self.len >= KEPT
            && let Some((b, k)) = self
                .stages
                .iter()
                .flat_map(|(b, v)| v.iter().enumerate().map(move |(k, e)| (*b, k, e.2)))
                .min_by_key(|x| x.2)
                .map(|x| (x.0, x.1))
            && let Some(v) = self.stages.get_mut(&b)
        {
            v.swap_remove(k);
            self.len -= 1;
            if v.is_empty() {
                self.stages.remove(&b);
            }
        }
        let b = bucket(&step.before, &step.text, step.id);
        let v = self.stages.entry(b).or_default();
        if !v.iter().any(|e| Rc::ptr_eq(&e.0, &step)) {
            v.push((step, st, self.tick));
            self.len += 1;
        }
    }
}

pub struct Builder {
    pub features: Vec<Feature>,
    pub ids: Vec<u64>,
    /// The part after each built feature.
    pub stages: Vec<Rc<Stage>>,
    /// Whether to send features to the kernel (false in `check`, and after a failure).
    pub building: bool,
    origins: Vec<Lineage>,
    pub(crate) cache: Cache,
    /// The cached stages still matching this run's prefix.
    reuse: bool,
    /// Whether a step built here (a sheet) holds bodies the kernel's history cannot step: then
    /// features that make nothing pass the bodies on as they are.
    hosted: bool,
    pub kernel_ms: f64,
    pub rebuilt: usize,
    pub reused: usize,
    /// The memo entries this run used or made: what the next run keeps.
    memo: HashMap<String, Rc<dyn Any>>,
    /// Memo entries worked out afresh this run.
    pub remade: usize,
    /// The steps of this run's history so far, and whether they are all keyed: not after a step
    /// that reads what its text does not hold (an imported file, a font).
    steps: Vec<Option<Rc<Step>>>,
}

impl Builder {
    pub fn new(building: bool, cache: Cache) -> Builder {
        Builder {
            features: vec![],
            ids: vec![],
            stages: vec![],
            building,
            origins: vec![],
            cache,
            reuse: true,
            hosted: false,
            kernel_ms: 0.0,
            rebuilt: 0,
            reused: 0,
            memo: HashMap::new(),
            remade: 0,
            steps: vec![],
        }
    }

    /// The value `key` stands for: the last run's or this one's if either worked it out (only
    /// while building, as stages are), else `make`'s, kept for the next run.
    pub fn memo<T: Clone + 'static>(&mut self, key: String, make: impl FnOnce() -> T) -> T {
        if let Some(v) = self.memo.get(&key).and_then(|v| v.downcast_ref::<T>()) {
            return v.clone();
        }
        if let Some(v) = self.cache.memo.remove(&key)
            && let Some(t) = v.downcast_ref::<T>()
        {
            let t = t.clone();
            self.memo.insert(key, v);
            return t;
        }
        let t = make();
        self.remade += 1;
        if self.building {
            self.memo.insert(key, Rc::new(t.clone()));
        }
        t
    }

    pub fn into_cache(self) -> Cache {
        // this run's memo entries, and as many of the ones before it as there is room for: an edit
        // taken back finds its work done
        let mut memo = self.memo;
        for (k, v) in self.cache.memo {
            if memo.len() >= MEMO_KEPT {
                break;
            }
            memo.entry(k).or_insert(v);
        }
        Cache {
            features: self.features,
            ids: self.ids,
            stages: self.stages,
            measured: self.cache.measured,
            asm: self.cache.asm,
            memo,
            kept: self.cache.kept,
            steps: self.steps,
        }
    }

    /// The measures of the finished part, if the last run took them of this same stage.
    pub fn measured(&self) -> Option<crate::Measure> {
        let (st, m) = self.cache.measured.as_ref()?;
        Rc::ptr_eq(st, self.stages.last()?).then(|| m.clone())
    }

    pub fn keep_measure(&mut self, m: crate::Measure) {
        if let Some(st) = self.stages.last() {
            self.cache.measured = Some((st.clone(), m));
        }
    }

    /// Adds a feature and, when building, builds it onto the part. Returns its index.
    pub fn push(&mut self, f: Feature, id: u64) -> Result<usize, RegenError> {
        let i = self.features.len();
        let text = keyed(&self.steps, &f);
        self.features.push(f);
        self.ids.push(id);
        if !self.building {
            return Ok(i);
        }
        if self.reuse
            && self.cache.features.get(i) == Some(&self.features[i])
            && self.cache.ids.get(i) == Some(&id)
            && let Some(st) = self.cache.stages.get(i)
        {
            self.stages.push(st.clone());
            self.reused += 1;
            let step = self.cache.steps.get(i).cloned().flatten();
            self.steps.push(step);
            return Ok(i);
        }
        self.reuse = false;
        if let Some(text) = &text
            && let Some((step, st)) = self.cache.kept.get(&self.before(), text, id)
        {
            self.stages.push(st);
            self.steps.push(Some(step));
            self.reused += 1;
            return Ok(i);
        }
        if self.hosted
            && matches!(
                self.features[i],
                Feature::Sketch(_) | Feature::Param(_) | Feature::Plane(_) | Feature::Work(_)
            )
            && let Some(last) = self.stages.last()
        {
            self.stages.push(last.clone());
            self.keep(text, id);
            return Ok(i);
        }
        let before = self.stages.last().map(|s| s.solids.clone()).unwrap_or_default();
        let t = Instant::now();
        let r = step_solids(History::new(&self.features, Some(&self.ids)), i, before);
        self.kernel_ms += t.elapsed().as_secs_f64() * 1e3;
        self.rebuilt += 1;
        match r {
            Ok(solids) => {
                self.stages.push(Rc::new(Stage::new(solids)));
                self.keep(text, id);
                Ok(i)
            }
            Err(e) => {
                self.building = false;
                self.steps.push(None);
                Err(e)
            }
        }
    }

    /// Adds a step built here, not by the kernel's history (a sheet): `f` stands for it in the
    /// history (equal exactly when the step is), `make` gives the bodies after it.
    pub fn push_with<E>(
        &mut self,
        f: Feature,
        id: u64,
        make: impl FnOnce(&[Solid]) -> Result<Vec<Solid>, E>,
    ) -> Result<usize, E> {
        let i = self.features.len();
        let text = keyed(&self.steps, &f);
        self.features.push(f);
        self.ids.push(id);
        self.hosted = true;
        if !self.building {
            return Ok(i);
        }
        if self.reuse
            && self.cache.features.get(i) == Some(&self.features[i])
            && self.cache.ids.get(i) == Some(&id)
            && let Some(st) = self.cache.stages.get(i)
        {
            self.stages.push(st.clone());
            self.reused += 1;
            let step = self.cache.steps.get(i).cloned().flatten();
            self.steps.push(step);
            return Ok(i);
        }
        self.reuse = false;
        if let Some(text) = &text
            && let Some((step, st)) = self.cache.kept.get(&self.before(), text, id)
        {
            self.stages.push(st);
            self.steps.push(Some(step));
            self.reused += 1;
            return Ok(i);
        }
        let before = self.stages.last().map(|s| s.solids.clone()).unwrap_or_default();
        let t = Instant::now();
        let r = make(&before);
        self.kernel_ms += t.elapsed().as_secs_f64() * 1e3;
        self.rebuilt += 1;
        match r {
            Ok(solids) => {
                self.stages.push(Rc::new(Stage::new(solids)));
                self.keep(text, id);
                Ok(i)
            }
            Err(e) => {
                self.building = false;
                self.steps.push(None);
                Err(e)
            }
        }
    }

    /// The step this run's history stands at: None at its start (and when it is not keyed).
    fn before(&self) -> Option<Rc<Step>> {
        self.steps.last().cloned().flatten()
    }

    /// Keeps the stage just made as this step's (`text` None: the step is not keyed).
    fn keep(&mut self, text: Option<String>, id: u64) {
        let step = text.map(|text| {
            Rc::new(Step {
                before: self.before(),
                text,
                id,
            })
        });
        if let (Some(step), Some(st)) = (&step, self.stages.last()) {
            self.cache.kept.put(step.clone(), st.clone());
        }
        self.steps.push(step);
    }

    pub fn history(&self) -> History<'_> {
        History::new(&self.features, Some(&self.ids))
    }

    /// The part as it stands, joined.
    pub fn body(&self) -> Option<&Body> {
        self.stages.last().and_then(|s| s.view())
    }

    pub fn built(&self) -> usize {
        self.stages.len()
    }

    /// Which feature made each face of the part after `n` features, on the joined body.
    pub fn origin(&mut self, n: usize) -> Lineage {
        while self.origins.len() < n {
            let k = self.origins.len();
            let work = |m: usize| {
                m.checked_sub(1)
                    .and_then(|j| self.stages.get(j))
                    .and_then(|s| s.work())
            };
            let prev = work(k);
            let o = match work(k + 1) {
                None => HashMap::new(),
                Some(b) => select::lineage(
                    &b,
                    prev.as_ref().map(|p| (p, &self.origins[k - 1])),
                    k,
                    &HashMap::new(),
                ),
            };
            self.origins.push(o);
        }
        let Some(mut origin) = n.checked_sub(1).map(|i| self.origins[i].clone()) else {
            return HashMap::new();
        };
        if let Some(b) = self.stages.get(n - 1).and_then(|s| s.view()) {
            for &(piece, whole) in &b.joins.aliases {
                if let Some(mut o) = origin.get(&piece).cloned() {
                    let w = origin.entry(whole).or_default();
                    o.append(w);
                    o.sort_unstable();
                    o.dedup();
                    *w = o;
                }
            }
        }
        origin
    }
}

/// The faces of `body` that are the face named `role`: itself, a piece of it, or the face it was
/// joined into.
pub fn role_faces(body: &Body, role: Name) -> Vec<FaceId> {
    let mut wholes: Vec<Name> = vec![role];
    for &(piece, whole) in &body.joins.aliases {
        if piece == role || piece.is_piece_of(role) {
            wholes.push(whole);
        }
    }
    body.face_ids()
        .filter(|&f| {
            let n = body.face(f).name;
            wholes.iter().any(|&w| n == w || n.is_piece_of(w))
        })
        .collect()
}

/// A feature id (its full path) as the u64 the kernel names its entities from: a hash of the
/// path, so ids come from names, call sites and loop indices, never from places.
pub fn history_id(path: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in path.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    // nonzero, and clear of the top bit
    (h >> 1) | 1
}
