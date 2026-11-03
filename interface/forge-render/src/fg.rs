//! Forge (.fg) parts in the app: the source, typed edits applied through forge-lang, and a worker
//! that keeps the build's stages from run to run (as `fg serve` does), so a rebuild starts at the
//! first feature that changed and an unchanged one costs milliseconds.

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
    /// The first build's renderer and scene: pipelines are dear, made here and never in a frame.
    pub view: Option<(Renderer, Scene)>,
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

/// The worker: owns the stage cache (it is not `Send`: it never leaves this thread) and builds
/// the latest text it was given.
fn work(jobs: Receiver<(u64, String)>, out: Sender<Built>, mut gpu: Option<Gpu>, waker: Arc<dyn Fn() + Send + Sync>) {
    let mut cache = Cache::default();
    let mut shown_ids: Option<Vec<String>> = None;
    while let Ok(mut job) = jobs.recv() {
        // only the latest text matters
        while let Ok(j) = jobs.try_recv() {
            job = j;
        }
        let t = Instant::now();
        let (run, c) = forge_lang::run(&job.1, Options::default(), std::mem::take(&mut cache));
        cache = c;
        let build_ms = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let ids: Vec<String> = run.feats.iter().map(|f| f.id.clone()).collect();
        // all stages reused and the same features: the same body, already on screen
        let same = run.rebuilt == 0 && shown_ids.as_ref() == Some(&ids);
        let mut mesh = match &run.body {
            Some(b) if !same => {
                shown_ids = Some(ids);
                Some(tess::build_part(b).unwrap_or_else(|e| {
                    eprintln!("forge-app: can't mesh it: {e:?}");
                    PartMesh::empty()
                }))
            }
            _ => None,
        };
        let mesh_ms = t.elapsed().as_secs_f64() * 1e3;
        let view = gpu.take().map(|(device, queue, adapter)| {
            let parts = [mesh.take().unwrap_or_else(PartMesh::empty)];
            let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
            scene.set_explode(0.0);
            let r = Renderer::new(device, queue, adapter, render::OFFSCREEN, (16, 16), &parts, &scene);
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
            view,
            build_ms,
            mesh_ms,
        };
        if out.send(built).is_err() {
            return;
        }
        waker();
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
    built: Receiver<Built>,
    /// The last text sent to the worker, and the last one back.
    pub seq: u64,
    pub done: u64,
    sent_at: Vec<(u64, Instant)>,
    /// From sending a text to its build being back, the last time.
    pub round_ms: f64,
    /// The ops applied, in order.
    pub ops: Vec<Op>,
}

impl Session {
    /// A session on this text; the first build starts at once.
    pub fn new(path: Option<String>, src: String, gpu: Option<Gpu>, waker: Arc<dyn Fn() + Send + Sync>) -> Session {
        let (jobs, rx) = channel();
        let (tx, built) = channel();
        std::thread::Builder::new()
            .name("forge-fg".into())
            .spawn(move || work(rx, tx, gpu, waker))
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
    pub fn poll(&mut self) -> Vec<Built> {
        let got: Vec<Built> = self.built.try_iter().collect();
        if let Some(b) = got.last() {
            self.done = b.seq;
            if let Some(&(_, t)) = self.sent_at.iter().find(|s| s.0 == b.seq) {
                self.round_ms = t.elapsed().as_secs_f64() * 1e3;
            }
            self.sent_at.retain(|s| s.0 > b.seq);
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
