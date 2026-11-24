// The page draws no window controls: a page has no window of its own to minimize, maximize or
// close. Before: the desktop's three buttons sat at the title bar's right end and did nothing.
//   node web/tests/chrome.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (10, 10))
}
base = extrude(plan, 5)
`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("chrome");
  fs.writeFileSync(path.join(dir, "a.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=a.fg&offscreen=1280x800`);
    await t.until(p, (s) => s.built?.checked, "the first build");
    await t.loaded(p);
    await p.mouse.move(640, 500);
    await pause(1000);
    // where the desktop draws them: the title bar's last 110 px, its 36 px height; the bar is one
    // colour there now, and the icons were near white on it
    const light = await p.evaluate(async () => {
      const f = await window.forgecad.pixels();
      let n = 0;
      for (let y = 6; y < 30; y++)
        for (let x = f.width - 110; x < f.width - 4; x++) {
          const i = (y * f.width + x) * 4;
          if (f.rgba[i] > 150 && f.rgba[i + 1] > 150 && f.rgba[i + 2] > 150) n++;
        }
      return n;
    });
    assert.equal(light, 0, `${light} icon pixels where the window buttons were`);
    console.log("ok - no window buttons in the page's title bar");
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
