//! The default browser build's build worker, in a Web Worker of its own: the same `Worker` as
//! natively, in another wasm instance with its own memory (no shared memory, so no allocator or
//! lock is shared with the page, and the page needs no cross-origin isolation). The page keeps
//! WebGPU, egui and the files; the worker keeps the stage cache and the kernel's bodies.
//!
//! Everything between them is bytes (`wire`), in one message each way per thing said:
//! - to the worker (`ToWorker`): a session opened or closed, its work (a text, drives), a new view
//!   wanted, where its build stops, and every change of the page's files, sent before the work
//!   that may read it (`postMessage` keeps order), so a build sees the files as the page had them
//!   when it asked;
//! - to the page (`ToPage`): each `Msg` a session's worker sends, what its build read, and the files
//!   the worker wrote (kept caches), for the page to keep as it keeps its own.
//!
//! The codec is built on every target, so its round trips are tested natively; the page's and the
//! worker's ends are the browser's (`page`, `worker`).

use super::{AsmView, Built, Msg, ViewMade, Work};
use crate::wire::{R, W, Wire, choice, record};

choice!(Work {
    0 => Text(a0),
    1 => Drive(a0, a1),
});

choice!(Msg {
    0 => Built(a0),
    1 => Warm(a0),
    2 => Final { seq, mesh, flat, mesh_ms },
    3 => Posed { seq, poses, ms },
});

record!(Built {
    seq,
    ok,
    feats,
    params,
    issues,
    measure,
    timings,
    rebuilt,
    reused,
    mesh,
    delta,
    flat,
    sheets,
    thermos,
    view,
    anchors,
    asm,
    build_ms,
    mesh_ms,
    cutlist,
    inspect,
    preview,
    checked,
    hash,
});

record!(AsmView {
    occ,
    placed,
    meshes,
    restage,
    joints,
    mobility,
    bom,
    expects,
    overlap,
    timings,
    parts,
    meshed,
    boxes,
    grounded,
    insts,
    links,
    link_at,
    poses,
    inspect,
    thermo_files,
    body_chains,
    harnesses,
    harness_placed,
});

/// The browser's view to make: its parts, placements and boxes. (Natively a view is the GPU's
/// own and never crosses: a native session's worker never sends one this way.)
impl Wire for ViewMade {
    #[cfg(target_arch = "wasm32")]
    fn put(&self, o: &mut W) {
        self.0.put(o);
        self.1.put(o);
        self.2.put(o);
    }
    #[cfg(target_arch = "wasm32")]
    fn take(i: &mut R) -> Option<Self> {
        Some(ViewMade(Wire::take(i)?, Wire::take(i)?, Wire::take(i)?))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn put(&self, _: &mut W) {
        unreachable!("a native view is the GPU's: it never crosses")
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn take(_: &mut R) -> Option<Self> {
        None
    }
}

/// What the Inspect tools are given, less the exact bodies and an assembly's solve, which stay
/// in the worker: each body's brief instead, and the build's number to ask the rest by (`Ask`,
/// `Job`).
impl Wire for crate::gui::gui_inspect::InspectData {
    fn put(&self, o: &mut W) {
        self.placed.put(o);
        self.names.put(o);
        self.props.put(o);
        self.physical.put(o);
        self.briefs.put(o);
        self.assembly.put(o);
        self.number.put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        Some(crate::gui::gui_inspect::InspectData {
            bodies: vec![],
            placed: Wire::take(i)?,
            names: Wire::take(i)?,
            props: Wire::take(i)?,
            physical: Wire::take(i)?,
            asm: None,
            briefs: Wire::take(i)?,
            assembly: Wire::take(i)?,
            number: Wire::take(i)?,
        })
    }
}

record!(crate::gui::gui_inspect::Brief {
    faces,
    edges,
    vertices
});

/// A harness as the page shows it: its tubes' bodies stay in the worker (only the worker's
/// meshing reads them: `asm_view`).
impl Wire for forge_lang::harness::HarnessOut {
    fn put(&self, o: &mut W) {
        let forge_lang::harness::HarnessOut {
            id,
            file,
            connectors,
            nodes,
            segments,
            wires,
            bom,
            types,
            tubes: _,
            paths,
            ties,
            clips,
            spans,
        } = self;
        id.put(o);
        file.put(o);
        connectors.put(o);
        nodes.put(o);
        segments.put(o);
        wires.put(o);
        bom.put(o);
        types.put(o);
        paths.put(o);
        ties.put(o);
        clips.put(o);
        spans.put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        Some(forge_lang::harness::HarnessOut {
            id: Wire::take(i)?,
            file: Wire::take(i)?,
            connectors: Wire::take(i)?,
            nodes: Wire::take(i)?,
            segments: Wire::take(i)?,
            wires: Wire::take(i)?,
            bom: Wire::take(i)?,
            types: Wire::take(i)?,
            tubes: vec![],
            paths: Wire::take(i)?,
            ties: Wire::take(i)?,
            clips: Wire::take(i)?,
            spans: Wire::take(i)?,
        })
    }
}

/// A thermoform's drawing: what the views draw crosses; the formed part's and the mould's bodies,
/// the wall's model and the mould's side (what the views work more out of, when asked) stay in the
/// worker.
impl Wire for crate::thermo_view::Gfx {
    fn put(&self, o: &mut W) {
        let crate::thermo_view::Gfx {
            steps,
            draft,
            part: _,
            mould: _,
            trim,
            sheet,
            vents,
            points,
            sim,
            has_sim,
            sim_mesh,
            sim_modelled,
            sim_feet,
            sim_heat,
            blank,
            part_span,
            mould_span,
            field: _,
            tool: _,
            heat,
            heat_for,
            extremes,
            sim_extremes,
            range,
            at,
            t,
            stretch,
            stretch_range,
            web,
            print_src: _,
            made: _,
            key,
            remote: _,
        } = self;
        steps.put(o);
        draft.put(o);
        trim.put(o);
        sheet.put(o);
        vents.put(o);
        points.put(o);
        sim.put(o);
        has_sim.put(o);
        sim_mesh.put(o);
        sim_modelled.put(o);
        sim_feet.put(o);
        sim_heat.put(o);
        blank.put(o);
        part_span.put(o);
        mould_span.put(o);
        heat.put(o);
        heat_for.put(o);
        extremes.put(o);
        sim_extremes.put(o);
        range.put(o);
        at.put(o);
        t.put(o);
        stretch.put(o);
        stretch_range.put(o);
        web.put(o);
        key.put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        Some(crate::thermo_view::Gfx {
            steps: Wire::take(i)?,
            draft: Wire::take(i)?,
            trim: Wire::take(i)?,
            sheet: Wire::take(i)?,
            vents: Wire::take(i)?,
            points: Wire::take(i)?,
            sim: Wire::take(i)?,
            has_sim: Wire::take(i)?,
            sim_mesh: Wire::take(i)?,
            sim_modelled: Wire::take(i)?,
            sim_feet: Wire::take(i)?,
            sim_heat: Wire::take(i)?,
            blank: Wire::take(i)?,
            part_span: Wire::take(i)?,
            mould_span: Wire::take(i)?,
            heat: Wire::take(i)?,
            heat_for: Wire::take(i)?,
            extremes: Wire::take(i)?,
            sim_extremes: Wire::take(i)?,
            range: Wire::take(i)?,
            at: Wire::take(i)?,
            t: Wire::take(i)?,
            stretch: Wire::take(i)?,
            stretch_range: Wire::take(i)?,
            web: Wire::take(i)?,
            key: Wire::take(i)?,
            remote: true,
            ..Default::default()
        })
    }
}

use crate::gui::gui_inspect::{Answer, Ask, Job, Tgt, Told};

choice!(Tgt {
    0 => Ent(a0, a1),
    1 => Whole(a0, a1),
    2 => Free(a0),
});

choice!(Ask {
    0 => Filtered(a0, a1, a2, a3, a4),
    1 => FaceAt(a0, a1, a2, a3, a4),
    2 => Plane(a0, a1),
    3 => Project(a0, a1),
    4 => Locate(a0, a1),
    5 => ThermoSim(a0),
    6 => ThermoHeat(a0, a1),
    7 => ThermoExport(a0, a1, a2, a3),
});

choice!(Told {
    0 => Pick(a0),
    1 => Face(a0),
    2 => Plane(a0),
    3 => Project(a0),
    4 => Located(a0),
    5 => ThermoSim(a0),
    6 => ThermoHeat(a0, a1),
    7 => Bytes(a0),
});

choice!(Answer {
    0 => Measure(a0, a1),
    1 => Wall(a0, a1),
    2 => Physical(a0),
    3 => Told(a0, a1, a2),
});

/// An Inspect job as it crosses: the wall's search's control stays on the page (the worker is
/// told to stop, and says how far it is).
pub(crate) enum JobW {
    Measure(u64, Vec<Tgt>),
    Warm,
    Wall(u64, u32),
    Physical,
}

choice!(JobW {
    0 => Measure(a0, a1),
    1 => Warm,
    2 => Wall(a0, a1),
    3 => Physical,
});

impl JobW {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn of(j: Job) -> JobW {
        match j {
            Job::Measure(id, t) => JobW::Measure(id, t),
            Job::Warm => JobW::Warm,
            Job::Wall(id, pl, _) => JobW::Wall(id, pl),
            Job::Physical => JobW::Physical,
        }
    }
}

/// What the page says to the build worker. A session is the page's number for one document.
// (natively only its tests use it)
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum ToWorker {
    /// A document opened: its folder (fonts and imports beside it), its file, whether its view is
    /// wanted.
    Open {
        id: u32,
        dir: Option<std::path::PathBuf>,
        file: Option<String>,
        gpu: bool,
    },
    Work(u32, Work),
    /// A new view wanted (`Session::regpu`).
    Gpu(u32),
    /// Where its builds stop (`Session::set_until`).
    Until(u32, Option<String>),
    /// Closed: its worker finishes nothing more.
    Close(u32),
    /// A file of the page's written (its bytes) or removed (none).
    File(String, Option<Vec<u8>>),
    /// A file put by the page without being a change (loaded from where it's kept).
    Load(String, Vec<u8>),
    /// Every file under a prefix let go (`fs_unload`).
    Unload(String),
    /// An Inspect job on the bodies of the build with this number.
    Job(u64, JobW),
    /// A question of the bodies of the build with this number.
    Ask(u64, Ask),
    /// The wall's search with this id no longer wanted.
    StopWall(u64),
}

choice!(ToWorker {
    0 => Open { id, dir, file, gpu },
    1 => Work(a0, a1),
    2 => Gpu(a0),
    3 => Until(a0, a1),
    4 => Close(a0),
    5 => File(a0, a1),
    6 => Load(a0, a1),
    7 => Unload(a0),
    8 => Job(a0, a1),
    9 => Ask(a0, a1),
    10 => StopWall(a0),
});

/// What the build worker says to the page.
// (natively only its tests use it)
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum ToPage {
    Msg(u32, Msg),
    /// The files a session's last build read (for the host's watch).
    Read(u32, Vec<std::path::PathBuf>),
    /// A file the worker wrote (its bytes) or removed (none): a kept cache, for the page to keep.
    File(String, Option<Vec<u8>>),
    /// An Inspect job's answer, or a question's.
    Answer(Answer),
    /// How far the wall's search with this id is, in thousandths.
    WallDone(u64, u32),
}

choice!(ToPage {
    0 => Msg(a0, a1),
    1 => Read(a0, a1),
    2 => File(a0, a1),
    3 => Answer(a0),
    4 => WallDone(a0, a1),
});

/// The page's end: sessions whose worker is the build worker, their messages routed back to
/// them, the page's file changes sent ahead of the work that may read them.
#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
pub mod page {
    use super::{JobW, ToPage, ToWorker};
    use crate::fg::{Msg, Reads};
    use crate::gui::gui_inspect::{Answer, Ask, Job, WallCtl};
    use crate::wire::{from_bytes, to_bytes};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::mpsc::Sender;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// The page's (web/app.js): bytes to the build worker (copied, then transferred).
        #[wasm_bindgen(js_namespace = globalThis, js_name = forge_build_post)]
        fn post(bytes: &[u8]);
    }

    /// A session's way back: where its messages go, what its build read, its waker.
    struct Back {
        out: Sender<Msg>,
        deps: Reads,
        waker: Arc<dyn Fn() + Send + Sync>,
    }

    thread_local! {
        /// The page started a build worker (`app_build_worker`); else builds step on the page.
        static ON: Cell<bool> = const { Cell::new(false) };
        static NEXT: Cell<u32> = const { Cell::new(1) };
        static BACK: RefCell<HashMap<u32, Back>> = RefCell::new(HashMap::new());
        /// A file the worker wrote, being put in the page's files: not sent back to it.
        static FROM_WORKER: Cell<bool> = const { Cell::new(false) };
        /// What crossed, for the page's tests: messages and bytes each way, and the ms spent
        /// writing and reading them.
        static COST: RefCell<Cost> = RefCell::new(Cost::default());
    }

    /// The page's count of what crossed (`app_build_cost`).
    #[derive(Default)]
    struct Cost {
        sent: u64,
        sent_bytes: u64,
        send_ms: f64,
        got: u64,
        got_bytes: u64,
        read_ms: f64,
        largest: u64,
    }

    /// The page's: it started a build worker (web/build.js) and set `forge_build_post`.
    #[wasm_bindgen]
    pub fn app_build_worker() {
        ON.with(|o| o.set(true));
    }

    /// Whether sessions' builds go to the build worker.
    pub fn on() -> bool {
        ON.with(Cell::get)
    }

    fn send(m: &ToWorker) {
        let t = crate::viewer::now_ms();
        let b = to_bytes(m);
        post(&b);
        COST.with(|c| {
            let mut c = c.borrow_mut();
            c.sent += 1;
            c.sent_bytes += b.len() as u64;
            c.send_ms += crate::viewer::now_ms() - t;
        });
    }

    /// A session's worker opened in the build worker: its number.
    pub(crate) fn open(
        (dir, file): (Option<std::path::PathBuf>, Option<String>),
        gpu: bool,
        out: Sender<Msg>,
        deps: Reads,
        waker: Arc<dyn Fn() + Send + Sync>,
    ) -> u32 {
        let id = NEXT.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        BACK.with(|b| b.borrow_mut().insert(id, Back { out, deps, waker }));
        send(&ToWorker::Open { id, dir, file, gpu });
        id
    }

    pub(crate) fn work(id: u32, w: crate::fg::Work) {
        send(&ToWorker::Work(id, w));
    }

    pub(crate) fn gpu(id: u32) {
        send(&ToWorker::Gpu(id));
    }

    pub(crate) fn until(id: u32, upto: Option<String>) {
        send(&ToWorker::Until(id, upto));
    }

    pub(crate) fn close(id: u32) {
        BACK.with(|b| b.borrow_mut().remove(&id));
        send(&ToWorker::Close(id));
    }

    /// A change of the page's files, for the worker's copy (not one the worker made).
    pub fn file_changed(path: &str, bytes: Option<&[u8]>) {
        if on() && !FROM_WORKER.with(Cell::get) {
            send(&ToWorker::File(path.into(), bytes.map(<[u8]>::to_vec)));
        }
    }

    /// A file the page put without changing it (loaded from where it's kept).
    pub fn file_loaded(path: &str, bytes: &[u8]) {
        if on() {
            send(&ToWorker::Load(path.into(), bytes.to_vec()));
        }
    }

    pub fn files_unloaded(prefix: &str) {
        if on() {
            send(&ToWorker::Unload(prefix.into()));
        }
    }

    thread_local! {
        /// Where the Inspect tools' answers go, and the wall's search the worker runs.
        static INSPECT: RefCell<Option<(Sender<Answer>, crate::gui::Waker)>> = const { RefCell::new(None) };
        static WALL: RefCell<Option<(u64, Arc<WallCtl>)>> = const { RefCell::new(None) };
    }

    pub(crate) fn answers_to(to: Sender<Answer>, waker: crate::gui::Waker) {
        INSPECT.with(|i| *i.borrow_mut() = Some((to, waker)));
    }

    /// An Inspect job on the bodies of the build `number` (the worker's).
    pub(crate) fn inspect(number: u64, job: Job) {
        if let Job::Wall(id, _, ctl) = &job {
            WALL.with(|w| *w.borrow_mut() = Some((*id, ctl.clone())));
        }
        send(&ToWorker::Job(number, JobW::of(job)));
    }

    /// A question of the bodies of the build `number`.
    pub(crate) fn ask(number: u64, a: Ask) {
        send(&ToWorker::Ask(number, a));
    }

    type Then = Box<dyn FnOnce(crate::gui::gui_inspect::Told)>;

    thread_local! {
        /// Questions whose answer goes to a function of their own (by the question's bytes).
        static THEN: RefCell<Vec<(Vec<u8>, Then)>> = const { RefCell::new(Vec::new()) };
    }

    /// A question of the bodies of the build `number`, its answer to `then` (once).
    pub(crate) fn ask_then(number: u64, a: Ask, then: Then) {
        let key = [to_bytes(&number), to_bytes(&a)].concat();
        THEN.with(|t| t.borrow_mut().push((key, then)));
        send(&ToWorker::Ask(number, a));
    }

    /// Each frame: the wall's search told to stop once the page no longer wants it.
    pub(crate) fn tick() {
        use std::sync::atomic::Ordering::Relaxed;
        let stop = WALL.with(|w| {
            let mut w = w.borrow_mut();
            match &*w {
                Some((id, ctl)) if ctl.wanted.load(Relaxed) != *id => w.take().map(|x| x.0),
                _ => None,
            }
        });
        if let Some(id) = stop {
            send(&ToWorker::StopWall(id));
        }
    }

    /// The page's (web/app.js): a message from the build worker.
    #[wasm_bindgen]
    pub fn app_build_msg(bytes: &[u8]) {
        let t = crate::viewer::now_ms();
        let Some(m) = from_bytes::<ToPage>(bytes) else {
            crate::viewer::say("forge-app: a build worker's message it could not read");
            return;
        };
        COST.with(|c| {
            let mut c = c.borrow_mut();
            c.got += 1;
            c.got_bytes += bytes.len() as u64;
            c.largest = c.largest.max(bytes.len() as u64);
            c.read_ms += crate::viewer::now_ms() - t;
        });
        match m {
            ToPage::Msg(id, msg) => BACK.with(|b| {
                if let Some(back) = b.borrow().get(&id) {
                    let _ = back.out.send(msg);
                    (back.waker)();
                }
            }),
            ToPage::Read(id, files) => BACK.with(|b| {
                if let Some(back) = b.borrow().get(&id) {
                    *back.deps.lock().unwrap_or_else(|e| e.into_inner()) = files;
                }
            }),
            ToPage::File(path, bytes) => {
                // kept as the page keeps its own files (the page's storage is told), not sent back
                FROM_WORKER.with(|f| f.set(true));
                let _ = match bytes {
                    Some(b) => forge_dirs::fs::write(&path, b),
                    None if forge_dirs::fs::is_dir(&path) => forge_dirs::fs::remove_dir_all(&path),
                    None => forge_dirs::fs::remove_file(&path),
                };
                FROM_WORKER.with(|f| f.set(false));
            }
            ToPage::Answer(Answer::Told(n, a, t))
                if {
                    let key = [to_bytes(&n), to_bytes(&a)].concat();
                    THEN.with(|th| th.borrow().iter().any(|x| x.0 == key))
                } =>
            {
                let key = [to_bytes(&n), to_bytes(&a)].concat();
                let then = THEN.with(|th| {
                    let mut th = th.borrow_mut();
                    let k = th.iter().position(|x| x.0 == key)?;
                    Some(th.remove(k).1)
                });
                if let Some(f) = then {
                    f(t);
                }
            }
            ToPage::Answer(a) => INSPECT.with(|i| {
                if let Some((to, waker)) = &*i.borrow() {
                    let _ = to.send(a);
                    waker();
                }
            }),
            ToPage::WallDone(id, k) => WALL.with(|w| {
                if let Some((n, ctl)) = &*w.borrow()
                    && *n == id
                {
                    ctl.done.store(k, std::sync::atomic::Ordering::Relaxed);
                }
            }),
        }
    }

    /// What a message of a mesh `bytes` long costs to write and read in this wasm (the page's
    /// instance; the worker's is the same code): the mean ms of each over `n` tries, as JSON.
    /// The copies out of and into wasm memory and the transfer are the page's to time (web/tests).
    #[wasm_bindgen]
    pub fn app_build_bench(bytes: u32, n: u32) -> String {
        use crate::mesh::{Lod, PartMesh};
        let mesh = PartMesh {
            center: [0.0; 3],
            radius: 1.0,
            lods: vec![Lod {
                vertices: (0..bytes).map(|i| (i * 7) as u8).collect(),
                ..Lod::default()
            }],
        };
        let m = ToPage::Msg(
            1,
            Msg::Final {
                seq: 1,
                mesh,
                flat: None,
                mesh_ms: 0.0,
            },
        );
        // n of each timed together: a page that isn't cross-origin isolated has a coarse clock
        let n = n.max(1);
        let t = crate::viewer::now_ms();
        let mut b = vec![];
        for _ in 0..n {
            b = to_bytes(&m);
        }
        let write = (crate::viewer::now_ms() - t) / n as f64;
        let t = crate::viewer::now_ms();
        let mut ok = true;
        for _ in 0..n {
            ok &= from_bytes::<ToPage>(&b).is_some();
        }
        let read = (crate::viewer::now_ms() - t) / n as f64;
        format!(
            "{{\"bytes\":{},\"write_ms\":{write},\"read_ms\":{read},\"ok\":{ok}}}",
            b.len()
        )
    }

    /// What crossed so far, as JSON: messages and bytes each way, the largest, the ms spent
    /// writing and reading them on the page.
    #[wasm_bindgen]
    pub fn app_build_cost() -> String {
        COST.with(|c| {
            let c = c.borrow();
            format!(
                "{{\"sent\":{},\"sent_bytes\":{},\"send_ms\":{},\"got\":{},\"got_bytes\":{},\"read_ms\":{},\"largest\":{}}}",
                c.sent, c.sent_bytes, c.send_ms, c.got, c.got_bytes, c.read_ms, c.largest
            )
        })
    }
}

/// The build worker's end (web/build.js): each session's `Worker`, stepped one batch of work at a
/// time with the worker's event loop between, so a newer message (a text, a file) is read before
/// the next.
#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
pub mod worker {
    use super::{JobW, ToPage, ToWorker};
    use crate::fg::{Gpu, Msg, Reads, Until, Work, Worker};
    use crate::gui::gui_inspect::{
        Answer, Ask, InspectData, Job, Meshes, Told, handle, tell, thermo_tell, wall_found,
    };
    use crate::wire::{from_bytes, to_bytes};
    use forge_mesh::TriMesh;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// The worker's (web/build.js): bytes to the page (copied, then transferred).
        #[wasm_bindgen(js_namespace = globalThis, js_name = forge_build_post)]
        fn post(bytes: &[u8]);
        /// The worker's: `build_worker_step` again once its event loop has read what waits.
        #[wasm_bindgen(js_namespace = globalThis, js_name = forge_build_soon)]
        fn soon();
        #[wasm_bindgen(js_namespace = performance, js_name = now)]
        fn perf_now() -> f64;
        #[wasm_bindgen(js_namespace = Date, js_name = now)]
        fn date_now() -> f64;
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        fn console_error(s: &str);
    }

    /// One session: its worker, its queue, what it sends, how to ask it for a new view.
    struct Doc {
        w: Worker,
        to: Sender<Work>,
        jobs: Receiver<Work>,
        out: Receiver<Msg>,
        gpus: Sender<Gpu>,
        until: Until,
        deps: Reads,
        told: Vec<std::path::PathBuf>,
        /// Work came since its last step. (A step takes all that waits, so after one the queue is
        /// empty: what's left is in `w.next`.)
        fresh: bool,
    }

    thread_local! {
        static DOCS: RefCell<BTreeMap<u32, Doc>> = const { RefCell::new(BTreeMap::new()) };
        /// The session stepped last: the next goes to the one after (each document in turn).
        static LAST: Cell<u32> = const { Cell::new(0) };
        /// A file the page sent being put: not sent back.
        static FROM_PAGE: Cell<bool> = const { Cell::new(false) };
        static ASKED: Cell<bool> = const { Cell::new(false) };
        /// The last builds' Inspect data (with their bodies), by number: what the page asks of.
        static BUILDS: RefCell<Vec<Arc<InspectData>>> = const { RefCell::new(Vec::new()) };
        /// The last builds' thermoform drawings (with their bodies and wall models), by key.
        static THERMOS: RefCell<Vec<crate::thermo_view::Gfx>> = const { RefCell::new(Vec::new()) };
        static MESHES: RefCell<Meshes> = RefCell::new(Meshes::default());
        static WALK: RefCell<Option<Walk>> = const { RefCell::new(None) };
    }

    /// The wall's search being stepped: its id, the search, where its part stands.
    type Walk = (u64, forge_wall::WallSearch<Arc<TriMesh>>, forge_math::Frame);

    /// How many builds' bodies are kept for the page's questions.
    const KEPT: usize = 6;

    /// A build's Inspect data kept (bodies and all) as its message goes to the page.
    fn keep(m: &Msg) {
        let Msg::Built(b) = m else { return };
        THERMOS.with(|all| {
            let mut all = all.borrow_mut();
            for (_, g) in &b.thermos {
                if !all.iter().any(|x| x.key == g.key) {
                    all.push(g.clone());
                }
            }
            let n = all.len();
            if n > KEPT {
                all.drain(..n - KEPT);
            }
        });
        for d in b.inspect.iter().chain(b.asm.iter().map(|a| &a.inspect)) {
            BUILDS.with(|all| {
                let mut all = all.borrow_mut();
                if !all.iter().any(|x| x.number == d.number) {
                    all.push(d.clone());
                    let n = all.len();
                    if n > KEPT {
                        all.drain(..n - KEPT);
                    }
                }
                let kept: Vec<&InspectData> = all.iter().map(|d| &**d).collect();
                MESHES.with(|m| m.borrow_mut().keep(&kept));
            });
        }
    }

    fn build(number: u64) -> Option<Arc<InspectData>> {
        BUILDS.with(|all| all.borrow().iter().find(|d| d.number == number).cloned())
    }

    /// An Inspect job: answered now, but the wall's search, stepped between other work.
    fn job(number: u64, j: JobW) {
        let Some(d) = build(number) else {
            // a build no longer kept: its answers are of no use
            return;
        };
        let j = match j {
            JobW::Wall(id, pl) => {
                let r = MESHES.with(|m| {
                    let (b, at) = d.body_at(pl).ok_or("the part isn't there".to_string())?;
                    let mesh = m.borrow_mut().of(b)?;
                    Ok::<_, String>((forge_wall::WallSearch::new(b, mesh), at))
                });
                match r {
                    Ok((w, at)) => {
                        WALK.with(|k| *k.borrow_mut() = Some((id, w, at)));
                        later();
                    }
                    Err(e) => send(&ToPage::Answer(Answer::Wall(id, Err(e)))),
                }
                return;
            }
            JobW::Measure(id, t) => Job::Measure(id, t),
            JobW::Warm => Job::Warm,
            JobW::Physical => Job::Physical,
        };
        if let Some(a) = MESHES.with(|m| handle(&d, j, &mut m.borrow_mut())) {
            send(&ToPage::Answer(a));
        }
    }

    /// An answer as it can cross: a projected edge whose curve is the kernel's own (a blend's, an
    /// intersection's) can't, and is refused in words.
    fn crossing(t: Told) -> Told {
        use crate::shell::sketching::rows::Src;
        use forge_sketch::project::Source;
        match t {
            Told::Project(Ok(refs))
                if refs.iter().flat_map(|r| &r.1.sources).any(|s| {
                    matches!(&s.1, Src::Model(Source::Edge { curve, .. }) if !crate::wire::crossable(curve))
                }) =>
            {
                Told::Project(Err(
                    "Project Geometry can't project this edge in the browser yet: its curve comes \
                     from a blend or an intersection. The desktop app can."
                        .into(),
                ))
            }
            t => t,
        }
    }

    /// A stretch of the wall's search (a few ms); its answer once found. True while it goes on.
    fn walk() -> bool {
        WALK.with(|k| {
            let mut k = k.borrow_mut();
            let Some((id, w, at)) = &mut *k else { return false };
            let t0 = perf_now();
            let mut finished = false;
            while !finished && perf_now() - t0 < 20.0 {
                finished = w.step(4096);
            }
            if finished {
                send(&ToPage::Answer(Answer::Wall(*id, wall_found(w.found(), *at))));
                *k = None;
                false
            } else {
                send(&ToPage::WallDone(*id, (w.done() * 1000.0) as u32));
                true
            }
        })
    }

    fn send(m: &ToPage) {
        post(&to_bytes(m));
    }

    /// Steps once the worker's event loop has read what waits (at most one ask in flight).
    fn later() {
        if !ASKED.with(|a| a.replace(true)) {
            soon();
        }
    }

    /// The worker's files: each change made here (a kept cache) to the page.
    fn tell_page(path: &std::path::Path, c: forge_dirs::fs::mem::Change<'_>) {
        if FROM_PAGE.with(Cell::get) {
            return;
        }
        let bytes = match c {
            forge_dirs::fs::mem::Change::Wrote(b) => Some(b.to_vec()),
            forge_dirs::fs::mem::Change::Removed => None,
        };
        send(&ToPage::File(path.to_string_lossy().into_owned(), bytes));
    }

    /// Set up once, before any message: the clock, the panic hook, the files' hook.
    #[wasm_bindgen]
    pub fn build_worker_init() {
        std::panic::set_hook(Box::new(|p| {
            console_error(&format!("forge build worker panicked: {p}"))
        }));
        forge_math::clock::set(perf_now, date_now);
        forge_dirs::fs::mem::set_hook(Some(tell_page));
    }

    /// A message from the page.
    #[wasm_bindgen]
    pub fn build_worker_msg(bytes: &[u8]) {
        let Some(m) = from_bytes::<ToWorker>(bytes) else {
            console_error("forge build worker: a message it could not read");
            return;
        };
        match m {
            ToWorker::Open { id, dir, file, gpu } => {
                let (to, jobs) = channel();
                let (gpus, gpu_rx) = channel();
                let (tx, out) = channel();
                let deps = Reads::default();
                let until: Until = Arc::new(std::sync::Mutex::new(None));
                let w = Worker::new(
                    (dir, file, deps.clone()),
                    gpu_rx,
                    tx,
                    gpu.then_some(Gpu),
                    (Arc::new(|| {}), Arc::new(AtomicBool::new(false)), until.clone()),
                );
                let d = Doc {
                    w,
                    to,
                    jobs,
                    out,
                    gpus,
                    until,
                    deps,
                    told: vec![],
                    fresh: false,
                };
                DOCS.with(|ds| ds.borrow_mut().insert(id, d));
            }
            ToWorker::Work(id, work) => {
                DOCS.with(|ds| {
                    if let Some(d) = ds.borrow_mut().get_mut(&id) {
                        let _ = d.to.send(work);
                        d.fresh = true;
                    }
                });
                later();
            }
            ToWorker::Gpu(id) => DOCS.with(|ds| {
                if let Some(d) = ds.borrow().get(&id) {
                    let _ = d.gpus.send(Gpu);
                }
            }),
            ToWorker::Until(id, upto) => DOCS.with(|ds| {
                if let Some(d) = ds.borrow().get(&id) {
                    *d.until.lock().unwrap_or_else(|e| e.into_inner()) = upto;
                }
            }),
            ToWorker::Close(id) => {
                DOCS.with(|ds| ds.borrow_mut().remove(&id));
            }
            ToWorker::File(path, bytes) => {
                FROM_PAGE.with(|f| f.set(true));
                let _ = match bytes {
                    Some(b) => forge_dirs::fs::write(&path, b),
                    None if forge_dirs::fs::is_dir(&path) => forge_dirs::fs::remove_dir_all(&path),
                    None => forge_dirs::fs::remove_file(&path),
                };
                FROM_PAGE.with(|f| f.set(false));
            }
            ToWorker::Load(path, bytes) => forge_dirs::fs::mem::load(&path, bytes),
            ToWorker::Unload(prefix) => forge_dirs::fs::mem::unload(&prefix),
            ToWorker::Job(number, j) => job(number, j),
            ToWorker::Ask(number, a) => {
                let t = match &a {
                    Ask::ThermoSim(k) | Ask::ThermoHeat(k, _) | Ask::ThermoExport(k, ..) => THERMOS.with(|all| {
                        let all = all.borrow();
                        thermo_tell(all.iter().find(|g| g.key == *k), &a)
                    }),
                    _ => match build(number) {
                        Some(d) => tell(&d, &a),
                        None => tell(&InspectData::default(), &a),
                    },
                };
                let t = crossing(t);
                send(&ToPage::Answer(Answer::Told(number, a, t)));
            }
            ToWorker::StopWall(id) => WALK.with(|k| {
                let mut k = k.borrow_mut();
                if k.as_ref().is_some_and(|w| w.0 == id) {
                    *k = None;
                }
            }),
        }
    }

    /// One batch of one session's work (the next session with work after the last stepped), its
    /// messages to the page; asks to be called again while any session has work waiting.
    #[wasm_bindgen]
    pub fn build_worker_step() {
        ASKED.with(|a| a.set(false));
        // the wall's search a stretch at a time, the documents' work between
        if walk() {
            later();
        }
        let last = LAST.with(Cell::get);
        // a session's worker is taken out while it steps: a message read meanwhile can't reach it
        // (none is: the step runs to its end before the event loop reads another)
        let next = DOCS.with(|ds| {
            let ds = ds.borrow();
            let has = |d: &Doc| d.fresh || d.w.next.is_some();
            ds.range(last + 1..)
                .chain(ds.range(..=last))
                .find(|(_, d)| has(d))
                .map(|(&id, _)| id)
        });
        let Some(id) = next else { return };
        LAST.with(|l| l.set(id));
        let Some(mut d) = DOCS.with(|ds| ds.borrow_mut().remove(&id)) else {
            return;
        };
        d.fresh = false;
        let first = d.w.next.take().or_else(|| d.jobs.try_recv().ok());
        if let Some(first) = first {
            d.w.step(first, &d.jobs);
        }
        for m in d.out.try_iter() {
            keep(&m);
            send(&ToPage::Msg(id, m));
        }
        let read = d.deps.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if read != d.told {
            d.told = read.clone();
            send(&ToPage::Read(id, read));
        }
        DOCS.with(|ds| ds.borrow_mut().insert(id, d));
        if DOCS.with(|ds| ds.borrow().values().any(|d| d.fresh || d.w.next.is_some())) {
            later();
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::wire::{from_bytes, to_bytes};

    #[test]
    fn protocol() {
        let all = vec![
            ToWorker::Open {
                id: 3,
                dir: Some("/project/a".into()),
                file: Some("/project/a/b.fg".into()),
                gpu: true,
            },
            ToWorker::Work(3, Work::Text((7, "x = 1".into(), true))),
            ToWorker::Work(3, Work::Drive(8, vec![("j".into(), "angle".into(), 0.5)])),
            ToWorker::Gpu(3),
            ToWorker::Until(3, Some("boss".into())),
            ToWorker::Close(3),
            ToWorker::File("/project/a/c.fg".into(), None),
            ToWorker::Load("/project/a/d.fg".into(), vec![1, 2, 3]),
            ToWorker::Unload("/project".into()),
        ];
        for m in &all {
            let b = to_bytes(m);
            let back: ToWorker = from_bytes(&b).expect("reads back");
            assert_eq!(to_bytes(&back), b);
        }
        let page = ToPage::Read(2, vec!["/project/x.fg".into()]);
        let b = to_bytes(&page);
        assert_eq!(to_bytes(&from_bytes::<ToPage>(&b).expect("reads back")), b);
    }

    /// Whole messages of native sessions (no view: a native session's is the GPU's) read back as
    /// written.
    #[test]
    fn messages() {
        for (f, checks) in [
            ("demo/sheet_tools.fg", false),
            ("thermoform/tray.fg", false),
            ("rack18u/hw/tests/hinge_pin.fg", true),
            ("harness/robot.fg", true),
        ] {
            let path = format!("{}/../../examples/{f}", env!("CARGO_MANIFEST_DIR"));
            let mut s =
                super::super::Session::open(&path, None, std::sync::Arc::new(|| {})).expect("the example");
            let want = if checks { s.check_overlap() } else { s.seq };
            let t = std::time::Instant::now();
            let mut n = 0;
            while s.done < want || s.whole < s.changed {
                assert!(t.elapsed().as_secs() < 600, "{f} built in time");
                for m in s.poll() {
                    let m = ToPage::Msg(1, m);
                    let b = to_bytes(&m);
                    let back: ToPage = from_bytes(&b).expect("reads back");
                    assert_eq!(to_bytes(&back), b, "{f}");
                    n += 1;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(n > 0, "{f}");
        }
    }
}
