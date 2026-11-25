// The page stays live while a document builds and checks: in the default build its builds run on
// the build worker (a Web Worker with its own wasm memory), so no page task holds the page longer
// than Chrome's long-task line (50 ms) through an 18U rack's interference check, which holds the
// page's own thread for seconds (?worker=0). With the threaded artifact built (THREADS=1), that one
// too. Also measured and said: what a mesh message costs to cross (written and read in the wasm,
// copied out and in, transferred) at a part's typical and largest edits (p50 10.7 KB, p99 3.9 MB),
// and how long a small part's edit waits behind another document's long check (one build worker
// serves them all).
//   node web/tests/responsive.cjs OUT
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const RACK = "asm/rack.fg";
const RACK_EDIT = '{"op":"SetParam","param":"y0_front","value":{"value":51,"dim":"Length"}}';
const SMALL = "frame/base_pan.fg";
// the longest a page task may be on the build worker, ms (Chrome reports none shorter)
const LONGEST = Number(process.env.LONGEST_MS ?? 50);

/// A page watching its long tasks and the gaps between its frames.
async function watched(b, url) {
  return t.page(b, url, (p) =>
    p.addInitScript(() => {
      window.longest = 0;
      window.gap = 0;
      new PerformanceObserver((l) => {
        for (const e of l.getEntries()) window.longest = Math.max(window.longest, e.duration);
      }).observe({ type: "longtask", buffered: true });
      let last = 0;
      const tick = (now) => {
        if (last) window.gap = Math.max(window.gap, now - last);
        last = now;
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    }),
  );
}

async function click(p, name, listed = true) {
  if (listed) await t.until(p, (s) => s.places?.some((x) => x.name === name), name);
  await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), name);
  // a frame drawn after it (an idle page draws none unasked)
  await p.evaluate(() => window.forgecad.pixels());
  await p.evaluate(() => window.forgecad.pixels());
}

/// The rack opened, its interference check on, then an edit: how long the page's tasks and frame
/// gaps were through the edit's build and check.
async function rack(b, url, extra = "") {
  const p = await watched(b, `${url}?open=${RACK}&offscreen=1280x800${extra}`);
  await t.until(p, (s) => s.built?.checked, "the rack built");
  let st = await t.loaded(p);
  let seq = st.seq;
  await click(p, "tab:Inspect");
  await click(p, "tool:Analyze Interference");
  await click(p, "asm:check", false);
  st = await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked, "the interference check");
  await p.evaluate(() => {
    window.longest = 0;
    window.gap = 0;
  });
  seq = st.seq;
  const t0 = Date.now();
  await p.evaluate((e) => window.forgecad.fc.app_apply(e), RACK_EDIT);
  st = await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked && s.built.seq >= s.seq, "the edit checked");
  const r = {
    how: await p.evaluate(() => (window.forgecad.threaded ? "threaded" : window.forgecad.buildWorker ? "build worker" : "on the page")),
    ms: Date.now() - t0,
    build_ms: st.build_ms,
    built: st.built,
    longest: await p.evaluate(() => window.longest),
    gap: await p.evaluate(() => window.gap),
  };
  t.check(p);
  await p.close();
  return r;
}

/// What a mesh message costs to cross, at `sizes` (bytes of mesh): written and read in the wasm
/// (`app_build_bench`), copied out of the worker's memory and into the page's, and transferred.
async function cost(b, url, sizes) {
  const p = await t.page(b, `${url}?open=${SMALL}&offscreen=640x400`);
  await t.until(p, (s) => s.built?.checked, "the part built");
  const out = [];
  for (const n of sizes) {
    const r = await p.evaluate(async (n) => {
      const reps = n > 1e6 ? 20 : 400;
      const wasm = JSON.parse(window.forgecad.fc.app_build_bench(n, reps));
      // a worker that sends back what it's sent (transferred both ways): the transfer's time
      const src = "self.onmessage = (e) => self.postMessage(e.data, [e.data]);";
      const w = new Worker(URL.createObjectURL(new Blob([src], { type: "text/javascript" })));
      const bytes = new Uint8Array(wasm.bytes);
      // the copy out of wasm memory (`bytes.slice()` in build.js) and into it (wasm-bindgen's):
      // a slice each, timed together
      let t0 = performance.now();
      let c = bytes;
      for (let i = 0; i < reps; i++) c = bytes.slice();
      const copy = (performance.now() - t0) / reps;
      const once = (buf) =>
        new Promise((ok) => {
          w.onmessage = (e) => ok(e.data);
          w.postMessage(buf, [buf]);
        });
      let buf = c.buffer;
      for (let i = 0; i < 5; i++) buf = await once(buf);
      t0 = performance.now();
      for (let i = 0; i < reps; i++) buf = await once(buf);
      const trip = (performance.now() - t0) / reps;
      w.terminate();
      return { bytes: wasm.bytes, write_ms: wasm.write_ms, read_ms: wasm.read_ms, copy_ms: copy, transfer_ms: trip / 2 };
    }, n);
    // the whole way: written in the worker, copied out, transferred, copied in, read
    r.total_ms = r.write_ms + r.copy_ms + r.transfer_ms + r.copy_ms + r.read_ms;
    out.push(r);
  }
  t.check(p);
  await p.close();
  return out;
}

/// A small part's edit while another document's long check runs (one build worker for both):
/// how long it waits, against the same edit with nothing else building.
async function crossDoc(b, url) {
  const p = await t.page(b, `${url}?open=${SMALL}&offscreen=1280x800&worker=1`);
  await t.until(p, (s) => s.built?.checked, "the part built");
  await t.loaded(p);
  const edit = async (v) => {
    const seq = (await p.evaluate(() => window.forgecad.state())).seq;
    const t0 = Date.now();
    await p.evaluate(
      (v) => window.forgecad.fc.app_apply(`{"op":"SetParam","param":"t1","value":{"value":${v},"dim":"Length"}}`),
      v,
    );
    await t.until(p, (s) => s.path?.endsWith(SMALL) && s.seq > seq && s.done >= s.seq && s.built?.checked, "the part's edit", 600000);
    return Date.now() - t0;
  };
  const idle = await edit(1.6);
  // the rack opened beside it, its check on, an edit started; then the part's edit at once
  await p.evaluate((f) => window.forgecad.fc.app_open(`/project/${f}`), RACK);
  await t.until(p, (s) => s.path?.endsWith(RACK) && s.built?.checked, "the rack built", 600000);
  let seq = (await p.evaluate(() => window.forgecad.state())).seq;
  await click(p, "tab:Inspect");
  await click(p, "tool:Analyze Interference");
  await click(p, "asm:check", false);
  await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked, "the rack's check");
  await p.evaluate((e) => window.forgecad.fc.app_apply(e), RACK_EDIT);
  await p.evaluate((f) => window.forgecad.fc.app_open(`/project/${f}`), SMALL);
  await t.until(p, (s) => s.path?.endsWith(SMALL), "the part's tab");
  const behind = await edit(1.7);
  t.check(p);
  await p.close();
  return { idle, behind };
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("responsive");
  fs.cpSync(path.join(t.repo, "examples/rack18u"), dir, { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  const said = (r) =>
    `${r.how}: edit ${r.ms} ms (build ${r.build_ms?.toFixed?.(0)} ms), longest page task ${r.longest.toFixed(0)} ms, longest frame gap ${r.gap.toFixed(0)} ms`;
  try {
    // the default build on the page's own thread (?worker=0), then on the build worker
    const page = await rack(b, url, "&worker=0");
    console.log(said(page));
    const worker = await rack(b, url, "&worker=1");
    assert.equal(JSON.stringify(worker.built), JSON.stringify({ ...page.built, seq: worker.built.seq }), "the build worker's check differs from the page's");
    console.log(said(worker));
    assert.equal(worker.how, "build worker", "the build worker isn't the one building");
    assert.ok(worker.longest <= LONGEST, `a page task held the page ${worker.longest.toFixed(0)} ms on the build worker`);
    console.log(`ok - on the build worker the page never holds a task over ${LONGEST} ms through the rack's check`);
    if (fs.existsSync(path.join(app, "threads/forge_render.js"))) {
      const threaded = await rack(b, url, "&threads=1");
      console.log(said(threaded));
    }
    for (const c of await cost(b, `${url}`, [10700, 3900000])) {
      console.log(
        `crossing a ${(c.bytes / 1024).toFixed(1)} KB mesh message: ${c.total_ms.toFixed(3)} ms ` +
          `(write ${c.write_ms.toFixed(3)}, copy out ${c.copy_ms.toFixed(3)}, transfer ${c.transfer_ms.toFixed(3)}, ` +
          `copy in ${c.copy_ms.toFixed(3)}, read ${c.read_ms.toFixed(3)})`,
      );
    }
    const x = await crossDoc(b, url);
    console.log(`a small part's edit: ${x.idle} ms alone, ${x.behind} ms behind another document's check`);
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
