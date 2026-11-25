// The default build's build worker (a Web Worker with its own wasm memory): the page stays live
// through an assembly's long check, and what's built is what the page's own thread builds. Each
// case runs twice, its builds on the page (?worker=0) and on the build worker (?worker=1): the
// built state must be the same, and on the worker no page task may run long (Chrome's long-task
// timing, and the gaps between frames).
//   node web/tests/buildworker.cjs OUT   (OUT: web/build.sh's output, the default artifact)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

// an assembly with its overlap check on: an edit's check takes seconds on the page's thread
const CASES = [
  {
    name: "hinge_pin (assembly, overlap check)",
    open: "hw/tests/hinge_pin.fg",
    check: true,
    edit: '{"op":"SetParam","param":"z_door","value":{"value":40,"dim":"Length"}}',
  },
  {
    name: "rack (18U assembly, overlap check)",
    open: "asm/rack.fg",
    check: true,
    edit: '{"op":"SetParam","param":"y0_front","value":{"value":51,"dim":"Length"}}',
  },
  {
    name: "base_pan (part)",
    open: "frame/base_pan.fg",
    check: false,
    edit: null,
  },
];
// the longest a page task may hold the page on the build worker, ms
const LONGEST = Number(process.env.LONGEST_MS ?? 50);

async function run(b, url, c) {
  const p = await t.page(b, url, (p) =>
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
  await t.until(p, (s) => s.built?.checked, `${c.name} built`);
  let st = await t.loaded(p);
  if (c.check) {
    const seq = st.seq;
    // Inspect > Analyze Interference, then the dialog's Analyze
    // (a floating dialog's buttons aren't listed in the state's places: clicked by name blind)
    const click = async (name, listed = true) => {
      if (listed) await t.until(p, (s) => s.places?.some((x) => x.name === name), name);
      await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), name);
      // a frame drawn after it (an idle page draws none unasked): what the click opened shows
      await p.evaluate(() => window.forgecad.pixels());
      await p.evaluate(() => window.forgecad.pixels());
    };
    await click("tab:Inspect");
    await click("tool:Analyze Interference");
    await click("asm:check", false);
    st = await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked, "the overlap check");
  }
  // the first frames (the view put on the GPU) are long either way: from here on
  await p.evaluate(() => {
    window.longest = 0;
    window.gap = 0;
  });
  let ms = 0;
  if (c.edit) {
    const seq = st.seq;
    const t0 = Date.now();
    await p.evaluate((e) => window.forgecad.fc.app_apply(e), c.edit);
    st = await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked && s.built.seq >= s.seq, "the edit's build", 600000);
    ms = Date.now() - t0;
  }
  const r = {
    worker: await p.evaluate(() => window.forgecad.buildWorker),
    built: st.built,
    issues: st.issues,
    build_ms: st.build_ms,
    ms,
    longest: await p.evaluate(() => window.longest),
    gap: await p.evaluate(() => window.gap),
    cost: await p.evaluate(() => window.forgecad.buildCost?.()),
  };
  t.check(p);
  await p.close();
  return r;
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("buildworker");
  fs.cpSync(path.join(__dirname, "../../../../examples/rack18u"), dir, { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    for (const c of CASES) {
      const at = `${url}?open=${c.open}&offscreen=1280x800`;
      const here = await run(b, `${at}&worker=0`, c);
      const there = await run(b, `${at}&worker=1`, c);
      assert.equal(here.worker, false);
      assert.equal(there.worker, true, "the build worker isn't the one building");
      const said = (r) =>
        `edit ${r.ms} ms (build ${r.build_ms?.toFixed?.(0)} ms), longest page task ${r.longest.toFixed(0)} ms, longest frame gap ${r.gap.toFixed(0)} ms`;
      console.log(`${c.name}\n  on the page:   ${said(here)}\n  build worker:  ${said(there)}`);
      if (there.cost) console.log(`  crossed: ${JSON.stringify(there.cost)}`);
      // the same build, wherever it ran
      const keep = (r) => JSON.stringify({ ...r.built, seq: undefined, build_ms: undefined, mesh_ms: undefined, round_ms: undefined });
      assert.equal(keep(there), keep(here), "the build worker's build differs from the page's");
      if (c.edit) {
        assert.ok(there.longest <= LONGEST, `a page task held the page ${there.longest.toFixed(0)} ms on the build worker`);
      }
      console.log(`ok - ${c.name}: the same build both ways`);
    }
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
