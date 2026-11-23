// The Manufacture workspace in the browser: open a machining document (the flange's), see it
// generate as natively (every op certified, as `forgecad cam` says), select an op, edit it in its
// dialog (the document's text changes, the op goes stale and generates again), open the post
// dialog and the tool library. FORGE_WEB_SHOTS=DIR keeps a picture of each step.
//   node web/tests/cam.cjs OUT   (OUT: web/build.sh's output)
// STATIC=1: no `forgecad web`: OUT served as static files and the project kept in the page (its
// examples written into it), for a machine without the CLI built.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const t = require("./lib.cjs");

/// OUT as static files on a free port.
function serveStatic(root) {
  const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".txt": "text/plain" };
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

const T0 = Date.now();
const log = (m) => console.log(`[${((Date.now() - T0) / 1000).toFixed(1)}s] ${m}`);

async function click(p, name, button = 0, double = false) {
  log(`click ${name}`);
  await t.until(p, (s) => s.places?.some((x) => x.name === name), `\`${name}\` on screen`, 60000);
  await p.evaluate(([n, b, d]) => window.forgecad.fc.app_click(n, b, d), [name, button, double]);
  await settle(p);
}

/// A few frames drawn after the last ask.
async function settle(p) {
  const f0 = (await p.evaluate(() => window.forgecad.state())).frames;
  await t.until(p, (s) => s.frames >= f0 + 3, "frames", 60000);
}

async function shot(p, name) {
  await settle(p);
  const f = await t.pixels(p);
  log(`${name}: ${t.shot(name, f)}`);
  return f;
}

const generated = (s) => s.cam && s.cam.generated && !s.cam.generating;

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  let served, doc;
  if (process.env.STATIC) {
    served = await serveStatic(app);
    doc = "/project/examples/machined/flange_cam.fg";
  } else {
    const dir = t.tmpdir("cam");
    for (const f of ["flange.fg", "flange_cam.fg"]) {
      fs.copyFileSync(path.join(t.repo, "examples/machined", f), path.join(dir, f));
    }
    served = await t.serve(dir, app);
    doc = "/project/flange_cam.fg";
  }
  const store = process.env.STATIC ? "&storage=memory" : "";
  const b = await t.browser();
  try {
    const p = await t.page(b, `${served.url}?offscreen=1440x900${store}`);
    await t.until(p, (s) => s.ready, "the app");
    await p.evaluate(() => window.forgecad.loaded);
    await p.evaluate((d) => window.forgecad.fc.app_open(d), doc);
    let st = await t.until(p, generated, "the document generated");
    assert.equal(st.cam.certification, "except G5", JSON.stringify(st.cam));
    assert.deepEqual(
      st.cam.ops.map((o) => [o.name, o.status]),
      [["skim", "built"], ["top", "built"], ["prof", "built"], ["bore", "built"], ["bolts", "built"]],
    );
    console.log("ok - opened on Manufacture, every op certified as natively");
    await shot(p, "cam-1-opened.png");

    // the pictures first, driven by the workspace's commands (a click also asks the view for a
    // pick, which the offscreen software adapter may never finish: frames then spin)
    const cam = async (w) => {
      log(`cam ${w}`);
      await p.evaluate((x) => window.forgecad.fc.app_cam(x), w);
      await settle(p);
    };
    await cam("select prof");
    st = await t.until(p, (s) => s.cam.selected === "prof", "prof selected");
    await shot(p, "cam-2-selected.png");
    console.log("ok - an op selected: its tool, feeds, path and certificate");
    await cam("edit prof");
    await t.until(p, (s) => s.cam.dialogs.includes("op"), "the op dialog");
    await shot(p, "cam-3-op-dialog.png");
    await cam("tab 3");
    await cam("set leave 0.2");
    await shot(p, "cam-4-op-passes.png");
    await cam("close");
    await cam("edit bolts");
    await cam("tab 0");
    await shot(p, "cam-5-drill-geometry.png");
    await cam("close");
    await cam("post");
    await t.until(p, (s) => s.cam.dialogs.includes("post"), "the post dialog");
    await shot(p, "cam-6-post.png");
    await cam("close");
    await cam("tools");
    await t.until(p, (s) => s.cam.dialogs.includes("tools"), "the tool library");
    await shot(p, "cam-7-tools.png");
    await cam("close");
    await cam("new-setup");
    await t.until(p, (s) => s.cam.dialogs.includes("setup"), "the setup dialog");
    await shot(p, "cam-8-setup.png");
    await cam("tab 1");
    await shot(p, "cam-9-setup-stock.png");
    await cam("close");
    await cam("checks");
    await shot(p, "cam-10-checks.png");
    await cam("close");
    console.log("ok - op, post, tool library, setup and checks drawn");

    // then by hand: the op dialog opened from the browser, a field typed, OK
    await click(p, "cam:row:prof", 0, true);
    await t.until(p, (s) => s.cam.dialogs.includes("op"), "the op dialog, by a double click");
    await click(p, "cam:op:tab:3");
    await click(p, "cam:op:arg:leave");
    await p.evaluate(() => window.forgecad.fc.app_type("0.2", false));
    const before = st.cam.text;
    await click(p, "cam:op:ok");
    st = await t.until(p, (s) => s.cam.text !== before, "the edit in the text");
    assert.match(st.cam.text, /prof = contour\(op1, tool: t1, side: "outside", from: 12, to: -0.5, step: 4, leave: 0.2\)/);
    console.log("ok - the dialog's edit is the document's text");
    st = await t.until(p, generated, "generated again");
    assert.equal(st.cam.ops.find((o) => o.name === "prof").status, "built");
    console.log("ok - generated again: prof certified with its new stock to leave");
    t.check(p);
  } finally {
    await b.close();
    served.proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
