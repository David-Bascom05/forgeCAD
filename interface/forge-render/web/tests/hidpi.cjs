// The canvas on a high-density screen: its own pixels are its CSS size times the device's pixel
// ratio and the frames are drawn at that (else they fill a part of it: two thirds at 1.5), and it
// follows a change of the ratio (another monitor, the page zoomed).
//   node web/tests/hidpi.cjs OUT
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const close = (a, b) => Math.abs(a - b) <= 1;

function fits(c, dpr, what) {
  assert.ok(c, `${what}: no canvas state`);
  assert.equal(c.dpr, dpr, `${what}: the page's pixel ratio`);
  const [cw, ch] = c.css;
  assert.ok(cw >= 1000 && ch >= 600, `${what}: the canvas fills the page (css ${cw}x${ch})`);
  assert.ok(close(c.backing[0], cw * dpr) && close(c.backing[1], ch * dpr),
    `${what}: its pixels ${c.backing} are its CSS size ${c.css} times ${dpr}`);
  assert.deepEqual(c.drawn, c.backing, `${what}: the frames are drawn at its pixels`);
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("hidpi");
  fs.writeFileSync(path.join(dir, "box.fg"), "sketch s(on: xy) {\n  r = rect(corner: (0, 0), size: (20, 10))\n}\nb = extrude(s, 5)\n");
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    // the canvas itself (a headless browser can't show its frames, but sizes it as any)
    const ctx = await b.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });
    const p = await ctx.newPage();
    p.problems = [];
    p.on("pageerror", (e) => p.problems.push(`page error: ${e.message}`));
    p.on("console", (m) => /panicked/.test(m.text()) && p.problems.push(m.text()));
    await p.goto(`${url}?open=box.fg`);
    let st = await t.until(p, (s) => s.canvas?.drawn && s.built, "the first frames");
    fits(st.canvas, 2, "at a ratio of 2");
    console.log(`ok - at 2: css ${st.canvas.css}, pixels ${st.canvas.backing}, drawn ${st.canvas.drawn}`);

    // the ratio changes under it (as moving to another monitor does)
    const cdp = await ctx.newCDPSession(p);
    await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1.5, mobile: false });
    st = await t.until(p, (s) => s.canvas?.dpr === 1.5 && close(s.canvas.backing[0], s.canvas.css[0] * 1.5), "the change to 1.5", 60000);
    fits(st.canvas, 1.5, "after a change to 1.5");
    console.log(`ok - followed to 1.5: pixels ${st.canvas.backing}, drawn ${st.canvas.drawn}`);
    t.check(p);
    await ctx.close();

    // offscreen at 2: the frame read back is the CSS size times 2, chrome and part laid out in it
    const ctx2 = await b.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });
    const q = await ctx2.newPage();
    q.problems = [];
    q.on("pageerror", (e) => q.problems.push(`page error: ${e.message}`));
    await q.goto(`${url}?open=box.fg&offscreen=1280x800`);
    await t.until(q, (s) => s.built?.checked, "the offscreen build");
    const f = await t.pixels(q);
    assert.deepEqual([f.width, f.height], [2560, 1600]);
    assert.ok(f.colours > 40, `a frame of ${f.colours} colours`);
    console.log(`ok - offscreen at 2: a ${f.width}x${f.height} frame (${t.shot("hidpi-2x.png", f)})`);
    t.check(q);
    await ctx2.close();
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
