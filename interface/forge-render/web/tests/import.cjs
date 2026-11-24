// File > Open > From this computer in the page: a DWG drawing opens in its environment; a STEP
// model (read by a part, nothing to open itself) is added to the project and the status says so
// ("Added NAME to the project").
// Before: a STEP file was added without a word (the dialog closed, nothing changed on screen).
//   node web/tests/import.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
base = extrude(plan, 5)
`;
const at = { file: [33, 55], open: [40, 139] };
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("import");
  fs.writeFileSync(path.join(dir, "a.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    const pick = async (file) => {
      await p.mouse.click(...at.file);
      await pause(800);
      await p.mouse.click(...at.open);
      await p.waitForSelector("#forge-open-file", { timeout: 30000 });
      await p.setInputFiles("#forge-open-file", file);
    };

    // a STEP model: added, and said
    const step = path.join(t.repo, "examples/rack18u/asm/order/RACK-322_latch_base.step");
    await pick(step);
    let s = await t.until(p, (s) => /^Added .* to the project$/.test(s.status), "the STEP file said added", 60000);
    assert.equal(s.status, "Added RACK-322_latch_base.step to the project");
    await p.evaluate(() => window.forgecad.settled());
    assert.ok(fs.existsSync(path.join(dir, "RACK-322_latch_base.step")), "the STEP file not kept with the project");
    console.log(`ok - a STEP file: ${s.status}`);
    t.check(p);

    // a DWG drawing: opened (the part's status line gives way to the drawing's)
    const s0 = (await p.evaluate(() => window.forgecad.state())).status;
    await pick(path.join(t.repo, "kernel/forge-dwg/tests/data/mix_2018.dwg"));
    s = await t.until(p, (s) => s.path === null && s.tabs.length > 0 && s.status !== s0, "the DWG opened", 120000);
    console.log("ok - a DWG file opened");
    t.check(p);

    // the DWG linked straight to, in a new page: no failure from the start left in its status bar
    // (the agent's terminal connection, which a page has none of, said "Agent connection: operation
    // not supported on this platform", and a drawing's opening said nothing over it)
    await p.evaluate(() => window.forgecad.settled());
    const q = await t.page(b, `${url}?open=mix_2018.dwg&offscreen=1280x800`);
    await t.until(q, (s) => s.ready, "the page", 60000);
    await q.evaluate(() => window.forgecad.loaded);
    await pause(2000);
    s = await q.evaluate(() => window.forgecad.state());
    assert.doesNotMatch(s.status, /Agent connection/, s.status);
    console.log(`ok - ?open=mix_2018.dwg: status "${s.status}"`);
    t.check(q);
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
