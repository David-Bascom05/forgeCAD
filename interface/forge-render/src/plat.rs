//! What differs between the desktop and the browser, in one place: the clock, background work,
//! and (see `storage`) where files are kept. Everything above this is the same on both.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
/// std's clock panics in a browser; this one reads `performance.now()` there.
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

/// Runs `f` off the caller's thread where there are threads: natively a named thread; in the
/// browser (one thread, no shared memory) right away, before this returns. A job's result goes
/// back through a channel either way, so the caller reads it the same.
pub fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::Builder::new()
            .name(name.into())
            .spawn(f)
            .expect("a thread");
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        f();
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
    #[cfg(target_arch = "wasm32")]
    run: std::rc::Rc<std::cell::RefCell<ServerRun<J>>>,
}

#[cfg(target_arch = "wasm32")]
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                while let Ok(j) = rx.recv() {
                    if !f(j, &rx) {
                        return;
                    }
                }
            })
            .expect("a thread");
        Server { tx }
    }
    #[cfg(target_arch = "wasm32")]
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
        #[cfg(target_arch = "wasm32")]
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
