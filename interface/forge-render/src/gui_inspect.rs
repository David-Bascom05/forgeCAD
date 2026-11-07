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

/// A pick as the worker takes it: what it is, with the body it's of.
#[derive(Clone)]
enum Tgt {
    Ent(Picked),
    /// A whole body where it stands, and its density (kg/mm³) where its material says.
    Whole(Arc<Body>, Frame, Option<f64>),
    Free(Free),
}

enum Job {
    Measure(u64, Vec<Tgt>),
    /// Bodies meshed ahead, as Measure opens: the first pick's answer doesn't wait for them.
    Warm(Vec<Arc<Body>>),
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
    pub(crate) data: Option<Arc<InspectData>>,
    /// The view's click count last seen, and the picks last measured.
    clicks: u32,
    open: bool,
    measured: Option<(Vec<MPick>, bool)>,
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
        Job::Warm(bodies) => {
            for b in &bodies {
                let _ = mesh(b);
            }
            return Ok(());
        }
        Job::Measure(id, picks) => {
            let mut meshes = vec![];
            let mut err = None;
            // one edge or point reads off the body alone; faces, bodies and pairs want the mesh
            let wants = picks.len() > 1
                || picks
                    .iter()
                    .any(|p| matches!(p, Tgt::Whole(..) | Tgt::Ent((_, _, Ent::Face(_)))));
            let bare = Arc::new(TriMesh::default());
            for p in &picks {
                let m = match p {
                    Tgt::Ent(..) | Tgt::Whole(..) if !wants => Some(bare.clone()),
                    Tgt::Ent((b, _, _)) | Tgt::Whole(b, _, _) => match mesh(b) {
                        Ok(m) => Some(m),
                        Err(e) => {
                            err = Some(e);
                            None
                        }
                    },
                    Tgt::Free(_) => None,
                };
                meshes.push(m);
            }
            let r = match err {
                Some(e) => Err(e),
                None => {
                    let ts: Vec<Target> = picks
                        .iter()
                        .zip(&meshes)
                        .map(|(p, m)| match (p, m) {
                            (Tgt::Ent((b, at, e)), Some(m)) => Target::Ent(Pick {
                                body: b,
                                mesh: m,
                                at: *at,
                                ent: *e,
                            }),
                            (Tgt::Whole(b, at, density), Some(m)) => Target::Whole(Whole {
                                body: b,
                                mesh: m,
                                at: *at,
                                density: *density,
                            }),
                            (Tgt::Free(f), _) => Target::Free(f.clone()),
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
            };
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
        // a pick the rebuilt part hasn't got, or one on a part whose topology changed, is gone:
        // measuring starts again rather than read some other face by its old number
        let was = self.inspector.data.take();
        let kept = |p: &MPick| {
            let shape = |d: &InspectData, pl: u32, e: Option<Entity>| {
                let &(mesh, _) = d.placed.get(pl as usize)?;
                let b = d.bodies.get(mesh)?;
                let n = match e {
                    Some(Entity::Face(i)) => (i as usize) < b.faces.len(),
                    Some(Entity::Edge(i)) => (i as usize) < b.edges.len(),
                    Some(Entity::Vertex(i)) => (i as usize) < b.vertices.len(),
                    None => true,
                };
                n.then_some((b.faces.len(), b.edges.len(), b.vertices.len()))
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

    /// A Measure pick as the worker takes it, and its words.
    fn target_of(&self, p: &MPick) -> Option<(Tgt, String)> {
        match p {
            MPick::Ent(pl, e) => self.picked(*pl, *e).map(|(t, w)| (Tgt::Ent(t), w)),
            MPick::Whole(pl) => {
                let d = self.inspector.data.as_ref()?;
                let &(mesh, at) = d.placed.get(*pl as usize)?;
                let b = d.bodies.get(mesh)?.clone();
                let name = d.names.get(*pl as usize).filter(|n| !n.is_empty());
                Some((
                    Tgt::Whole(b, at, self.density_of(*pl)),
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
            None if d.asm.is_none() => phys.mass.map(|m| m / phys.volume.max(1e-12)),
            None => None,
        }
    }

    /// What a click (or the pointer) at a placement's entity picks under the selection priority:
    /// the face of an edge facing the eye, the nearest edge of a face, the nearest point, the
    /// whole body.
    fn filtered(&self, filter: Filter, pl: u32, e: Entity, at: Option<[f32; 3]>) -> Option<MPick> {
        if filter == Filter::Bodies {
            return Some(MPick::Whole(pl));
        }
        if filter == Filter::Any {
            return Some(MPick::Ent(pl, e));
        }
        let d = self.inspector.data.as_ref()?;
        let &(mesh, frame) = d.placed.get(pl as usize)?;
        let b = d.bodies.get(mesh)?;
        let near = at.map(|p| frame.point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)));
        let near = near.unwrap_or_else(|| select::centre(b, ent(e)));
        let eye = self
            .view
            .as_ref()
            .map(|v| v.nav.camera.eye())
            .map(|q| frame.point_to_local(Point3::new(q[0] as f64, q[1] as f64, q[2] as f64)));
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
            (Filter::Faces, Ent::Face(_))
            | (Filter::Edges, Ent::Edge(_))
            | (Filter::Points, Ent::Vert(_)) => ent(e),
            (Filter::Faces, Ent::Edge(x)) => {
                Ent::Face(self.facing(b, &select::faces_of_edge(b, x), near, eye)?)
            }
            (Filter::Faces, Ent::Vert(v)) => {
                let mut fs: Vec<FaceId> = edges_at(v)
                    .into_iter()
                    .flat_map(|x| select::faces_of_edge(b, x))
                    .collect();
                fs.sort_by_key(|f| f.0);
                fs.dedup();
                Ent::Face(self.facing(b, &fs, near, eye)?)
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

    /// Of faces meeting at a point, the one turned most to the eye there.
    fn facing(&self, b: &Body, faces: &[FaceId], near: Point3, eye: Option<Point3>) -> Option<FaceId> {
        let to_eye = eye.map_or(Vec3::Z, |e| e - near);
        faces.iter().copied().max_by(|a, c| {
            let n = |f: FaceId| forge_agent::measure::normal_near(b, f, near).map_or(-1.0, |n| n.dot(to_eye));
            n(*a).total_cmp(&n(*c))
        })
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
                let faces = d
                    .and_then(|d| d.placed.get(*pl as usize).and_then(|(k, _)| d.bodies.get(*k)))
                    .map_or(0, |b| b.faces.len());
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
                    // a body measured before its material was known: read again, with its mass
                    self.inspector.measured = None;
                }
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
        if let Some((p, n)) = self.shell.inspect.want_outline.take() {
            let doc = self.fg.as_ref().map(|s| (s.src.clone(), s.path.clone()));
            let (send, recv) = channel();
            self.inspector.outline = Some(recv);
            let waker = self.waker.clone();
            std::thread::spawn(move || {
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
            Some((pl, e)) => self.filtered(filter, pl, e, at),
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
            Some((pl, e)) => self.filtered(filter, pl, e, at),
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
        let Some(m) = &self.shell.inspect.measure else {
            self.inspector.measured = None;
            return;
        };
        let (picks, wall) = (m.picks.clone(), m.wall);
        let marked = m.marked();
        if !self.inspector.warmed
            && let Some(d) = &self.inspector.data
        {
            self.inspector.warmed = true;
            // a part's body, or an assembly's first few: the rest as they're picked
            let bodies: Vec<Arc<Body>> = d.bodies.iter().take(8).cloned().collect();
            self.inspect_send(Job::Warm(bodies));
        }
        if let Some(v) = &mut self.view
            && v.nav.selected != marked
        {
            v.nav.selected = marked;
            v.mark_dirty();
        }
        if self.inspector.measured.as_ref() == Some(&(picks.clone(), wall)) {
            return;
        }
        self.inspector.measured = Some((picks.clone(), wall));
        // an occurrence's mass is from the assembly's materials: worked out once, when asked
        let s = &mut self.shell.inspect;
        if picks.iter().any(|p| matches!(p, MPick::Whole(_)))
            && self.inspector.data.as_ref().is_some_and(|d| d.asm.is_some())
            && s.physical.is_none()
            && !s.physical_busy
        {
            s.want_physical = true;
        }
        let got: Vec<(Tgt, String)> = picks.iter().filter_map(|p| self.target_of(p)).collect();
        self.inspector.seq += 1;
        let id = self.inspector.seq;
        let names: Vec<String> = got.iter().map(|g| g.1.clone()).collect();
        let job = if wall {
            // the part picked, or the part (an assembly's first)
            let first = got.first().and_then(|g| match &g.0 {
                Tgt::Ent((b, at, _)) | Tgt::Whole(b, at, _) => Some((b.clone(), *at)),
                Tgt::Free(_) => None,
            });
            let target = first.or_else(|| {
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
        // the same picks read again (the part rebuilt): the last reading stands till the new one
        let again = self.inspector.asked.as_ref() == Some(&(picks.clone(), wall));
        self.inspector.asked = Some((picks, wall));
        if let Some(m) = &mut self.shell.inspect.measure {
            m.names = names;
            if !again {
                m.reading = None;
                m.wall_at = None;
            }
            m.busy = job.is_some();
        }
        if let Some(j) = job {
            self.inspect_send(j);
        }
    }
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
        let mut meshes: Vec<Option<Arc<TriMesh>>> = vec![];
        for (t, _) in &targets {
            meshes.push(match t {
                Tgt::Ent((b, _, _)) | Tgt::Whole(b, _, _) => Some(Arc::new(
                    forge_mesh::tessellate::tessellate_tolerant(b, &forge_mesh::Quality::default())
                        .map_err(|e| forge_lang::error::words(&e))?,
                )),
                Tgt::Free(_) => None,
            });
        }
        let ts: Vec<Target> = targets
            .iter()
            .zip(&meshes)
            .map(|((t, _), m)| match (t, m) {
                (Tgt::Ent((b, at, e)), Some(m)) => Target::Ent(Pick {
                    body: b,
                    mesh: m,
                    at: *at,
                    ent: *e,
                }),
                (Tgt::Whole(b, at, density), Some(m)) => Target::Whole(Whole {
                    body: b,
                    mesh: m,
                    at: *at,
                    density: *density,
                }),
                (Tgt::Free(f), _) => Target::Free(f.clone()),
                _ => unreachable!("a body's mesh is made above"),
            })
            .collect();
        let r = match &ts[..] {
            [x] => read(x),
            [x, y] => read_pair(x, y),
            _ => Reading::default(),
        };
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
