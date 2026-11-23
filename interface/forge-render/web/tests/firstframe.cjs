// How soon the app is up on a real project: `forgecad web` on a copy of examples/ (every file of
// it, 500 and more), the page opened on one part. Says the time to the app's first frame and to
// the part built (checked), from the page's start; fails if the first frame takes longer than
// FIRST_FRAME_MS (default 10 s: it was 20 to 60 s when every file was fetched first).
//   node web/tests/firstframe.cjs OUT [PART]
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const part = process.argv[3] ?? "unified/knob.fg";
  const dir = t.tmpdir("firstframe");
  fs.cpSync(path.join(t.repo, "examples"), dir, { recursive: true });
  // as the server lists them: no dot files
  const files = fs
    .readdirSync(dir, { recursive: true })
    .filter((f) => fs.statSync(path.join(dir, f)).isFile() && !f.split(path.sep).some((s) => s.startsWith(".")));
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=${part}&offscreen=1280x800`);
    const st = await t.until(p, (s) => s.first_frame_ms > 0, "the first frame", 300000);
    const first = st.first_frame_ms;
    await t.until(p, (s) => s.built?.checked && s.path === `/project/${part}`, "the part built", 300000);
    const built = await p.evaluate(() => performance.now());
    const all = await p.evaluate(() => window.forgecad.files().length);
    console.log(`${files.length} files served; first frame at ${(first / 1000).toFixed(2)} s, ${part} built by ${(built / 1000).toFixed(2)} s; ${all} files in the page`);
    const most = Number(process.env.FIRST_FRAME_MS ?? 10000);
    assert.ok(first < most, `the first frame took ${first.toFixed(0)} ms (most ${most})`);
    // every file arrives (behind the first frame)
    await t.until(p, () => true, "", 1000);
    const loaded = await p.waitForFunction((n) => window.forgecad.files().length >= n, files.length, { timeout: 300000 });
    assert.ok(loaded);
    console.log(`ok - first frame in ${(first / 1000).toFixed(2)} s; all ${files.length} files in the page after`);
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
