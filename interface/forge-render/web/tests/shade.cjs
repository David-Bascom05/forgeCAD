// Parts of any size are drawn shaded, not black: boxes of 30, 80 and 400 mm, each read back and
// the middle of the view (where the box is) looked at.
// Before the fix every part past about 45 mm came out a black silhouette: ambient occlusion kept
// each pixel's distance in mm in a half-float target, which ends at 65504; a parallel view stands
// a thousand radii off, so the distance overflowed, the smoothing's weights went NaN, and the
// occlusion multiplied the part (faces and edges) to black.
//   node web/tests/shade.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const box = (s) => `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (${s}, ${s}))
}
base = extrude(plan, ${s / 2})
`;

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("shade");
  const sizes = [30, 80, 400];
  for (const s of sizes) fs.writeFileSync(path.join(dir, `box${s}.fg`), box(s));
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    for (const s of sizes) {
      const p = await t.page(b, `${url}?open=box${s}.fg&offscreen=1280x800`);
      await t.until(p, (st) => st.built?.checked, "the first build");
      await t.loaded(p);
      // the meshes in and drawn
      await new Promise((r) => setTimeout(r, 1500));
      // the view's middle (the chrome is left and above; the box fills the fitted view), looked
      // at in the page: a frame is megabytes
      const { sum, n, black } = await p.evaluate(async () => {
        const f = await window.forgecad.pixels();
        let sum = 0;
        let n = 0;
        let black = 0;
        for (let y = 380; y < 520; y += 4) {
          for (let x = 700; x < 860; x += 4) {
            const i = (y * f.width + x) * 4;
            const l = (f.rgba[i] + f.rgba[i + 1] + f.rgba[i + 2]) / 3;
            sum += l;
            n++;
            if (l < 8) black++;
          }
        }
        return { sum, n, black };
      });
      const f = await t.pixels(p);
      const mean = sum / n;
      console.log(`${s} mm: mean ${mean.toFixed(1)}, ${black} of ${n} samples black (${t.shot(`shade-${s}.png`, f)})`);
      assert.ok(black < n / 10 && mean > 80, `the ${s} mm box drawn dark: mean ${mean.toFixed(1)}, ${black} of ${n} black`);
      t.check(p);
      await p.close();
    }
    console.log("ok - every box shaded");
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
