// The Render Studio's live view in the browser keeps the page answering: its trace and denoise
// go a band of rows a frame, sized from the GPU time the page's queue reports done
// (on_submitted_work_done), so while it traces a frame read back takes little longer than with it
// stopped, and its samples go on. The browser's adapter is often a software one (SwiftShader in
// CI), so the budget is against the same page stopped: the time between frames while it traces
// is at most LIVE_MS, or half a stopped frame when those are slower than 160 ms, more than a
// stopped frame takes to draw and read back.
//   node web/tests/studio_live.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const LIVE_MS = 80;
const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const median = (v) => [...v].sort((a, b) => a - b)[v.length >> 1];

/// How long the page takes to draw and read back a frame, `n` times.
async function readbacks(p, n) {
  const ms = [];
  for (let i = 0; i < n; i++) {
    ms.push(
      await p.evaluate(async () => {
        const t0 = performance.now();
        await window.forgecad.pixels();
        return performance.now() - t0;
      }),
    );
  }
  return ms;
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("studio-live");
  fs.writeFileSync(path.join(dir, "a.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800${process.env.Q ?? ""}`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    await p.evaluate(() => window.forgecad.fc.app_env("render"));
    // a picture shown: the pace has measured the GPU, a denoise has run
    const st = await t.until(p, (s) => s.env === "render" && s.studio?.shown >= 8, "the live view's first picture", 180000);
    console.log(`ok - the live view shows ${st.studio.shown} samples, ${st.studio.rows} rows a frame`);
    // while it traces the page draws on: the time between its frames
    const s0 = await p.evaluate(() => window.forgecad.state());
    const t0 = Date.now();
    await pause(10000);
    const s1 = await p.evaluate(() => window.forgecad.state());
    const before = s0.studio.traced;
    const after = s1.studio;
    const frames = s1.frames - s0.frames;
    assert.ok(frames > 0, "no frames drawn while tracing");
    const l = (Date.now() - t0) / frames;
    assert.ok(after.traced > before, `no samples traced while timed: ${before} -> ${after.traced} (${frames} frames, ${JSON.stringify(after)})`);
    // stopped where it is: the same view, nothing traced
    await p.evaluate(() => window.forgecad.fc.app_studio("stop"));
    await pause(1500);
    // stopped, the page draws only when asked: a frame drawn and read back
    const stopped = await readbacks(p, 12);
    const s = median(stopped);
    const limit = Math.max(LIVE_MS, 0.5 * s);
    console.log(
      `studio frames in the browser: tracing ${l.toFixed(0)} ms a frame (${frames} frames in 10 s), ` +
        `stopped ${s.toFixed(0)} ms (median, drawn and read back); ${after.traced - before} samples while ` +
        `timed, ${after.rows} rows a frame`,
    );
    assert.ok(l - s <= limit, `a frame while tracing takes ${(l - s).toFixed(0)} ms more than stopped, over ${limit.toFixed(0)}`);
    console.log(`ok - tracing costs a frame ${(l - s).toFixed(0)} ms more than stopped (within ${limit.toFixed(0)} ms), samples go on`);
    // and on again
    await p.evaluate(() => window.forgecad.fc.app_studio("go"));
    await t.until(p, (q) => q.studio?.traced > after.traced, "the live view going on");
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
