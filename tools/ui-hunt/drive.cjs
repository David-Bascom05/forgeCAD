// A long-lived headless app for hunting UI bugs by hand: `node tools/ui-hunt/drive.cjs OUT DIR [QUERY]`
// serves DIR with `forgecad web --app OUT`, opens the app offscreen, and takes commands on
// http://127.0.0.1:$DRIVE_PORT (default 9555) as JSON lines POSTed to /:
//   {"shot":"name.png"}  {"click":[x,y]}  {"dbl":[x,y]}  {"move":[x,y]}  {"drag":[x0,y0,x1,y1]}
//   {"key":"Control+z"}  {"type":"text"}  {"wheel":[x,y,dy]}  {"eval":"js"}  {"state":1}  {"log":1}
//   {"goto":"?query"}  {"wait":ms}
// Console lines (and page errors) are kept; {"log":1} returns and clears them.
const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const t = require("../../interface/forge-render/web/tests/lib.cjs");

(async () => {
  const [app, dir, query = "?offscreen=1280x800"] = process.argv.slice(2);
  const shots = process.env.FORGE_WEB_SHOTS ?? "/tmp/ui-hunt";
  process.env.FORGE_WEB_SHOTS = shots;
  const { url, proc } = await t.serve(path.resolve(dir), path.resolve(app));
  const b = await t.browser();
  const log = [];
  let p;
  async function open(q) {
    if (p) await p.close();
    p = await b.newPage({ viewport: { width: 1280, height: 800 } });
    p.on("console", (m) => log.push(m.text()));
    p.on("pageerror", (e) => log.push(`PAGEERROR ${e.message}\n${e.stack ?? ""}`));
    p.on("worker", (w) => w.on?.("console", (m) => log.push(`[worker] ${m.text()}`)));
    // a panic's whole wasm stack, not V8's ten frames
    await p.addInitScript(() => {
      Error.stackTraceLimit = 200;
    });
    await p.goto(url + q);
  }
  await open(query);
  const run = async (c) => {
    if (c.goto) return open(c.goto);
    if (c.shot) return t.shot(c.shot, await t.pixels(p));
    if (c.click) return p.mouse.click(...c.click, c.button ? { button: c.button } : {});
    if (c.dbl) return p.mouse.dblclick(...c.dbl);
    if (c.move) return p.mouse.move(...c.move);
    if (c.drag) {
      const [a, bb, x, y] = c.drag;
      await p.mouse.move(a, bb);
      await p.mouse.down();
      await p.mouse.move(x, y, { steps: 8 });
      return p.mouse.up();
    }
    if (c.key) return p.keyboard.press(c.key);
    if (c.type) return p.keyboard.type(c.type);
    if (c.wheel) {
      await p.mouse.move(c.wheel[0], c.wheel[1]);
      return p.mouse.wheel(0, c.wheel[2]);
    }
    if (c.eval) return p.evaluate(c.eval);
    if (c.state) return p.evaluate(() => window.forgecad?.state?.());
    if (c.log) return log.splice(0);
    if (c.wait) return new Promise((r) => setTimeout(r, c.wait));
    throw new Error("unknown command");
  };
  http
    .createServer((req, res) => {
      let body = "";
      req.on("data", (d) => (body += d));
      req.on("end", async () => {
        const out = [];
        for (const line of body.split("\n").filter((l) => l.trim())) {
          try {
            out.push(await run(JSON.parse(line)));
          } catch (e) {
            out.push(`ERROR ${e.message}`);
          }
        }
        res.end(JSON.stringify(out, null, 1) + "\n");
      });
    })
    .listen(Number(process.env.DRIVE_PORT ?? 9555), "127.0.0.1");
  console.log("driving", url + query);
  process.on("SIGTERM", async () => {
    await b.close();
    proc.kill();
    process.exit(0);
  });
})();
