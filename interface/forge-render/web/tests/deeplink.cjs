// Links into the page (`?open=PATH`): one to a path outside the project starts the app with
// nothing open (its empty state: New part, Open...) and says why; one to a drawing draws its part, which comes after the drawing (the
// page opens the linked file first, then the project's other files, then builds what's open
// again). Before: `?open=../x` threw before the app started (a dead page, `window.forgecad`
// never set), and a linked drawing kept "flange.fg: no such file" with an empty sheet.
//   node web/tests/deeplink.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("deeplink");
  fs.cpSync(path.join(t.repo, "examples/machined"), path.join(dir, "machined"), { recursive: true });
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    // outside the project: the app runs, opens nothing, and says so
    let p = await t.page(b, `${url}?open=../../../etc/passwd&offscreen=1280x800`);
    let s = await t.until(p, (s) => s.ready && /^Couldn't open /.test(s.status), "the app, saying why", 60000);
    // every file in, and still nothing opened in its place
    await p.evaluate(() => window.forgecad.loaded);
    s = await p.evaluate(() => window.forgecad.state());
    assert.equal(s.path, null, `opened ${s.path}`);
    assert.equal(s.status, "Couldn't open ../../../etc/passwd: it isn't in this project.");
    console.log(`ok - ?open=../../../etc/passwd: ${s.status}`);
    t.check(p);
    await p.close();

    // a drawing: its part read once it has come
    p = await t.page(b, `${url}?open=machined/flange_dwg.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.ready, "the app", 60000);
    // every file in (and what's open asked to be made again: a drawing is no part document, so
    // not t.loaded's build count)
    await p.evaluate(() => window.forgecad.loaded);
    s = await t.until(p, (s) => s.status === "Ready" || /problem/.test(s.status), "the drawing made", 300000);
    // (made again after the files came: wait out a first "problems" made before them)
    s = await t.until(p, (s) => s.status === "Ready", "the drawing made with its part", 120000).catch(() => s);
    assert.equal(s.status, "Ready", `the linked drawing: ${s.status}`);
    console.log("ok - ?open=machined/flange_dwg.fg: the drawing made with its part");
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
