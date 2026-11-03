//! Forge (.fg) parts in the app: the source, typed edits applied through forge-lang, and a worker
//! that keeps the build's stages from run to run (as `fg serve` does), so a rebuild starts at the
//! first feature that changed and an unchanged one costs milliseconds.

use crate::live::Mesher;
use crate::livegpu::Delta;
use crate::mesh::PartMesh;
use crate::render::{self, Renderer, part_info};
use crate::scene::{Placement, Scene};
use crate::shell::fg::{DimRow, FeatRow, From, Issue, ParamRow, Severity, Status};
use crate::tess;
use forge_lang::build::Cache;
use forge_lang::edit::{self, Reach};
use forge_lang::error::{Error, Op, Warning};
use forge_lang::eval::Value;
use forge_lang::values::{Dim, Src};
use forge_lang::{Measure, Options, Run, Timings};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

pub type Gpu = (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo);

/// What one build gave: the panel's data, and the mesh when the part's shape changed.
pub struct Built {
    pub seq: u64,
    pub ok: bool,
    pub feats: Vec<FeatRow>,
    pub params: Vec<ParamRow>,
    pub issues: Vec<Issue>,
    pub measure: Option<Measure>,
    pub timings: Timings,
    pub rebuilt: usize,
    pub reused: usize,
    /// The part's new mesh; none when the part is the one already shown, or didn't build.
    pub mesh: Option<PartMesh>,
    /// The faces the build changed, for the screen at once (the whole mesh follows).
    pub delta: Option<Delta>,
    /// When `mesh` is new: the flat pattern of the part's first sheet; none without one.
    pub flat: Option<PartMesh>,
    /// Each sheet metal part: its id, how many bends, and its flat pattern as DXF.
    pub sheets: Vec<SheetInfo>,
    /// The first build's renderer and scene: pipelines are dear, made here and never in a frame.
    pub view: Option<(Renderer, Scene)>,
    /// What Forge names each face and edge, when the text changed since the last.
    pub anchors: Option<Arc<AnchorView>>,
    /// The worker's time for it: the build, and the mesh.
    pub build_ms: f64,
    pub mesh_ms: f64,
}

impl Built {
    pub fn summary(&self) -> String {
        let mut s = match &self.measure {
            Some(m) => format!(
                "volume {} mm³ · {} faces · {} · ",
                crate::shell::fg::short((m.volume * 10.0).round() / 10.0),
                m.faces,
                if m.problems == 0 { "valid" } else { "INVALID" }
            ),
            None if self.ok => "nothing built · ".into(),
            None => "not built · ".into(),
        };
        s.push_str(&format!(
            "rebuilt {}, reused {} · {:.1} ms",
            self.rebuilt,
            self.reused,
            self.build_ms + self.mesh_ms
        ));
        s
    }
}

/// Each face and edge of the part (in the mesh's entity order, which is the body's) as Forge
/// names it, if anything does; the planes a sketch on one is drawn in; the sketches.
#[derive(Clone, Debug, Default)]
pub struct AnchorView {
    pub faces: Vec<Option<String>>,
    pub edges: Vec<Option<String>>,
    /// How many faces each face anchor names.
    pub sizes: HashMap<String, usize>,
    pub planes: HashMap<String, forge_math::Frame>,
    pub sketches: Vec<forge_lang::anchors::SketchRefs>,
    /// Sheet edges a flange can stand on, with the part's edges (by index) each is.
    pub sheet_edges: Vec<(String, Vec<usize>)>,
}

impl AnchorView {
    fn of(a: &forge_lang::anchors::Anchors, body: Option<&forge_topo::Body>) -> AnchorView {
        let Some(b) = body else {
            return AnchorView {
                sketches: a.sketches.clone(),
                planes: a.planes.clone(),
                ..AnchorView::default()
            };
        };
        let edge_names: Vec<forge_topo::Name> = b.edge_ids().map(|e| b.edge(e).name).collect();
        AnchorView {
            faces: b.face_ids().map(|f| a.faces.get(&b.face(f).name).cloned()).collect(),
            edges: edge_names.iter().map(|n| a.edges.get(n).cloned()).collect(),
            sizes: a.sizes.clone(),
            planes: a.planes.clone(),
            sketches: a.sketches.clone(),
            sheet_edges: a
                .sheet_edges
                .iter()
                .map(|s| {
                    let idx = edge_names
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| s.names.contains(n))
                        .map(|(k, _)| k)
                        .collect();
                    (s.text.clone(), idx)
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SheetInfo {
    pub id: String,
    pub bends: usize,
    pub dxf: Result<String, String>,
}

/// The first sheet's flat solid (several sheets: each one's DXF, the first one shown).
fn flat_body(run: &Run) -> Option<forge_topo::Body> {
    run.sheets
        .iter()
        .find_map(|s| forge_sheet::build(&s.sheet, forge_sheet::Map::Flat).ok())
}

fn shown(v: f64, d: Dim) -> f64 {
    if d == Dim::Angle { v.to_degrees() } else { v }
}

fn line_of(src: &str, at: u32) -> usize {
    src[..(at as usize).min(src.len())].matches('\n').count() + 1
}

/// The run as the panel's rows.
pub fn rows(run: &Run, src: &str) -> (Vec<FeatRow>, Vec<ParamRow>, Vec<Issue>) {
    let uses = forge_lang::report::uses(run);
    let feats = run
        .feats
        .iter()
        .map(|f| FeatRow {
            id: f.id.clone(),
            kind: f.kind.to_string(),
            line: line_of(src, f.span.lo),
            status: match f.status {
                forge_lang::eval::Status::Ok => Status::Ok,
                forge_lang::eval::Status::Failed => Status::Failed,
                forge_lang::eval::Status::NotBuilt => Status::NotBuilt,
            },
            dims: f
                .dims
                .iter()
                .map(|d| DimRow {
                    what: d.what.to_string(),
                    value: shown(d.num.v, d.num.dim),
                    dim: d.num.dim,
                    from: match &d.num.src {
                        Src::Const => From::Const,
                        Src::Driving(p) => From::Driving(p.clone()),
                        Src::Driven(ps) => From::Driven(ps.iter().cloned().collect()),
                    },
                })
                .collect(),
            reads: f.reads.iter().cloned().collect(),
        })
        .collect();
    let params = run
        .shared
        .iter()
        .chain(&run.locals)
        .map(|p| ParamRow {
            key: p.key.clone(),
            value: match &p.value {
                Value::Num(n) => Some((shown(n.v, n.dim), n.dim)),
                _ => None,
            },
            text: src
                .get(p.rhs.lo as usize..p.rhs.hi as usize)
                .unwrap_or("")
                .to_string(),
            links: p.links.iter().cloned().collect(),
            used_by: uses.get(&p.key).cloned().unwrap_or_default(),
            structural: run.observed.contains(&p.key),
            line: line_of(src, p.span.lo),
        })
        .collect();
    // the feature an issue is in: its own span's, else its statement's
    let feature_at = |span: Option<forge_lang::syntax::Span>| {
        let s = span?;
        let inside = |o: forge_lang::syntax::Span| o.lo <= s.lo && s.hi <= o.hi;
        run.feats
            .iter()
            .find(|f| inside(f.span))
            .or_else(|| run.feats.iter().find(|f| inside(f.stmt)))
            .map(|f| f.id.clone())
    };
    let issue = |e: &Error, severity: Severity| {
        let (line, col) = e.span().map_or((0, 0), |s| {
            let (l, c, ..) = s.lines(src);
            (l, c)
        });
        let feature = match e {
            Error::Build { id, .. } => Some(id.clone()),
            _ => feature_at(e.span()),
        };
        Issue {
            severity,
            kind: e.kind().to_string(),
            text: e.text(),
            line,
            col,
            fixes: e.fixes(),
            feature,
        }
    };
    let mut issues: Vec<Issue> = run.errors.iter().map(|e| issue(e, Severity::Error)).collect();
    for w in &run.warnings {
        let (kind, span, text) = match w {
            Warning::Lint(e) => {
                issues.push(issue(e, Severity::Lint));
                continue;
            }
            Warning::Sheet(n) => {
                let (line, col, ..) = n.span.lines(src);
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: n.kind().into(),
                    text: n.text(),
                    line,
                    col,
                    fixes: n.fixes.clone(),
                    feature: Some(n.id.clone()),
                });
                continue;
            }
            Warning::UnderConstrained { span, sketch, dof } => (
                "UnderConstrained",
                *span,
                format!("sketch {sketch} can still move ({dof} degrees of freedom)"),
            ),
            Warning::UnusedParam { span, param } => ("UnusedParam", *span, format!("nothing reads {param}")),
            Warning::NoEffect { span, id } => ("NoEffect", *span, format!("{id} changed nothing")),
        };
        let (line, col, ..) = span.lines(src);
        issues.push(Issue {
            severity: Severity::Warning,
            kind: kind.into(),
            text,
            line,
            col,
            fixes: vec![],
            feature: feature_at(Some(span)),
        });
    }
    (feats, params, issues)
}

/// What the worker sends: a build (its rows, and a delta of the faces it changed), the faces of
/// the first build for the GPU's live set, or the whole mesh once nothing newer waits.
pub enum Msg {
    Built(Box<Built>),
    Warm(Delta),
    Final {
        seq: u64,
        mesh: PartMesh,
        flat: Option<PartMesh>,
        mesh_ms: f64,
    },
}

fn whole(body: &forge_topo::Body, flat: Option<&forge_topo::Body>) -> (PartMesh, Option<PartMesh>) {
    let mesh = tess::build_part(body).unwrap_or_else(|e| {
        eprintln!("forge-app: can't mesh it: {e:?}");
        PartMesh::empty()
    });
    let flat = flat.map(|b| tess::build_part(b).unwrap_or_else(|_| PartMesh::empty()));
    (mesh, flat)
}

type Whole = (u64, forge_topo::Body, Option<forge_topo::Body>);

/// The whole meshes, on a thread of their own so a build never waits on one: the latest only.
fn mesh_whole(bodies: Receiver<Whole>, out: Sender<Msg>, waker: Arc<dyn Fn() + Send + Sync>) {
    while let Ok(mut w) = bodies.recv() {
        while let Ok(n) = bodies.try_recv() {
            w = n;
        }
        let t = Instant::now();
        let (mesh, flat) = whole(&w.1, w.2.as_ref());
        let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
        if out
            .send(Msg::Final {
                seq: w.0,
                mesh,
                flat,
                mesh_ms,
            })
            .is_err()
        {
            return;
        }
        waker();
    }
}

/// The worker: owns the stage cache (it is not `Send`: it never leaves this thread) and builds
/// the latest text it was given. A changed part goes to the screen first as the faces that changed
/// (meshed once per face, as a drag's), then whole.
fn work(
    dir: Option<std::path::PathBuf>,
    jobs: Receiver<(u64, String)>,
    out: Sender<Msg>,
    mut gpu: Option<Gpu>,
    waker: Arc<dyn Fn() + Send + Sync>,
) {
    let mut cache = Cache::default();
    let mut mesher = Mesher::default();
    let mut shown_ids: Option<Vec<String>> = None;
    // the text the last anchors were found for: unchanged text, unchanged anchors
    let mut anchored: Option<String> = None;
    let (to_mesh, bodies) = channel::<Whole>();
    {
        let (out, waker) = (out.clone(), waker.clone());
        std::thread::Builder::new()
            .name("forge-fg-mesh".into())
            .spawn(move || mesh_whole(bodies, out, waker))
            .expect("a thread");
    }
    let send = |m: Msg| {
        let ok = out.send(m).is_ok();
        waker();
        ok
    };
    loop {
        let Ok(mut job) = jobs.recv() else { return };
        // only the latest text matters
        while let Ok(j) = jobs.try_recv() {
            job = j;
        }
        let t = Instant::now();
        let want = anchored.as_deref() != Some(job.1.as_str());
        let opts = Options {
            anchors: want,
            dir: dir.clone(),
            ..Options::default()
        };
        let (run, c) = forge_lang::run(&job.1, opts, std::mem::take(&mut cache));
        let anchors = run.anchors.as_ref().map(|a| Arc::new(AnchorView::of(a, run.body.as_ref())));
        if anchors.is_some() {
            anchored = Some(job.1.clone());
        }
        cache = c;
        let build_ms = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let ids: Vec<String> = run.feats.iter().map(|f| f.id.clone()).collect();
        // all stages reused and the same features: the same body, already on screen
        let same = run.rebuilt == 0 && shown_ids.as_ref() == Some(&ids);
        let changed = run.body.as_ref().filter(|_| !same);
        if changed.is_some() {
            shown_ids = Some(ids);
        }
        let first = gpu.is_some();
        let (mut mesh, mut flat, mut delta) = (None, None, None);
        match changed {
            Some(b) if first => {
                let (m, f) = whole(b, flat_body(&run).as_ref());
                (mesh, flat) = (Some(m), f);
            }
            Some(b) => delta = Some(mesher.delta(b)),
            None => {}
        }
        let sheets = run
            .sheets
            .iter()
            .map(|s| SheetInfo {
                id: s.id.clone(),
                bends: s.layout.as_ref().map_or(0, |l| l.bends.len()),
                dxf: s.dxf().map_err(|e| format!("{e:?}")),
            })
            .collect();
        let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
        let view = gpu.take().map(|(device, queue, adapter)| {
            let parts = [mesh.clone().unwrap_or_else(PartMesh::empty)];
            let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
            scene.set_explode(0.0);
            let r = Renderer::new(
                device,
                queue,
                adapter,
                render::OFFSCREEN,
                (16, 16),
                &parts,
                &scene,
            );
            (r, scene)
        });
        let (feats, params, issues) = rows(&run, &job.1);
        let built = Built {
            seq: job.0,
            ok: run.ok(),
            feats,
            params,
            issues,
            measure: run.measure.clone(),
            timings: run.timings.clone(),
            rebuilt: run.rebuilt,
            reused: run.reused,
            mesh,
            flat,
            delta,
            sheets,
            view,
            anchors,
            build_ms,
            mesh_ms,
        };
        if !send(Msg::Built(Box::new(built))) {
            return;
        }
        let Some(b) = changed else { continue };
        if first {
            // the GPU's live set starts as the whole part, so later deltas carry only changes
            if !send(Msg::Warm(mesher.delta(b))) {
                return;
            }
            continue;
        }
        if to_mesh.send((job.0, b.clone(), flat_body(&run))).is_err() {
            return;
        }
    }
}

/// A .fg part open in the app.
pub struct Session {
    pub path: Option<String>,
    pub src: String,
    /// What the file holds.
    pub saved: String,
    undo: Vec<String>,
    redo: Vec<String>,
    jobs: Sender<(u64, String)>,
    built: Receiver<Msg>,
    /// The last text sent to the worker, and the last one back.
    pub seq: u64,
    pub done: u64,
    sent_at: Vec<(u64, Instant)>,
    /// From sending a text to its build being back, the last time.
    pub round_ms: f64,
    /// The ops applied, in order.
    pub ops: Vec<Op>,
    /// The last build's times: forge-lang's (its stages), and meshing for the screen.
    pub timings: Timings,
    pub build_ms: f64,
    pub mesh_ms: f64,
    /// The last build that changed the part's shape.
    pub changed: u64,
    /// The last build whose whole mesh came, and that mesh's time.
    pub whole: u64,
    pub whole_ms: f64,
}

impl Session {
    /// A session on this text; the first build starts at once.
    pub fn new(
        path: Option<String>,
        src: String,
        gpu: Option<Gpu>,
        waker: Arc<dyn Fn() + Send + Sync>,
    ) -> Session {
        let (jobs, rx) = channel();
        let (tx, built) = channel();
        // fonts and imported files are found beside the part
        let dir = path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        std::thread::Builder::new()
            .name("forge-fg".into())
            .spawn(move || work(dir, rx, tx, gpu, waker))
            .expect("a thread");
        let mut s = Session {
            path,
            saved: src.clone(),
            src,
            undo: vec![],
            redo: vec![],
            jobs,
            built,
            seq: 0,
            done: 0,
            sent_at: vec![],
            round_ms: 0.0,
            ops: vec![],
            timings: Timings::default(),
            build_ms: 0.0,
            mesh_ms: 0.0,
            whole: 0,
            whole_ms: 0.0,
            changed: 0,
        };
        s.rebuild();
        s
    }

    pub fn open(path: &str, gpu: Option<Gpu>, waker: Arc<dyn Fn() + Send + Sync>) -> Result<Session, String> {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Ok(Session::new(Some(path.to_string()), src, gpu, waker))
    }

    /// Builds the text as it stands (through the warm stages).
    pub fn rebuild(&mut self) -> u64 {
        self.seq += 1;
        self.sent_at.push((self.seq, Instant::now()));
        let _ = self.jobs.send((self.seq, self.src.clone()));
        self.seq
    }

    /// What the worker has finished.
    pub fn poll(&mut self) -> Vec<Msg> {
        let got: Vec<Msg> = self.built.try_iter().collect();
        for m in &got {
            match m {
                Msg::Built(b) => {
                    self.done = b.seq;
                    if b.delta.is_some() || b.mesh.is_some() {
                        self.changed = b.seq;
                    }
                    // the first build comes whole
                    if b.mesh.is_some() {
                        self.whole = b.seq;
                    }
                    self.timings = b.timings.clone();
                    (self.build_ms, self.mesh_ms) = (b.build_ms, b.mesh_ms);
                    if let Some(&(_, t)) = self.sent_at.iter().find(|s| s.0 == b.seq) {
                        self.round_ms = t.elapsed().as_secs_f64() * 1e3;
                    }
                    self.sent_at.retain(|s| s.0 > b.seq);
                }
                Msg::Final { seq, mesh_ms, .. } => {
                    self.whole = *seq;
                    self.whole_ms = *mesh_ms;
                }
                Msg::Warm(_) => {}
            }
        }
        got
    }

    /// Applies a typed op through forge-lang: refused, with its typed error, if the result
    /// wouldn't check; else the new text builds.
    pub fn apply(&mut self, op: Op) -> Result<Option<Reach>, Error> {
        let (text, reach) = edit::apply_checked(&self.src, &op)?;
        self.set_text(text);
        self.ops.push(op);
        Ok(reach)
    }

    fn set_text(&mut self, text: String) {
        let old = std::mem::replace(&mut self.src, text);
        self.undo.push(old);
        self.redo.clear();
        self.rebuild();
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.src, prev));
        self.rebuild();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.src, next));
        self.rebuild();
        true
    }

    pub fn dirty(&self) -> bool {
        self.src != self.saved
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("no file to save to")?;
        std::fs::write(path, &self.src).map_err(|e| format!("{path}: {e}"))?;
        self.saved = self.src.clone();
        Ok(())
    }
}
