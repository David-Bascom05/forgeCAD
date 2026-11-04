//! The host side of the Inspect tools: the exact bodies behind what's drawn, the view's picks
//! turned into measurements on a thread of their own (a body meshed once for them), the section
//! put on the renderer, and the physical properties worked out when asked.

use super::Gui;
use crate::render::Entity;
use crate::shell::inspect::CutPlane;
use forge_agent::measure::{Pick, Reading, read_one, read_two};
use forge_agent::select::Ent;
use forge_lang::props::{Physical, Props};
use forge_math::{Frame, Point3};
use forge_mesh::TriMesh;
use forge_topo::{Body, EdgeId, FaceId, VertexId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// What a build gives the Inspect tools: each part mesh's exact body, each placement's mesh and
/// pose, the document's iProperties, a part's physical properties, an assembly's occurrences.
#[derive(Default)]
pub struct InspectData {
    pub bodies: Vec<Arc<Body>>,
    pub placed: Vec<(usize, Frame)>,
    pub names: Vec<String>,
    pub props: Props,
    pub physical: Option<Physical>,
    pub asm: Option<Arc<forge_lang::asm::AsmOut>>,
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

type Picked = (Arc<Body>, Frame, Ent);

enum Job {
    Measure(u64, Vec<Picked>),
    Wall(u64, Arc<Body>, Frame),
    Physical(Arc<forge_lang::asm::AsmOut>),
}

enum Answer {
    Measure(u64, Result<Reading, String>),
    Wall(u64, Result<(f64, Point3), String>),
    Physical(Box<Result<Physical, String>>),
}

/// The host's state for the tools.
#[derive(Default)]
pub struct Inspector {
    to: Option<Sender<Job>>,
    from: Option<Receiver<Answer>>,
    seq: u64,
    data: Option<Arc<InspectData>>,
    /// The view's click count last seen, and the picks last measured.
    clicks: u32,
    open: bool,
    measured: Option<(Vec<(u32, Entity)>, bool)>,
    /// What the renderer was last given.
    cut: Option<crate::shell::inspect::Cut>,
}

fn ent(e: Entity) -> Ent {
    match e {
        Entity::Face(i) => Ent::Face(FaceId(i)),
        Entity::Edge(i) => Ent::Edge(EdgeId(i)),
        Entity::Vertex(i) => Ent::Vert(VertexId(i)),
    }
}

fn kind_word(b: &Body, e: Ent) -> String {
    match e {
        Ent::Vert(_) => "Vertex".into(),
        Ent::Edge(x) => format!("Edge ({})", forge_agent::select::curve_kind(&b.edge(x).curve)),
        Ent::Face(x) => format!("Face ({})", forge_agent::select::surface_kind(&b.face(x).surface)),
    }
}

fn work(jobs: Receiver<Job>, out: Sender<Answer>, wake: super::Waker) {
    let mut meshes: HashMap<usize, Arc<TriMesh>> = HashMap::new();
    let mut mesh = |b: &Arc<Body>| -> Result<Arc<TriMesh>, String> {
        let key = Arc::as_ptr(b) as usize;
        if let Some(m) = meshes.get(&key) {
            return Ok(m.clone());
        }
        let m = forge_mesh::tessellate::tessellate_tolerant(b, &forge_mesh::Quality::default())
            .map_err(|e| forge_lang::error::words(&e))?;
        let m = Arc::new(m);
        meshes.insert(key, m.clone());
        Ok(m)
    };
    while let Ok(mut job) = jobs.recv() {
        // a newer job of the same kind replaces one not yet started
        while let Ok(j) = jobs.try_recv() {
            if std::mem::discriminant(&j) != std::mem::discriminant(&job) {
                let _ = handle(job, &mut mesh, &out);
            }
            job = j;
        }
        if handle(job, &mut mesh, &out).is_err() {
            return;
        }
        wake();
    }
}

fn handle(
    job: Job,
    mesh: &mut impl FnMut(&Arc<Body>) -> Result<Arc<TriMesh>, String>,
    out: &Sender<Answer>,
) -> Result<(), ()> {
    let a = match job {
        Job::Measure(id, picks) => {
            let got: Result<Vec<Arc<TriMesh>>, String> = picks.iter().map(|p| mesh(&p.0)).collect();
            let r = got.map(|ms| {
                let ps: Vec<Pick> = picks
                    .iter()
                    .zip(&ms)
                    .map(|(p, m)| Pick {
                        body: &p.0,
                        mesh: m,
                        at: p.1,
                        ent: p.2,
                    })
                    .collect();
                match &ps[..] {
                    [a] => read_one(a),
                    [a, b] => read_two(a, b),
                    _ => Reading::default(),
                }
            });
            Answer::Measure(id, r)
        }
        Job::Wall(id, b, at) => {
            let r = mesh(&b).and_then(|m| {
                forge_agent::measure::wall(&b, &m)
                    .map(|(t, p)| (t, at.point_to_world(p)))
                    .ok_or("no wall: nothing opposite any face".to_string())
            });
            Answer::Wall(id, r)
        }
        Job::Physical(asm) => Answer::Physical(Box::new(forge_lang::props::asm_physical(&asm, None))),
    };
    out.send(a).map_err(|_| ())
}

impl Gui {
    fn inspect_send(&mut self, job: Job) {
        let i = &mut self.inspector;
        if i.to.is_none() {
            let (to, jobs) = channel();
            let (answers, from) = channel();
            let wake = self.waker.clone();
            std::thread::Builder::new()
                .name("forge-inspect".into())
                .spawn(move || work(jobs, answers, wake))
                .expect("a thread");
            i.to = Some(to);
            i.from = Some(from);
        }
        if let Some(to) = &i.to {
            let _ = to.send(job);
        }
    }

    /// A build's data for the tools.
    pub(crate) fn inspect_built(&mut self, d: Option<Arc<InspectData>>) {
        let Some(d) = d else { return };
        let s = &mut self.shell.inspect;
        s.doc = d.props.clone();
        s.assembly = d.asm.is_some();
        if d.asm.is_none() {
            s.physical = d.physical.clone().map(Ok);
        } else {
            if s.props.as_ref().is_some_and(|p| p.tab == 5) || s.cg {
                s.want_physical = true;
            }
        }
        self.inspector.data = Some(d);
        // what's picked may be gone
        self.inspector.measured = None;
    }

    /// A pick on placement `pl`: its body, pose and name.
    fn picked(&self, pl: u32, e: Entity) -> Option<(Picked, String)> {
        let d = self.inspector.data.as_ref()?;
        let &(mesh, at) = d.placed.get(pl as usize)?;
        let b = d.bodies.get(mesh)?.clone();
        let e = ent(e);
        let ok = match e {
            Ent::Face(f) => (f.0 as usize) < b.faces.len(),
            Ent::Edge(x) => (x.0 as usize) < b.edges.len(),
            Ent::Vert(v) => (v.0 as usize) < b.vertices.len(),
        };
        if !ok {
            return None;
        }
        let name = match d.names.get(pl as usize) {
            Some(n) if !n.is_empty() => format!("{} · {n}", kind_word(&b, e)),
            _ => kind_word(&b, e),
        };
        Some(((b, at, e), name))
    }

    /// Once a frame: the view's clicks taken as picks, jobs sent and answers taken, the section
    /// put on the renderer.
    pub(crate) fn inspect_tick(&mut self) {
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
                Answer::Wall(id, r) if id == self.inspector.seq => {
                    if let Some(m) = &mut s.measure {
                        m.wall_at = Some(r);
                        m.busy = false;
                    }
                }
                Answer::Physical(r) => {
                    s.physical = Some(*r);
                    s.physical_busy = false;
                }
                _ => {}
            }
            if let Some(v) = &mut self.view {
                v.mark_dirty();
            }
        }
        self.inspect_clicks();
        self.inspect_measure();
        if std::mem::take(&mut self.shell.inspect.want_physical) {
            let asm = self.inspector.data.as_ref().and_then(|d| d.asm.clone());
            match asm {
                Some(a) => {
                    self.shell.inspect.physical_busy = true;
                    self.inspect_send(Job::Physical(a));
                }
                None => {
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
        self.inspector.clicks = clicks;
        if let Some(k) = self.shell.inspect.pick_plane {
            if let Some((pl, e @ Entity::Face(_))) = hit
                && let Some(((b, at, Ent::Face(f)), name)) = self.picked(pl, e)
                && let Some(n) = forge_agent::select::plane_normal(&b, f)
            {
                let o = at.point_to_world(forge_agent::select::centre(&b, Ent::Face(f)));
                let n = at.vec_to_world(n);
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
        let Some(m) = &mut self.shell.inspect.measure else {
            return;
        };
        match hit {
            Some(h) if m.wall => m.picks = vec![h],
            Some(h) => {
                if m.picks.len() >= 2 {
                    m.picks.clear();
                }
                if !m.picks.contains(&h) {
                    m.picks.push(h);
                }
            }
            None => m.picks.clear(),
        }
    }

    /// What Measure has picked, highlighted and measured.
    fn inspect_measure(&mut self) {
        let Some(m) = &self.shell.inspect.measure else {
            self.inspector.measured = None;
            return;
        };
        let (picks, wall) = (m.picks.clone(), m.wall);
        if let Some(v) = &mut self.view
            && v.nav.selected != picks
        {
            v.nav.selected = picks.clone();
            v.mark_dirty();
        }
        if self.inspector.measured.as_ref() == Some(&(picks.clone(), wall)) {
            return;
        }
        self.inspector.measured = Some((picks.clone(), wall));
        let got: Vec<(Picked, String)> = picks.iter().filter_map(|&(pl, e)| self.picked(pl, e)).collect();
        self.inspector.seq += 1;
        let id = self.inspector.seq;
        let names: Vec<String> = got.iter().map(|g| g.1.clone()).collect();
        let job = if wall {
            // the part picked, or the part (an assembly's first)
            let target = got.first().map(|g| (g.0.0.clone(), g.0.1)).or_else(|| {
                let d = self.inspector.data.as_ref()?;
                let &(mesh, at) = d.placed.first()?;
                Some((d.bodies.get(mesh)?.clone(), at))
            });
            target.map(|(b, at)| Job::Wall(id, b, at))
        } else if got.is_empty() {
            None
        } else {
            Some(Job::Measure(id, got.into_iter().map(|g| g.0).collect()))
        };
        if let Some(m) = &mut self.shell.inspect.measure {
            m.names = names;
            m.reading = None;
            m.wall_at = None;
            m.busy = job.is_some();
        }
        if let Some(j) = job {
            self.inspect_send(j);
        }
    }
}
