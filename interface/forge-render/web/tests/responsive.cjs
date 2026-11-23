// The page stays live while a part builds: with the threaded artifact the build runs on a Web
// Worker, so no task on the page's main thread is as long as the build (Chrome's long-task
// timing). The single-threaded build is measured too, for contrast: there the build is a task.
//   node web/tests/responsive.cjs OUT   (OUT built with THREADS=1)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = "frame";
// its first parameter: nearly every feature builds again
const EDIT = '{"op":"SetParam","param":"reach","value":{"value":390,"dim":"Length"}}';

async function measure(b, url) {
  const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
  await p.addInitScript(() => {
    window.longest = 0;
    new PerformanceObserver((l) => {
      for (const e of l.getEntries()) window.longest = Math.max(window.longest, e.duration);
    }).observe({ type: "longtask", buffered: true });
  });
  p.problems = [];
  p.on("pageerror", (e) => p.problems.push(`page error: ${e.message}`));
  p.on("console", (m) => /panicked/.test(m.text()) && p.problems.push(m.text()));
  await p.goto(url);
  const st = await t.until(p, (s) => s.built?.checked, `${PART} built`);
  // the app's first frames (the view put on the GPU) are long tasks either way: measure an edit's
  // rebuild
  await p.evaluate(() => (window.longest = 0));
  const seq = st.seq;
  await p.evaluate((e) => window.forgecad.fc.app_apply(e), EDIT);
  const st2 = await t.until(p, (s) => s.done >= s.seq && s.seq > seq && s.built?.checked, "the rebuild", 600000);
  const r = {
    threaded: await p.evaluate(() => window.forgecad.threaded),
    build_ms: st2.build_ms,
    longest: await p.evaluate(() => window.longest),
  };
  t.check(p);
  await p.close();
  return r;
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("responsive");
  t.native(["--bench", dir]);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const one = await measure(b, `${url}?open=${PART}.fg&offscreen=1280x800&threads=0`);
    console.log(`single-threaded: rebuild ${one.build_ms.toFixed(0)} ms, longest main-thread task ${one.longest.toFixed(0)} ms`);
    const two = await measure(b, `${url}?open=${PART}.fg&offscreen=1280x800`);
    console.log(`threaded:        rebuild ${two.build_ms.toFixed(0)} ms, longest main-thread task ${two.longest.toFixed(0)} ms`);
    assert.equal(two.threaded, true, "the threaded build isn't the one running");
    assert.ok(two.longest < two.build_ms, "a main-thread task as long as the build: the page blocked");
    console.log("ok - the threaded build's page never blocks for a build");
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
