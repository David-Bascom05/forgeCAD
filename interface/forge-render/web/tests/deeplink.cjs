// Links into the page (`?open=PATH`): one to a path outside the project starts the app with
// nothing open (its empty state: New part, Open...) and says why; one to a drawing draws its part;
// one to a machining document generates it certified, never first refused. The page says
// "Loading project · opening FILE when it's in" (and a spinner in the empty view) until the
// project's files have come, then opens the linked file; one opened or made from the empty view
// meanwhile is kept, and the link's file said not opened.
// Before: `?open=../x` threw before the app started (a dead page, `window.forgecad` never set);
// a linked drawing kept "flange.fg: no such file" with an empty sheet; a linked machining
// document opened before its part came: "0/5 ops certified", every op an error and Export NC
// refused, for seconds, until a rebuild.
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

    // a drawing: made with its part, no first sheet of problems
    p = await t.page(b, `${url}?open=machined/flange_dwg.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.ready, "the app", 60000);
    await p.evaluate(() => window.forgecad.loaded);
    s = await t.until(p, (s) => s.status === "Ready" || /problem/.test(s.status), "the drawing made", 300000);
    assert.equal(s.status, "Ready", `the linked drawing: ${s.status}`);
    console.log("ok - ?open=machined/flange_dwg.fg: the drawing made with its part");
    t.check(p);
    await p.close();

    // a machining document: said waited for while the files come, then generated certified,
    // with no refused generation on the way (every state watched from the start)
    // (the project's files held back 4 s, as from a slow server: the app is up before they come)
    const slow = (p) =>
      p.route("**/api/bundle", async (r) => {
        await new Promise((ok) => setTimeout(ok, 4000));
        await r.continue();
      });
    p = await t.page(b, `${url}?open=machined/flange_cam.fg&offscreen=1280x800`, slow);
    s = await t.until(p, (s) => s.ready, "the app", 60000);
    const seen = [s];
    const watch = (async () => {
      for (;;) {
        const x = await p.evaluate(() => window.forgecad.state()).catch(() => null);
        if (!x) return;
        seen.push(x);
        if (x.cam?.generated && !x.cam.generating) return;
        await new Promise((r) => setTimeout(r, 25));
      }
    })();
    const WAIT = "Loading project · opening machined/flange_cam.fg when it's in";
    // the waiting view, kept for a look (FORGE_WEB_SHOTS=DIR)
    await t.until(p, (s) => s.status === WAIT, "the wait said", 30000);
    const f = await t.pixels(p);
    console.log(`  waiting: ${t.shot("deeplink-waiting.png", f)}`);
    await p.evaluate(() => window.forgecad.loaded);
    await watch;
    s = seen[seen.length - 1];
    assert.ok(seen.some((x) => x.status === WAIT), `never said the wait: ${JSON.stringify([...new Set(seen.map((x) => x.status))].slice(0, 8))}`);
    const early = seen.find((x) => x.cam?.generated && x.cam.certification !== "full");
    assert.equal(early, undefined, `a generation not certified: ${JSON.stringify(early?.cam?.certification)} ${early?.status}`);
    assert.equal(s.cam.certification, "full", JSON.stringify(s.cam.certification));
    console.log(`ok - ?open=machined/flange_cam.fg: "${WAIT}", then ${s.status}`);
    t.check(p);
    await p.close();

    // a new part made from the empty view while the link waits: kept, and the link said not opened
    p = await t.page(b, `${url}?open=machined/flange_cam.fg&offscreen=1280x800`, slow);
    await t.until(p, (s) => s.ready && s.status === WAIT, "the empty view, waiting", 60000);
    await p.evaluate(() => window.forgecad.fc.app_click("empty:new", 0, false));
    const made = await t.until(p, (s) => s.status !== WAIT, "the new part", 30000);
    console.log(`  made meanwhile: path ${made.path}, status "${made.status}"`);
    await p.evaluate(() => window.forgecad.loaded);
    s = await t.until(p, (s) => / not opened: /.test(s.status), "the link said not opened", 30000);
    assert.match(s.status, /^machined\/flange_cam\.fg not opened: you opened \S/, s.status);
    assert.equal(s.path, made.path, "the new part taken over");
    assert.equal(s.cam, null, "the machining document opened anyway");
    console.log(`ok - opened meanwhile: ${s.status}`);
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
