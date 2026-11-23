//! Forge (.fg): a typed, parametric part language for agents. A file is a program whose
//! statements make named features; it is checked (syntax, static rules, types and dimensions),
//! evaluated, and each feature built by forge-model as it is made, so anchors and selections
//! read the part as it stands. Errors, warnings and fixes are typed data (`error`), and edits
//! are typed operations on the source (`edit`).

pub mod anchors;
pub mod asm;
pub mod bench;
pub mod bendsim;
pub mod bodies;
pub mod build;
pub mod builtins;
pub mod compare;
pub mod deps;
pub mod dfm;
pub mod drawing;
pub mod edit;
pub mod edit_ops;
pub mod error;
pub mod eval;
pub mod explode;
pub mod export;
pub mod feat_error;
pub mod fillets;
pub mod fonts;
pub mod fuzz;
pub mod geomemo;
pub mod harness;
pub mod holes;
pub mod kept;
pub mod media;
pub mod meshfile;
pub mod par;
pub mod patterns;
pub mod pins;
pub mod project;
pub mod props;
pub mod query;
pub mod refactor;
pub mod release;
pub mod report;
pub mod shapes;
pub mod sheet;
pub mod sketch;
pub mod sketch_text;
pub mod stdparts;
pub mod store;
pub mod syntax;
mod taper_lint;
pub mod thermo;
pub mod thermo_kept;
pub mod tightbox;
pub mod undo;
pub mod units;
pub mod uses;
pub mod validity;
pub mod values;
pub mod vendor;
pub mod verts;
pub mod vtools;
pub mod weld;
pub mod wired;
pub mod work;

use build::Cache;
use error::{Error, Metric, Warning};
use eval::{Eval, FeatInfo, ParamInfo, Rules, Value};
use forge_math::clock::Instant;
use syntax::{Cmp, ExprKind, StmtKind};

pub const REFERENCE: &str = include_str!("reference.md");

/// The reference's `## ` sections: (heading, the section with its heading).
fn sections() -> Vec<(&'static str, &'static str)> {
    let starts: Vec<usize> = REFERENCE
        .match_indices("\n## ")
        .map(|(i, _)| i + 1)
        .chain([REFERENCE.len()])
        .collect();
    starts
        .windows(2)
        .map(|w| {
            let s = &REFERENCE[w[0]..w[1]];
            let head = s[3..].lines().next().unwrap_or("");
            (head, s.trim_end())
        })
        .collect()
}

/// `help TOPIC`: `forge` (or nothing named) the whole reference; a word, each section whose
/// heading has it (`holes`, `sheet`, `patterns`, `queries`, `selectors`); none: the headings.
pub fn help(topic: &str) -> String {
    let topic = topic.trim().to_ascii_lowercase();
    if matches!(topic.as_str(), "forge" | "fg" | ".fg" | "all") {
        return REFERENCE.to_string();
    }
    let heads = || {
        sections()
            .iter()
            .map(|(h, _)| h.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if topic.is_empty() {
        return format!(
            "help forge: the whole .fg language. help WORD: its sections whose heading has WORD:\n{}",
            heads()
        );
    }
    let mut out: Vec<&str> = Vec::new();
    for t in topic.split_whitespace() {
        // `selectors` and `selector` are the Selections section
        let t = if t.starts_with("selector") {
            "selections"
        } else {
            t
        };
        let stem = t.strip_suffix('s').unwrap_or(t);
        let found: Vec<&str> = sections()
            .into_iter()
            .filter(|(h, _)| {
                h.split(|c: char| !c.is_ascii_alphanumeric())
                    .map(str::to_ascii_lowercase)
                    .any(|w| w == t || w.strip_suffix('s').unwrap_or(&w) == stem)
            })
            .map(|(_, s)| s)
            .collect();
        if found.is_empty() {
            return format!(
                "no section on '{t}'. Sections: {}. help forge: all of it",
                heads()
            );
        }
        for s in found {
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out.join("\n\n")
}

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
    /// A part's measures, expects and physical properties, a thermoform's wall simulation and
    /// the features that changed nothing (`Warning::NoEffect`) and a sketch's freedom left
    /// (`Warning::UnderConstrained`); off, a UI shows the shape first and runs them after.
    pub checks: bool,
    /// Threads on an axis not modelled but recorded (`Run::threads`), as a drawing draws them.
    pub cosmetic_threads: bool,
    /// An assembly: mesh its parts for the overlap check and the expects' clearances. Off (a
    /// drawing, which asks for neither), a part loads without them, and a run that needs them
    /// does not share its loads (the setting is in their key).
    pub meshes: bool,
    /// Expressions evaluated on the finished part, each answered in `Run::asked` (`query.rs`).
    pub asks: Vec<String>,
    /// An assembly: each in-file component call's anchors too (`Run::comp_anchors`), for a UI
    /// naming a face of an occurrence as its component names it.
    pub comp_anchors: bool,
    /// A refused fillet, chamfer, shell or face fillet whose size a param drives: search for the
    /// largest value that builds, the error's fix (a few rebuilds from that feature on).
    pub fit_sizes: bool,
    /// Rolled back: build only up to the top-level statement binding this name (a browser's end
    /// of part marker); what follows is evaluated (its rows, its errors) but not built, and the
    /// part is the one as it stood there.
    pub until: Option<String>,
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
            asks: vec![],
            comp_anchors: false,
            fit_sizes: false,
            until: None,
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
    /// The part's box, tight (`tightbox`): its edges', and the meshed points of a face that
    /// bulges past them.
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
    /// The part's weldments: members, plates, for the cut list (`weld.rs`).
    pub welds: weld::Welds,
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
    /// With `Options::comp_anchors`: each in-file component call's anchors, by the occurrence
    /// file the assembly names it with (`#lid`, `#box(w=60)/lid(w=60)`).
    pub comp_anchors: Vec<(String, anchors::Anchors)>,
    /// The document's iProperties (`props.rs`).
    pub props: props::Props,
    /// A part's mass, centre of gravity and inertia from its solids' materials.
    pub physical: Option<props::Physical>,
    /// The inline components (`component NAME(..) { .. }`), each as its own part.
    pub components: Vec<asm::CompOut>,
    /// A document with components: each body and the features that made it (`asm::body_chains`).
    pub body_chains: Vec<(String, Vec<String>)>,
    /// `Options::asks`, each with what it evaluated to.
    pub asked: Vec<(String, anchors::Asked)>,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Parses, checks, evaluates and (with `opts.build`) builds `src`, reusing `cache`'s stages
/// where the history is unchanged. Returns the run and the cache for the next. With
/// `opts.fit_sizes`, a fillet, chamfer, shell or face fillet the kernel refused, its size driven
/// by a param, gets the largest value of that param that builds as its fix.
pub fn run(src: &str, opts: Options, cache: Cache) -> (Run, Cache) {
    let fit = opts.fit_sizes;
    let (mut run, mut cache) = run_once(src, opts.clone(), cache);
    if fit && opts.build {
        cache = fit_sizes(src, &opts, &mut run, cache);
    }
    (run, cache)
}

/// Halvings, then bisection steps, in the search for a size that builds.
const FIT_HALVINGS: i32 = 8;
const FIT_STEPS: usize = 12;

/// `Options::fit_sizes`: each refused size's largest value that builds, put on its error.
fn fit_sizes(src: &str, opts: &Options, run: &mut Run, mut cache: Cache) -> Cache {
    let inner = Options {
        fit_sizes: false,
        checks: false,
        ..opts.clone()
    };
    for k in 0..run.errors.len() {
        let Error::Build { id, .. } = &run.errors[k] else {
            continue;
        };
        let Some(f) = run.feats.iter().find(|f| &f.id == id) else {
            continue;
        };
        if !matches!(f.kind, "fillet" | "chamfer" | "shell" | "facefillet") {
            continue;
        }
        let Some(d) = f.dims.first() else { continue };
        let values::Src::Driving(key) = &d.num.src else {
            continue;
        };
        let (id, key, top, dim) = (id.clone(), (**key).clone(), d.num.v, d.num.dim);
        let builds = |v: f64, cache: Cache| -> (bool, Cache) {
            let op = error::Op::SetParam {
                param: key.clone(),
                value: error::Literal { value: v, dim },
            };
            let Ok(text) = edit::apply(src, &op) else {
                return (false, cache);
            };
            let (r, c) = run_once(&text, inner.clone(), cache);
            let ok = !r
                .errors
                .iter()
                .any(|e| matches!(e, Error::Build { id: i, .. } if *i == id));
            (ok && r.feats.iter().any(|f| f.id == id), c)
        };
        let mut bad = top;
        let mut good = None;
        for h in 1..=FIT_HALVINGS {
            let v = top * 0.5f64.powi(h);
            let (ok, c) = builds(v, cache);
            cache = c;
            if ok {
                good = Some(v);
                break;
            }
            bad = v;
        }
        let Some(mut lo) = good else { continue };
        for _ in 0..FIT_STEPS {
            let mid = (lo + bad) * 0.5;
            let (ok, c) = builds(mid, cache);
            cache = c;
            if ok {
                lo = mid;
            } else {
                bad = mid;
            }
        }
        // three figures, rounded down so it still builds
        let p = 10f64.powi(2 - lo.abs().log10().floor() as i32);
        let shown = (lo * p).floor() / p;
        let value = if shown > 0.0 && shown < lo {
            let (ok, c) = builds(shown, cache);
            cache = c;
            if ok { shown } else { lo }
        } else {
            lo
        };
        if let Error::Build { fit, .. } = &mut run.errors[k] {
            *fit = Some(Box::new((key, error::Literal { value, dim })));
        }
    }
    cache
}

/// Bodies' mass properties as one: volumes and areas summed, the centroid weighed by volume, each
/// inertia moved to it (parallel axes), errors summed.
fn sum_masses(ms: &[forge_topo::mass::MassProps]) -> forge_topo::mass::MassProps {
    use forge_math::{Point3, Vec3};
    let volume: f64 = ms.iter().map(|m| m.volume).sum();
    let area: f64 = ms.iter().map(|m| m.area).sum();
    let c = if volume != 0.0 {
        ms.iter()
            .fold(Vec3::ZERO, |a, m| a + (m.centroid - Point3::ORIGIN) * m.volume)
            / volume
    } else {
        Vec3::ZERO
    };
    let centroid = Point3::ORIGIN + c;
    let mut inertia = [[0.0; 3]; 3];
    let mut err = [0.0; 4];
    for m in ms {
        let d = m.centroid - centroid;
        let d = [d.x, d.y, d.z];
        let dd = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        for i in 0..3 {
            for j in 0..3 {
                let shift = if i == j { dd - d[i] * d[j] } else { -d[i] * d[j] };
                inertia[i][j] += m.inertia[i][j] + m.volume * shift;
            }
        }
        for (e, x) in err.iter_mut().zip(m.err) {
            *e += x;
        }
    }
    let (principal, axes) = forge_topo::mass::eigen(inertia);
    forge_topo::mass::MassProps {
        volume,
        area,
        centroid,
        inertia,
        principal,
        axes,
        err,
        faces: vec![],
    }
}

fn run_once(src: &str, opts: Options, mut cache: Cache) -> (Run, Cache) {
    let t0 = Instant::now();
    let mut timings = Timings::default();
    // the run before's tree, patched where only a number changed (an edit of one dimension)
    let prog = match cache.parsed.take() {
        Some((was, mut prog)) => {
            if syntax::reparse_number(&was, &mut prog, src) {
                Ok(prog)
            } else {
                syntax::parse(src)
            }
        }
        None => syntax::parse(src),
    };
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
                    welds: Default::default(),
                    threads: vec![],
                    solids: vec![],
                    stage: 0,
                    asm: None,
                    anchors: None,
                    comp_anchors: vec![],
                    props: props::Props::default(),
                    physical: None,
                    components: vec![],
                    body_chains: vec![],
                    asked: vec![],
                },
                cache,
            );
        }
    };
    let mut ev = Eval::new(src, opts.rules, opts.build, cache);
    ev.masses_owed = Some(vec![]);
    ev.volumes = opts.volumes;
    ev.checks = opts.checks;
    ev.cosmetic_threads = opts.cosmetic_threads;
    ev.until = opts.until.clone();
    ev.dir = opts.dir.clone();
    ev.root = fence_of(&opts);
    // any document may hold components and joints (an `assembly` first line is optional)
    ev.asm = Some(asm::Doc {
        declared: asm::declared(&prog),
        ..Default::default()
    });
    // a harness file is evaluated where an assembly places it, with the occurrences it is given
    let placed_only = harness::header(&prog).map(|(names, _)| names);
    let keep_tree = placed_only.is_none();
    let prog = match &placed_only {
        Some(names) => {
            harness::placed_only(&mut ev, prog[0].span, names);
            vec![]
        }
        None => prog,
    };
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
    taper_lint::lint(&mut ev);
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
    // rolled back: everything up to the marker built is all there is to build
    let made = ev.rolled.unwrap_or(ev.b.features.len());
    let built_all = opts.build && ev.errors.is_empty() && ev.b.built() == made;
    // the whole part's mass (its measure) and each solid's (its physical properties) worked out
    // side by side, each as it is alone, and the part's tight box beside them
    let ((whole_mass, whole_box), solid_masses) = {
        let checks = built_all && opts.checks;
        let whole = (checks && ev.b.measured().is_none())
            .then(|| ev.b.body())
            .flatten();
        let each = checks
            && !assembly
            && !(whole.is_some() && ev.b.stages.last().is_some_and(|st| st.solids.len() == 1));
        let solids = if each { props::solids(&ev) } else { vec![] };
        // every solid's own mass is wanted anyway: the whole is their sum, not worked out again
        // over the joined body (that would do the integrals twice)
        // (the joined body still gives the box)
        let boxed = whole;
        let whole = match whole {
            Some(_) if each && solids.len() == ev.b.stages.last().map_or(0, |st| st.solids.len()) => None,
            w => w,
        };
        let from_parts = each && whole.is_none() && checks && ev.b.measured().is_none();
        // a thermoform's modelled part too (`thermo.rs`)
        let owed = ev.masses_owed.take().unwrap_or_default();
        let mass = |b: &forge_topo::Body| forge_topo::mass::mass(b);
        let each_mass =
            || -> Option<Vec<_>> { each.then(|| solids.iter().map(|x| mass(&x.2).ok()).collect()) };
        // under a budget (spent on this thread), or with one thread to run on, one after another
        let (w, e, o) = if forge_geom::budget::left() == u64::MAX && forge_math::par::threads() > 1 {
            forge_math::par::scope(|s| {
                let w = whole.map(|b| s.spawn(move || mass(b)));
                let bx = boxed.map(|b| s.spawn(move || tightbox::tight_box(b)));
                let o: Vec<_> = owed.iter().map(|x| s.spawn(move || mass(&x.2).ok())).collect();
                let e = each_mass();
                let w = (w.map(joined), bx.map(joined));
                (w, e, o.into_iter().map(joined).collect::<Vec<_>>())
            })
        } else {
            // in the order they were asked for: the modelled parts' in the run, then the rest
            let o: Vec<_> = owed.iter().map(|x| mass(&x.2).ok()).collect();
            let w = (whole.map(mass), boxed.map(tightbox::tight_box));
            (w, each_mass(), o)
        };
        for ((i, key, _), m) in owed.into_iter().zip(o) {
            if let Some(t) = ev.thermos.get_mut(i) {
                t.modelled = m.clone();
            }
            ev.b.memo_keep(key, m);
        }
        let (w, bx) = w;
        let w = match (&e, from_parts) {
            (Some(parts), true) => Some(
                parts
                    .iter()
                    .map(|m| m.clone().ok_or(()))
                    .collect::<Result<Vec<_>, ()>>()
                    .map(|ms| sum_masses(&ms))
                    .or_else(|()| ev.b.body().map_or(Err(()), |b| mass(b).map_err(|_| ()))),
            )
            .and_then(Result::ok)
            .map(Ok),
            _ => w,
        };
        ((w, bx), e)
    };
    let measure = match (built_all && opts.checks, ev.b.measured()) {
        (false, _) => None,
        (true, Some(m)) => Some(m),
        (true, None) => {
            let m = measure(&mut ev, whole_mass, whole_box);
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
        props::part_physical(
            &mut ev,
            measure.as_ref().and_then(|m| m.mass.as_ref()),
            solid_masses,
        )
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
        welds: std::mem::take(&mut ev.welds),
        threads: std::mem::take(&mut ev.threads),
        solids: if built_all {
            ev.b.stages.last().map_or(vec![], |s| s.spans().to_vec())
        } else {
            vec![]
        },
        stage: if built_all { ev.b.stage_id() } else { 0 },
        asm: asm_out,
        anchors: None,
        comp_anchors: vec![],
        props: std::mem::take(&mut ev.props),
        physical,
        components: vec![],
        body_chains: vec![],
        asked: vec![],
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
    run.asked = opts
        .asks
        .iter()
        .map(|t| (t.clone(), anchors::ask(&mut ev, t)))
        .collect();
    if opts.comp_anchors {
        run.comp_anchors = asm::comp_anchors(&mut ev.comps);
    }
    run.components = asm::components(&ev);
    run.body_chains = chains;
    asm::keep_comp_caches(&mut ev);
    let mut cache = ev.b.into_cache();
    if keep_tree {
        cache.parsed = Some((src.to_string(), prog));
    }
    (run, cache)
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
    forge_dirs::fs::canonicalize(root?).ok()
}

/// Whether `file` beside `dir` is outside the fence: as written (whether or not it is there, so
/// the refusal says nothing of what is outside), or once its links are resolved.
pub(crate) fn outside(dir: Option<&std::path::Path>, fence: Option<&std::path::Path>, file: &str) -> bool {
    let (Some(d), Some(f)) = (dir, fence) else {
        return false;
    };
    let Ok(d) = forge_dirs::fs::canonicalize(d) else {
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
    !lexical.starts_with(f) || forge_dirs::fs::canonicalize(d.join(file)).is_ok_and(|p| !p.starts_with(f))
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

/// A scoped thread's answer, its panic carried on.
fn joined<T>(h: forge_math::par::ScopedJoinHandle<'_, T>) -> T {
    h.join().unwrap_or_else(|p| std::panic::resume_unwind(p))
}

/// The finished part's measure; `mass` and `bbox`, its mass and box if worked out already.
fn measure(
    ev: &mut Eval,
    mass: Option<Result<forge_topo::mass::MassProps, forge_topo::mass::MassError>>,
    bbox: Option<forge_math::Box3>,
) -> Option<Measure> {
    let invalid = validity::problems(ev);
    let body = ev.b.body()?;
    let solids = ev.b.stages.last().map_or(0, |s| s.solids.len());
    let m = match mass {
        Some(m) => m,
        None => forge_topo::mass::mass(body),
    }
    .ok()?;
    let bb = bbox.unwrap_or_else(|| tightbox::tight_box(body));
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
    let StmtKind::Expect {
        metric, cmp, args, ..
    } = &s.kind
    else {
        return None;
    };
    let (metric, measured) = match metric.as_str() {
        "volume" => (Metric::Volume, m.volume),
        "area" => (Metric::Area, m.area),
        "faces" => (Metric::Faces, m.faces as f64),
        "edges" => (Metric::Edges, m.edges as f64),
        "bodies" => (Metric::Bodies, m.bodies as f64),
        "valid" => (Metric::Valid, if m.problems == 0 { 1.0 } else { 0.0 }),
        "holes" => (
            Metric::Holes,
            ev.b.body()
                .map_or(0.0, |b| forge_agent::features::holes(b).len() as f64),
        ),
        // the material's area where the plane z = V cuts the part
        "section_area" => {
            let z = match args.first().map(|a| ev.expr(a)) {
                Some(Ok(Value::Num(n))) => n.v,
                _ => f64::NAN,
            };
            let at = forge_math::Frame::from_z(forge_math::Point3::new(0.0, 0.0, z), forge_math::UnitVec3::Z);
            let area = match ev.b.body() {
                Some(b) if z.is_finite() => forge_agent::features::section(b, &at)
                    .map_or(f64::NAN, |ls| ls.iter().map(|l| l.area).sum()),
                _ => f64::NAN,
            };
            (Metric::SectionArea, area)
        }
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
    let inch = |x: f64, u: syntax::Unit| if u == syntax::Unit::In { x * 25.4 } else { x };
    let lit = |e: &syntax::Expr| match e.kind {
        ExprKind::Num(x, u) => Some(inch(x, u)),
        ExprKind::Neg(ref a) => match a.kind {
            ExprKind::Num(x, u) => Some(-inch(x, u)),
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
    matches!(
        m,
        Metric::Volume | Metric::Area | Metric::Mass | Metric::SectionArea
    )
}

/// A PNG of the finished part.
pub fn render(body: &forge_topo::Body, view: &str, px: usize) -> Result<Vec<u8>, String> {
    let mut cache = forge_agent::facemesh::Cache::default();
    let faces = cache.faces(body);
    let mesh = forge_agent::facemesh::join(&faces);
    forge_agent::render::render(body, &mesh, view, px, &[])
}
