// Hosted collaboration in the browser (research/hosted/DESIGN.md phase 1): two tabs of the app
// open one part of a project SERVED by forge-server (?sync=), edit it at once as the panel does,
// and end on the same text; a Rename race resolves the same in both; a tab with no ?sync= stays
// LOCAL. Budget B2 (edit in one tab → the other has it) is measured and printed.
//   node web/tests/served.cjs OUT   (OUT: web/build.sh's output; forge-server built release)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");
const t = require("./lib.cjs");

const PART = `shared L = 60
shared W = 30
shared t = 6
param top.r = 8

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
sketch top(on: base.end) {
  c = circle((L / 2, W / 2), r)
}
boss = extrude(top.c, t)
`;
const TOKEN = "0123456789abcdef-browser";
const server = process.env.FORGE_SERVER ?? path.join(t.repo, "target/release/forge-server");

function startServer(dir, origin, port) {
  return new Promise((ok, fail) => {
    const s = spawn(server, [dir, "--port", String(port), "--user", `tab:${TOKEN}`, "--origin", origin], {
      stdio: ["ignore", "ignore", "pipe"],
    });
    s.on("error", fail);
    s.stderr.on("data", (d) => {
      if (process.env.VERBOSE) process.stderr.write(d);
      const m = /at (ws:\S+)/.exec(String(d));
      if (m) ok({ ws: m[1], proc: s });
    });
  });
}

const sync = (p) => p.evaluate(() => window.forgecad.state().sync);
const apply = (p, op) => p.evaluate((o) => window.forgecad.fc.app_apply(JSON.stringify(o)), op);
const setParam = (param, value) => ({ op: "SetParam", param, value: { value, dim: "Length" } });

async function same(a, b, what, min = 0) {
  const t0 = Date.now();
  while (Date.now() - t0 < 60000) {
    const [x, y] = [await sync(a), await sync(b)];
    if (x && y && x.ready && y.ready && !x.pending && !y.pending && x.seq >= min && x.seq === y.seq && x.text === y.text)
      return x;
    await new Promise((r) => setTimeout(r, 20));
  }
  throw new Error(`the tabs never agreed on ${what}: ${JSON.stringify([await sync(a), await sync(b)])}`);
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("served");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  // the page's CSP lets it connect to its own origin and the sync server named by --sync only:
  // both ports are chosen first, the sync server started, then the page's server with --sync
  const wport = 20000 + Math.floor(Math.random() * 20000);
  const sport = 40000 + Math.floor(Math.random() * 10000);
  const { ws, proc: srv } = await startServer(dir, `http://localhost:${wport}`, sport);
  const { url, proc: web } = await t.serve(dir, app, ["--port", String(wport), "--sync", ws]);
  const b = await t.browser();
  try {
    const q = `?open=plate.fg&offscreen=640x400&sync=${encodeURIComponent(ws)}&sync_token=${TOKEN}`;
    const [pa, pb] = [await t.page(b, url + q), await t.page(b, url + q)];
    for (const p of [pa, pb]) {
      await t.until(p, (s) => s.built?.checked, "the first build");
      await t.loaded(p);
    }
    const s0 = await same(pa, pb, "the snapshot");
    console.log(`ok - two tabs joined the served part at seq ${s0.seq}`);

    // one edits, the other sees it
    await apply(pa, setParam("t", 9));
    const s1 = await same(pa, pb, "t = 9", s0.seq + 1);
    assert.ok(s1.seq >= 1);
    console.log(`ok - a's edit reached b (seq ${s1.seq})`);

    // both edit at once
    await Promise.all([apply(pa, setParam("L", 70)), apply(pb, setParam("W", 40))]);
    const s2 = await same(pa, pb, "two edits at once", s1.seq + 2);
    assert.equal(s2.seq, s1.seq + 2);
    console.log(`ok - concurrent edits merged (seq ${s2.seq})`);

    // a Rename race: one wins, both end on the same text, the loser is told
    await Promise.all([
      apply(pa, { op: "Rename", from: "boss", to: "hub" }),
      apply(pb, { op: "Rename", from: "boss", to: "pin" }),
    ]);
    const s3 = await same(pa, pb, "the rename race", s2.seq + 1);
    // the losing rename's refusal arrives with it: give the tabs a moment to show it
    await new Promise((r) => setTimeout(r, 500));
    assert.equal(s3.seq, s2.seq + 1, "exactly one rename committed");
    const notes = [(await sync(pa)).note, (await sync(pb)).note].filter(Boolean);
    assert.ok(notes.some((n) => /RenameConflict|renamed/.test(n)), `the loser is told: ${notes}`);
    console.log("ok - a rename race: one won, both tabs agree, the other was told");

    // B2: edit in a → b has it
    const lat = [];
    for (let k = 0; k < 20; k++) {
      const want = (await sync(pb)).seq + 1;
      const t0 = Date.now();
      await apply(pa, setParam("t", 5 + (k % 7)));
      while ((await sync(pb)).seq < want) await new Promise((r) => setTimeout(r, 1));
      lat.push(Date.now() - t0);
    }
    lat.sort((x, y) => x - y);
    console.log(`ok - B2 in the browser (edit in a → b has the text, incl. evaluate round trips): p50 ${lat[10]} ms, max ${lat[19]} ms`);

    // a tab with no ?sync= stays LOCAL
    const local = await t.page(b, `${url}?open=plate.fg&offscreen=640x400`);
    await t.until(local, (s) => s.built?.checked, "the local tab's first build");
    await t.loaded(local);
    assert.equal(await sync(local), null, "no ?sync=: LOCAL, no link");
    console.log("ok - a tab with no ?sync= is LOCAL");
    for (const p of [pa, pb, local]) t.check(p);
  } finally {
    await b.close();
    srv.kill();
    web.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
