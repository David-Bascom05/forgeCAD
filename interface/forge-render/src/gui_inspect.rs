//! The host side of the Inspect tools: the exact bodies behind what's drawn, the view's picks
//! turned into measurements on a thread of their own (a body meshed once for them), the section
//! put on the renderer, and the physical properties worked out when asked.

use super::Gui;
use crate::render::{Entity, Mark};
use crate::shell::inspect::{CutPlane, Filter, MPick};
use forge_agent::measure::{Free, Pick, Reading, Target, Whole, read, read_pair};
use forge_agent::select::{self, Ent};
use forge_lang::props::{Physical, Props};
use forge_math::{Frame, Point3, Vec3};
use forge_mesh::TriMesh;
use forge_topo::{Body, EdgeId, FaceId, VertexId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// What a build gives the Inspect tools: each part mesh's exact body, each placement's mesh and
/// pose, the document's iProperties, a part's physical properties, an assembly's occurrences.
///
/// The bodies (and an assembly's solve) are where the build ran: on the page natively and in the
/// browser's in-page build; in the default browser build they stay in the build worker, and the
/// page has each body's `Brief` and asks the rest of the worker (`Ask`).
#[derive(Default)]
pub struct InspectData {
    pub bodies: Vec<Arc<Body>>,
    pub placed: Vec<(usize, Frame)>,
    pub names: Vec<String>,
    pub props: Props,
    pub physical: Option<Physical>,
    pub asm: Option<Arc<forge_lang::asm::AsmOut>>,
    /// What the page reads of each body as it draws, by body.
    pub briefs: Vec<Brief>,
    /// It's an assembly's (its solve is `asm`, where the bodies are).
    pub assembly: bool,
    /// Which build's this is: what the page names it by when it asks the worker.
    pub number: u64,
}

/// What the page reads of a body each frame, made where the body is: each face's and edge's own
/// name (the kernel's) and kind, and how many vertices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Brief {
    pub faces: Vec<(forge_topo::Name, &'static str)>,
    pub edges: Vec<(forge_topo::Name, &'static str)>,
    pub vertices: usize,
}

impl Brief {
    pub fn of(b: &Body) -> Brief {
        Brief {
            faces: b
                .faces
                .iter()
                .map(|f| (f.name, select::surface_kind(&f.surface)))
                .collect(),
            edges: b
                .edges
                .iter()
                .map(|e| (e.name, select::curve_kind(&e.curve)))
                .collect(),
            vertices: b.vertices.len(),
        }
    }

    /// Whether the body has this entity.
    pub fn has(&self, e: Entity) -> bool {
        match e {
            Entity::Face(i) => (i as usize) < self.faces.len(),
            Entity::Edge(i) => (i as usize) < self.edges.len(),
            Entity::Vertex(i) => (i as usize) < self.vertices,
        }
    }
}

/// Each build's inspect data its own number, where it's made.
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl InspectData {
    /// These bodies placed so, with their briefs and a number of their own.
    pub fn new(bodies: Vec<Arc<Body>>, placed: Vec<(usize, Frame)>, names: Vec<String>) -> InspectData {
        InspectData {
            briefs: bodies.iter().map(|b| Brief::of(b)).collect(),
            bodies,
            placed,
            names,
            number: GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ..Default::default()
        }
    }

    /// The brief of the body at placement `pl`, and its pose.
    pub fn brief_at(&self, pl: u32) -> Option<(&Brief, Frame)> {
        let &(mesh, at) = self.placed.get(pl as usize)?;
        Some((self.briefs.get(mesh)?, at))
    }

    /// The body at placement `pl`, and its pose, where the bodies are.
    pub fn body_at(&self, pl: u32) -> Option<(&Arc<Body>, Frame)> {
        let &(mesh, at) = self.placed.get(pl as usize)?;
        Some((self.bodies.get(mesh)?, at))
    }
}

impl std::fmt::Debug for InspectData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InspectData({} bodies, {} placed)",
            self.bodies.len(),
            self.placed.len()
        )
    }
}

/// A pick as the worker takes it: what it is, by its placement (the body there is found where
/// the bodies are).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tgt {
    Ent(u32, Ent),
    /// A whole body where it stands, and its density (kg/mm³) where its material says.
    Whole(u32, Option<f64>),
    Free(Free),
}

/// What the tools ask of the bodies, of the build `InspectData` they were given with.
pub(crate) enum Job {
    Measure(u64, Vec<Tgt>),
    /// Bodies meshed ahead, as Measure opens (a part's, an assembly's first few): the first pick's
    /// answer doesn't wait for them.
    Warm,
    /// The thinnest wall of the body at a placement; the worker stops when `WallCtl` says.
    Wall(u64, u32, Arc<WallCtl>),
    /// An assembly's physical properties, from its solve.
    Physical,
}

/// A wall's search on the worker, as the panel follows it: the measurement still wanted (its id;
/// another, or none, and the worker stops), and how far it is (in thousandths).
#[derive(Default)]
pub(crate) struct WallCtl {
    pub(crate) wanted: std::sync::atomic::AtomicU64,
    pub(crate) done: std::sync::atomic::AtomicU32,
}

/// A wall's search stopped before its end: no answer.
pub(crate) const STOPPED: &str = "stopped";

#[derive(Debug)]
pub(crate) enum Answer {
    Measure(u64, Result<Reading, String>),
    Wall(u64, Result<(f64, Point3), String>),
    Physical(Box<Result<Physical, String>>),
    /// The build worker's answer to an `Ask` of the build with this number.
    Told(u64, Ask, Told),
}

/// The host's state for the tools.
#[derive(Default)]
pub struct Inspector {
    to: Option<crate::plat::Server<(Arc<InspectData>, Job)>>,
    answers: Option<Sender<Answer>>,
    from: Option<Receiver<Answer>>,
    seq: u64,
    pub(crate) data: Option<Arc<InspectData>>,
    /// The view's click count last seen, and the picks last measured.
    clicks: u32,
    open: bool,
    measured: Option<(Vec<MPick>, bool, bool)>,
    /// The picks the last job was for, kept across a rebuild.
    asked: Option<(Vec<MPick>, bool)>,
    /// The origin item last chosen in the browser, taken once.
    origin_seen: Option<crate::origin::Item>,
    /// The bodies were sent to be meshed ahead for this build.
    warmed: bool,
    /// What the renderer was last given.
    cut: Option<crate::shell::inspect::Cut>,
    /// The section's outline being worked out (`fg query F section ..`).
    outline: Option<Receiver<Result<(String, crate::shell::inspect::Outline), String>>>,
    /// The worker's wall search, as the panel follows and stops it.
    wall: Arc<WallCtl>,
    /// The build worker's answers to `Ask`s (by the question's bytes), none while asked.
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    told: std::cell::RefCell<HashMap<Vec<u8>, Option<Told>>>,
    /// Where jobs run on the page's one thread: the wall's search, stepped a few milliseconds a
    /// frame (its measurement, the search, where its part stands).
    walking: Option<(u64, forge_wall::WallSearch<Arc<TriMesh>>, Frame)>,
}

fn ent(e: Entity) -> Ent {
    match e {
        Entity::Face(i) => Ent::Face(FaceId(i)),
        Entity::Edge(i) => Ent::Edge(EdgeId(i)),
        Entity::Vertex(i) => Ent::Vert(VertexId(i)),
    }
}

fn kind_word(b: &Brief, e: Ent) -> String {
    match e {
        Ent::Vert(_) => "Vertex".into(),
        Ent::Edge(x) => format!("Edge ({})", b.edges[x.0 as usize].1),
        Ent::Face(x) => format!("Face ({})", b.faces[x.0 as usize].1),
    }
}

/// The tools' worker where the bodies are on the page: a thread natively, run at once in the
/// browser's in-page build (`plat::serve`).
fn work(out: Sender<Answer>, wake: super::Waker) -> crate::plat::Server<(Arc<InspectData>, Job)> {
    let mut meshes = Meshes::default();
    crate::plat::serve(
        "forge-inspect",
        move |mut job, jobs: &Receiver<(Arc<InspectData>, Job)>| {
            // a newer job of the same kind replaces one not yet started
            while let Ok(j) = jobs.try_recv() {
                if std::mem::discriminant(&j.1) != std::mem::discriminant(&job.1) {
                    let _ = out.send_some(handle(&job.0, job.1, &mut meshes));
                }
                job = j;
            }
            // the meshes of this build's bodies only (each holds its body)
            meshes.keep(&[&job.0]);
            if out.send_some(handle(&job.0, job.1, &mut meshes)).is_err() {
                return false;
            }
            wake();
            true
        },
    )
}

trait SendSome {
    fn send_some(&self, a: Option<Answer>) -> Result<(), ()>;
}

impl SendSome for Sender<Answer> {
    fn send_some(&self, a: Option<Answer>) -> Result<(), ()> {
        match a {
            Some(a) => self.send(a).map_err(|_| ()),
            None => Ok(()),
        }
    }
}

/// Each body's mesh for the tools, made once (by the body: the same body, the same mesh).
#[derive(Default)]
pub(crate) struct Meshes(HashMap<usize, (Arc<Body>, Arc<TriMesh>)>);

impl Meshes {
    pub(crate) fn of(&mut self, b: &Arc<Body>) -> Result<Arc<TriMesh>, String> {
        let key = Arc::as_ptr(b) as usize;
        if let Some((_, m)) = self.0.get(&key) {
            return Ok(m.clone());
        }
        let m = forge_mesh::tessellate::tessellate_tolerant(b, &forge_mesh::Quality::default())
            .map_err(|e| forge_lang::error::words(&e))?;
        let m = Arc::new(m);
        // the body kept with it: its address is never another body's while it's here
        self.0.insert(key, (b.clone(), m.clone()));
        Ok(m)
    }

    /// Only these builds' bodies' meshes kept.
    pub(crate) fn keep(&mut self, data: &[&InspectData]) {
        let live: std::collections::HashSet<usize> = data
            .iter()
            .flat_map(|d| d.bodies.iter().map(|b| Arc::as_ptr(b) as usize))
            .collect();
        self.0.retain(|k, _| live.contains(k));
    }
}

/// A job's answer, worked out of `d`'s bodies (none for a warm-up). The wall's search stops when
/// its `WallCtl` no longer wants it.
pub(crate) fn handle(d: &InspectData, job: Job, meshes: &mut Meshes) -> Option<Answer> {
    Some(match job {
        Job::Warm => {
            for b in d.bodies.iter().take(8) {
                let _ = meshes.of(b);
            }
            return None;
        }
        Job::Measure(id, picks) => Answer::Measure(id, measure_of(d, &picks, meshes)),
        Job::Wall(id, pl, ctl) => {
            use std::sync::atomic::Ordering::Relaxed;
            let r = d
                .body_at(pl)
                .ok_or("the part isn't there".to_string())
                .and_then(|(b, at)| {
                    let m = meshes.of(b)?;
                    let mut w = forge_wall::WallSearch::new(b, &*m);
                    // a few thousand rays at a time: how far, and whether it's still wanted
                    while !w.step(4096) {
                        if ctl.wanted.load(Relaxed) != id {
                            return Err(STOPPED.to_string());
                        }
                        ctl.done.store((w.done() * 1000.0) as u32, Relaxed);
                    }
                    wall_found(w.found(), at)
                });
            Answer::Wall(id, r)
        }
        Job::Physical => Answer::Physical(Box::new(match &d.asm {
            Some(asm) => forge_lang::props::asm_physical(asm, None),
            None => Err("the assembly isn't solved".into()),
        })),
    })
}

/// Picks measured on `d`'s bodies.
pub(crate) fn measure_of(d: &InspectData, picks: &[Tgt], meshes: &mut Meshes) -> Result<Reading, String> {
    let gone = || "a pick isn't there to measure".to_string();
    // the body at each pick's placement, its pose
    let mut at: Vec<Option<(&Arc<Body>, Frame)>> = vec![];
    for p in picks {
        at.push(match p {
            Tgt::Ent(pl, _) | Tgt::Whole(pl, _) => Some(d.body_at(*pl).ok_or_else(gone)?),
            Tgt::Free(_) => None,
        });
    }
    // one edge or point reads off the body alone; faces, bodies and pairs want the mesh
    let wants = picks.len() > 1
        || picks
            .iter()
            .any(|p| matches!(p, Tgt::Whole(..) | Tgt::Ent(_, Ent::Face(_))));
    let bare = Arc::new(TriMesh::default());
    let mut ms: Vec<Option<Arc<TriMesh>>> = vec![];
    for b in &at {
        ms.push(match b {
            Some(_) if !wants => Some(bare.clone()),
            Some((b, _)) => Some(meshes.of(b)?),
            None => None,
        });
    }
    let ts: Vec<Target> = picks
        .iter()
        .zip(at.iter().zip(&ms))
        .map(|(p, (b, m))| match (p, b, m) {
            (Tgt::Ent(_, e), Some((b, at)), Some(m)) => Target::Ent(Pick {
                body: b,
                mesh: m,
                at: *at,
                ent: *e,
            }),
            (Tgt::Whole(_, density), Some((b, at)), Some(m)) => Target::Whole(Whole {
                body: b,
                mesh: m,
                at: *at,
                density: *density,
            }),
            (Tgt::Free(f), _, _) => Target::Free(f.clone()),
            _ => unreachable!("a body's mesh is made above"),
        })
        .collect();
    Ok(match &ts[..] {
        [a] => read(a),
        [a, b] => read_pair(a, b),
        [a, b, c] => forge_agent::measure::read_three(a, b, c),
        _ => Reading::default(),
    })
}

impl Gui {
    /// A job on the current build's bodies: to the worker where they are (a thread natively;
    /// the build worker in the default browser build).
    fn inspect_send(&mut self, job: Job) {
        let Some(d) = self.inspector.data.clone() else {
            return;
        };
        let i = &mut self.inspector;
        if i.from.is_none() {
            let (answers, from) = channel();
            i.answers = Some(answers);
            i.from = Some(from);
        }
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        if crate::fg::remote::page::on() {
            crate::fg::remote::page::answers_to(i.answers.clone().expect("made above"), self.waker.clone());
            crate::fg::remote::page::inspect(d.number, job);
            return;
        }
        if i.to.is_none()
            && let Some(a) = &i.answers
        {
            i.to = Some(work(a.clone(), self.waker.clone()));
        }
        if let Some(to) = &i.to {
            to.send((d, job));
        }
    }

    /// A build's data for the tools.
    pub(crate) fn inspect_built(&mut self, d: Option<Arc<InspectData>>) {
        let Some(d) = d else { return };
        let s = &mut self.shell.inspect;
        s.doc = d.props.clone();
        s.assembly = d.assembly;
        if !d.assembly {
            s.physical = d.physical.clone().map(Ok);
        } else {
            if s.props.as_ref().is_some_and(|p| p.tab == 5) || s.cg {
                s.want_physical = true;
            }
        }
        // a pick the rebuilt part hasn't got, or one on a part whose topology changed, is gone:
        // measuring starts again rather than read some other face by its old number
        let was = self.inspector.data.take();
        let kept = |p: &MPick| {
            let shape = |d: &InspectData, pl: u32, e: Option<Entity>| {
                let (b, _) = d.brief_at(pl)?;
                let n = e.is_none_or(|e| b.has(e));
                n.then_some((b.faces.len(), b.edges.len(), b.vertices))
            };
            let (pl, e) = match p {
                MPick::Ent(pl, e) => (*pl, Some(*e)),
                MPick::Whole(pl) => (*pl, None),
                MPick::Free(..) => return true,
            };
            let now = shape(&d, pl, e);
            now.is_some() && was.as_ref().is_none_or(|w| shape(w, pl, e) == now)
        };
        if let Some(m) = &mut self.shell.inspect.measure
            && !m.picks.iter().all(kept)
        {
            m.restart();
            self.inspector.asked = None;
        }
        self.inspector.data = Some(d);
        self.inspector.measured = None;
        self.inspector.warmed = false;
        // where answers come back: made now, so a question asked as it draws has one
        if self.inspector.from.is_none() {
            let (answers, from) = channel();
            #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
            crate::fg::remote::page::answers_to(answers.clone(), self.waker.clone());
            self.inspector.answers = Some(answers);
            self.inspector.from = Some(from);
        }
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        self.inspector.told.borrow_mut().clear();
    }

    /// A pick on placement `pl`, if its body has it: the entity and its name.
    fn picked(&self, pl: u32, e: Entity) -> Option<(Ent, String)> {
        let d = self.inspector.data.as_ref()?;
        let (b, _) = d.brief_at(pl)?;
        if !b.has(e) {
            return None;
        }
        let e = ent(e);
        let name = match d.names.get(pl as usize) {
            Some(n) if !n.is_empty() => format!("{} · {n}", kind_word(b, e)),
            _ => kind_word(b, e),
        };
        Some((e, name))
    }

    /// A Measure pick as the worker takes it, and its words.
    fn target_of(&self, p: &MPick) -> Option<(Tgt, String)> {
        match p {
            MPick::Ent(pl, e) => self.picked(*pl, *e).map(|(e, w)| (Tgt::Ent(*pl, e), w)),
            MPick::Whole(pl) => {
                let d = self.inspector.data.as_ref()?;
                d.brief_at(*pl)?;
                let name = d.names.get(*pl as usize).filter(|n| !n.is_empty());
                Some((
                    Tgt::Whole(*pl, self.density_of(*pl)),
                    name.map_or("Body".into(), |n| format!("Body · {n}")),
                ))
            }
            MPick::Free(f, w) => Some((Tgt::Free(f.clone()), w.clone())),
        }
    }

    /// The density (kg/mm³) of what's at a placement: its occurrence's material, else the part's.
    fn density_of(&self, pl: u32) -> Option<f64> {
        let d = self.inspector.data.as_ref()?;
        let phys = self
            .shell
            .inspect
            .physical
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .or(d.physical.as_ref())?;
        let name = d.names.get(pl as usize);
        let solid =
            phys.solids
                .iter()
                .find(|s| name.is_some_and(|n| *n == s.id))
                .or(if phys.solids.len() == 1 {
                    phys.solids.first()
                } else {
                    None
                });
        match solid {
            // kg/m³ to kg/mm³
            Some(s) => s.density.map(|k| k * 1e-9),
            None if !d.assembly => phys.mass.map(|m| m / phys.volume.max(1e-12)),
            None => None,
        }
    }

    /// What a click (or the pointer) at a placement's entity picks under the selection priority:
    /// the face of an edge facing the eye, the nearest edge of a face, the nearest point, the
    /// whole body.
    fn filtered(
        &self,
        filter: Filter,
        pl: u32,
        e: Entity,
        at: Option<[f32; 3]>,
    ) -> Result<Option<MPick>, Waiting> {
        if filter == Filter::Bodies {
            return Ok(Some(MPick::Whole(pl)));
        }
        if filter == Filter::Any {
            return Ok(Some(MPick::Ent(pl, e)));
        }
        let eye = self.view.as_ref().map(|v| v.nav.camera.eye());
        match self.inspector.ask(Ask::Filtered(filter, pl, e, at, eye))? {
            Told::Pick(p) => Ok(p),
            _ => Ok(None),
        }
    }

    /// The face a click on an edge or a corner meant, of those `ok` takes (a sketch: the planar
    /// ones): of the faces meeting there, the one turned most to the eye. `Err` while the build
    /// worker works it out.
    pub(crate) fn face_at(
        &self,
        pl: u32,
        e: Entity,
        at: Option<[f32; 3]>,
        ok: impl Fn(u32) -> bool,
    ) -> Result<Option<Entity>, Waiting> {
        let Some((b, _)) = self.inspector.data.as_ref().and_then(|d| d.brief_at(pl)) else {
            return Ok(None);
        };
        if matches!(e, Entity::Face(_)) {
            return Ok(Some(e));
        }
        // the faces `ok` takes, as a list (the question crosses as data)
        let allowed: Vec<u32> = (0..b.faces.len() as u32).filter(|&f| ok(f)).collect();
        let eye = self.view.as_ref().map(|v| v.nav.camera.eye());
        match self.inspector.ask(Ask::FaceAt(pl, e, at, eye, allowed))? {
            Told::Face(f) => Ok(f),
            _ => Ok(None),
        }
    }

    /// What the view marks for Measure: what's picked, and what a click would pick now (a whole
    /// body all its faces).
    pub(crate) fn measure_view_marks(&self) -> Option<Vec<Mark>> {
        let m = self.shell.inspect.measure.as_ref()?;
        let d = self.inspector.data.as_ref();
        let mut out = vec![];
        let mut put = |p: &MPick, selected: bool, second: bool| match p {
            MPick::Ent(placement, entity) => out.push(Mark {
                placement: *placement,
                entity: *entity,
                selected,
                second,
            }),
            MPick::Whole(pl) => {
                let faces = d.and_then(|d| d.brief_at(*pl)).map_or(0, |(b, _)| b.faces.len());
                out.extend((0..faces as u32).map(|f| Mark {
                    placement: *pl,
                    entity: Entity::Face(f),
                    selected,
                    second,
                }));
            }
            MPick::Free(..) => {}
        };
        // the second selection in its own colour, as Inventor's sets
        for (k, p) in m.picks.iter().enumerate() {
            put(p, true, k == 1);
        }
        if let Some(h) = &m.hover
            && !m.picks.contains(h)
        {
            put(h, false, false);
        }
        Some(out)
    }

    /// Once a frame: the view's clicks taken as picks, jobs sent and answers taken, the section
    /// put on the renderer.
    pub(crate) fn inspect_tick(&mut self) {
        // the build worker's wall search: stopped when no longer wanted
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        crate::fg::remote::page::tick();
        if let Some(v) = &self.view {
            self.shell.inspect.bounds = v.scene.bounds();
        }
        while let Some(a) = self.inspector.from.as_ref().and_then(|f| f.try_recv().ok()) {
            let s = &mut self.shell.inspect;
            match a {
                Answer::Measure(id, r) if id == self.inspector.seq => {
                    if let Some(m) = &mut s.measure {
                        m.reading = Some(r);
                        m.busy = false;
                    }
                }
                Answer::Wall(_, Err(e)) if e == STOPPED => {}
                Answer::Wall(id, r) if id == self.inspector.seq => {
                    if let Some(m) = &mut s.measure {
                        m.wall_at = Some(r);
                        m.busy = false;
                    }
                }
                Answer::Physical(r) => {
                    s.physical = Some(*r);
                    s.physical_busy = false;
                    // a body measured before its material was known: read again, with its mass
                    self.inspector.measured = None;
                }
                #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
                Answer::Told(n, a, t) => self.inspector.told(n, &a, t),
                _ => {}
            }
            if let Some(v) = &mut self.view {
                v.mark_dirty();
            }
        }
        self.inspect_clicks();
        self.measure_browser();
        self.measure_hover();
        self.inspect_measure();
        self.inspect_walk();
        if let Some((p, n)) = self.shell.inspect.want_outline.take() {
            let doc = self.fg.as_ref().map(|s| (s.src.clone(), s.path.clone()));
            let (send, recv) = channel();
            self.inspector.outline = Some(recv);
            let waker = self.waker.clone();
            crate::plat::spawn("forge-job", move || {
                let got = match doc {
                    None => Err("Section numbers are a Forge (.fg) document's".to_string()),
                    Some((src, path)) => {
                        let dir = path
                            .as_deref()
                            .and_then(|f| std::path::Path::new(f).parent().map(|d| d.to_path_buf()));
                        let q = format!(
                            "section at {} {} {} normal {} {} {}",
                            p[0], p[1], p[2], n[0], n[1], n[2]
                        );
                        let a = forge_lang::query::ask(&src, dir, None, &q);
                        if a.ok {
                            Ok((a.text, outline(&a.json)))
                        } else {
                            Err(a.text)
                        }
                    }
                };
                let _ = send.send(got);
                waker();
            });
        }
        if let Some(r) = &self.inspector.outline
            && let Ok(got) = r.try_recv()
        {
            self.inspector.outline = None;
            self.shell.inspect.outline = Some(got.map(|(text, o)| {
                self.ctx.copy_text(text);
                o
            }));
        }
        if std::mem::take(&mut self.shell.inspect.want_physical) {
            let assembly = self.inspector.data.as_ref().is_some_and(|d| d.assembly);
            match assembly {
                true => {
                    self.shell.inspect.physical_busy = true;
                    self.inspect_send(Job::Physical);
                }
                false => {
                    let p = self.inspector.data.as_ref().and_then(|d| d.physical.clone());
                    self.shell.inspect.physical = Some(p.ok_or("the part isn't built".into()));
                }
            }
        }
        let cut = self.shell.inspect.cut.as_ref().map(|s| s.renderer());
        if cut != self.inspector.cut {
            self.inspector.cut = cut;
            if let Some(v) = &mut self.view {
                v.mark_dirty();
            }
        }
    }

    /// The renderer's section: the Section view's, else the View tab's axis cut.
    pub(crate) fn inspect_section(&mut self) {
        let cut = self.inspector.cut;
        if let Some(v) = &mut self.view {
            match cut {
                Some((a, b, end)) => {
                    v.renderer.section = Some(a);
                    v.renderer.section2 = b;
                    v.renderer.section_end = end;
                }
                None => {
                    v.renderer.section = v.nav.section_plane();
                    v.renderer.section2 = None;
                    v.renderer.section_end = false;
                }
            }
        }
    }

    /// A click in the view: a plane picked for the section, or a pick for Measure.
    fn inspect_clicks(&mut self) {
        let open = self.shell.inspect.measure.is_some() || self.shell.inspect.pick_plane.is_some();
        let Some(v) = &self.view else { return };
        let clicks = v.nav.clicks;
        let hit = v.nav.last_click;
        if !open || !self.inspector.open {
            self.inspector.open = open;
            self.inspector.clicks = clicks;
            return;
        }
        if clicks == self.inspector.clicks {
            return;
        }
        let was = self.inspector.clicks;
        self.inspector.clicks = clicks;
        if let Some(k) = self.shell.inspect.pick_plane {
            if let Some((pl, e @ Entity::Face(f))) = hit
                && let Some((_, name)) = self.picked(pl, e)
            {
                let plane = match self.inspector.ask(Ask::Plane(pl, f)) {
                    Ok(Told::Plane(p)) => p,
                    Ok(_) => None,
                    // the click again next frame, once the build worker has said
                    Err(Waiting) => {
                        self.inspector.clicks = was;
                        return;
                    }
                };
                let Some((o, n)) = plane else { return };
                if let Some(s) = &mut self.shell.inspect.cut {
                    s.planes[k] = CutPlane {
                        name,
                        origin: [o.x, o.y, o.z],
                        normal: [n.x, n.y, n.z],
                        offset: 0.0,
                        flip: false,
                    };
                }
                self.shell.inspect.pick_plane = None;
            }
            return;
        }
        let Some(m) = &self.shell.inspect.measure else {
            return;
        };
        let (filter, wall) = (m.filter, m.wall);
        // in a sketch its own geometry is what's measured (sketching.rs takes those clicks)
        if self.shell.sketch.is_some() {
            return;
        }
        let at = self.view.as_ref().and_then(|v| v.nav.last_click_at);
        let pick = match hit {
            Some((pl, e)) => match self.filtered(filter, pl, e, at) {
                Ok(p) => p,
                // the click again next frame, once the build worker has said
                Err(Waiting) => {
                    self.inspector.clicks = was;
                    return;
                }
            },
            // an origin plane, axis or point under the pointer
            None => self.shell.origin_hover.map(origin_free),
        };
        let shift = self.view.as_ref().is_some_and(|v| v.nav.shift_held());
        let Some(m) = &mut self.shell.inspect.measure else {
            return;
        };
        match pick {
            Some(p) if wall => m.picks = vec![p],
            Some(p) => m.take_with(p, shift),
            None => m.restart(),
        }
    }

    /// What's under the pointer, as Measure would pick it: marked before the click.
    fn measure_hover(&mut self) {
        let Some(m) = &self.shell.inspect.measure else {
            return;
        };
        let filter = m.filter;
        let Some(v) = &self.view else { return };
        // what's under the pointer even when it's picked already: else the plane behind it shows
        let hover = v.nav.hover_hit();
        let at = v.nav.hit_point();
        let pick = match hover {
            Some((pl, e)) => match self.filtered(filter, pl, e, at) {
                Ok(p) => p,
                // what was under the pointer stays marked till the build worker says
                Err(Waiting) => return,
            },
            None => self.shell.origin_hover.map(origin_free),
        };
        if let Some(m) = &mut self.shell.inspect.measure
            && m.hover != pick
        {
            m.hover = pick;
            if let Some(v) = &mut self.view {
                v.mark_dirty();
            }
        }
    }

    /// An origin item chosen in the browser while measuring: a pick.
    fn measure_browser(&mut self) {
        let now = self.shell.origin_selected;
        if now == self.inspector.origin_seen {
            return;
        }
        self.inspector.origin_seen = now;
        if let (Some(i), Some(m)) = (now, &mut self.shell.inspect.measure)
            && !m.wall
        {
            m.take(origin_free(i));
        }
    }

    /// What Measure has picked, highlighted and measured.
    fn inspect_measure(&mut self) {
        use std::sync::atomic::Ordering::Relaxed;
        let Some(m) = &mut self.shell.inspect.measure else {
            self.inspector.measured = None;
            // closed: a wall's search stops
            self.inspector.wall.wanted.store(0, Relaxed);
            self.inspector.walking = None;
            return;
        };
        // the panel's Measure thinnest wall, and Cancel
        if std::mem::take(&mut m.wall_go) {
            m.wall_held = false;
        }
        if std::mem::take(&mut m.wall_stop) {
            m.wall_held = true;
            m.busy = false;
            m.wall_done = 0.0;
            self.inspector.wall.wanted.store(0, Relaxed);
            self.inspector.walking = None;
        }
        if m.busy && m.wall {
            m.wall_done = match &self.inspector.walking {
                Some((_, w, _)) => w.done(),
                None => self.inspector.wall.done.load(Relaxed) as f32 / 1000.0,
            };
        }
        let (picks, wall) = (m.picks.clone(), m.wall);
        let held = m.wall_held;
        let marked = m.marked();
        if !self.inspector.warmed
            && let Some(d) = &self.inspector.data
        {
            self.inspector.warmed = true;
            // a part's body, or an assembly's first few: the rest as they're picked
            let _ = d;
            self.inspect_send(Job::Warm);
        }
        if let Some(v) = &mut self.view
            && v.nav.selected != marked
        {
            v.nav.selected = marked;
            v.mark_dirty();
        }
        if self.inspector.measured.as_ref() == Some(&(picks.clone(), wall, held)) {
            return;
        }
        self.inspector.measured = Some((picks.clone(), wall, held));
        // an occurrence's mass is from the assembly's materials: worked out once, when asked
        let s = &mut self.shell.inspect;
        if picks.iter().any(|p| matches!(p, MPick::Whole(_)))
            && self.inspector.data.as_ref().is_some_and(|d| d.assembly)
            && s.physical.is_none()
            && !s.physical_busy
        {
            s.want_physical = true;
        }
        let got: Vec<(Tgt, String)> = picks.iter().filter_map(|p| self.target_of(p)).collect();
        self.inspector.seq += 1;
        let id = self.inspector.seq;
        let names: Vec<String> = got.iter().map(|g| g.1.clone()).collect();
        // the same picks read again (the part rebuilt): the last reading stands till the new one
        let again = self.inspector.asked.as_ref() == Some(&(picks.clone(), wall));
        // other picks, where a search would hold the page: wait to be asked again
        let held = held || (wall && !again && crate::plat::INLINE_JOBS);
        self.inspector.walking = None;
        let mut walking = false;
        let job = if wall && held {
            None
        } else if wall {
            // the part picked, or the part (an assembly's first)
            let first = got.first().and_then(|g| match &g.0 {
                Tgt::Ent(pl, _) | Tgt::Whole(pl, _) => Some(*pl),
                Tgt::Free(_) => None,
            });
            let target = first.or_else(|| self.inspector.data.as_ref()?.brief_at(0).map(|_| 0));
            // stepped here when the bodies are here (not the build worker's)
            let here = self
                .inspector
                .data
                .clone()
                .filter(|d| !d.bodies.is_empty() && crate::plat::INLINE_JOBS);
            self.inspector.wall.wanted.store(id, Relaxed);
            self.inspector.wall.done.store(0, Relaxed);
            match target {
                // stepped here, frame by frame (`inspect_walk`): a job would run to its end first
                Some(pl) if let Some((b, at)) = here.as_ref().and_then(|d| d.body_at(pl)) => {
                    let r = forge_mesh::tessellate::tessellate_tolerant(b, &forge_mesh::Quality::default())
                        .map_err(|e| forge_lang::error::words(&e));
                    match r {
                        Ok(mesh) => {
                            let w = forge_wall::WallSearch::new(b, Arc::new(mesh));
                            self.inspector.walking = Some((id, w, at));
                            walking = true;
                        }
                        Err(e) => {
                            if let Some(m) = &mut self.shell.inspect.measure {
                                m.wall_at = Some(Err(e));
                            }
                        }
                    }
                    None
                }
                Some(pl) => Some(Job::Wall(id, pl, self.inspector.wall.clone())),
                None => None,
            }
        } else if got.is_empty() {
            None
        } else {
            Some(Job::Measure(id, got.into_iter().map(|g| g.0).collect()))
        };
        self.inspector.asked = Some((picks.clone(), wall));
        if let Some(m) = &mut self.shell.inspect.measure {
            m.names = names;
            if !again {
                m.reading = None;
                m.wall_at = None;
            }
            m.wall_held = held;
            m.busy = job.is_some() || walking;
        }
        self.inspector.measured = Some((picks, wall, held));
        if let Some(j) = job {
            self.inspect_send(j);
        }
    }
}

/// What a click (or the pointer) at a placement's entity picks under the selection priority, on
/// the bodies: the face of an edge facing the eye, the nearest edge of a face, the nearest point.
/// `at` and `eye` in the world.
fn filtered_on(
    d: &InspectData,
    filter: Filter,
    pl: u32,
    e: Entity,
    at: Option<[f32; 3]>,
    eye: Option<[f32; 3]>,
) -> Option<MPick> {
    let (b, frame) = d.body_at(pl)?;
    let near = at.map(|p| frame.point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)));
    let near = near.unwrap_or_else(|| select::centre(b, ent(e)));
    let eye = eye.map(|q| frame.point_to_local(Point3::new(q[0] as f64, q[1] as f64, q[2] as f64)));
    let edge_gap = |x: EdgeId| {
        b.edge_points(x, 48)
            .into_iter()
            .map(|q| q.distance(near))
            .fold(f64::INFINITY, f64::min)
    };
    let ends = |x: EdgeId| [b.edge(x).start, b.edge(x).end];
    let edges_at = |v: VertexId| b.edge_ids().filter(|&x| ends(x).contains(&v)).collect::<Vec<_>>();
    let nearest_vertex = |vs: Vec<VertexId>| {
        vs.into_iter().min_by(|a, c| {
            b.vertex(*a)
                .point
                .distance(near)
                .total_cmp(&b.vertex(*c).point.distance(near))
        })
    };
    let picked = match (filter, ent(e)) {
        (Filter::Faces, Ent::Face(_)) | (Filter::Edges, Ent::Edge(_)) | (Filter::Points, Ent::Vert(_)) => {
            ent(e)
        }
        (Filter::Faces, Ent::Edge(x)) => Ent::Face(facing(b, &select::faces_of_edge(b, x), near, eye)?),
        (Filter::Faces, Ent::Vert(v)) => {
            let mut fs: Vec<FaceId> = edges_at(v)
                .into_iter()
                .flat_map(|x| select::faces_of_edge(b, x))
                .collect();
            fs.sort_by_key(|f| f.0);
            fs.dedup();
            Ent::Face(facing(b, &fs, near, eye)?)
        }
        (Filter::Edges, Ent::Face(f)) => Ent::Edge(
            select::edges_of_face(b, f)
                .into_iter()
                .min_by(|a, c| edge_gap(*a).total_cmp(&edge_gap(*c)))?,
        ),
        (Filter::Edges, Ent::Vert(v)) => Ent::Edge(*edges_at(v).first()?),
        (Filter::Points, Ent::Edge(x)) => Ent::Vert(nearest_vertex(ends(x).to_vec())?),
        (Filter::Points, Ent::Face(f)) => Ent::Vert(nearest_vertex(
            select::edges_of_face(b, f).into_iter().flat_map(ends).collect(),
        )?),
        _ => return None,
    };
    Some(MPick::Ent(
        pl,
        match picked {
            Ent::Face(f) => Entity::Face(f.0),
            Ent::Edge(x) => Entity::Edge(x.0),
            Ent::Vert(v) => Entity::Vertex(v.0),
        },
    ))
}

/// The face a click on an edge or a corner meant, of those `ok` takes, on the bodies.
fn face_at_on(
    d: &InspectData,
    pl: u32,
    e: Entity,
    at: Option<[f32; 3]>,
    eye: Option<[f32; 3]>,
    ok: impl Fn(u32) -> bool,
) -> Option<Entity> {
    let (b, frame) = d.body_at(pl)?;
    let near = at.map(|p| frame.point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)));
    let near = near.unwrap_or_else(|| select::centre(b, ent(e)));
    let eye = eye.map(|q| frame.point_to_local(Point3::new(q[0] as f64, q[1] as f64, q[2] as f64)));
    let mut faces: Vec<FaceId> = match ent(e) {
        Ent::Face(_) => return Some(e),
        Ent::Edge(x) => select::faces_of_edge(b, x),
        Ent::Vert(v) => b
            .edge_ids()
            .filter(|&x| [b.edge(x).start, b.edge(x).end].contains(&v))
            .flat_map(|x| select::faces_of_edge(b, x))
            .collect(),
    };
    faces.sort_by_key(|f| f.0);
    faces.dedup();
    faces.retain(|f| ok(f.0));
    Some(Entity::Face(facing(b, &faces, near, eye)?.0))
}

/// Of faces meeting at a point, the one turned most to the eye there.
fn facing(b: &Body, faces: &[FaceId], near: Point3, eye: Option<Point3>) -> Option<FaceId> {
    let to_eye = eye.map_or(Vec3::Z, |e| e - near);
    faces.iter().copied().max_by(|a, c| {
        let n = |f: FaceId| forge_agent::measure::normal_near(b, f, near).map_or(-1.0, |n| n.dot(to_eye));
        n(*a).total_cmp(&n(*c))
    })
}

/// A question of the bodies the page asks as it draws (a pick refined, a plane picked): answered
/// at once where the bodies are on the page, else by the build worker (`Told` comes back a frame
/// or so later; the page asks again till then).
#[derive(Clone, Debug)]
pub(crate) enum Ask {
    /// `filtered_on`: the filter, the placement, the entity hit, the point hit and the eye (world).
    Filtered(Filter, u32, Entity, Option<[f32; 3]>, Option<[f32; 3]>),
    /// `face_at_on`: the placement, the entity hit, the point and the eye, the faces it may be.
    FaceAt(u32, Entity, Option<[f32; 3]>, Option<[f32; 3]>, Vec<u32>),
    /// A planar face's plane in the world: its centre and normal.
    Plane(u32, u32),
    /// Project Geometry's picks on the part (`sketching::project_picks`), by these anchors.
    Project(Option<Box<crate::fg::AnchorView>>, Vec<Entity>),
    /// Each interference's place and faces (`gui::locate`): the document's text and folder.
    Locate(String, Option<std::path::PathBuf>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Told {
    Pick(Option<MPick>),
    Face(Option<Entity>),
    Plane(Option<(Point3, Vec3)>),
    Project(Result<Vec<(String, crate::shell::sketching::rows::Refd)>, String>),
    Located(Result<Vec<crate::shell::fgasm::Located>, String>),
}

/// Asked of the build worker, not answered yet.
#[derive(Debug)]
pub(crate) struct Waiting;

/// The answer, on `d`'s bodies.
pub(crate) fn tell(d: &InspectData, a: &Ask) -> Told {
    match a {
        Ask::Filtered(filter, pl, e, at, eye) => Told::Pick(filtered_on(d, *filter, *pl, *e, *at, *eye)),
        Ask::FaceAt(pl, e, at, eye, allowed) => {
            Told::Face(face_at_on(d, *pl, *e, *at, *eye, |f| allowed.contains(&f)))
        }
        Ask::Plane(pl, f) => Told::Plane((|| {
            let (b, at) = d.body_at(*pl)?;
            let f = FaceId(*f);
            if f.0 as usize >= b.faces.len() {
                return None;
            }
            let n = forge_agent::select::plane_normal(b, f)?;
            Some((
                at.point_to_world(select::centre(b, Ent::Face(f))),
                at.vec_to_world(n),
            ))
        })()),
        Ask::Project(anchors, picked) => Told::Project(crate::shell::sketching::project_picks(
            anchors.as_deref(),
            d.bodies.first().map(|b| &**b),
            picked,
        )),
        Ask::Locate(src, dir) => Told::Located(super::locate(src, dir.clone(), Some(d))),
    }
}

impl Inspector {
    /// A question of the current build's bodies: answered now where they're on the page; else
    /// the build worker's answer once it's come (`Err` till then: asked once).
    pub(crate) fn ask(&self, a: Ask) -> Result<Told, Waiting> {
        let Some(d) = &self.data else {
            return Ok(tell(&InspectData::default(), &a));
        };
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        if crate::fg::remote::page::on() {
            let key = [crate::wire::to_bytes(&d.number), crate::wire::to_bytes(&a)].concat();
            let mut told = self.told.borrow_mut();
            if let Some(t) = told.get(&key) {
                return t.clone().ok_or(Waiting);
            }
            // asked: none till it's told
            if told.len() > 512 {
                told.clear();
            }
            told.insert(key, None);
            crate::fg::remote::page::ask(d.number, a);
            return Err(Waiting);
        }
        Ok(tell(d, &a))
    }

    /// The build worker's answer to an `Ask` of the build `number`.
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    pub(crate) fn told(&self, number: u64, a: &Ask, t: Told) {
        let key = [crate::wire::to_bytes(&number), crate::wire::to_bytes(a)].concat();
        self.told.borrow_mut().insert(key, Some(t));
    }
}

impl Gui {
    /// Where jobs run on the page's one thread: the wall's search, a few milliseconds of it each
    /// frame, so the page draws on and Cancel can be clicked.
    pub(crate) fn inspect_walk(&mut self) {
        let Some((id, w, at)) = &mut self.inspector.walking else {
            return;
        };
        let t0 = crate::viewer::now_ms();
        let mut finished = false;
        // (25 ms of each frame: the page still draws and takes clicks at about 30 a second)
        while !finished && crate::viewer::now_ms() - t0 < 25.0 {
            finished = w.step(256);
        }
        let (id, found, at) = (*id, w.found(), *at);
        if !finished || id != self.inspector.seq {
            return;
        }
        self.inspector.walking = None;
        if let Some(m) = &mut self.shell.inspect.measure {
            m.wall_at = Some(wall_found(found, at));
            m.busy = false;
        }
    }
}

/// A wall search's answer, in the world.
pub(crate) fn wall_found(found: Option<(f64, Point3)>, at: Frame) -> Result<(f64, Point3), String> {
    found
        .map(|(t, p)| (t, at.point_to_world(p)))
        .ok_or("no wall: nothing opposite any face".to_string())
}

/// An origin item as Measure takes it: in the world, with its name.
fn origin_free(i: crate::origin::Item) -> MPick {
    use crate::origin::Item;
    let o = Point3::ORIGIN;
    let f = match i {
        Item::XyPlane => Free::Plane(o, Vec3::Z),
        Item::XzPlane => Free::Plane(o, Vec3::Y),
        Item::YzPlane => Free::Plane(o, Vec3::X),
        Item::XAxis => Free::Axis(o, Vec3::X),
        Item::YAxis => Free::Axis(o, Vec3::Y),
        Item::ZAxis => Free::Axis(o, Vec3::Z),
        Item::Center => Free::Point(o),
    };
    MPick::Free(f, i.name().to_string())
}

/// Measure as a tool for the in-app agent, in the shape the Agent tab's tool table takes (name,
/// what it does, its JSON schema's properties, the required ones).
pub const MEASURE_TOOL: (&str, &str, &str, &[&str]) = (
    "measure",
    "Measure the open part or assembly exactly, as the Measure command does: one thing (a face's area and perimeter, an edge's length, a radius, a point's place, a body's volume, mass and box) or two (minimum distance, deltas, centre to centre, angle). A thing is a face or edge by its Forge name (`extrude1.end`, `hole1.bore`), `face:N` / `edge:N` / `vertex:N` (the view's numbers), an origin item (`xy`, `xz`, `yz`, `x`, `y`, `z`, `origin`), `body` or an occurrence's name for a whole body, or `selected` for what's chosen in the view.",
    r#"{"a":{"type":"string"},"b":{"type":"string"}}"#,
    &["a"],
);

impl Gui {
    /// What each placement is called, for `name/face:N` in `measure`.
    pub fn measure_names(&self) -> Vec<String> {
        self.inspector.data.as_ref().map_or(vec![], |d| d.names.clone())
    }

    /// A thing named for Measure: see `MEASURE_TOOL`.
    fn measure_ref(&self, text: &str) -> Result<MPick, String> {
        use crate::origin::Item;
        let t = text.trim();
        let origin = match t {
            "xy" => Some(Item::XyPlane),
            "xz" => Some(Item::XzPlane),
            "yz" => Some(Item::YzPlane),
            "x" => Some(Item::XAxis),
            "y" => Some(Item::YAxis),
            "z" => Some(Item::ZAxis),
            "origin" => Some(Item::Center),
            _ => None,
        };
        if let Some(i) = origin {
            return Ok(origin_free(i));
        }
        let d = self
            .inspector
            .data
            .as_ref()
            .ok_or("nothing is built to measure yet")?;
        let (pl, rest) = match t
            .split_once('/')
            .filter(|(occ, _)| d.names.iter().any(|n| n == occ))
        {
            // an occurrence's entity: `name/face:3`
            Some((occ, r)) => (
                d.names
                    .iter()
                    .position(|n| n == occ)
                    .ok_or(format!("no occurrence `{occ}`"))? as u32,
                r,
            ),
            None => (0, t),
        };
        let number = |w: &str| {
            w.parse::<u32>()
                .map_err(|_| format!("`{t}`: not a number after `:`"))
        };
        if let Some((kind, k)) = rest
            .split_once(':')
            .filter(|(k, _)| matches!(*k, "face" | "edge" | "vertex" | "body"))
        {
            let e = match kind {
                "face" => Entity::Face(number(k)?),
                "edge" => Entity::Edge(number(k)?),
                "vertex" => Entity::Vertex(number(k)?),
                _ => return Ok(MPick::Whole(number(k)?)),
            };
            self.picked(pl, e).ok_or(format!("`{t}` isn't there"))?;
            return Ok(MPick::Ent(pl, e));
        }
        if rest == "body" {
            return Ok(MPick::Whole(pl));
        }
        if let Some(k) = d.names.iter().position(|n| n == rest) {
            return Ok(MPick::Whole(k as u32));
        }
        if rest == "selected" {
            return Err("`selected` is one of two: say `selected` as a and leave b out, or `selected:1` and `selected:2`".into());
        }
        // a Forge name: the faces or edges anchors call so
        let a = self
            .fg_anchors
            .as_ref()
            .ok_or(format!("`{t}`: the part's names aren't found yet"))?;
        if let Some(f) = a.faces.iter().position(|n| n.as_deref() == Some(rest)) {
            return Ok(MPick::Ent(pl, Entity::Face(f as u32)));
        }
        if let Some(e) = a.edges.iter().position(|n| n.as_deref() == Some(rest)) {
            return Ok(MPick::Ent(pl, Entity::Edge(e as u32)));
        }
        Err(format!(
            "nothing called `{t}` to measure (try face:N, an origin plane, or body)"
        ))
    }

    /// The agent's `measure(a, b)`: the reading in words, a value a line, in the panel's units
    /// (Settings' when Measure isn't open). Works it out here and now.
    pub fn measure(&mut self, a: &str, b: Option<&str>) -> Result<String, String> {
        let chosen = self
            .shell
            .inspect
            .measure
            .as_ref()
            .map(|m| m.picks.clone())
            .unwrap_or_else(|| {
                self.view.as_ref().map_or(vec![], |v| {
                    v.nav.selected.iter().map(|&(pl, e)| MPick::Ent(pl, e)).collect()
                })
            });
        let one = |text: &str| -> Result<MPick, String> {
            match text.trim() {
                "selected" | "selected:1" => chosen.first().cloned().ok_or("nothing is selected".into()),
                "selected:2" => chosen.get(1).cloned().ok_or("only one thing is selected".into()),
                t => self.measure_ref(t),
            }
        };
        let mut picks = vec![one(a)?];
        if let Some(b) = b.filter(|b| !b.trim().is_empty()) {
            picks.push(one(b)?);
        }
        let targets: Vec<(Tgt, String)> = picks
            .iter()
            .map(|p| self.target_of(p).ok_or(format!("{p:?} isn't there to measure")))
            .collect::<Result<_, _>>()?;
        // here and now, on the bodies where they are (none on the page with the build worker)
        let d = self
            .inspector
            .data
            .clone()
            .ok_or("nothing is built to measure yet")?;
        if d.bodies.is_empty() && !d.briefs.is_empty() {
            return Err("measure: the bodies are the build worker's; measure in the Measure panel".into());
        }
        let tgts: Vec<Tgt> = targets.iter().map(|t| t.0.clone()).collect();
        let r = measure_of(&d, &tgts, &mut Meshes::default())?;
        let (unit, angle, n) = match &self.shell.inspect.measure {
            Some(m) => (m.unit, m.angle, m.precision),
            None => (
                self.shell.units,
                self.shell.settings.angle,
                self.shell.settings.decimals,
            ),
        };
        let mut out: Vec<String> = targets.iter().map(|t| t.1.clone()).collect();
        out[0] = format!("Measured: {}", out.join(" and "));
        out.truncate(1);
        for (q, x) in &r.values {
            out.push(format!(
                "{}: {}",
                q.label(),
                crate::shell::inspect::shown(*q, *x, (unit, angle), n)
            ));
        }
        if r.values.is_empty() {
            out.push("nothing to measure between those".into());
        }
        Ok(out.join("\n"))
    }
}

/// A section answer's numbers for the Section View: loops by kind, area, extent.
fn outline(j: &forge_agent::json::Json) -> crate::shell::inspect::Outline {
    use forge_agent::json::Json;
    let mut o = crate::shell::inspect::Outline {
        outlines: 0,
        holes: 0,
        area: j.get("area").and_then(Json::num).unwrap_or(0.0),
        lo: [f64::INFINITY; 3],
        hi: [f64::NEG_INFINITY; 3],
    };
    if let Some(Json::Arr(ls)) = j.get("loops") {
        for l in ls {
            match l.get("kind").and_then(Json::str) {
                Some("hole") => o.holes += 1,
                _ => o.outlines += 1,
            }
            for (k, far) in [("min", false), ("max", true)] {
                if let Some(Json::Arr(p)) = l.get(k) {
                    for (i, x) in p.iter().take(3).enumerate() {
                        let v = x.num().unwrap_or(0.0);
                        if far {
                            o.hi[i] = o.hi[i].max(v);
                        } else {
                            o.lo[i] = o.lo[i].min(v);
                        }
                    }
                }
            }
        }
    }
    if o.outlines + o.holes == 0 {
        o.lo = [0.0; 3];
        o.hi = [0.0; 3];
    }
    o
}
