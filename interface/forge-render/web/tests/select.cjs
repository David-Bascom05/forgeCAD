// A face clicked in the view is drawn selected after the pointer leaves it, and the pointer over
// the part never lights an origin plane behind it: the page as tests/selection.rs has the desktop.
// Before: a clicked face showed nothing once the pointer left (the frame left out what was
// selected), and the XZ plane behind the face lit up green with the pointer on the face.
//   node web/tests/select.cjs OUT   (OUT: web/build.sh's output)
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
// the top face as the view fits this plate at 1280 x 800, and a point of the XZ plane's square
const FACE = [760, 440];
const PLANE = [450, 300];

async function at(p, [x, y]) {
  return p.evaluate(
    async ([x, y]) => {
      const f = await window.forgecad.pixels();
      const i = (y * f.width + x) * 4;
      return [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]];
    },
    [x, y],
  );
}
const warm = (c) => c[0] - c[2] > 25;
// the selection's outline, full strength, around the plate's top face
async function outlined(p) {
  return p.evaluate(async () => {
    const f = await window.forgecad.pixels();
    let n = 0;
    for (let y = 300; y < 620; y++)
      for (let x = 580; x < 1000; x++) {
        const i = (y * f.width + x) * 4;
        const [r, g, b] = [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]];
        if (r > 200 && g > 120 && g < 200 && b < 90) n++;
      }
    return n;
  });
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("select");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=plate.fg&offscreen=1280x800${process.env.Q ?? ""}`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    await p.mouse.move(300, 650);
    await pause(1500);
    const face0 = await at(p, FACE);
    const out0 = await outlined(p);
    const plane0 = await at(p, PLANE);
    assert.ok(!warm(face0), `the face warm before any click: ${face0}`);
    await p.mouse.move(...FACE);
    await pause(1500);
    await p.mouse.down();
    await p.mouse.up();
    await t.until(p, (s) => s.selected === 1, "the face selected", 20000);
    await pause(1500);
    assert.deepEqual(await at(p, PLANE), plane0, "the origin plane behind the face lit up with the pointer on the face");
    await p.mouse.move(300, 650);
    await pause(1500);
    const face1 = await at(p, FACE);
    assert.ok(warm(face1), `the selected face not drawn selected: ${face1} (before ${face0})`);
    const out1 = await outlined(p);
    assert.ok(out1 > 500 && out1 > out0 * 10, `the selected face's outline: ${out1} amber pixels (before ${out0})`);
    console.log(`ok - the clicked face drawn selected (${face0} -> ${face1}, outline ${out0} -> ${out1}), no plane lit`);
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
