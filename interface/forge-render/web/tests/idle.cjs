// An idle page draws nothing, and what's under the pointer is still found: open a part, leave it,
// count the frames drawn; then hover the part and see the hover drawn.
//   node web/tests/idle.cjs OUT   (OUT: web/build.sh's output)
// Before the fix the page drew every frame for good once a hover pick was out: a frame drawn
// while the pick's read-back was out queued more GPU work ahead of it, and on a slow adapter (the
// software one here) it never came back, so the frame read-back below never answered either.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (-40, -40), size: (80, 80))
}
base = extrude(plan, 40)
`;

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("idle");
  fs.writeFileSync(path.join(dir, "cube.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=cube.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    // settled: the build's meshes in, the pick it asks for answered
    await new Promise((r) => setTimeout(r, 3000));
    const s0 = await p.evaluate(() => window.forgecad.state());
    await new Promise((r) => setTimeout(r, 5000));
    const s1 = await p.evaluate(() => window.forgecad.state());
    const drawn = s1.frames - s0.frames;
    assert.ok(drawn <= 30, `an idle page drew ${drawn} frames in 5 s`);
    console.log(`ok - idle: ${drawn} frames drawn in 5 s`);
    // the frame reads back (it hung behind the endless frames)
    const ms = 30000;
    const f0 = await Promise.race([t.pixels(p), new Promise((_, no) => setTimeout(() => no(new Error(`no frame in ${ms} ms`)), ms))]);
    console.log(`ok - read back a frame of ${f0.colours} colours (${t.shot("idle-0.png", f0)})`);
    // hovered: the part under the pointer is picked and drawn marked
    const box = await p.evaluate(() => {
      const c = document.querySelector("canvas").getBoundingClientRect();
      return [c.x, c.y, c.width, c.height];
    });
    await p.mouse.move(box[0] + box[2] / 2, box[1] + box[3] / 2, { steps: 4 });
    let changed = 0;
    for (let i = 0; i < 20 && changed < 500; i++) {
      await new Promise((r) => setTimeout(r, 500));
      changed = (await t.pixels(p)).changed;
    }
    assert.ok(changed >= 500, `the hover drawn: ${changed} bytes changed`);
    console.log(`ok - hovered: ${changed} bytes changed`);
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
