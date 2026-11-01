//! The live loop: a worker thread owns the part and rebuilds it from what changed, always to the
//! newest value asked for; a value overtaken before it started is dropped. The UI thread only
//! swaps a value into a slot and collects what is ready, so it never waits on the kernel.

use crate::handles::{Facts, facts, facts_at};
use crate::model::Model;
use crate::tools::{Call, Done, Part};
use forge_doc::{ItemId, Op};
use forge_topo::Body;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

/// Why the worker is asking for meshes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Before any value: everything, so the first frame of a drag is as quick as the rest.
    Warm,
    Drag,
    /// The value was committed: the part at full quality.
    Committed,
}

/// What came of one value.
#[derive(Debug)]
pub struct Frame<T> {
    /// The `drag` it answers; 0 before any.
    pub seq: u64,
    /// The result: none if this value does not build (the last good one stays on screen).
    pub out: Option<T>,
    pub failure: Option<String>,
    /// Rebuilding from the changed feature (parsing, regenerating).
    pub rebuild_ms: f64,
    /// Of that, the kernel's share; the rest read lines and resolved references.
    pub kernel_ms: f64,
    /// Lines rebuilt.
    pub lines: usize,
    /// Joining the bodies into the one that is drawn.
    pub finish_ms: f64,
    pub mesh_ms: f64,
}

#[derive(Debug)]
pub enum Event<T> {
    Frame(Frame<T>),
    /// The commit's answer: what it did and its `.fc` line.
    Done(Done),
    /// The committed part's mesh, when it comes.
    Final(T),
    /// The document after a commit, and at the start: what a UI's tree and panels read.
    Doc(DocState),
    /// Which feature made each face and each feature's number, once the part stands.
    Facts(Box<Facts>),
}

/// What a UI needs of the part it can't ask the worker for: it parses `src` itself.
#[derive(Clone, Debug, PartialEq)]
pub struct DocState {
    pub src: String,
    pub version: u64,
    /// The item that failed to build, and why.
    pub failed: Option<(u64, String)>,
}

fn doc_state(part: &Part) -> DocState {
    let m = &part.model;
    DocState {
        src: m.src().to_string(),
        version: part.version(),
        failed: m.error.as_ref().map(|f| (m.lines[f.at].id, f.msg.clone())),
    }
}

enum Job {
    Commit(Op),
    Cancel,
    Calls(Vec<Call>),
    /// Look at the part as of this item, or all of it.
    Roll(Option<ItemId>),
    /// A dialog's values, built and shown with nothing committed; a new one replaces it.
    Preview(u64, Vec<Call>),
    PreviewEnd,
}

struct Slot<T> {
    /// The newest value, not yet started.
    want: Option<(u64, Op)>,
    jobs: VecDeque<Job>,
    ready: VecDeque<Event<T>>,
    /// Values overtaken before they started.
    dropped: u64,
    stop: bool,
}

struct Shared<T> {
    slot: Mutex<Slot<T>>,
    wake: Condvar,
    /// Called when something is ready, so a UI can sleep till then.
    ready: Mutex<Option<Waker>>,
}

type Waker = Box<dyn Fn() + Send>;

/// The UI's end of the loop.
pub struct Live<T> {
    shared: Arc<Shared<T>>,
    seq: u64,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl<T: Send + 'static> Live<T> {
    /// Starts the worker on this part; `mesher` turns a built body into whatever the screen needs.
    pub fn start(part: Part, mut mesher: impl FnMut(&Body, Stage) -> T + Send + 'static) -> Live<T> {
        let shared = Arc::new(Shared {
            slot: Mutex::new(Slot {
                want: None,
                jobs: VecDeque::new(),
                ready: VecDeque::new(),
                dropped: 0,
                stop: false,
            }),
            wake: Condvar::new(),
            ready: Mutex::new(None),
        });
        let theirs = shared.clone();
        let worker = std::thread::Builder::new()
            .name("live".into())
            .spawn(move || run(part, &mut mesher, &theirs))
            .expect("a thread");
        Live {
            shared,
            seq: 0,
            worker: Some(worker),
        }
    }

    /// Asks for this value. Returns at once; its number comes back on its frame, unless a newer
    /// value overtook it.
    pub fn drag(&mut self, op: Op) -> u64 {
        self.seq += 1;
        let mut s = self.shared.slot.lock().expect("the worker is alive");
        if s.want.replace((self.seq, op)).is_some() {
            s.dropped += 1;
        }
        drop(s);
        self.shared.wake.notify_one();
        self.seq
    }

    /// Ends the drag on this value.
    pub fn commit(&mut self, op: Op) {
        self.job(Job::Commit(op));
    }

    /// Ends the drag where it started.
    pub fn cancel(&mut self) {
        self.job(Job::Cancel);
    }

    /// Commits tool calls, in turn with the rest.
    pub fn call(&mut self, calls: Vec<Call>) {
        self.job(Job::Calls(calls));
    }

    /// Builds and shows these calls with nothing committed. Answers as a frame, its number given
    /// back. A `call` then commits, from where the preview began; `preview_end` goes back there.
    pub fn preview(&mut self, calls: Vec<Call>) -> u64 {
        self.seq += 1;
        let job = Job::Preview(self.seq, calls);
        let mut s = self.shared.slot.lock().expect("the worker is alive");
        if matches!(s.jobs.back(), Some(Job::Preview(..))) {
            s.jobs.pop_back();
            s.dropped += 1;
        }
        s.jobs.push_back(job);
        drop(s);
        self.shared.wake.notify_one();
        self.seq
    }

    pub fn preview_end(&mut self) {
        self.job(Job::PreviewEnd);
    }

    /// Shows the part as of this item (its end-of-part marker), or all of it. It stays so through
    /// commits, till told otherwise.
    pub fn roll(&mut self, to: Option<ItemId>) {
        self.job(Job::Roll(to));
    }

    /// `f` is called whenever something becomes ready. Whatever is already there, it's called for now.
    pub fn on_ready(&mut self, f: impl Fn() + Send + 'static) {
        f();
        *self.shared.ready.lock().expect("the worker is alive") = Some(Box::new(f));
    }

    fn job(&mut self, j: Job) {
        let mut s = self.shared.slot.lock().expect("the worker is alive");
        // a commit overtakes any value still waiting
        if s.want.take().is_some() {
            s.dropped += 1;
        }
        s.jobs.push_back(j);
        drop(s);
        self.shared.wake.notify_one();
    }

    /// What is ready, oldest first.
    pub fn poll(&mut self) -> Vec<Event<T>> {
        let mut s = self.shared.slot.lock().expect("the worker is alive");
        s.ready.drain(..).collect()
    }

    /// Values overtaken so far.
    pub fn dropped(&self) -> u64 {
        self.shared.slot.lock().expect("the worker is alive").dropped
    }
}

impl<T> Drop for Live<T> {
    fn drop(&mut self) {
        if let Ok(mut s) = self.shared.slot.lock() {
            s.stop = true;
        }
        self.shared.wake.notify_one();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// A part the kernel can't say these of still stands, with nothing to point at.
fn say_facts<T>(part: &mut Part, lines: Option<usize>, say: &impl Fn(Event<T>)) {
    let f = catch_unwind(AssertUnwindSafe(|| match lines {
        Some(n) => facts_at(&part.model, n),
        None => facts(&mut part.model),
    }))
    .unwrap_or_default();
    say(Event::Facts(Box::new(f)));
}

/// How many lines to show, for an end-of-part at this item.
fn lines_to(part: &Part, to: Option<ItemId>) -> Option<usize> {
    let id = to?;
    let k = part.model.lines.iter().position(|l| l.id == id.0)?;
    Some(k + 1)
}

enum Next {
    Value(u64, Op),
    Job(Job),
    Stop,
}

fn run<T>(mut part: Part, mesher: &mut impl FnMut(&Body, Stage) -> T, shared: &Shared<T>) {
    let say = |e: Event<T>| {
        shared.slot.lock().expect("the UI is alive").ready.push_back(e);
        if let Some(f) = &*shared.ready.lock().expect("the UI is alive") {
            f();
        }
    };
    say(Event::Doc(doc_state(&part)));
    // the last text that built, to go back to if the kernel panics on a value
    let mut good = part.model.src().to_string();
    // the last value that built, for a drag that ends on one that doesn't
    let mut last_good: Option<Op> = None;
    let t = Instant::now();
    if let Some(b) = part.model.body() {
        let out = mesher(b, Stage::Warm);
        say(Event::Frame(Frame {
            seq: 0,
            out: Some(out),
            failure: None,
            rebuild_ms: 0.0,
            kernel_ms: 0.0,
            lines: 0,
            finish_ms: 0.0,
            mesh_ms: t.elapsed().as_secs_f64() * 1e3,
        }));
        say_facts(&mut part, None, &say);
    }
    let mut rolled: Option<ItemId> = None;
    let mut before_preview: Option<crate::model::Saved> = None;
    loop {
        let next = {
            let mut s = shared.slot.lock().expect("the UI is alive");
            loop {
                if s.stop {
                    break Next::Stop;
                }
                if let Some(j) = s.jobs.pop_front() {
                    break Next::Job(j);
                }
                if let Some((seq, op)) = s.want.take() {
                    break Next::Value(seq, op);
                }
                s = shared.wake.wait(s).expect("the UI is alive");
            }
        };
        match next {
            Next::Stop => return,
            Next::Job(j) => {
                if let Job::Preview(seq, calls) = j {
                    let t = Instant::now();
                    let base = before_preview.take().unwrap_or_else(|| part.model.save());
                    part.model.restore(base);
                    before_preview = Some(part.model.save());
                    let tried = catch_unwind(AssertUnwindSafe(|| part.preview(calls)));
                    let failure = match tried {
                        Ok(Ok(())) => None,
                        Ok(Err(e)) => Some(e),
                        Err(_) => {
                            let fonts = part.model.fonts.take();
                            part.model = Model::default();
                            part.model.fonts = fonts;
                            part.model.update(&good);
                            before_preview = None;
                            Some("the kernel panicked on these values".into())
                        }
                    };
                    let rebuild_ms = t.elapsed().as_secs_f64() * 1e3;
                    let mut frame = Frame {
                        seq,
                        out: None,
                        failure,
                        rebuild_ms,
                        kernel_ms: part.model.kernel_ms,
                        lines: part.model.rebuilt,
                        finish_ms: 0.0,
                        mesh_ms: 0.0,
                    };
                    if frame.failure.is_none()
                        && let Some(b) = part.model.body()
                    {
                        let t = Instant::now();
                        match catch_unwind(AssertUnwindSafe(|| mesher(b, Stage::Drag))) {
                            Ok(out) => frame.out = Some(out),
                            Err(_) => frame.failure = Some("the mesher panicked on these values".into()),
                        }
                        frame.mesh_ms = t.elapsed().as_secs_f64() * 1e3;
                    }
                    say(Event::Frame(frame));
                    continue;
                }
                if let Job::PreviewEnd = j {
                    if let Some(base) = before_preview.take() {
                        part.model.restore(base);
                    }
                    if let Some(b) = part.model.body() {
                        let out = mesher(b, Stage::Committed);
                        say(Event::Final(out));
                    }
                    continue;
                }
                if let Some(base) = before_preview.take() {
                    part.model.restore(base);
                }
                if let Job::Roll(to) = j {
                    rolled = to;
                    let lines = lines_to(&part, rolled);
                    let m = &part.model;
                    let body = match lines {
                        Some(n) => m.body_after(n),
                        None => m.body(),
                    };
                    if let Some(b) = body {
                        let out = mesher(b, Stage::Committed);
                        say(Event::Final(out));
                    }
                    say_facts(&mut part, lines, &say);
                    continue;
                }
                let d = match j {
                    Job::Roll(_) | Job::Preview(..) | Job::PreviewEnd => unreachable!("taken above"),
                    Job::Commit(op) => part.drag_commit(op, last_good.take()),
                    Job::Cancel => {
                        last_good = None;
                        part.drag_cancel();
                        Done {
                            ok: true,
                            text: "cancelled".into(),
                            lines: Vec::new(),
                            version: part.version(),
                        }
                    }
                    Job::Calls(calls) => {
                        last_good = None;
                        match catch_unwind(AssertUnwindSafe(|| part.call(calls, None))) {
                            Ok(d) => d,
                            Err(_) => {
                                let fonts = part.model.fonts.take();
                                part.model = Model::default();
                                part.model.fonts = fonts;
                                part.model.update(&good);
                                Done {
                                    ok: false,
                                    text: "error: the kernel panicked; nothing applied".into(),
                                    lines: Vec::new(),
                                    version: part.version(),
                                }
                            }
                        }
                    }
                };
                good = part.model.src().to_string();
                say(Event::Done(d));
                say(Event::Doc(doc_state(&part)));
                let lines = lines_to(&part, rolled);
                let m = &part.model;
                let body = match lines {
                    Some(n) => m.body_after(n),
                    None => m.body(),
                };
                if let Some(b) = body {
                    let out = mesher(b, Stage::Committed);
                    say(Event::Final(out));
                    say_facts(&mut part, lines, &say);
                }
            }
            Next::Value(seq, op) => {
                let t = Instant::now();
                let built = catch_unwind(AssertUnwindSafe(|| part.drag(op.clone())));
                let rebuild_ms = t.elapsed().as_secs_f64() * 1e3;
                let failure = match built {
                    Ok(Ok(())) => part.model.error.as_ref().map(|f| f.msg.clone()),
                    Ok(Err(e)) => Some(e),
                    Err(_) => {
                        // the kernel panicked on this value: the part as it last built
                        let fonts = part.model.fonts.take();
                        part.model = Model::default();
                        part.model.fonts = fonts;
                        part.model.update(&good);
                        Some("the kernel panicked on this value".into())
                    }
                };
                let mut frame = Frame {
                    seq,
                    out: None,
                    failure,
                    rebuild_ms,
                    kernel_ms: part.model.kernel_ms,
                    lines: part.model.rebuilt,
                    finish_ms: 0.0,
                    mesh_ms: 0.0,
                };
                if frame.failure.is_none() {
                    last_good = Some(op);
                    good = part.model.src().to_string();
                    let t = Instant::now();
                    if let Some(b) = part.model.body() {
                        frame.finish_ms = t.elapsed().as_secs_f64() * 1e3;
                        let t = Instant::now();
                        let out = catch_unwind(AssertUnwindSafe(|| mesher(b, Stage::Drag)));
                        frame.mesh_ms = t.elapsed().as_secs_f64() * 1e3;
                        match out {
                            Ok(out) => frame.out = Some(out),
                            Err(_) => frame.failure = Some("the mesher panicked on this value".into()),
                        }
                    }
                }
                say(Event::Frame(frame));
            }
        }
    }
}

// wasm32-wasip1 has no threads
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::tools::Target;

    fn wait<T: Send + 'static>(l: &mut Live<T>, done: impl Fn(&Event<T>) -> bool) -> Vec<Event<T>> {
        let mut got = Vec::new();
        for _ in 0..20_000 {
            let new = l.poll();
            let end = new.iter().any(&done);
            got.extend(new);
            if end {
                return got;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("the worker never answered");
    }

    fn start() -> (Live<f64>, Target) {
        let p = Part::new("w = 40\nbase: extrude 10 rect 0 0 w 30\n");
        let w = p.dims()[0].at;
        let live = Live::start(p, |b, _| {
            forge_topo::mass::mass(b).map(|m| m.volume).unwrap_or(0.0)
        });
        (live, w)
    }

    #[test]
    fn calls_go_through_the_worker_and_say_what_the_document_is() {
        let (mut l, _) = start();
        let mut waker = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let w2 = waker.clone();
        l.on_ready(move || {
            w2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        let got = wait(&mut l, |e| matches!(e, Event::Doc(_)));
        let Some(Event::Doc(d)) = got.iter().find(|e| matches!(e, Event::Doc(_))) else {
            panic!("a doc state")
        };
        assert!(d.src.contains("base: extrude"));
        let id = forge_doc::Document::parse(&d.src).find("base").unwrap();
        l.call(vec![Call::SetArg {
            item: id,
            arg: 0,
            text: "12".into(),
        }]);
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        assert!(got.iter().any(|e| matches!(e, Event::Done(d) if d.ok)));
        let doc = got.iter().find_map(|e| match e {
            Event::Doc(d) => Some(d),
            _ => None,
        });
        assert!(doc.unwrap().src.contains("base: extrude 12"));
        assert!(std::sync::Arc::get_mut(&mut waker).is_none());
        assert!(waker.load(std::sync::atomic::Ordering::Relaxed) >= 2);
    }

    #[test]
    fn rolled_back_the_part_is_as_of_that_line_through_commits_till_told_otherwise() {
        let p = Part::new("w = 40\nbase: extrude 10 rect 0 0 w 30\ntop: extrude 5 at 0 0 10 rect 0 0 w 30\n");
        let base = p.model.doc.find("base").unwrap();
        let mut l = Live::start(p, |b, _| {
            forge_topo::mass::mass(b).map(|m| m.volume).unwrap_or(0.0)
        });
        wait(&mut l, |e| matches!(e, Event::Facts(_)));
        l.roll(Some(base));
        let got = wait(&mut l, |e| matches!(e, Event::Facts(_)));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Final(v) if *v == 40.0 * 30.0 * 10.0))
        );
        l.roll(None);
        let got = wait(&mut l, |e| matches!(e, Event::Facts(_)));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Final(v) if *v == 40.0 * 30.0 * 15.0))
        );
    }

    #[test]
    fn a_preview_shows_with_nothing_committed_and_a_call_commits_from_the_start() {
        let (mut l, _) = start();
        wait(&mut l, |e| matches!(e, Event::Facts(_)));
        let doc = forge_doc::Document::parse("w = 40\nbase: extrude 10 rect 0 0 w 30\n");
        let id = doc.find("base").unwrap();
        let call = |t: &str| Call::SetArg {
            item: id,
            arg: 0,
            text: t.into(),
        };
        // the newest wins, and the part shown is that one's
        let s = l.preview(vec![call("20")]);
        let got = wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Frame(f) if f.seq == s && f.out == Some(40.0 * 30.0 * 20.0)))
        );
        let s = l.preview(vec![call("nope")]);
        let got = wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Frame(f) if f.seq == s && f.failure.is_some()))
        );
        // ended, it's as it was
        l.preview_end();
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Final(v) if *v == 40.0 * 30.0 * 10.0))
        );
        // previewed then called: one commit, from the original
        l.preview(vec![call("25")]);
        l.call(vec![call("30")]);
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Final(v) if *v == 40.0 * 30.0 * 30.0))
        );
        assert!(
            got.iter()
                .any(|e| matches!(e, Event::Done(d) if d.ok && d.lines == ["set base 1 30"]))
        );
    }

    #[test]
    fn a_commit_that_overtakes_every_value_still_commits() {
        let (mut l, w) = start();
        l.commit(w.op("50".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        assert!(got.iter().any(|e| matches!(e, Event::Done(d) if d.ok)));
        assert_eq!(
            got.iter().find_map(|e| match e {
                Event::Final(v) => Some(*v),
                _ => None,
            }),
            Some(50.0 * 30.0 * 10.0)
        );
    }

    #[test]
    fn the_newest_value_is_what_gets_built() {
        let (mut l, w) = start();
        let mut last = 0;
        for k in 0..50 {
            last = l.drag(w.op(format!("{}", 40 + k)));
        }
        let got = wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == last));
        let Some(Event::Frame(f)) = got.last() else {
            panic!("a frame")
        };
        assert_eq!(f.out, Some(89.0 * 30.0 * 10.0));
        // every value was built or dropped, none out of order
        let seqs: Vec<u64> = got
            .iter()
            .filter_map(|e| match e {
                Event::Frame(f) => Some(f.seq),
                _ => None,
            })
            .collect();
        assert!(seqs.windows(2).all(|x| x[0] < x[1]));
        assert_eq!(seqs.iter().filter(|&&s| s > 0).count() as u64 + l.dropped(), 50);
    }

    #[test]
    fn a_value_that_fails_leaves_the_last_good_part_and_a_commit_prints_back() {
        let (mut l, w) = start();
        let s = l.drag(w.op("nope".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        let Some(Event::Frame(f)) = got.last() else {
            panic!("a frame")
        };
        assert!(f.out.is_none() && f.failure.is_some(), "{f:?}");
        let s = l.drag(w.op("50".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        let Some(Event::Frame(f)) = got.last() else {
            panic!("a frame")
        };
        assert_eq!(f.out, Some(15000.0));
        l.commit(w.op("50".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        let done = got.iter().find_map(|e| match e {
            Event::Done(d) => Some(d),
            _ => None,
        });
        assert_eq!(done.expect("done").lines, ["w = 50"]);
    }

    #[test]
    fn a_commit_that_cannot_build_takes_the_last_that_did() {
        let (mut l, w) = start();
        let s = l.drag(w.op("55".into()));
        wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        l.commit(w.op("nope".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        let last = got.iter().rfind(|e| matches!(e, Event::Final(_))).expect("final");
        assert!(matches!(last, Event::Final(v) if *v == 16500.0), "{last:?}");
        // none did: where it started
        let s = l.drag(w.op("nope".into()));
        wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        l.commit(w.op("nope".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        let last = got.iter().rfind(|e| matches!(e, Event::Final(_))).expect("final");
        assert!(matches!(last, Event::Final(v) if *v == 16500.0), "{last:?}");
    }
}
