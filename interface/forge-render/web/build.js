// The default build's build worker: a Web Worker with its own instance of the app's wasm (its own
// memory, nothing shared with the page), where each open document's builds run, so the page's
// thread never holds a build or a check (fg::remote). Its first message is the page's compiled
// module; every other is bytes either way (copied out of the wasm's memory, then transferred).
import init, { build_worker_init, build_worker_msg, build_worker_step } from "./forge_render.js";

let ready = null;
const waiting = [];
// the next step once the event loop has read what waits (a page's message goes first): a
// message, not a timer (a timer nested this deep waits 4 ms each time)
const soon = new MessageChannel();
soon.port1.onmessage = () => build_worker_step();
self.forge_build_soon = () => soon.port2.postMessage(0);
self.forge_build_post = (bytes) => {
  const b = bytes.slice();
  self.postMessage(b.buffer, [b.buffer]);
};
self.onmessage = async (e) => {
  if (ready === null) {
    // the page's module: compiled once, by the page
    ready = (async () => {
      await init({ module_or_path: e.data });
      build_worker_init();
      for (const b of waiting.splice(0)) build_worker_msg(b);
      ready = true;
    })().catch((err) => {
      console.error(`forge build worker failed to start: ${err?.stack ?? err}`);
      throw err;
    });
    return;
  }
  const b = new Uint8Array(e.data);
  if (ready === true) build_worker_msg(b);
  else waiting.push(b);
};
