//! What differs between the desktop and the browser, in one place: the clock, background work,
//! and (see `storage`) where files are kept. Everything above this is the same on both.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
/// std's clock panics in a browser; this one reads `performance.now()` there.
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

/// Runs `f` off the caller's thread where there are threads: natively a named thread; in the
/// threaded browser build a Web Worker on the page's memory (`threads`); in the single-threaded
/// browser build right away, before this returns. A job's result goes back through a channel
/// either way, so the caller reads it the same.
pub fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::Builder::new()
            .name(name.into())
            .spawn(f)
            .expect("a thread");
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    {
        let _ = name;
        threads::spawn(Box::new(f));
    }
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    {
        let _ = name;
        f();
    }
}

/// A lock taken. Natively (and single-threaded) std's; in the threaded browser build it spins on
/// `try_lock` and never waits: the page's main thread may not block (`memory.atomic.wait` traps
/// there), and the locks shared with a worker are held for a few instructions.
pub fn lock<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    loop {
        match m.try_lock() {
            Ok(g) => return g,
            Err(std::sync::TryLockError::Poisoned(e)) => return e.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
    #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The threaded browser build's threads: a job handed to a new Web Worker (web/worker.js) that
/// instantiates this same module on the page's shared memory and runs it (`worker_run`).
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub mod threads {
    use std::collections::VecDeque;
    use std::sync::{Condvar, Mutex};
    use wasm_bindgen::prelude::*;

    type Job = Box<dyn FnOnce() + Send>;

    /// The jobs waiting, how many workers wait for one, and the wakes handed out: a job pushed
    /// either reserves a waiting worker (a wake) or starts a new one, so a job that waits for
    /// jobs it spawned (a scope) always has a worker for them.
    struct Pool {
        jobs: VecDeque<Job>,
        idle: usize,
        wakes: usize,
    }

    static POOL: Mutex<Pool> = Mutex::new(Pool {
        jobs: VecDeque::new(),
        idle: 0,
        wakes: 0,
    });
    static READY: Condvar = Condvar::new();

    #[wasm_bindgen]
    extern "C" {
        /// The page's (and each worker's): a Web Worker on the shared memory, into the pool
        /// (web/threads.js, worker.js).
        #[wasm_bindgen(js_namespace = globalThis, js_name = forge_thread)]
        fn js_thread(module: JsValue, memory: JsValue, id: u32);
    }

    /// A job for the pool's workers. Never waits: the page's main thread calls it.
    pub fn spawn(f: Job) {
        let new_worker = {
            let mut p = super::lock(&POOL);
            p.jobs.push_back(f);
            if p.idle > 0 {
                p.idle -= 1;
                p.wakes += 1;
                READY.notify_one();
                false
            } else {
                true
            }
        };
        if new_worker {
            js_thread(wasm_bindgen::module(), wasm_bindgen::memory(), 0);
        }
    }

    /// A worker's life: the pool's jobs, one after another, waiting between them (a worker may).
    #[wasm_bindgen]
    pub fn worker_run(_id: u32) {
        let mut p = POOL.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(job) = p.jobs.pop_front() {
                drop(p);
                job();
                p = POOL.lock().unwrap_or_else(|e| e.into_inner());
                continue;
            }
            p.idle += 1;
            loop {
                p = READY.wait(p).unwrap_or_else(|e| e.into_inner());
                if p.wakes > 0 {
                    p.wakes -= 1;
                    break;
                }
            }
        }
    }
}

/// A file to open, by the platform's picker: natively the system's dialog; in the browser, one
/// of the project's files (see `storage`), none while the page's own picker is still to answer.
pub fn pick_file(title: Option<&str>, kind: &str, exts: &[&str]) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut d = rfd::FileDialog::new().add_filter(kind, exts);
        if let Some(t) = title {
            d = d.set_title(t);
        }
        d.pick_file().map(|p| p.to_string_lossy().into_owned())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (title, kind, exts);
        web::pick_pending()
    }
}

/// A small value kept between sessions (settings, an endpoint's key): natively in the config
/// directory; in the browser in the page's `localStorage`, under `forgecad:KEY`.
pub fn kv_get(key: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        forge_dirs::fs::read_to_string(forge_dirs::config()?.join("forgecad").join(key)).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::local()?.get_item(&format!("forgecad:{key}")).ok()?
    }
}

pub fn kv_set(key: &str, value: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(d) = forge_dirs::config().map(|c| c.join("forgecad")) {
            let _ = forge_dirs::fs::create_dir_all(&d);
            let _ = forge_dirs::fs::write(d.join(key), value);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web::local() {
            let _ = s.set_item(&format!("forgecad:{key}"), value);
        }
    }
}

pub fn kv_remove(key: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(d) = forge_dirs::config() {
            let _ = forge_dirs::fs::remove_file(d.join("forgecad").join(key));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web::local() {
            let _ = s.remove_item(&format!("forgecad:{key}"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub mod web {
    use std::cell::RefCell;

    thread_local! {
        /// What the page's picker gave, for the next `pick_file` to take.
        static PICKED: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    pub fn local() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn pick_pending() -> Option<String> {
        PICKED.with(|p| p.borrow_mut().take())
    }

    pub fn picked(path: String) {
        PICKED.with(|p| *p.borrow_mut() = Some(path));
    }
}

/// A long-lived worker fed jobs in order: `f` is given the first job waiting and the queue (to
/// take newer ones from), and says whether to go on. Natively a thread of its own; in the browser
/// each `send` runs it on what waits, before returning.
pub struct Server<J> {
    tx: std::sync::mpsc::Sender<J>,
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    run: std::rc::Rc<std::cell::RefCell<ServerRun<J>>>,
}

#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
type ServerRun<J> = (
    std::sync::mpsc::Receiver<J>,
    Box<dyn FnMut(J, &std::sync::mpsc::Receiver<J>) -> bool>,
    bool,
);

pub fn serve<J: Send + 'static>(
    name: &str,
    #[allow(unused_mut)] mut f: impl FnMut(J, &std::sync::mpsc::Receiver<J>) -> bool + Send + 'static,
) -> Server<J> {
    let (tx, rx) = std::sync::mpsc::channel::<J>();
    #[cfg(not(all(target_arch = "wasm32", not(target_feature = "atomics"))))]
    {
        spawn(name, move || {
            while let Ok(j) = rx.recv() {
                if !f(j, &rx) {
                    return;
                }
            }
        });
        Server { tx }
    }
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    {
        let _ = name;
        Server {
            tx,
            run: std::rc::Rc::new(std::cell::RefCell::new((rx, Box::new(f), true))),
        }
    }
}

impl<J> Server<J> {
    /// False once the worker has stopped.
    pub fn send(&self, j: J) -> bool {
        if self.tx.send(j).is_err() {
            return false;
        }
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        {
            // a send from inside the worker's own run is taken by that run
            let Ok(mut r) = self.run.try_borrow_mut() else {
                return true;
            };
            let (rx, f, live) = &mut *r;
            while *live && let Ok(j) = rx.try_recv() {
                *live = f(j, rx);
            }
            return *live;
        }
        #[allow(unreachable_code)]
        true
    }
}

/// Maps `f` over `items` on up to `threads` threads (natively; one after another in the
/// browser), the results in the items' order.
pub fn par_map<T: Sync, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let at = std::sync::atomic::AtomicUsize::new(0);
        let mut got: Vec<(usize, R)> = std::thread::scope(|s| {
            let workers: Vec<_> = (0..threads.max(1))
                .map(|_| {
                    s.spawn(|| {
                        let mut got = vec![];
                        loop {
                            let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(x) = items.get(i) else { return got };
                            got.push((i, f(x)));
                        }
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|w| w.join().expect("a worker panicked"))
                .collect()
        });
        got.sort_by_key(|x| x.0);
        got.into_iter().map(|x| x.1).collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = threads;
        items.iter().map(f).collect()
    }
}
