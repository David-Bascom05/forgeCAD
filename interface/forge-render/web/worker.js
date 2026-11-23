// A thread of the threaded browser build: a Web Worker on the page's shared memory, running one
// job the wasm handed over (`plat::threads::spawn` -> `worker_run`). Not used by the default,
// single-threaded build. A job's wake goes to whoever started this worker (the page, or the
// worker that started it, which passes it on).
import init, { worker_run } from "./forge_render.js";
import { threadsOn } from "./threads.js";

self.forge_wake_page = () => self.postMessage("wake");
self.onmessage = async (e) => {
  if (!Array.isArray(e.data)) return;
  const [module, memory, id] = e.data;
  const wasm = await init({ module_or_path: module, memory });
  threadsOn(() => self.forge_wake_page());
  worker_run(id);
  // the job is done: this thread's stack and storage back to the heap, then the worker gone
  wasm.__wbindgen_thread_destroy?.();
  close();
};
