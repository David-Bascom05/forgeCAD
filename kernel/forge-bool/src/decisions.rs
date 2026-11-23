//! The decision log (feature `decisions`, off by default; off, none of this is compiled and the
//! sites that feed it are empty statements).
//!
//! A boolean decides the same geometric fact at several places, each with its own formula and
//! threshold (research/kernel-audit/README.md, 2.1). Here every such decision is recorded under the
//! entities it is about, keyed so that the same fact gets the same key whichever site decides it
//! and in whatever order the entities come; when one plan (one try of the sequential boolean, or
//! one n-ary boolean) closes, every fact decided more than once is checked for two answers. The
//! retry ladder's rungs are counted beside it: which fired, on which input, and how the try ended.
//!
//! Keys: an entity is (body, vertex | edge | face, index): within one plan the bodies do not move,
//! so the body's address names it. A fact about a point (on an edge, at a vertex, one place with
//! another cut) carries the point too: two decisions are about one fact when their keys are equal
//! and their points within `MATCH` of each other.
//!
//! FORGE_DECISIONS_STRICT set: a contradiction panics where the plan closes (the assertion).
//! FORGE_DECISIONS_TRACE set: each rung tried is written to stderr with its thread's name (a test's).
use forge_math::{Point3, tol};
use forge_topo::{Body, EdgeId, FaceId, VertexId};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::Location;
use std::sync::Mutex;

/// Two points of one fact: within this of each other.
pub const MATCH: f64 = tol::LINEAR;
/// Decisions are recorded only where the measured quantity is within this of the threshold's
/// scale: beyond it no formula in the boolean can answer the other way.
pub const BAND: f64 = 100.0 * tol::LINEAR;

/// A kind of geometric fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A point lies on an edge (within the edge's or its end's tolerance). Key: the edge.
    OnEdge,
    /// A point is a vertex (within the vertex's tolerance). Key: the vertex.
    AtVertex,
    /// A vertex lies on a face's surface. Key: the vertex, the face.
    OnSurface,
    /// A face pair's intersection curve runs along an edge rather than crossing it. Key: the two
    /// faces (unordered), the edge.
    RunsAlong,
    /// Two cuts are one vertex. Key: what each cut lies on (unordered), with both points.
    SamePlace,
    /// Two faces lie on one surface. Key: the two faces (unordered).
    Coincident,
    /// How a face pair's curve meets an edge's across-surface near a point: 0 touch, 1 crossing,
    /// 2 two crossings. Key: the two faces (unordered), the edge.
    Contact,
    /// An edge lies on a face of its (within the edge's tolerance), as `check` samples it. Key: the
    /// edge, the face, of the body checked.
    EdgeOnFace,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::OnEdge,
        Kind::AtVertex,
        Kind::OnSurface,
        Kind::RunsAlong,
        Kind::SamePlace,
        Kind::Coincident,
        Kind::Contact,
        Kind::EdgeOnFace,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Kind::OnEdge => "point on edge",
            Kind::AtVertex => "point at vertex",
            Kind::OnSurface => "vertex on surface",
            Kind::RunsAlong => "curve along edge",
            Kind::SamePlace => "two cuts one vertex",
            Kind::Coincident => "two faces one surface",
            Kind::Contact => "touch or crossing",
            Kind::EdgeOnFace => "edge on face",
        }
    }
}

/// An entity of one plan's bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ent {
    body: usize,
    tag: u8,
    idx: u32,
}

impl Ent {
    pub const NONE: Ent = Ent {
        body: 0,
        tag: 0,
        idx: 0,
    };
    pub fn v(body: &Body, v: VertexId) -> Ent {
        Ent {
            body: body as *const Body as usize,
            tag: b'V',
            idx: v.0,
        }
    }
    pub fn e(body: &Body, e: EdgeId) -> Ent {
        Ent {
            body: body as *const Body as usize,
            tag: b'E',
            idx: e.0,
        }
    }
    pub fn f(body: &Body, f: FaceId) -> Ent {
        Ent {
            body: body as *const Body as usize,
            tag: b'F',
            idx: f.0,
        }
    }
    /// An entity of the body at address `body`: tag b'V', b'E' or b'F'.
    pub fn raw(body: usize, tag: u8, idx: u32) -> Ent {
        Ent { body, tag, idx }
    }
    fn label(self) -> String {
        if self == Ent::NONE {
            "free".into()
        } else {
            format!("{}{}", self.tag as char, self.idx)
        }
    }
}

/// A fact's key, unordered where the fact is symmetric.
pub fn key1(a: Ent) -> [Ent; 3] {
    [a, Ent::NONE, Ent::NONE]
}
pub fn key2(a: Ent, b: Ent) -> [Ent; 3] {
    [a, b, Ent::NONE]
}
pub fn pair(a: Ent, b: Ent) -> [Ent; 3] {
    [a.min(b), a.max(b), Ent::NONE]
}
pub fn pair_and(a: Ent, b: Ent, c: Ent) -> [Ent; 3] {
    [a.min(b), a.max(b), c]
}

/// Where a decision was made.
#[derive(Clone, Copy, Debug)]
pub enum Site {
    Named(&'static str),
    /// `on_edge` (or another shared helper), by its caller.
    Via(&'static str, &'static Location<'static>),
}

impl Site {
    fn name(self) -> String {
        match self {
            Site::Named(s) => s.to_string(),
            Site::Via(f, l) => {
                let file = l.file().rsplit(['/', '\\']).next().unwrap_or("");
                format!("{f} <- {file}:{}", l.line())
            }
        }
    }
}

struct Rec {
    kind: Kind,
    key: [Ent; 3],
    at: Option<Point3>,
    at2: Option<Point3>,
    answer: u8,
    site: Site,
    measure: f64,
    limit: f64,
}

struct Ledger {
    path: &'static str,
    mode: String,
    recs: Vec<Rec>,
}

thread_local! {
    static LEDGER: RefCell<Option<Ledger>> = const { RefCell::new(None) };
    static INPUT: RefCell<(String, String)> = const { RefCell::new((String::new(), String::new())) };
}

/// The input the decisions on this thread are about, from now on: its family and its name.
pub fn input(family: &str, name: &str) {
    INPUT.with(|i| *i.borrow_mut() = (family.to_string(), name.to_string()));
}

fn family() -> String {
    INPUT.with(|i| {
        let f = &i.borrow().0;
        if f.is_empty() { "?".into() } else { f.clone() }
    })
}

fn input_name() -> String {
    INPUT.with(|i| i.borrow().1.clone())
}

/// One decision: `answer` about the fact (kind, key, points), by `site`, from `measure` against
/// `limit` (NaN where the site measures nothing of its own).
#[allow(clippy::too_many_arguments)]
pub fn decide(
    kind: Kind,
    key: [Ent; 3],
    at: Option<Point3>,
    at2: Option<Point3>,
    answer: u8,
    site: Site,
    measure: f64,
    limit: f64,
) {
    let open = LEDGER.with(|l| {
        let mut l = l.borrow_mut();
        match l.as_mut() {
            Some(l) => {
                l.recs.push(Rec {
                    kind,
                    key,
                    at,
                    at2,
                    answer,
                    site,
                    measure,
                    limit,
                });
                true
            }
            None => false,
        }
    });
    if !open {
        with_stats(|s| s.unscoped += 1);
    }
    if dump() {
        // (measurement: curved-fallback-rates.md buckets these by |measure|)
        eprintln!(
            "decision {} | {} | {:e} | {:e} | {}",
            kind.name(),
            site.name(),
            measure,
            limit,
            answer
        );
    }
}

/// FORGE_DECISIONS_DUMP set: every decision to stderr, one line each, and each plan's close as a
/// `decision-plan` line. Read once.
fn dump() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGE_DECISIONS_DUMP").is_some())
}

/// `f` (a check of a result: forge-topo's), with its edge-on-face answers recorded under the
/// edge and face of the body checked, in a "result check" ledger of its own (or the open plan's).
pub fn checked<T>(f: impl FnOnce() -> T) -> T {
    let _scope = scope("result check", String::from("plain"));
    let (r, answers) = forge_topo::check::face_log::collect(f);
    for a in &answers {
        let key = key2(Ent::raw(a.body, b'E', a.edge.0), Ent::raw(a.body, b'F', a.face.0));
        let site = Site::Named(a.site);
        decide(
            Kind::EdgeOnFace,
            key,
            None,
            None,
            a.answer,
            site,
            a.measure,
            a.limit,
        );
    }
    r
}

/// How a try ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    /// The boolean lost its way (`BoolError::Internal`): what the ladder retries.
    Internal,
    /// Refused otherwise (unsupported, zero thickness).
    Other,
    /// Not known where the plan closed.
    Unknown,
}

impl Outcome {
    pub fn of<T>(r: &Result<T, crate::BoolError>) -> Outcome {
        match r {
            Ok(_) => Outcome::Ok,
            Err(crate::BoolError::Internal(_)) => Outcome::Internal,
            Err(_) => Outcome::Other,
        }
    }
    fn index(self) -> usize {
        match self {
            Outcome::Ok => 0,
            Outcome::Internal => 1,
            Outcome::Other => 2,
            Outcome::Unknown => 3,
        }
    }
}

/// An open plan's ledger, closed (and checked) when this goes. Nested in an open one, it is
/// nothing: the outer plan holds the decisions.
pub struct Scope {
    opened: bool,
    outcome: Outcome,
}

/// Opens a plan's ledger: `path` the evaluator (sequential, n-ary, imprint), `mode` the retry
/// flags it runs under.
pub fn scope(path: &'static str, mode: String) -> Scope {
    let opened = LEDGER.with(|l| {
        let mut l = l.borrow_mut();
        if l.is_some() {
            return false;
        }
        *l = Some(Ledger {
            path,
            mode,
            recs: Vec::new(),
        });
        true
    });
    Scope {
        opened,
        outcome: Outcome::Unknown,
    }
}

impl Scope {
    pub fn outcome(mut self, o: Outcome) {
        self.outcome = o;
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !self.opened {
            return;
        }
        let Some(ledger) = LEDGER.with(|l| l.borrow_mut().take()) else {
            return;
        };
        if dump() {
            eprintln!("decision-plan {} {}", ledger.path, ledger.recs.len());
        }
        let found = close(ledger, self.outcome);
        if found > 0 && std::env::var_os("FORGE_DECISIONS_STRICT").is_some() && !std::thread::panicking() {
            panic!("decision log: {found} facts decided two ways within one boolean (see decisions::report)");
        }
    }
}

#[derive(Default, Clone)]
pub struct KindStat {
    pub decisions: u64,
    pub facts: u64,
    /// Facts decided at more than one site.
    pub multi_site: u64,
    /// Facts with two answers.
    pub contradictions: u64,
    /// Of those, the ones whose disagreeing decisions all came from one site (the same formula on
    /// points a hair apart: the rule is not transitive).
    pub same_site: u64,
}

#[derive(Default, Clone)]
pub struct PairStat {
    pub count: u64,
    pub families: BTreeMap<String, u64>,
    pub paths: BTreeMap<&'static str, u64>,
    pub example: String,
}

#[derive(Default, Clone)]
pub struct RungStat {
    /// Tries by outcome: ok, internal, other, unknown (a rung that is no retry: counted only).
    pub outcomes: [u64; 4],
    pub families: BTreeMap<String, [u64; 4]>,
    pub inputs: BTreeSet<String>,
}

#[derive(Default)]
pub struct Stats {
    pub kinds: BTreeMap<Kind, KindStat>,
    /// (kind, site, site) -> facts on which those two sites disagreed.
    pub pairs: BTreeMap<(Kind, String, String), PairStat>,
    /// (kind, site) -> decisions made there.
    pub sites: BTreeMap<(Kind, String), u64>,
    pub rungs: BTreeMap<&'static str, RungStat>,
    /// Plans closed, by path, by (with a contradiction, outcome): ok, internal, other, unknown.
    pub plans: BTreeMap<&'static str, [[u64; 4]; 2]>,
    /// Plans with a contradiction, by family.
    pub contradicted: BTreeMap<String, u64>,
    pub unscoped: u64,
}

static STATS: Mutex<Option<Stats>> = Mutex::new(None);

fn with_stats<T>(f: impl FnOnce(&mut Stats) -> T) -> T {
    let mut g = STATS.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Stats::default))
}

/// Everything recorded so far, cleared.
pub fn take() -> Stats {
    let mut g = STATS.lock().unwrap_or_else(|e| e.into_inner());
    g.take().unwrap_or_default()
}

/// A rung of the retry ladder tried: `rung` its name, `outcome` how the try ended.
pub fn rung(rung: &'static str, outcome: Outcome) {
    let (fam, name) = (family(), input_name());
    if std::env::var_os("FORGE_DECISIONS_TRACE").is_some() {
        let thread = std::thread::current().name().unwrap_or("").to_string();
        eprintln!("decisions rung {rung}: {outcome:?} [{thread}] {name}");
    }
    with_stats(|s| {
        let r = s.rungs.entry(rung).or_default();
        let k = outcome.index();
        r.outcomes[k] += 1;
        r.families.entry(fam).or_default()[k] += 1;
        if r.inputs.len() < 12 {
            r.inputs.insert(name);
        }
    });
}

fn near(a: Option<Point3>, b: Option<Point3>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(p), Some(q)) => p.distance(q) <= MATCH,
        _ => false,
    }
}

fn show(p: Option<Point3>) -> String {
    p.map_or(String::new(), |p| {
        format!(" ({:.6}, {:.6}, {:.6})", p.x, p.y, p.z)
    })
}

/// Checks a closed plan's decisions; returns the facts with two answers.
fn close(l: Ledger, outcome: Outcome) -> u64 {
    let mut recs = l.recs;
    recs.sort_by_key(|a| (a.kind, a.key));
    let fam = family();
    let name = input_name();
    let mut kinds: BTreeMap<Kind, KindStat> = BTreeMap::new();
    let mut sites: BTreeMap<(Kind, String), u64> = BTreeMap::new();
    let mut found: Vec<(Kind, String, String, String)> = Vec::new();
    let mut contradictions = 0;
    for group in recs.chunk_by(|a, b| (a.kind, a.key) == (b.kind, b.key)) {
        let kind = group[0].kind;
        let st = kinds.entry(kind).or_default();
        st.decisions += group.len() as u64;
        // facts: decisions about points within MATCH of the first of each
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        for (i, r) in group.iter().enumerate() {
            match clusters
                .iter_mut()
                .find(|c| near(group[c[0]].at, r.at) && near(group[c[0]].at2, r.at2))
            {
                Some(c) => c.push(i),
                None => clusters.push(vec![i]),
            }
        }
        for c in &clusters {
            st.facts += 1;
            let names: BTreeSet<String> = c.iter().map(|&i| group[i].site.name()).collect();
            if names.len() > 1 {
                st.multi_site += 1;
            }
            let answers: BTreeSet<u8> = c.iter().map(|&i| group[i].answer).collect();
            if answers.len() < 2 {
                continue;
            }
            st.contradictions += 1;
            contradictions += 1;
            let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
            for (x, &i) in c.iter().enumerate() {
                for &j in &c[x + 1..] {
                    let (a, b) = (&group[i], &group[j]);
                    if a.answer == b.answer {
                        continue;
                    }
                    let (sa, sb) = (a.site.name(), b.site.name());
                    let (lo, hi) = if sa <= sb { (a, b) } else { (b, a) };
                    if pairs.insert((lo.site.name(), hi.site.name())) {
                        let key: Vec<String> =
                            lo.key.iter().filter(|e| e.tag != 0).map(|e| e.label()).collect();
                        let example = format!(
                            "{name} [{}; {}] {}{}{}: `{}` says {} ({:.3e} vs {:.3e}), `{}` says {} ({:.3e} vs {:.3e})",
                            l.path,
                            l.mode,
                            key.join(" "),
                            show(lo.at),
                            show(lo.at2),
                            lo.site.name(),
                            lo.answer,
                            lo.measure,
                            lo.limit,
                            hi.site.name(),
                            hi.answer,
                            hi.measure,
                            hi.limit,
                        );
                        found.push((kind, lo.site.name(), hi.site.name(), example));
                    }
                }
            }
            if names.len() == 1 {
                st.same_site += 1;
            }
        }
        for r in group {
            *sites.entry((kind, r.site.name())).or_default() += 1;
        }
    }
    with_stats(|s| {
        for (k, v) in kinds {
            let e = s.kinds.entry(k).or_default();
            e.decisions += v.decisions;
            e.facts += v.facts;
            e.multi_site += v.multi_site;
            e.contradictions += v.contradictions;
            e.same_site += v.same_site;
        }
        for (k, n) in sites {
            *s.sites.entry(k).or_default() += n;
        }
        for (kind, a, b, example) in found {
            let p = s.pairs.entry((kind, a, b)).or_default();
            p.count += 1;
            *p.families.entry(fam.clone()).or_default() += 1;
            *p.paths.entry(l.path).or_default() += 1;
            if p.example.is_empty() {
                p.example = example;
            }
        }
        s.plans.entry(l.path).or_default()[(contradictions > 0) as usize][outcome.index()] += 1;
        if contradictions > 0 {
            *s.contradicted.entry(fam.clone()).or_default() += 1;
        }
    });
    contradictions
}

/// The name of a boolean's ending, for the structural-site counts: `Ok`, or the refusal's kind.
pub fn ending<T>(r: &Result<T, crate::BoolError>) -> &'static str {
    use crate::boolean::{Lost, Unhandled};
    match r {
        Ok(_) => "Ok",
        Err(crate::BoolError::ZeroThickness { .. }) => "ZeroThickness",
        Err(crate::BoolError::Unsupported(u)) => match **u {
            Unhandled::NoClosedForm { .. } => "Unsupported NoClosedForm",
            Unhandled::OpenCurve { .. } => "Unsupported OpenCurve",
            Unhandled::SplitThroughSingular { .. } => "Unsupported SplitThroughSingular",
            Unhandled::UntouchedOnOther { .. } => "Unsupported UntouchedOnOther",
            Unhandled::ToolVerticesMeet { .. } => "Unsupported ToolVerticesMeet",
            Unhandled::ProbeOnToolEdge { .. } => "Unsupported ProbeOnToolEdge",
        },
        Err(crate::BoolError::Internal(l)) => match **l {
            Lost::InconsistentClassification { .. } => "Internal InconsistentClassification",
            Lost::SplitOpen { .. } => "Internal SplitOpen",
            Lost::SplitCrossed { .. } => "Internal SplitCrossed",
            Lost::HoleOutside { .. } => "Internal HoleOutside",
            Lost::EmptyCycle { .. } => "Internal EmptyCycle",
            Lost::NoInteriorPoint { .. } => "Internal NoInteriorPoint",
            Lost::NoNormal { .. } => "Internal NoNormal",
            Lost::DegenerateEdge { .. } => "Internal DegenerateEdge",
            Lost::MergedRegion { .. } => "Internal MergedRegion",
            Lost::RetryInvalid => "Internal RetryInvalid",
            Lost::Uncut { .. } => "Internal Uncut",
        },
    }
}

/// A structural site of today's boolean (research/decide-once/old-path-statements.md §G: the
/// retry ladder, UNDECIDED, ODD, pinched, strays, n-ary verdict 3, how each entry ended) reached
/// `n` times: counted beside the rungs, and with FORGE_DECISIONS_SITES set written to stderr as
/// `decisions site NAME xN [thread] input`.
pub fn site(name: &'static str, n: u64) {
    if n == 0 {
        return;
    }
    if std::env::var_os("FORGE_DECISIONS_SITES").is_some() {
        let thread = std::thread::current().name().unwrap_or("").to_string();
        eprintln!("decisions site {name} x{n} [{thread}] {}", input_name());
    }
    let (fam, name_in) = (family(), input_name());
    with_stats(|s| {
        let r = s.rungs.entry(name).or_default();
        r.outcomes[3] += n;
        r.families.entry(fam).or_default()[3] += n;
        if r.inputs.len() < 12 {
            r.inputs.insert(name_in);
        }
    });
}

/// How one entry of today's boolean ended (`path`: "seq" or "n-ary"), as a site
/// `end PATH: ENDING`.
pub fn end<T>(path: &'static str, r: &Result<T, crate::BoolError>) {
    static NAMES: Mutex<BTreeMap<(&'static str, &'static str), &'static str>> = Mutex::new(BTreeMap::new());
    let e = ending(r);
    let name = *NAMES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry((path, e))
        .or_insert_with(|| Box::leak(format!("end {path}: {e}").into_boxed_str()));
    site(name, 1);
}
