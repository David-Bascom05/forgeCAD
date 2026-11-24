// The page keeps up with the mouse: frames are paced to the GPU (at most two out), so after an
// orbit what's on screen is the orbit's end, not a backlog. Unpaced, the page submitted a frame
// per mouse move however slow the adapter: after a 1.3 s orbit a frame read back waited 12.8 s
// (27 s on a large assembly) for the frames queued before it.
//   node web/tests/pace.cjs OUT   (OUT: web/build.sh's output)
// Q="&threads=0": the single-threaded build where the threaded one would load.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("pace");
  fs.writeFileSync(path.join(dir, "a.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800${process.env.Q ?? ""}`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    await pause(1500);
    await t.pixels(p);
    // an orbit: shift + middle drag, a move every few ms
    await p.mouse.move(700, 450);
    await p.keyboard.down("Shift");
    await p.mouse.down({ button: "middle" });
    for (let i = 1; i <= 60; i++) await p.mouse.move(700 + i * 4, 450 + i * 2);
    await p.mouse.up({ button: "middle" });
    await p.keyboard.up("Shift");
    const g0 = Date.now();
    await t.pixels(p);
    const behind = Date.now() - g0;
    assert.ok(behind < 4000, `a frame after the orbit waited ${behind} ms for the frames queued before it`);
    console.log(`ok - a frame after the orbit read back in ${behind} ms`);
    // the orbit's end is drawn, and then nothing more
    await pause(1500);
    const end = await t.pixels(p);
    await pause(1500);
    const again = await t.pixels(p);
    assert.equal(again.changed, 0, "still drawing after the orbit settled");
    const before = end.changed;
    assert.ok(before !== 0, "the orbit's end drawn");
    console.log("ok - the orbit's end drawn, then the page settled");
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
