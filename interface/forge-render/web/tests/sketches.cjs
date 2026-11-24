// A long session in the page keeps no more sketches than one build made: each frame frees what
// its runs made (in the single-threaded page, every build's). A sketch with a named entity is an
// Rc cycle and a forge-lang run frees none itself: without the release every edit kept its
// sketches (~30 KB an edit of a small part, without bound).
//   node web/tests/sketches.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
  bore = circle((15, 15), 3)
}
base = extrude(plan, t)
`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("sketches");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=plate.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    for (let k = 0; k < 15; k++) {
      const seq = (await p.evaluate(() => window.forgecad.state())).seq;
      const op = JSON.stringify({ op: "SetParam", param: "t", value: { value: 5 + (k % 3), dim: "Length" } });
      await p.evaluate((op) => window.forgecad.fc.app_apply(op), op);
      await t.until(p, (s) => s.seq > seq && s.done === s.seq, `edit ${k} built`, 120000);
      await pause(200);
      const kept = await p.evaluate(() => window.forgecad.fc.app_sketches_kept());
      assert.equal(kept, 0, `edit ${k}: ${kept} sketches kept between frames`);
    }
    console.log("ok - 15 edits in the page, no sketches kept between frames");
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
