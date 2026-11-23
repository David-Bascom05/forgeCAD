// Environments in the browser (shell/env, research/ui/environments.md): a deep link opens a part
// straight in one (?open=..&env=..), the page's app_env enters and leaves one, the state says
// which. Pictures for research/ui/environments/:
//   FORGE_WEB_SHOTS=DIR node web/tests/environments.cjs OUT   (OUT: web/build.sh's, served static)
const path = require("node:path"), http = require("node:http"), fs = require("node:fs");
const t = require("./lib.cjs");
const root = process.argv[2];
if (!root) throw new Error("usage: node web/tests/environments.cjs OUT");

const SHEET = `shared t = 2
shared r = 2
shared k = ratio(0.4)
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (120, 60))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: 30)
down = flange(base.edge(plan.outline[3]), height: 20)
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
  await t.frames(p, st, 4);
  const f = await t.pixels(p);
  console.log(`  ${name}: ${t.shot(name, f)} (${f.colours} colours)`);
  if (f.colours < 8) throw new Error(`${name}: a blank frame`);
}

server.listen(0, async () => {
  const base = `http://127.0.0.1:${server.address().port}/?offscreen=1400x860&storage=memory`;
  const b = await t.browser();
  let failed = 0;
  const run = async (name, f) => {
    const T0 = Date.now();
    try {
      await f();
      console.log(`ok ${name} ${((Date.now() - T0) / 1000).toFixed(1)}s`);
    } catch (e) {
      failed++;
      console.log(`FAILED ${name}: ${e.message.slice(0, 400)}`);
    }
  };

  // a deep link: the part opens in Thermoform, its tabs the environment's
  await run("deep link ?env=thermoform", async () => {
    const p = await t.page(b, `${base}&open=examples/thermoform/cover.fg&env=thermoform`);
    const s = await t.until(p, (s) => s.ready && s.env === "thermoform", "in Thermoform");
    // Thermoform (7), then Inspect, View, Manage
    if (JSON.stringify(s.tabs) !== "[7,2,3,4]") throw new Error(`tabs ${JSON.stringify(s.tabs)}`);
    await picture(p, "web-thermoform.png");
    // Finish: the design's tabs, Environments (20) last
    await p.evaluate(() => window.forgecad.fc.app_env("exit"));
    const d = await t.until(p, (s) => s.env === null, "back in the design");
    if (d.tabs[d.tabs.length - 1] !== 20 || d.tabs.includes(7)) throw new Error(`tabs ${JSON.stringify(d.tabs)}`);
    t.check(p);
    await p.close();
  });

  // entered from the page once the part has built: the bend simulation drawn in the view
  await run("app_env bendsim", async () => {
    const p = await t.page(b, base);
    await t.until(p, (s) => s.ready, "ready");
    await p.evaluate(() => window.forgecad.loaded);
    await p.evaluate((text) => {
      window.forgecad.fc.fs_write("/project/env/sheet.fg", new TextEncoder().encode(text));
      window.forgecad.fc.app_open("/project/env/sheet.fg");
      window.forgecad.fc.app_env("bendsim");
    }, SHEET);
    const s = await t.until(p, (s) => s.env === "bendsim", "in Bend Simulation");
    if (s.tab !== 21) throw new Error(`tab ${s.tab}`);
    await picture(p, "web-bend-simulation.png");
    await p.evaluate(() => window.forgecad.fc.app_env("render"));
    await t.until(p, (s) => s.env === "render" && s.tab === 22, "in Render Studio");
    await picture(p, "web-render-studio.png");
    // one not on main yet: refused, the one in kept
    await p.evaluate(() => window.forgecad.fc.app_env("manufacture"));
    const m = await t.until(p, (s) => /Manufacture|manufacture/.test(s.status), "refused");
    if (m.env !== "render") throw new Error(`env ${m.env}`);
    t.check(p);
    await p.close();
  });

  await b.close();
  server.close();
  process.exit(failed ? 1 : 0);
});
