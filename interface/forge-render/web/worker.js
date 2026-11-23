// A thread of the threaded browser build: a Web Worker on the page's shared memory, one of the
// pool's workers: it runs the jobs the wasm hands over (`plat::threads::spawn` -> `worker_run`,
// which never returns). Not used by the default, single-threaded build. A job's wake goes to
// whoever started this worker (the page, or the worker that started it, which passes it on).
import init, { worker_run } from "./forge_render.js";
import { threadsOn } from "./threads.js";

self.forge_wake_page = () => self.postMessage("wake");
self.onmessage = async (e) => {
  if (!Array.isArray(e.data)) return;
  const [module, memory, id] = e.data;
  try {
    await init({ module_or_path: module, memory });
    threadsOn(() => self.forge_wake_page());
    worker_run(id);
  } catch (err) {
    // the job it was started for waits on: the page must hear of it
    console.error(`forge worker died: ${err?.stack ?? err}`);
    throw err;
  }
};
