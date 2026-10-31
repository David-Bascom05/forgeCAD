//! The live loop: a worker thread owns the part and rebuilds it from what changed, always to the
//! newest value asked for; a value overtaken before it started is dropped. The UI thread only
//! swaps a value into a slot and collects what is ready, so it never waits on the kernel.

use crate::model::Model;
use crate::tools::{Done, Part};
use forge_doc::Op;
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
}

enum Job {
    Commit(Op),
    Cancel,
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
}

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

enum Next {
    Value(u64, Op),
    Job(Job),
    Stop,
}

fn run<T>(mut part: Part, mesher: &mut impl FnMut(&Body, Stage) -> T, shared: &Shared<T>) {
    let say = |e: Event<T>| shared.slot.lock().expect("the UI is alive").ready.push_back(e);
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
    }
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
                let d = match j {
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
                };
                good = part.model.src().to_string();
                say(Event::Done(d));
                if let Some(b) = part.model.body() {
                    let out = mesher(b, Stage::Committed);
                    say(Event::Final(out));
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
        let last = got.last().expect("final");
        assert!(matches!(last, Event::Final(v) if *v == 16500.0), "{last:?}");
        // none did: where it started
        let s = l.drag(w.op("nope".into()));
        wait(&mut l, |e| matches!(e, Event::Frame(f) if f.seq == s));
        l.commit(w.op("nope".into()));
        let got = wait(&mut l, |e| matches!(e, Event::Final(_)));
        let last = got.last().expect("final");
        assert!(matches!(last, Event::Final(v) if *v == 16500.0), "{last:?}");
    }
}
