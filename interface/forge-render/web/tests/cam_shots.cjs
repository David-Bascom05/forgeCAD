// The Manufacture workspace's pictures for research/cam/ui, one fresh page each (a page left
// running spins on an unanswered view pick in the offscreen software adapter; see cam.cjs):
//   FORGE_WEB_SHOTS=DIR node web/tests/cam_shots.cjs OUT [NAME]   (OUT served as static files)
const path = require("node:path"), http = require("node:http"), fs = require("node:fs");
const t = require("./lib.cjs");
const root = process.argv[2];
const TURN = "/project/examples/machined/shaft_turn.fg";
const SHOTS = [
  ["cam-32-quality.png", ["quality"], (s) => s.cam.dialogs.includes("quality")],
  ["cam-33-op-quality.png", ["select top"], (s) => s.cam.selected === "top"],
  ["cam-30-programs.png", ["programs"], (s) => s.cam.dialogs.includes("programs")],
  ["cam-31-setup-time.png", ["select op1"], (s) => s.cam.selected === "op1"],
  ["cam-25-section.png", ["select rough"], (s) => s.cam.selected === "rough", TURN],
  ["cam-26-section-full.png", ["select fin"], (s) => s.cam.selected === "fin", TURN],
  ["cam-21-lathe-setup.png", ["edit op1"], (s) => s.cam.dialogs.includes("setup"), TURN],
  ["cam-22-turn-rough.png", ["select rough", "edit rough", "tab 2"], (s) => s.cam.dialogs.includes("op"), TURN],
  ["cam-23-lathe-tools.png", ["tools"], (s) => s.cam.dialogs.includes("tools"), TURN],
  ["cam-24-lathe-post.png", ["select prog", "post"], (s) => s.cam.dialogs.includes("post"), TURN],
  ["cam-3-op-dialog.png", ["select prof", "edit prof"], (s) => s.cam.dialogs.includes("op")],
  ["cam-4-op-passes.png", ["select prof", "edit prof", "tab 3", "set leave 0.2"], (s) => s.cam.dialogs.includes("op")],
  ["cam-5-drill-geometry.png", ["select bolts", "edit bolts", "tab 0"], (s) => s.cam.dialogs.includes("op")],
  ["cam-6-post.png", ["select prog", "post"], (s) => s.cam.dialogs.includes("post")],
  ["cam-7-tools.png", ["tools"], (s) => s.cam.dialogs.includes("tools")],
  ["cam-8-setup.png", ["new-setup"], (s) => s.cam.dialogs.includes("setup")],
  ["cam-9-setup-stock.png", ["new-setup", "tab 1"], (s) => s.cam.dialogs.includes("setup")],
  ["cam-10-checks.png", ["select op1", "checks"], (s) => s.cam.dialogs.includes("checks")],
  ["cam-12-op-linking.png", ["select prof", "edit prof", "tab 4"], (s) => s.cam.dialogs.includes("op")],
  ["cam-11-new-pocket.png", ["select bore", "new-op pocket", "tab 1"], (s) => s.cam.dialogs.includes("op")],
  ["cam-35-nc-programs-web.png", ["programs"], (s) => s.cam.dialogs.includes("programs"), TURN],
  ["cam-1-opened.png", [], (s) => true],
  // a small screen (UX-LEAD's 1024 × 700): the windows fit the view, nothing runs under a panel
  ["cam-40-small-op.png", ["select prof"], (s) => s.cam.selected === "prof", undefined, "1024x700"],
  ["cam-41-small-tools.png", ["tools"], (s) => s.cam.dialogs.includes("tools"), undefined, "1024x700"],
  ["cam-42-small-programs.png", ["programs"], (s) => s.cam.dialogs.includes("programs"), undefined, "1024x700"],
  ["cam-44-small-sheet.png", ["select prof", "edit prof"], (s) => s.cam.dialogs.includes("op"), undefined, "1024x700"],
];
const only = process.argv[3];
const s = http.createServer((q, r) => { const u = decodeURIComponent(new URL(q.url, "http://x").pathname); const f = path.join(root, u === "/" ? "index.html" : u);
  if (!fs.existsSync(f) || fs.statSync(f).isDirectory()) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { "content-type": { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" }[path.extname(f)] ?? "application/octet-stream" }); fs.createReadStream(f).pipe(r); });
s.listen(0, async () => {
  const b = await t.browser();
  for (const [name, cmds, ready, doc, size] of SHOTS) {
    if (only && !name.includes(only)) continue;
    const T0 = Date.now();
    const p = await t.page(b, `http://127.0.0.1:${s.address().port}/?offscreen=${size ?? "1440x900"}&storage=memory`);
    try {
      await t.until(p, (s) => s.ready, "ready");
      await p.evaluate(() => window.forgecad.loaded);
      await p.evaluate((d) => window.forgecad.fc.app_open(d), doc ?? "/project/examples/machined/flange_cam.fg");
      await t.until(p, (s) => s.cam && s.cam.generated && !s.cam.generating, "gen");
      for (const c of cmds) await p.evaluate((x) => window.forgecad.fc.app_cam(x), c);
      await t.until(p, ready, name, 60000);
      const f = await t.pixels(p);
      console.log(`${name} ${t.shot(name, f)} ${((Date.now() - T0) / 1000).toFixed(1)}s`);
    } catch (e) { console.log(`${name} FAILED ${e.message.slice(0, 200)}`); }
    await p.close();
  }
  await b.close(); s.close();
});
