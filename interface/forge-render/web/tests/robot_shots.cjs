// The Robot environment's pictures in the browser, beside the native ones in
// research/export/urdf-ui (tests/robot_ui.rs), one fresh page each:
//   FORGE_WEB_SHOTS=DIR node web/tests/robot_shots.cjs OUT [NAME]   (OUT served as static files)
// Each opens examples/robot/arm2_robot.fg, waits for the robot to be built and the view fitted to
// it, clicks the browser's rows by their places (`robot:joint:NAME`, `robot:link:NAME`), and keeps
// the picture.
const path = require("node:path"), http = require("node:http"), fs = require("node:fs");
const t = require("./lib.cjs");
const root = process.argv[2];
const SHOTS = [
  ["web-01-robot-open.png", [], (s) => s.robot && s.robot.framed],
  ["web-02-robot-joint.png", ["robot:joint:elbow"], (s) => s.robot && s.robot.sel === "joint:elbow"],
  ["web-03-robot-link.png", ["robot:link:forearm"], (s) => s.robot && s.robot.sel === "link:forearm"],
];
const only = process.argv[3];
const s = http.createServer((q, r) => {
  const u = decodeURIComponent(new URL(q.url, "http://x").pathname);
  const f = path.join(root, u === "/" ? "index.html" : u);
  if (!fs.existsSync(f) || fs.statSync(f).isDirectory()) { r.writeHead(404); return r.end(); }
  const type = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" }[path.extname(f)];
  r.writeHead(200, { "content-type": type ?? "application/octet-stream" });
  fs.createReadStream(f).pipe(r);
});
s.listen(0, async () => {
  const b = await t.browser();
  let failed = 0;
  for (const [name, clicks, ready] of SHOTS) {
    if (only && !name.includes(only)) continue;
    const T0 = Date.now();
    const p = await t.page(b, `http://127.0.0.1:${s.address().port}/?offscreen=1600x1000&storage=memory`);
    try {
      await t.until(p, (s) => s.ready, "ready");
      await p.evaluate(() => window.forgecad.loaded);
      await p.evaluate(() => window.forgecad.fc.app_open("/project/examples/robot/arm2_robot.fg"));
      await t.until(p, (s) => s.robot && s.robot.built && s.robot.framed, "the robot built and framed");
      for (const c of clicks) {
        await t.until(p, (s) => s.places.some((x) => x.name === c), c);
        await p.evaluate((x) => window.forgecad.fc.app_click(x, 0, false), c);
      }
      const st = await t.until(p, ready, name, 60000);
      await t.frames(p, st, 3);
      const f = await t.pixels(p);
      console.log(`${name} ${t.shot(name, f)} ${((Date.now() - T0) / 1000).toFixed(1)}s`);
    } catch (e) {
      failed++;
      console.log(`${name} FAILED ${e.message.slice(0, 200)}`);
    }
    await p.close();
  }
  await b.close();
  s.close();
  process.exit(failed ? 1 : 0);
});
