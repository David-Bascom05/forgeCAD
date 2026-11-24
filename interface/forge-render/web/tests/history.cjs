// Versions under `forgecad web`: history turned on in the page is kept in the served folder's
// own .git (and comes back with the page), and where the server refuses it (read only) the page
// never says "History is on" over the refusal.
//   node web/tests/history.cjs OUT   (OUT: web/build.sh's output)
// Before: the server refused every dot path, so every .git write failed (console only), the
// status said "History is on: every save is kept", and a reload had no history.
// The page is 1280x800 (offscreen), so the chrome is where these coordinates say.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
base = extrude(plan, 5)
`;
const at = { versionsTab: [157, 190], turnOnHistory: [57, 250] };
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

async function turnOn(b, url) {
  const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800`);
  await t.until(p, (s) => s.built?.checked, "the first build");
  await t.loaded(p);
  await p.mouse.click(...at.versionsTab);
  await pause(800);
  await p.mouse.click(...at.turnOnHistory);
  await pause(1000);
  await p.evaluate(() => window.forgecad.settled());
  await pause(1000);
  return p;
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const b = await t.browser();
  const runs = [];
  try {
    // a writable server: the history is on the machine, and back after a reload
    const dir = t.tmpdir("history");
    fs.writeFileSync(path.join(dir, "a.fg"), PART);
    const s1 = await t.serve(dir, app);
    runs.push([s1.proc, dir]);
    let p = await turnOn(b, s1.url);
    const st = await p.evaluate(() => window.forgecad.state());
    assert.match(st.status, /History is on/, st.status);
    assert.deepEqual(await p.evaluate(() => window.forgecad.problems), []);
    for (const f of [".git/HEAD", ".git/config", ".gitignore"]) {
      assert.ok(fs.existsSync(path.join(dir, f)), `${f} kept on the machine`);
    }
    const objects = fs.readdirSync(path.join(dir, ".git/objects")).filter((d) => /^[0-9a-f]{2}$/.test(d));
    assert.ok(objects.length > 0, "the save's objects on the machine");
    console.log(`ok - history kept in the served folder's .git (${objects.length} object folders)`);
    await p.close();
    p = await t.page(b, `${s1.url}?open=a.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the build after a reload");
    await t.loaded(p);
    const head = await p.evaluate(() => window.forgecad.fc.fs_read("/project/.git/HEAD")?.length ?? 0);
    assert.ok(head > 0, "the history comes back with the page");
    console.log("ok - the history is in the page again after a reload");
    t.check(p);
    await p.close();

    // a read-only server: the page says the history isn't kept, not "History is on"
    const ro = t.tmpdir("history-ro");
    fs.writeFileSync(path.join(ro, "a.fg"), PART);
    const s2 = await t.serve(ro, app, ["--read-only"]);
    runs.push([s2.proc, ro]);
    p = await turnOn(b, s2.url);
    const st2 = await t.until(p, (s) => /Not kept/.test(s.status), "the refusal said", 20000);
    assert.doesNotMatch(st2.status, /History is on/);
    assert.ok(!fs.existsSync(path.join(ro, ".git")), "nothing written");
    console.log(`ok - read only: "${st2.status}"`);
  } finally {
    await b.close();
    for (const [proc, d] of runs) {
      proc.kill();
      fs.rmSync(d, { recursive: true, force: true });
    }
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
