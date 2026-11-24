// The Bend Simulation environment in the browser (wasm32, WebGPU): a channel opened straight in
// it, on the library's first real press with its maker's own tools, scrubbed to a bend; the
// press, tools and part drawn by the view's renderer. Pictures for research/bendsim/env/:
//   FORGE_WEB_SHOTS=DIR node web/tests/bendsim.cjs OUT   (OUT: web/build.sh's, served static)
const path = require("node:path"), http = require("node:http"), fs = require("node:fs");
const t = require("./lib.cjs");
const root = process.argv[2];
if (!root) throw new Error("usage: node web/tests/bendsim.cjs OUT");

const CHANNEL = `shared t = 2
shared r = 2
shared k = ratio(0.4)
shared h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (100, 200))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
right = flange(base.edge(plan.outline[1]), height: h)
left = flange(base.edge(plan.outline[3]), height: h)
`;

const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
const server = http.createServer((q, r) => {
  const u = decodeURIComponent(new URL(q.url, "http://x").pathname);
  const f = path.join(root, u === "/" ? "index.html" : u);
  if (!fs.existsSync(f) || fs.statSync(f).isDirectory()) {
    r.writeHead(404);
    return r.end();
  }
  r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" });
  fs.createReadStream(f).pipe(r);
});

async function picture(p, name) {
  const st = await p.evaluate(() => window.forgecad.state());
  await t.frames(p, st, 6);
  const f = await t.pixels(p);
  console.log(`  ${name}: ${t.shot(name, f)} (${f.colours} colours)`);
  if (f.colours < 8) throw new Error(`${name}: a blank frame`);
}

server.listen(0, async () => {
  const base = `http://127.0.0.1:${server.address().port}/?offscreen=1400x860&storage=memory`;
  const b = await t.browser();
  let failed = 0;
  try {
    const p = await t.page(b, base);
    await t.until(p, (s) => s.ready, "ready");
    await p.evaluate(() => window.forgecad.loaded);
    await p.evaluate((text) => {
      window.forgecad.fc.fs_write("/project/env/channel.fg", new TextEncoder().encode(text));
      window.forgecad.fc.app_open("/project/env/channel.fg");
      window.forgecad.fc.app_env("bendsim");
    }, CHANNEL);
    await t.until(p, (s) => s.env === "bendsim", "in Bend Simulation");
    await picture(p, "web-01-bendsim-start.png");
    for (const [k, at, name] of [
      [0, 0.6, "web-02-bendsim-bend-1.png"],
      [1, 0.7, "web-03-bendsim-bend-2.png"],
    ]) {
      await p.evaluate(([k, at]) => window.forgecad.fc.app_bendsim_at(k, at), [k, at]);
      await picture(p, name);
    }
    t.check(p);
    await p.close();
    console.log("ok bendsim");
  } catch (e) {
    failed++;
    console.log(`FAILED bendsim: ${e.message.slice(0, 600)}`);
  }
  await b.close();
  server.close();
  process.exit(failed ? 1 : 0);
});
