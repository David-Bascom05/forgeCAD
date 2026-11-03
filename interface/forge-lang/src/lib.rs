//! Forge (.fg): a typed, parametric part language for agents. A file is a program whose
//! statements make named features; it is checked (syntax, static rules, types and dimensions),
//! evaluated, and each feature built by forge-model as it is made, so anchors and selections
//! read the part as it stands. Errors, warnings and fixes are typed data (`error`), and edits
//! are typed operations on the source (`edit`).

pub mod asm;
pub mod bodies;
pub mod build;
pub mod builtins;
pub mod edit;
pub mod edit_ops;
pub mod error;
pub mod eval;
pub mod feat_error;
pub mod fillets;
pub mod holes;
pub mod media;
pub mod patterns;
pub mod project;
pub mod report;
pub mod shapes;
pub mod sheet;
pub mod sketch;
pub mod syntax;
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
    /// An assembly's solve, checks and timings (`asm.rs`).
    pub asm: Option<asm::AsmOut>,
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
                    asm: None,
                },
                cache,
            );
        }
    };
    let mut ev = Eval::new(src, opts.rules, opts.build, cache);
    ev.volumes = opts.volumes;
    ev.dir = opts.dir.clone();
    ev.root = opts.root.as_ref().and_then(|r| std::fs::canonicalize(r).ok());
    if asm::declared(&prog) {
        ev.asm = Some(asm::Doc::default());
    }
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
    let kept = ev.b.cache.asm.take();
    let (asm_out, kept) = asm::finish(&mut ev, &opts, kept);
    ev.b.cache.asm = kept;
    timings.eval_ms = t2.elapsed().as_secs_f64() * 1e3;
    timings.kernel_ms = ev.b.kernel_ms;
    let t3 = Instant::now();
    let built_all = opts.build && ev.errors.is_empty() && ev.b.built() == ev.b.features.len();
    let measure = match (built_all, ev.b.measured()) {
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
            if let Some(r) = expect(&mut ev, &s, m) {
                expects.push(r);
            }
        }
    }
    timings.measure_ms = t3.elapsed().as_secs_f64() * 1e3;
    timings.total_ms = t0.elapsed().as_secs_f64() * 1e3;
    let body = if built_all { ev.b.body().cloned() } else { None };
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
        asm: asm_out,
    };
    (run, ev.b.into_cache())
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
