// The threaded build's threads. Only the page starts a Web Worker: a worker that started one and
// then waited for its job (a scope joining) would hold the new one up, which needs the starter's
// own loop to start. So a worker asks whoever started it, and the ask goes up to the page.
// `forge_thread` starts a pool worker on the shared memory; its wakes are passed to `wake`.
export function threadsOn(wake) {
  if (typeof window === "undefined") {
    // a worker: up to the page
    globalThis.forge_thread = (module, memory, id) => self.postMessage(["thread", module, memory, id]);
    return;
  }
  const start = (module, memory, id) => {
    const w = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
    w.onmessage = (e) => {
      if (e.data === "wake") wake();
      else if (Array.isArray(e.data) && e.data[0] === "thread") start(e.data[1], e.data[2], e.data[3]);
    };
    w.postMessage([module, memory, id]);
  };
  globalThis.forge_thread = start;
}
