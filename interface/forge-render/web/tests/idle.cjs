// A page left alone goes idle: once a part is open and built, the view's pick of what is under the
// pointer is answered (none left in flight), the app stops drawing, and a frame reads back at
// once. Before the fix the web host never polled a pick that was in flight unless a window event
// came, so a headless page drew at 60 fps for ever and each readback took minutes.
//   node web/tests/idle.cjs OUT   (OUT: web/build.sh's output)
// STATIC=1: OUT served as static files, the project kept in the page (no `forgecad web`).
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `shared L = 60
shared W = 30
shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
`;

function serveStatic(root) {
  const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
  return new Promise((ok) => {
    const s = http.createServer((q, r) => {
      const u = decodeURIComponent(new URL(q.url, "http://x").pathname);
      const f = path.join(root, u === "/" ? "index.html" : u);
      if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) {
        r.writeHead(404);
        return r.end();
      }
      r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" });
      fs.createReadStream(f).pipe(r);
    });
    s.listen(0, () => ok({ url: `http://127.0.0.1:${s.address().port}/`, proc: { kill: () => s.close() } }));
  });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  let served, query;
  if (process.env.STATIC) {
    served = await serveStatic(app);
    query = "&storage=memory";
  } else {
    const dir = t.tmpdir("idle");
    fs.writeFileSync(path.join(dir, "plate.fg"), PART);
    served = await t.serve(dir, app);
    query = "&open=plate.fg";
  }
  const b = await t.browser();
  try {
    const p = await t.page(b, `${served.url}?offscreen=1280x800${query}`);
    await t.until(p, (s) => s.ready, "the app");
    await p.evaluate(() => window.forgecad.loaded);
    // the page's own project: one of its examples
    if (process.env.STATIC) await p.evaluate(() => window.forgecad.fc.app_open("/project/examples/machined/flange.fg"));
    await t.until(p, (s) => s.built?.checked && s.done >= s.seq, "the build");
    // a few asks (each wakes the page's loop, as a test's or a user's does)
    for (let k = 0; k < 3; k++) {
      await p.evaluate(() => window.forgecad.fc.app_rebuild());
      await sleep(300);
    }
    const st = await t.until(p, (s) => !s.pick_busy, "the view's pick answered", 15000);
    assert.equal(st.pick_busy, false);
    // idle: no more than a few frames in a second
    const f0 = (await p.evaluate(() => window.forgecad.state())).frames;
    await sleep(1500);
    const f1 = (await p.evaluate(() => window.forgecad.state())).frames;
    assert.ok(f1 - f0 < 10, `still drawing: ${f1 - f0} frames in 1.5 s`);
    console.log(`ok - idle: ${f1 - f0} frames in 1.5 s, no pick in flight`);
    const t0 = Date.now();
    const f = await t.pixels(p);
    const ms = Date.now() - t0;
    assert.ok(f.width === 1280 && ms < 5000, `a readback took ${ms} ms`);
    console.log(`ok - a frame read back in ${ms} ms`);
    t.check(p);
  } finally {
    await b.close();
    served.proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
