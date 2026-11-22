//! Building as the program evaluates: each feature goes to the kernel as it is made, so later
//! anchors and selections read the part as it stands. Stages are kept, and a session reuses
//! the stages of a run whose features (and ids) are the same up to that point.

use forge_model::{Feature, History, RegenError, Solid, step_solids_plain};
use forge_topo::{Body, FaceId, Name};
use std::any::Any;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub struct Stage {
    pub solids: Vec<Solid>,
    view: OnceCell<Body>,
    /// Each solid finished (faces on one surface joined), when there are several.
    finished: OnceCell<Vec<Body>>,
    spans: OnceCell<Vec<Span>>,
    /// Several solids as one unfinished body (`work`), worked out once.
    joined: OnceCell<Body>,
    /// What `Eval::no_effect` compares (`shape`), worked out once.
    shape: OnceCell<Option<Shape>>,
    /// Which stage this is, never another's: equal ids, the same stage (a kept one served again
    /// has its own).
    pub id: u64,
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
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Stage {
            solids,
            view: OnceCell::new(),
            finished: OnceCell::new(),
            spans: OnceCell::new(),
            joined: OnceCell::new(),
            shape: OnceCell::new(),
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
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
            // one solid: finished into the view itself; several: each finished once, kept for
            // what looks at them one by one (validity, each one's mass)
            let several = self.solids.len() > 1;
            let mut alone = (!several).then(|| forge_model::finish(self.solids[0].body.clone()));
            let each: &[Body] = if several { self.finished() } else { &[] };
            for (k, s) in self.solids.iter().enumerate() {
                let (f, e) = (all.faces.len() as u32, all.edges.len() as u32);
                match alone.take() {
                    Some(one) => all = one,
                    None => all.append(&each[k]),
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

    /// The stage's one solid finished, if it is one and its view was worked out: what the kernel
    /// would make of it again (`forge_model::step_solids_finished`).
    pub fn finished_alone(&self) -> Option<&Body> {
        match self.solids.len() {
            1 => self.view.get(),
            _ => None,
        }
    }

    /// Each solid's faces and edges in `view()`'s body.
    pub fn spans(&self) -> &[Span] {
        let _ = self.view();
        self.spans.get().map_or(&[], |s| &s[..])
    }

    /// Each solid finished, in order: worked out once for the view and whatever looks at the
    /// solids one by one.
    pub fn finished(&self) -> &[Body] {
        self.finished.get_or_init(|| {
            self.solids
                .iter()
                .map(|s| forge_model::finish(s.body.clone()))
                .collect()
        })
    }

    /// The bodies as one, unfinished: the pieces each feature made, for lineage.
    pub(crate) fn work(&self) -> Option<&Body> {
        match &self.solids[..] {
            [] => None,
            [one] => Some(&one.body),
            many => Some(self.joined.get_or_init(|| {
                let mut all = Body::new();
                for s in many {
                    all.append(&s.body.compact());
                }
                all
            })),
        }
    }

    /// The joined body's face names, edge count and corners (to 1e-5), sorted: equal for two
    /// stages exactly when a feature between them changed nothing (`Eval::no_effect`).
    pub fn shape(&self) -> Option<&Shape> {
        self.shape
            .get_or_init(|| {
                let b = self.view()?;
                let mut v: Vec<u64> = b.face_ids().map(|f| b.face(f).name.0).collect();
                v.sort_unstable();
                let q = |x: f64| (x * 1e5).round() as i64;
                let mut c: Vec<[i64; 3]> = b
                    .vertex_ids()
                    .map(|x| {
                        let p = b.vertex(x).point;
                        [q(p.x), q(p.y), q(p.z)]
                    })
                    .collect();
                c.sort_unstable();
                Some((v, b.edge_ids().count(), c))
            })
            .as_ref()
    }
}

/// A stage's shape as `Stage::shape` gives it.
pub type Shape = (Vec<u64>, usize, Vec<[i64; 3]>);

impl Cache {
    /// The kernel features the run sent, in order, and the id each is named by: the part as
    /// `forge_model` regenerates it on its own (`Part::regenerate_solids_ids`).
    pub fn history(&self) -> (&[Feature], &[u64]) {
        (&self.features, &self.ids)
    }

    /// The solids after each feature built, in order: the stages `history` stands at, for
    /// checking every step (`fuzz.rs`).
    pub fn stages(&self) -> impl Iterator<Item = &[Solid]> {
        self.stages.iter().map(|s| &s.solids[..])
    }

    /// The stages themselves, to share (`fuzz::Built`).
    pub fn stage_rcs(&self) -> &[Rc<Stage>] {
        &self.stages
    }

    /// How many kernel features the last run built.
    pub fn built(&self) -> usize {
        self.stages.len()
    }

    /// The part as it stood after the first `n` kernel features, joined (`Stage::view`).
    pub fn body_after(&self, n: usize) -> Option<&Body> {
        n.checked_sub(1)
            .and_then(|i| self.stages.get(i))
            .and_then(|s| s.view())
    }

    /// Where each solid is in `body_after(n)`: its faces and edges, by the feature that made it.
    pub fn spans_after(&self, n: usize) -> &[Span] {
        n.checked_sub(1)
            .and_then(|i| self.stages.get(i))
            .map_or(&[], |s| s.spans())
    }

    /// Which feature made each face of `body_after(n)`, as a selection's `of` reads it.
    pub fn origin(&mut self, n: usize) -> Rc<Lineage> {
        origin_upto(&self.stages, &mut self.origins, None, n)
    }
}

/// The last run's history and stages, for reuse.
#[derive(Default)]
pub struct Cache {
    features: Vec<Feature>,
    ids: Vec<u64>,
    stages: Vec<Rc<Stage>>,
    /// The finished part's measures, by the stage they were taken of (by its id; the stage held,
    /// so the id is its), the last `MEASURES` of them.
    pub measured: Vec<(Rc<Stage>, crate::Measure)>,
    /// An assembly's loaded parts, solver and checker, kept between runs (`asm.rs`).
    pub asm: Option<Box<crate::asm::AsmCache>>,
    /// Each inline component's own last run, by its name (`asm::component_block`).
    pub comps: HashMap<String, Cache>,
    /// Work done outside the kernel's history (a sheet's builds and shop checks), by a key
    /// that holds everything it read.
    memo: HashMap<String, Rc<dyn Any>>,
    /// Stages built before, by the whole history up to them (`Step`): an edit made and taken
    /// back, or tried again, builds nothing.
    kept: Kept,
    /// The steps of the last run's history, by position (as `stages`).
    steps: Vec<Option<Rc<Step>>>,
    /// The kernel's switches the last run's stages were built under (`forge_model::switches`).
    switches: Rc<str>,
    /// Each stage's lineage worked out so far (`Builder::origin`), by position (as `stages`).
    origins: Vec<Rc<Lineage>>,
    /// Each sketch's solve, by its id (`Sk::finish`).
    sketches: HashMap<String, Rc<crate::sketch::Solved>>,
}

/// The same step as one built before: equal features, and a file one reads (an import's body, a
/// text's font) the very one read then, so a file changed under its name is read again.
fn same_step(a: &Feature, b: &Feature) -> bool {
    fn files(f: &Feature) -> Option<(usize, usize)> {
        match f {
            Feature::Import(t) => Some((1, t.body.as_ref().map_or(0, |b| Arc::as_ptr(b) as usize))),
            Feature::Text(t) => Some((2, t.font.font.as_ref().map_or(0, |b| Arc::as_ptr(b) as usize))),
            Feature::Body(b) => match &**b {
                forge_model::BodyFeature::In { feature, .. } => files(feature),
                _ => None,
            },
            _ => None,
        }
    }
    a == b && files(a) == files(b)
}

/// One step of a history: the step before it (by identity), and this one's feature as text with
/// every literal (`{:?}`: shortest round-trip numbers, enum names) and its id. Two steps are
/// equal exactly when they are the same object: a lookup compares the full key, no hash trusted.
/// The text is written only when a lookup meets the step (most never are: a cold build looks up
/// nothing it could find), from the feature and the kernel's switches kept with it.
pub struct Step {
    before: Option<Rc<Step>>,
    feature: Feature,
    switches: Rc<str>,
    text: OnceCell<String>,
    id: u64,
}

impl Step {
    fn text(&self) -> &str {
        self.text.get_or_init(|| key_text(&self.switches, &self.feature))
    }
}

/// A step's key text: the kernel's switches and the feature with every literal.
fn key_text(switches: &str, f: &Feature) -> String {
    format!("{switches} {f:?}")
}

/// A kept stage: its full key, the stage, when it was last used.
type Entry = (Rc<Step>, Rc<Stage>, u64);

/// Stages by step, the least lately used dropped past `KEPT`.
#[derive(Default)]
struct Kept {
    /// By a hash of the key (`bucket`), each entry the full key and its stage.
    stages: HashMap<u128, Vec<Entry>>,
    len: usize,
    tick: u64,
}

/// How many stages a session keeps beyond its last run's.
const KEPT: usize = 512;
/// How many stages' measures a session keeps.
const MEASURES: usize = 16;
/// How many memo entries (`Builder::memo`) a session keeps beyond its last run's.
const MEMO_KEPT: usize = 128;

/// Whether a step can be keyed by its text: not when its feature reads what its text does not
/// hold (an imported file's body, a font), or a step before it could not be.
fn keyed(steps: &[Option<Rc<Step>>], f: &Feature) -> bool {
    let reads_a_file = |f: &Feature| matches!(f, Feature::Import(_) | Feature::Text(_));
    let inner = match f {
        Feature::Body(b) => match &**b {
            forge_model::BodyFeature::In { feature, .. } => reads_a_file(feature),
            _ => false,
        },
        _ => false,
    };
    !(steps.last().is_some_and(|s| s.is_none()) || reads_a_file(f) || inner)
}

/// The bucket of steps after `before` with id `id` (their texts told apart in it).
fn bucket(before: &Option<Rc<Step>>, id: u64) -> u128 {
    let mut h = forge_math::hash::Hash128::new();
    h.word(before.as_ref().map_or(0, |b| Rc::as_ptr(b) as usize as u64));
    h.word(id);
    h.finish()
}

impl Kept {
    /// The step and stage kept for exactly this key: feature `f` (as text, under `switches`) with
    /// id `id` after step `before`.
    fn get(
        &mut self,
        before: &Option<Rc<Step>>,
        f: &Feature,
        switches: &str,
        id: u64,
    ) -> Option<(Rc<Step>, Rc<Stage>)> {
        self.tick += 1;
        let t = self.tick;
        let v = self.stages.get_mut(&bucket(before, id))?;
        let text = key_text(switches, f);
        let e = v.iter_mut().find(|e| {
            e.0.id == id
                && match (&e.0.before, before) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                }
                && e.0.text() == text
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
        let b = bucket(&step.before, step.id);
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
    origins: Vec<Rc<Lineage>>,
    pub(crate) cache: Cache,
    /// The cached stages still matching this run's prefix.
    reuse: bool,
    /// Whether a step built here (a sheet) holds bodies the kernel's history cannot step: then
    /// features that make nothing pass the bodies on as they are.
    hosted: bool,
    /// `forge_model::unwrapped` of each feature so far, once one is wrapped (`In`): handed to
    /// each step, which would otherwise unwrap the whole history again.
    plain: Vec<Feature>,
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
    switches: Rc<str>,
    /// This run's sketch solves, by sketch id: what the next run takes where a sketch is the same.
    sketches: HashMap<String, Rc<crate::sketch::Solved>>,
}

impl Builder {
    pub fn new(building: bool, cache: Cache) -> Builder {
        Builder {
            features: vec![],
            ids: vec![],
            stages: vec![],
            building,
            origins: vec![],
            sketches: HashMap::new(),
            cache,
            reuse: true,
            hosted: false,
            plain: vec![],
            kernel_ms: 0.0,
            rebuilt: 0,
            reused: 0,
            memo: HashMap::new(),
            remade: 0,
            steps: vec![],
            switches: forge_model::switches().into(),
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

    /// What `memo` keeps under `key`, if it does, without making it.
    pub fn memo_kept<T: Clone + 'static>(&mut self, key: &str) -> Option<T> {
        if let Some(v) = self.memo.get(key).and_then(|v| v.downcast_ref::<T>()) {
            return Some(v.clone());
        }
        let v = self.cache.memo.remove(key)?;
        let t = v.downcast_ref::<T>()?.clone();
        self.memo.insert(key.to_string(), v);
        Some(t)
    }

    /// Keeps `t` under `key`, as `memo` keeps what it made.
    pub fn memo_keep<T: 'static>(&mut self, key: String, t: T) {
        self.remade += 1;
        if self.building {
            self.memo.insert(key, Rc::new(t));
        }
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
            switches: self.switches,
            comps: self.cache.comps,
            origins: self.origins,
            sketches: self.sketches,
        }
    }

    /// Sketch `id`'s solve in the run before, if it had one.
    pub fn solved_was(&self, id: &str) -> Option<Rc<crate::sketch::Solved>> {
        self.cache.sketches.get(id).cloned()
    }

    /// Keeps sketch `id`'s solve for the next run.
    pub fn keep_solved(&mut self, id: String, s: Rc<crate::sketch::Solved>) {
        self.sketches.insert(id, s);
    }

    /// This run's history and stages as a cache, the run going on (an `expect .. at` run of
    /// the same document starts from them: only what its params change is rebuilt).
    pub fn snapshot(&self) -> Cache {
        Cache {
            features: self.features.clone(),
            ids: self.ids.clone(),
            stages: self.stages.clone(),
            ..Cache::default()
        }
    }

    /// The measures of the finished part, if the last run took them of this same stage.
    pub fn measured(&self) -> Option<crate::Measure> {
        let last = self.stages.last()?;
        let (_, m) = self.cache.measured.iter().find(|(st, _)| Rc::ptr_eq(st, last))?;
        Some(m.clone())
    }

    pub fn keep_measure(&mut self, m: crate::Measure) {
        if let Some(st) = self.stages.last() {
            let kept = &mut self.cache.measured;
            kept.retain(|(s, _)| !Rc::ptr_eq(s, st));
            if kept.len() >= MEASURES {
                kept.remove(0);
            }
            kept.push((st.clone(), m));
        }
    }

    /// Adds a feature and, when building, builds it onto the part. Returns its index.
    pub fn push(&mut self, f: Feature, id: u64) -> Result<usize, RegenError> {
        let i = self.features.len();
        let keyable = keyed(&self.steps, &f);
        self.features.push(f);
        self.ids.push(id);
        if !self.building {
            return Ok(i);
        }
        if self.reuse
            && self.cache.switches == self.switches
            && self
                .cache
                .features
                .get(i)
                .is_some_and(|f| same_step(f, &self.features[i]))
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
        if keyable
            && let Some((step, st)) =
                self.cache
                    .kept
                    .get(&self.before(), &self.features[i], &self.switches, id)
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
            self.keep(keyable, id);
            return Ok(i);
        }
        // parameters, sketches and planes make nothing: the kernel hands the part on as it is
        // (`forge_model::step_on`), so the stage before is this one's too, its view, lineage and
        // shape worked out once
        if matches!(
            self.features[i],
            Feature::Sketch(_) | Feature::Param(_) | Feature::Plane(_)
        ) {
            let st = match self.stages.last() {
                Some(last) => last.clone(),
                None => Rc::new(Stage::new(vec![])),
            };
            self.rebuilt += 1;
            self.stages.push(st);
            self.keep(keyable, id);
            return Ok(i);
        }
        // a work plane, axis or point makes nothing either: the kernel only checks it can be
        // placed (`forge_model::work::datum`) and hands the part on. Placed, the stage before is
        // this one's; not placed, the kernel's step says why as before.
        if let Feature::Work(_) = &self.features[i]
            && forge_model::work::datum(History::new(&self.features, Some(&self.ids)), i).is_ok()
        {
            let st = match self.stages.last() {
                Some(last) => last.clone(),
                None => Rc::new(Stage::new(vec![])),
            };
            self.rebuilt += 1;
            self.stages.push(st);
            self.keep(keyable, id);
            return Ok(i);
        }
        let last = self.stages.last();
        let before = last.map(|s| s.solids.clone()).unwrap_or_default();
        // the part as a selection saw it (its one solid finished), for a feature that finishes
        // it again to work on it
        let finished = last.and_then(|s| s.finished_alone());
        let t = Instant::now();
        let wrapped =
            |f: &Feature| matches!(f, Feature::Body(b) if matches!(**b, forge_model::BodyFeature::In { .. }));
        if !self.plain.is_empty() || self.features.iter().any(wrapped) {
            while self.plain.len() < self.features.len() {
                let f = forge_model::unwrapped(&self.features[self.plain.len()]);
                self.plain.push(f);
            }
        }
        let history = History::new(&self.features, Some(&self.ids));
        let plain = (!self.plain.is_empty()).then_some(&self.plain[..]);
        let r = step_solids_plain(history, plain, i, before, finished);
        self.kernel_ms += t.elapsed().as_secs_f64() * 1e3;
        self.rebuilt += 1;
        match r {
            Ok(solids) => {
                self.stages.push(Rc::new(Stage::new(solids)));
                self.keep(keyable, id);
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
        let keyable = keyed(&self.steps, &f);
        self.features.push(f);
        self.ids.push(id);
        self.hosted = true;
        if !self.building {
            return Ok(i);
        }
        if self.reuse
            && self.cache.switches == self.switches
            && self
                .cache
                .features
                .get(i)
                .is_some_and(|f| same_step(f, &self.features[i]))
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
        if keyable
            && let Some((step, st)) =
                self.cache
                    .kept
                    .get(&self.before(), &self.features[i], &self.switches, id)
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
                self.keep(keyable, id);
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

    /// Keeps the stage just made as the last feature's step (unless it is not keyed).
    fn keep(&mut self, keyable: bool, id: u64) {
        let step = keyable.then(|| {
            Rc::new(Step {
                before: self.before(),
                feature: self.features.last().expect("the feature just pushed").clone(),
                switches: self.switches.clone(),
                text: OnceCell::new(),
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

    /// The id of the stage the part stands at (`Stage::id`), 0 before any.
    pub fn stage_id(&self) -> u64 {
        self.stages.last().map_or(0, |s| s.id)
    }

    /// The part as it stands, joined.
    pub fn body(&self) -> Option<&Body> {
        self.stages.last().and_then(|s| s.view())
    }

    pub fn built(&self) -> usize {
        self.stages.len()
    }

    /// Which feature made each face of the part after `n` features, on the joined body.
    pub fn origin(&mut self, n: usize) -> Rc<Lineage> {
        let kept = Some((&self.cache.stages[..], &self.cache.origins[..]));
        origin_upto(&self.stages, &mut self.origins, kept, n)
    }
}

/// Which features made each face, by its name (as `forge_agent::select::Lineage`, hashed for
/// names: they are hashes already, so a multiply spreads them; nothing iterates one).
pub type Lineage = HashMap<Name, Vec<usize>, std::hash::BuildHasherDefault<NameMix>>;

/// The hasher of `Lineage`.
#[derive(Default)]
pub struct NameMix(u64);

impl std::hash::Hasher for NameMix {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(8) ^ b as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0 ^ x).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
}

/// `forge_agent::select::lineage` with nothing copied, into a `Lineage`: each face's making
/// features after feature `i` built `body`, given those before. A face keeps its origin while its
/// name lives; a new face on a face before's surface, overlapping it, is that face's.
fn lineage(body: &Body, prev: Option<(&Body, &Lineage)>, i: usize) -> Lineage {
    let mut out = Lineage::with_capacity_and_hasher(body.faces.len(), Default::default());
    for f in body.face_ids() {
        let face = body.face(f);
        let o = prev.and_then(|(pb, po)| {
            po.get(&face.name).cloned().or_else(|| {
                // (the surfaces first: most differ, and a face's box is worked out only then)
                pb.face_ids()
                    .find(|&g| {
                        pb.face(g).surface == face.surface && pb.face_box(g).overlaps(&body.face_box(f))
                    })
                    .and_then(|g| po.get(&pb.face(g).name).cloned())
            })
        });
        out.insert(face.name, o.unwrap_or_else(|| vec![i]));
    }
    out
}

/// A run before's stages, and the lineages worked out for them, by position.
type KeptLineage<'a> = (&'a [Rc<Stage>], &'a [Rc<Lineage>]);

/// Which feature made each face of the part after `n` of `stages`, on the joined body: each
/// stage's lineage worked out once (`origins`), or taken from `kept` (stages and their lineages
/// of a run before) where every stage up to it is the very one kept.
fn origin_upto(
    stages: &[Rc<Stage>],
    origins: &mut Vec<Rc<Lineage>>,
    kept: Option<KeptLineage<'_>>,
    n: usize,
) -> Rc<Lineage> {
    while origins.len() < n {
        let k = origins.len();
        let same = kept.and_then(|(ks, ko)| {
            let o = ko.get(k)?;
            (ks.len() > k && (0..=k).all(|j| Rc::ptr_eq(&ks[j], &stages[j]))).then(|| o.clone())
        });
        if let Some(o) = same {
            origins.push(o);
            continue;
        }
        // the very stage as the one before (a feature that makes nothing): each face keeps the
        // origin it had, as `lineage` would find
        if k > 0 && k < stages.len() && Rc::ptr_eq(&stages[k], &stages[k - 1]) {
            let o = origins[k - 1].clone();
            origins.push(o);
            continue;
        }
        let prev = k.checked_sub(1).and_then(|j| stages[j].work());
        let o = match stages.get(k).and_then(|s| s.work()) {
            None => Lineage::default(),
            Some(b) => lineage(b, prev.map(|p| (p, &*origins[k - 1])), k),
        };
        origins.push(Rc::new(o));
    }
    let Some(origin) = n.checked_sub(1).map(|i| &origins[i]) else {
        return Rc::default();
    };
    // a face joined of pieces has every piece's origin
    let Some(b) = stages.get(n - 1).and_then(|s| s.view()) else {
        return origin.clone();
    };
    if !b
        .joins
        .aliases
        .iter()
        .any(|(piece, _)| origin.contains_key(piece))
    {
        return origin.clone();
    }
    let mut origin = (**origin).clone();
    for &(piece, whole) in &b.joins.aliases {
        if let Some(mut o) = origin.get(&piece).cloned() {
            let w = origin.entry(whole).or_default();
            o.append(w);
            o.sort_unstable();
            o.dedup();
            *w = o;
        }
    }
    Rc::new(origin)
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
