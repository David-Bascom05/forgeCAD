// The page's agent when the page is hosted elsewhere (case 2 of
// research/agent-tab/bridge-threat-model.md): the app from a static server on one origin, the
// project and the person's own Claude Code from `forgecad web DIR --agent --allow-origin ORIGIN`
// on another (here a stand-in `claude`, FORGECAD_CLAUDE). The page pairs from the link the
// bridge printed (#bridge=…&agent=…), reads the project from it, and its edit lands on the
// machine; the page's own CSP (the <meta>, no server header) lets it reach loopback only.
// node web/tests/agent_hosted.cjs OUT (from interface/forge-render; OUT is web/build.sh's).
const t = require("./lib.cjs");
const assert = require("node:assert");
const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");

const app = process.argv[2];
const PART = `shared L = 60
shared W = 30
param extrude1.d = 12
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
extrude1 = extrude(plan, d)
`;

/// The stand-in: each message, an Edit asked about, made if allowed, then the turn's end.
function fake(dir) {
  const ask = JSON.stringify({ type: "control_request", request_id: "req-1", request: { subtype: "can_use_tool", tool_name: "Edit", input: { file_path: "plate.fg", old_string: "= 12", new_string: "= 14" }, permission_suggestions: [], tool_use_id: "t-1" } });
  const res = JSON.stringify({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t-1", content: "ok" }] }, parent_tool_use_id: null });
  const end = JSON.stringify({ type: "result", subtype: "success", session_id: "s" });
  const q = (s) => `'${s.replace(/'/g, `'\\''`)}'`;
  const script = path.join(dir, "claude");
  fs.writeFileSync(script, `#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"s","model":"m","permissionMode":"default"}'
while read line; do
  case "$line" in *'"type":"user"'*) ;; *) continue;; esac
  echo ${q(ask)}
  read reply
  case "$reply" in *'"allow"'*) sed -i 's/= 12/= 14/' plate.fg;; esac
  echo ${q(res)}
  echo ${q(end)}
done
`);
  fs.chmodSync(script, 0o755);
  return script;
}

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".json": "application/json", ".txt": "text/plain" };

/// A static host: the built app's files, nothing else, no headers of its own.
function host(dir) {
  return new Promise((ok) => {
    const s = http.createServer((req, res) => {
      const rel = decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/^\/+/, "") || "index.html";
      const f = path.join(dir, rel);
      if (!f.startsWith(dir) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) {
        res.writeHead(404).end();
        return;
      }
      res.writeHead(200, { "Content-Type": TYPES[path.extname(f)] ?? "application/octet-stream" });
      fs.createReadStream(f).pipe(res);
    });
    s.listen(0, "127.0.0.1", () => ok(s));
  });
}

async function settle(p) {
  const f0 = (await p.evaluate(() => window.forgecad.state())).frames;
  const t0 = Date.now();
  while (Date.now() - t0 < 5000) {
    if ((await p.evaluate(() => window.forgecad.state())).frames > f0) return;
    await new Promise((r) => setTimeout(r, 50));
  }
}
async function click(p, name) {
  await t.until(p, (s) => s.places?.some((x) => x.name === name), `\`${name}\` on screen`, 60000);
  await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), name);
  await settle(p);
}
const agentState = (s) => s.agent ?? { entries: [] };

(async () => {
  const root = t.tmpdir("agent-hosted");
  const dir = path.join(root, "proj");
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  process.env.FORGECAD_CLAUDE = fake(root);
  const site = await host(path.resolve(app));
  const origin = `http://127.0.0.1:${site.address().port}`;
  const { url, proc } = await t.serve(dir, app, ["--agent", "--allow-origin", origin]);
  const bridge = new URL(url);
  const token = url.split("#agent=")[1];
  const bridgeOrigin = `http://127.0.0.1:${bridge.port}`;
  const b = await t.browser();
  try {
    const p = await t.page(b, `${origin}/?open=plate.fg&offscreen=1280x800&threads=0#bridge=${bridgeOrigin}&agent=${token}`);
    const elsewhere = [];
    p.on("request", (r) => {
      const u = r.url();
      if (![origin, bridgeOrigin, "data:", "blob:"].some((o) => u.startsWith(o))) elsewhere.push(u);
    });
    const csp = [];
    p.on("console", (m) => /Content Security Policy/.test(m.text()) && csp.push(m.text()));
    await p.waitForFunction(() => window.forgecad?.loaded, null, { timeout: 600000 });
    await t.loaded(p);
    const kept = await p.evaluate(() => ({ hash: location.hash, local: JSON.stringify({ ...localStorage }), session: JSON.stringify({ ...sessionStorage }) }));
    assert.equal(kept.hash, "", "the pairing is taken off the address bar");
    assert.ok(!kept.local.includes(token) && !kept.session.includes(token), "and the token kept in no storage");
    const st = await p.evaluate(() => window.forgecad.state());
    assert.ok(st.built?.checked, "the project's part, read from the bridge, built in the page");
    console.log("ok - a page hosted elsewhere reads the project from the paired bridge");

    await click(p, "side:tab:Agent");
    await click(p, "agent:input");
    await p.evaluate(() => window.forgecad.fc.app_type("make the plate thicker", true));
    await settle(p);
    await t.until(p, (s) => agentState(s).entries.some((e) => e.kind === "permission" && e.state === "Waiting"), "the question");
    await click(p, "agent:perm:req-1:allow");
    await t.until(p, (s) => !agentState(s).running && agentState(s).entries.some((e) => e.kind === "diff"), "the turn's diff");
    assert.match(fs.readFileSync(path.join(dir, "plate.fg"), "utf8"), /extrude1\.d = 14/, "edited on the machine");
    if (process.env.FORGE_WEB_SHOTS) t.shot("agent-hosted-after-turn.png", await t.pixels(p));
    console.log("ok - its agent is the machine's Claude Code: asked, allowed, the edit made there and shown here");

    assert.deepEqual(elsewhere, [], "no request but to its host and the bridge");
    assert.deepEqual(csp, [], "its CSP refused nothing it needs");
    t.check(p);
    console.log("ok - it asked nothing of any other host; its CSP refused nothing it needs");
  } finally {
    await b.close();
    proc.kill();
    site.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
