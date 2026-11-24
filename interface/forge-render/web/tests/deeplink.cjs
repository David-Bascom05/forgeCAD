// Links into the page (`?open=PATH`): one to a path outside the project starts the app with
// nothing open (its empty state: New part, Open...) and says why; one to a drawing draws its part;
// one to a machining document generates it certified, never first refused. The page says
// "Loading project…" until the project's files have come, then opens the linked file.
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

    // a machining document: "Loading project…" while the files come, then generated certified,
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
    await p.evaluate(() => window.forgecad.loaded);
    await watch;
    s = seen[seen.length - 1];
    assert.ok(seen.some((x) => x.status === "Loading project…"), `never said Loading project…: ${JSON.stringify([...new Set(seen.map((x) => x.status))].slice(0, 8))}`);
    const early = seen.find((x) => x.cam?.generated && x.cam.certification !== "full");
    assert.equal(early, undefined, `a generation not certified: ${JSON.stringify(early?.cam?.certification)} ${early?.status}`);
    assert.equal(s.cam.certification, "full", JSON.stringify(s.cam.certification));
    console.log(`ok - ?open=machined/flange_cam.fg: "Loading project…", then ${s.status}`);
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
