// The threaded build's threads, on the page and in every worker: `forge_thread` starts a Web
// Worker on the shared memory for one job; its wakes are passed to `wake`.
export function threadsOn(wake) {
  globalThis.forge_thread = (module, memory, id) => {
    const w = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
    w.onmessage = (e) => {
      if (e.data === "wake") wake();
    };
    w.postMessage([module, memory, id]);
  };
}
