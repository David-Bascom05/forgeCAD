// What a user feels as lag in the page, measured: time to open a document and have it built,
// frames while orbiting (the page's own frame count and the main thread's long tasks), and the
// time from a click on a ribbon tab to the frame that draws it.
//   node tools/ui-hunt/perf.cjs OUT [PROJECT_DIR FILE]...   (OUT: web/build.sh's output)
// With no files: a small part, and examples/rack18u/asm/rack.fg (a large assembly).
// Numbers only (SwiftShader draws on the CPU, so absolute frame times are the software
// adapter's): a run against a build before a change and one after compare like with like.
// PERF_QUERY="&threads=0": the single-threaded build where the threaded one would load.
const fs = require("node:fs");
const path = require("node:path");
const t = require("../../interface/forge-render/web/tests/lib.cjs");

const SMALL = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
`;

const pause = (ms) => new Promise((r) => setTimeout(r, ms));

async function measure(b, url, file) {
  const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
  await p.addInitScript(() => {
    window.__long = [];
    new PerformanceObserver((l) => {
      for (const e of l.getEntries()) window.__long.push([e.startTime, e.duration]);
    }).observe({ type: "longtask", buffered: true });
  });
  const t0 = Date.now();
  await p.goto(`${url}?open=${encodeURIComponent(file)}&offscreen=1280x800${process.env.PERF_QUERY ?? ""}`);
  const state = () => p.evaluate(() => window.forgecad?.state?.() ?? null);
  let st;
  for (;;) {
    st = await state().catch(() => null);
    if (st?.built?.checked && st.done >= st.seq && !/^not built/.test(st.status)) break;
    if (Date.now() - t0 > 600000) throw new Error(`${file}: not built in 10 min`);
    await pause(100);
  }
  const open = Date.now() - t0;
  await pause(2000);
  const openLong = await p.evaluate(() => window.__long.map((l) => l[1]));
  const long0 = await p.evaluate(() => window.__long.length);
  // orbit: shift + middle drag across the view, 60 steps (a left drag chooses by window)
  const f0 = (await state()).frames;
  const o0 = Date.now();
  await p.mouse.move(700, 450);
  await p.keyboard.down("Shift");
  await p.mouse.down({ button: "middle" });
  for (let i = 1; i <= 60; i++) await p.mouse.move(700 + i * 4, 450 + i * 2);
  await p.mouse.up({ button: "middle" });
  await p.keyboard.up("Shift");
  // the frames the drag asked for drawn
  for (let i = 0; i < 100; i++) {
    const s = await state();
    if (s.frames === (await pause(150), await state()).frames) break;
  }
  const orbitMs = Date.now() - o0;
  // how far the GPU is behind: a frame read back right after the orbit waits for every frame
  // queued before it (what the screen shows lags the mouse by that much)
  const g0 = Date.now();
  await p.evaluate(() => window.forgecad.pixels());
  const gpuBehind = Date.now() - g0;
  const orbitFrames = (await state()).frames - f0;
  const longs = await p.evaluate((k) => window.__long.slice(k), long0);
  const longSum = longs.reduce((a, l) => a + l[1], 0);
  const longMax = longs.reduce((a, l) => Math.max(a, l[1]), 0);
  // click to response: the Sketch tab (where every document's ribbon has it), the frame after
  const c0 = (await state()).frames;
  const ct = Date.now();
  await p.mouse.click(189, 55);
  for (;;) {
    if ((await state()).frames > c0) break;
    if (Date.now() - ct > 30000) break;
    await pause(5);
  }
  const click = Date.now() - ct;
  await p.close();
  return {
    file,
    open_ms: open,
    status: st.status,
    threaded: await p.evaluate(() => window.forgecad.threaded).catch(() => null),
    open_long_ms_max: Math.round(Math.max(0, ...openLong)),
    open_long_ms_total: Math.round(openLong.reduce((a, b) => a + b, 0)),
    orbit_ms: orbitMs,
    orbit_frames: orbitFrames,
    orbit_ms_per_frame: +(orbitMs / Math.max(orbitFrames, 1)).toFixed(1),
    gpu_behind_after_orbit_ms: gpuBehind,
    long_tasks: longs.length,
    long_ms_total: Math.round(longSum),
    long_ms_max: Math.round(longMax),
    click_to_frame_ms: click,
  };
}

(async () => {
  const [app, ...rest] = process.argv.slice(2);
  const jobs = [];
  for (let i = 0; i + 1 < rest.length; i += 2) jobs.push([path.resolve(rest[i]), rest[i + 1]]);
  if (!jobs.length) {
    const d = fs.mkdtempSync(path.join(require("node:os").tmpdir(), "perf-small-"));
    fs.writeFileSync(path.join(d, "small.fg"), SMALL);
    jobs.push([d, "small.fg"], [path.join(t.repo, "examples/rack18u"), "asm/rack.fg"]);
  }
  const b = await t.browser();
  try {
    for (const [dir, file] of jobs) {
      const { url, proc } = await t.serve(dir, path.resolve(app), ["--read-only"]);
      try {
        console.log(JSON.stringify(await measure(b, url, file)));
      } finally {
        proc.kill();
      }
    }
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
