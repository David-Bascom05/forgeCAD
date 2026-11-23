// A thread of the threaded browser build: a Web Worker on the page's shared memory, running one
// job the page's wasm handed over (`plat::spawn_thread` -> `worker_run`). Never used by the
// default (single-threaded) build.
import init, { worker_run } from "./forge_render.js";

self.onmessage = async (e) => {
  const [module, memory, id] = e.data;
  await init({ module_or_path: module, memory });
  // the job's thread wakes the page through this worker's messages
  worker_run(id);
  close();
};
