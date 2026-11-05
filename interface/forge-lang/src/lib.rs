//! Forge (.fg): a typed, parametric part language for agents. A file is a program whose
//! statements make named features; it is checked (syntax, static rules, types and dimensions),
//! evaluated, and each feature built by forge-model as it is made, so anchors and selections
//! read the part as it stands. Errors, warnings and fixes are typed data (`error`), and edits
//! are typed operations on the source (`edit`).

pub mod anchors;
pub mod asm;
pub mod bodies;
pub mod build;
pub mod builtins;
pub mod deps;
pub mod drawing;
pub mod edit;
pub mod edit_ops;
pub mod error;
pub mod eval;
pub mod explode;
pub mod export;
pub mod feat_error;
pub mod fillets;
pub mod geomemo;
pub mod holes;
pub mod kept;
pub mod media;
pub mod par;
pub mod patterns;
pub mod project;
pub mod props;
pub mod refactor;
pub mod report;
pub mod shapes;
pub mod sheet;
pub mod sketch;
pub mod stdparts;
pub mod store;
pub mod syntax;
pub mod thermo;
pub mod thermo_kept;
pub mod uses;
pub mod validity;
pub mod values;
pub mod wired;
pub mod work;

use build::Cache;
use error::{Error, Metric, Warning};
use eval::{Eval, FeatInfo, ParamInfo, Rules, Value};
use std::time::Instant;
use syntax::{Cmp, ExprKind, StmtKind};

pub const REFERENCE: &str = include_str!("reference.md");

#[derive(Clone, Debug)]
pub struct Options {
    /// Build with the kernel (`run`), or stop after evaluation (`check`).
    pub build: bool,
    pub rules: Rules,
    /// Measure the volume after every feature (for tests of what an edit moved).
    pub volumes: bool,
    /// The part file's directory: where fonts and imported files are found.
    pub dir: Option<std::path::PathBuf>,
    /// An assembly: keep each placed part for a picture (`asm::compound`).
    pub render: bool,
    /// The session's root: an assembly reads no file outside it (`fg serve`, MCP).
    pub root: Option<std::path::PathBuf>,
    /// An assembly: measure only these expects (by position), ignoring their `at` (`asm.rs`).
    pub probe: Option<Vec<usize>>,
    /// An assembly: run the overlap (interference) check; off, a UI shows the solve first.
    pub overlap: bool,
    /// An assembly: measure its expects (an `at` one runs the document again with its params).
    pub expects: bool,
    /// Find what anchor text names each face and edge of the finished part (`anchors`), for a UI
    /// turning picks into source.
    pub anchors: bool,
    /// An assembly: write its structure as STEP under this product name (`fg run --step`).
    pub step: Option<String>,
    /// A part's measures, expects and physical properties and a thermoform's wall simulation;
    /// off, a UI shows the shape first and runs them after.
    pub checks: bool,
    /// Threads on an axis not modelled but recorded (`Run::threads`), as a drawing draws them.
    pub cosmetic_threads: bool,
    /// An assembly: mesh its parts for the overlap check and the expects' clearances. Off (a
    /// drawing, which asks for neither), a part loads without them, and a run that needs them
    /// does not share its loads (the setting is in their key).
    pub meshes: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            build: true,
            rules: Rules::default(),
            volumes: false,
            dir: None,
            render: false,
            root: None,
            probe: None,
            overlap: true,
            expects: true,
            anchors: false,
            step: None,
            checks: true,
            cosmetic_threads: false,
            meshes: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Timings {
    pub parse_ms: f64,
    pub check_ms: f64,
    /// Evaluation and building together.
    pub eval_ms: f64,
    /// Of `eval_ms`, the kernel's share.
    pub kernel_ms: f64,
    pub measure_ms: f64,
    pub total_ms: f64,
}

#[derive(Clone, Debug)]
pub struct Measure {
    pub volume: f64,
    pub area: f64,
    pub centroid: [f64; 3],
    pub faces: usize,
    pub edges: usize,
    pub bodies: usize,
    pub problems: usize,
    /// The check problems, typed, each on its body and the faces and edges it names.
    pub invalid: Vec<validity::Invalid>,
    pub bbox: [f64; 6],
    /// The finished body's exact mass properties (unit density), when it was measured whole.
    pub mass: Option<forge_topo::mass::MassProps>,
}

#[derive(Clone, Debug)]
pub struct ExpectResult {
    pub metric: Metric,
    pub measured: f64,
    pub ok: bool,
}

pub struct Run {
    pub errors: Vec<Error>,
    pub warnings: Vec<Warning>,
    pub feats: Vec<FeatInfo>,
    pub shared: Vec<ParamInfo>,
    pub locals: Vec<ParamInfo>,
    pub measure: Option<Measure>,
    pub expects: Vec<ExpectResult>,
    pub timings: Timings,
    pub rebuilt: usize,
    pub reused: usize,
    /// Work outside the kernel's history (a sheet's builds and shop checks) done afresh, not
    /// taken from the last run.
    pub remade: usize,
    /// The finished part, for rendering.
    pub body: Option<forge_topo::Body>,
    /// Params whose values evaluation observed: editing one re-runs, any other re-reads.
    pub observed: std::collections::BTreeSet<error::ParamKey>,
    /// The part's sheets as they ended, for the report and the flat pattern.
    pub sheets: Vec<sheet::SheetOut>,
    /// The part's thermoforms as built (`thermo.rs`).
    pub thermos: Vec<thermo::ThermoOut>,
    /// The threads left unmodelled (`Options::cosmetic_threads`), in the part's frame.
    pub threads: Vec<holes::Cosmetic>,
    /// Where each solid is in `body`: its faces and edges.
    pub solids: Vec<build::Span>,
    /// The stage `body` is (`build::Stage::id`): equal ids, the very same body.
    pub stage: u64,
    /// An assembly's solve, checks and timings (`asm.rs`).
    pub asm: Option<asm::AsmOut>,
    /// With `Options::anchors`: the anchor text of each face and edge of the finished part.
    pub anchors: Option<anchors::Anchors>,
    /// The document's iProperties (`props.rs`).
    pub props: props::Props,
    /// A part's mass, centre of gravity and inertia from its solids' materials.
    pub physical: Option<props::Physical>,
    /// The inline components (`component NAME(..) { .. }`), each as its own part.
    pub components: Vec<asm::CompOut>,
    /// A document with components: each body and the features that made it (`asm::body_chains`).
    pub body_chains: Vec<(String, Vec<String>)>,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Parses, checks, evaluates and (with `opts.build`) builds `src`, reusing `cache`'s stages
/// where the history is unchanged. Returns the run and the cache for the next.
pub fn run(src: &str, opts: Options, cache: Cache) -> (Run, Cache) {
    let t0 = Instant::now();
    let mut timings = Timings::default();
    let prog = syntax::parse(src);
    timings.parse_ms = t0.elapsed().as_secs_f64() * 1e3;
    let prog = match prog {
        Ok(p) => p,
        Err(e) => {
            timings.total_ms = t0.elapsed().as_secs_f64() * 1e3;
            return (
                Run {
                    errors: vec![e],
                    warnings: vec![],
                    feats: vec![],
                    shared: vec![],
                    locals: vec![],
                    measure: None,
                    expects: vec![],
                    timings,
                    rebuilt: 0,
                    reused: 0,
                    remade: 0,
                    body: None,
                    observed: Default::default(),
                    sheets: vec![],
                    thermos: vec![],
                    threads: vec![],
                    solids: vec![],
                    stage: 0,
                    asm: None,
                    anchors: None,
                    props: props::Props::default(),
                    physical: None,
                    components: vec![],
                    body_chains: vec![],
                },
                cache,
            );
        }
    };
    let mut ev = Eval::new(src, opts.rules, opts.build, cache);
    ev.volumes = opts.volumes;
    ev.checks = opts.checks;
    ev.cosmetic_threads = opts.cosmetic_threads;
    ev.dir = opts.dir.clone();
    ev.root = fence_of(&opts);
    // any document may hold components and joints (an `assembly` first line is optional)
    ev.asm = Some(asm::Doc {
        declared: asm::declared(&prog),
        ..Default::default()
    });
    let t1 = Instant::now();
    ev.check_static(&prog);
    timings.check_ms = t1.elapsed().as_secs_f64() * 1e3;
    // static errors stop the build (they mean the source is not yet a model), not evaluation
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    let t2 = Instant::now();
    ev.run(&prog);
    sheet::finish(&mut ev);
    let assembly = ev.asm.as_ref().is_some_and(|d| !d.is_empty());
    if assembly && let Some(e) = props::material_unused(&ev) {
        ev.errors.push(e);
    }
    let kept = ev.b.cache.asm.take();
    let (asm_out, kept) = asm::finish(&mut ev, &opts, kept);
    ev.b.cache.asm = kept;
    timings.eval_ms = t2.elapsed().as_secs_f64() * 1e3;
    timings.kernel_ms = ev.b.kernel_ms;
    let t3 = Instant::now();
    let built_all = opts.build && ev.errors.is_empty() && ev.b.built() == ev.b.features.len();
    let measure = match (built_all && opts.checks, ev.b.measured()) {
        (false, _) => None,
        (true, Some(m)) => Some(m),
        (true, None) => {
            let m = measure(&mut ev);
            if let Some(m) = &m {
                ev.b.keep_measure(m.clone());
            }
            m
        }
    };
    let mut expects = vec![];
    if let Some(m) = &measure {
        for s in ev.expects.clone() {
            // an assembly's metrics are its solve's (`asm.rs`)
            if assembly && asm::is_metric(&s) {
                continue;
            }
            if let Some(r) = expect(&mut ev, &s, m) {
                expects.push(r);
            }
        }
    }
    timings.measure_ms = t3.elapsed().as_secs_f64() * 1e3;
    timings.total_ms = t0.elapsed().as_secs_f64() * 1e3;
    let physical = if built_all && opts.checks && !assembly {
        props::part_physical(&mut ev, measure.as_ref().and_then(|m| m.mass.as_ref()))
    } else {
        None
    };
    let body = if built_all { ev.b.body().cloned() } else { None };
    let chains = if assembly && built_all {
        asm::body_chains(&mut ev)
    } else {
        vec![]
    };
    let (rebuilt, reused) = (ev.b.rebuilt, ev.b.reused);
    let run = Run {
        errors: std::mem::take(&mut ev.errors),
        warnings: std::mem::take(&mut ev.warnings),
        feats: std::mem::take(&mut ev.feats),
        shared: std::mem::take(&mut ev.shared),
        locals: std::mem::take(&mut ev.locals),
        measure,
        expects,
        timings,
        rebuilt,
        reused,
        remade: ev.b.remade,
        body,
        observed: std::mem::take(&mut ev.observed),
        sheets: sheet::outputs(&ev),
        thermos: std::mem::take(&mut ev.thermos),
        threads: std::mem::take(&mut ev.threads),
        solids: if built_all {
            ev.b.stages.last().map_or(vec![], |s| s.spans().to_vec())
        } else {
            vec![]
        },
        stage: if built_all { ev.b.stage_id() } else { 0 },
        asm: asm_out,
        anchors: None,
        props: std::mem::take(&mut ev.props),
        physical,
        components: vec![],
        body_chains: vec![],
    };
    let mut run = run;
    // a sheet that names no stock is the part's material
    for s in &mut run.sheets {
        if s.material.is_none() {
            s.material = run.props.material.clone();
        }
    }
    if opts.anchors {
        let body = run.body.clone();
        run.anchors = Some(anchors::find(&mut ev, body.as_ref()));
    }
    run.components = asm::components(&ev);
    run.body_chains = chains;
    asm::keep_comp_caches(&mut ev);
    (run, ev.b.into_cache())
}

/// Where a run may read files: under the session's root if one is given (`fg serve`, MCP), else
/// anywhere a loader's kind check lets it (each loader reads only its own kind of file, checked
/// by its content, and no read file's bytes reach any output).
pub fn fence_of(opts: &Options) -> Option<std::path::PathBuf> {
    fence_for(opts.dir.as_deref(), opts.root.as_deref())
}

/// The fence of a document in `dir`: its session's `root`, canonical, if one is given.
pub fn fence_for(
    _dir: Option<&std::path::Path>,
    root: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    std::fs::canonicalize(root?).ok()
}

/// Whether `file` beside `dir` is outside the fence: as written (whether or not it is there, so
/// the refusal says nothing of what is outside), or once its links are resolved.
pub(crate) fn outside(dir: Option<&std::path::Path>, fence: Option<&std::path::Path>, file: &str) -> bool {
    let (Some(d), Some(f)) = (dir, fence) else {
        return false;
    };
    let Ok(d) = std::fs::canonicalize(d) else {
        return false;
    };
    let mut lexical = std::path::PathBuf::new();
    for c in d.join(file).components() {
        match c {
            std::path::Component::ParentDir => {
                lexical.pop();
            }
            std::path::Component::CurDir => {}
            c => lexical.push(c),
        }
    }
    !lexical.starts_with(f) || std::fs::canonicalize(d.join(file)).is_ok_and(|p| !p.starts_with(f))
}

/// A sketch as evaluation left it, for an editor reading the block back.
pub struct SketchView {
    /// Solved, in the sketch plane's coordinates.
    pub s: forge_sketch::Sketch,
    /// Each name the block bound to an entity: the entity, and a loop's segments.
    pub named: Vec<(String, forge_sketch::EntityId, Vec<forge_sketch::EntityId>)>,
}

/// The sketch `id` of `src` as evaluated (built with `opts.build`), if evaluation made it.
pub fn sketch_view(src: &str, id: &str, opts: Options) -> Option<SketchView> {
    let prog = syntax::parse(src).ok()?;
    let mut ev = Eval::new(src, opts.rules, opts.build, Default::default());
    ev.dir = opts.dir.clone();
    ev.root = fence_of(&opts);
    ev.asm = Some(asm::Doc::default());
    ev.check_static(&prog);
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    ev.run(&prog);
    // a sketch of a component (`peg.s`): as the component's defaults evaluate it
    let inner = id
        .split_once('.')
        .and_then(|(c, s)| asm::default_eval(&ev, c).map(|sub| (sub, s.to_string())));
    let (_, Value::Sketch(sk)) = (match &inner {
        Some((sub, s)) => sub.top_level().into_iter().find(|(n, _)| n == s),
        None => ev.top_level().into_iter().find(|(n, _)| n == id),
    })?
    else {
        return None;
    };
    let sk = sk.borrow();
    let named = sk
        .named
        .iter()
        .filter_map(|(n, v)| match v {
            Value::Ent(e) => Some((n.clone(), e.id, e.loop_segs.to_vec())),
            _ => None,
        })
        .collect();
    Some(SketchView {
        s: sk.s.clone(),
        named,
    })
}

fn measure(ev: &mut Eval) -> Option<Measure> {
    let invalid = validity::problems(ev);
    let body = ev.b.body()?;
    let solids = ev.b.stages.last().map_or(0, |s| s.solids.len());
    let m = forge_topo::mass::mass(body).ok()?;
    let bb = body
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |b, f| b.hull(&body.face_box(f)));
    Some(Measure {
        volume: m.volume,
        area: m.area,
        centroid: [m.centroid.x, m.centroid.y, m.centroid.z],
        faces: body.face_ids().count(),
        edges: body.edge_ids().count(),
        bodies: solids,
        problems: invalid.len(),
        invalid,
        bbox: [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi],
        mass: Some(m),
    })
}

fn expect(ev: &mut Eval, s: &syntax::Stmt, m: &Measure) -> Option<ExpectResult> {
    let StmtKind::Expect { metric, cmp, .. } = &s.kind else {
        return None;
    };
    let (metric, measured) = match metric.as_str() {
        "volume" => (Metric::Volume, m.volume),
        "area" => (Metric::Area, m.area),
        "faces" => (Metric::Faces, m.faces as f64),
        "edges" => (Metric::Edges, m.edges as f64),
        "bodies" => (Metric::Bodies, m.bodies as f64),
        "valid" => (Metric::Valid, if m.problems == 0 { 1.0 } else { 0.0 }),
        _ => {
            ev.errors.push(Error::UnknownMetric {
                span: s.span,
                metric: metric.clone(),
            });
            return None;
        }
    };
    let Some((c, v, tol)) = cmp else {
        let ok = metric != Metric::Valid || measured == 1.0;
        if !ok {
            ev.errors.push(Error::ExpectFailed {
                span: s.span,
                metric,
                measured,
                cmp: Cmp::Eq,
                limit: 1.0,
                tol: 0.0,
            });
        }
        return Some(ExpectResult { metric, measured, ok });
    };
    let lit = |e: &syntax::Expr| match e.kind {
        ExprKind::Num(x, _) => Some(x),
        ExprKind::Neg(ref a) => match a.kind {
            ExprKind::Num(x, _) => Some(-x),
            _ => None,
        },
        _ => None,
    };
    let limit = match ev.expr(v) {
        Ok(Value::Num(n)) => n.v,
        _ => lit(v).unwrap_or(f64::NAN),
    };
    let tol = match tol {
        Some((t, pct)) => {
            let x = lit(t).unwrap_or(0.0);
            if *pct { limit.abs() * x / 100.0 } else { x }
        }
        None => 0.0,
    };
    if *c == Cmp::Eq
        && tol == 0.0
        && limit != 0.0
        && (measured - limit).abs() <= 1e-9 * limit.abs()
        && metric_is_real(metric)
    {
        let e = Error::ExpectCopied {
            span: s.span,
            metric,
            value: limit,
        };
        if let Err(x) = ev.rule(ev.rules.expect_copied, e) {
            ev.errors.push(x);
        }
    }
    let ok = match c {
        Cmp::Eq => (measured - limit).abs() <= tol.max(1e-9 * limit.abs().max(1.0)),
        Cmp::Lt => measured < limit + tol,
        Cmp::Le => measured <= limit + tol,
        Cmp::Gt => measured > limit - tol,
        Cmp::Ge => measured >= limit - tol,
    };
    if !ok {
        ev.errors.push(Error::ExpectFailed {
            span: s.span,
            metric,
            measured,
            cmp: *c,
            limit,
            tol,
        });
    }
    Some(ExpectResult { metric, measured, ok })
}

/// Measures that are real numbers (a copied count is just a count).
fn metric_is_real(m: Metric) -> bool {
    matches!(m, Metric::Volume | Metric::Area | Metric::Mass)
}

/// A PNG of the finished part.
pub fn render(body: &forge_topo::Body, view: &str, px: usize) -> Result<Vec<u8>, String> {
    let mut cache = forge_agent::facemesh::Cache::default();
    let faces = cache.faces(body);
    let mesh = forge_agent::facemesh::join(&faces);
    forge_agent::render::render(body, &mesh, view, px, &[])
}
