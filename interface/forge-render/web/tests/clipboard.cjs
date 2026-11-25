// Copy in the page: where the browser allows the clipboard the text is there; where it refuses
// (no permission, the page not focused) the status says so. Before: the clipboard's promise was
// dropped, so a refusal was an unhandled rejection (a page error) and nothing told the person the
// copy had failed.
//   node web/tests/clipboard.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
`;
// Inspect > Measure, the plate's top face as the view fits it, the panel's Copy all (one pick)
const at = { inspectTab: [449, 55], measure: [35, 105], face: [760, 440], copyAll: [328, 417] };
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

async function measureAndCopy(p) {
  await p.mouse.click(...at.inspectTab);
  await pause(800);
  await p.mouse.click(...at.measure);
  await pause(1200);
  await p.mouse.move(...at.face);
  await pause(800);
  await p.mouse.click(...at.face);
  await pause(1500);
  await p.mouse.click(...at.copyAll);
  await pause(1500);
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("clipboard");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    // refused (a headless browser's default): said, and no page error
    let p = await t.page(b, `${url}?open=plate.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    await measureAndCopy(p);
    const s = await t.until(p, (s) => /^Couldn't copy/.test(s.status), "the refusal said", 30000);
    assert.equal(s.status, "Couldn't copy: the browser didn't allow it.");
    t.check(p);
    console.log(`ok - clipboard refused: "${s.status}", no page error`);
    await p.close();

    // allowed: the measurement is on the clipboard
    const ctx = await b.newContext({ viewport: { width: 1280, height: 800 } });
    await ctx.grantPermissions(["clipboard-read", "clipboard-write"], { origin: url });
    const q = await ctx.newPage();
    q.problems = [];
    q.on("pageerror", (e) => q.problems.push(`page error: ${e.message}`));
    await q.goto(`${url}?open=plate.fg&offscreen=1280x800`);
    await t.until(q, (s) => s.built?.checked, "the first build (allowed)");
    await t.loaded(q);
    await measureAndCopy(q);
    const text = await q.evaluate(() => navigator.clipboard.readText());
    assert.ok(text.length > 0, "nothing copied");
    const st = (await q.evaluate(() => window.forgecad.state())).status;
    assert.doesNotMatch(st, /Couldn't copy/, st);
    assert.deepEqual(q.problems, []);
    console.log(`ok - clipboard allowed: ${JSON.stringify(text.slice(0, 60))}...`);
    await ctx.close();
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
