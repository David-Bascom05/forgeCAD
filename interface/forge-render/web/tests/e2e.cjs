// The app end to end, on files kept by `forgecad web`: load the app, open a part, edit a
// parameter as the panel does, see it rebuild, the volume equal to the native build's of the same
// text, the edit saved to the machine's file, and a write from an old version refused (StaleBase).
//   node web/tests/e2e.cjs OUT   (OUT: web/build.sh's output)
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `shared L = 60
shared W = 30
shared t = 6
param top.r = 8
param round.r = 2

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
sketch top(on: base.end) {
  c = circle((L / 2, W / 2), r)
}
boss = extrude(top.c, t)
round = fillet(edges(of: boss, crease: "convex"), r)
`;

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("e2e");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=plate.fg`);
    // opened and built: the panel's word is the native build's
    let st = await t.until(p, (s) => s.built?.checked, "the first build");
    assert.equal(st.path, "/project/plate.fg");
    const n0 = t.native([path.join(dir, "plate.fg")])[path.join(dir, "plate.fg")];
    assert.deepEqual(t.differs(n0, st.built), [], `first build: native ${JSON.stringify(n0)} page ${JSON.stringify(st.built)}`);
    console.log(`ok - opened plate.fg: volume ${st.built.volume}, ${st.built.faces} faces, as native`);

    // a parameter edited as the panel edits it: one undo step, rebuilt live
    const seq = st.seq;
    await p.evaluate(() =>
      window.forgecad.fc.app_apply('{"op":"SetParam","param":"t","value":{"value":9,"dim":"Length"}}'));
    st = await t.until(p, (s) => s.seq > seq && s.done >= s.seq && s.built?.checked && s.built.seq >= s.seq, "the rebuild");
    assert.equal(st.dirty, true);
    const v1 = st.built.volume;
    assert.ok(v1 > n0.volume, `thicker is more: ${v1} > ${n0.volume}`);
    console.log(`ok - t = 9 rebuilt: volume ${v1}`);

    // saved: the machine's file holds the edit, and it builds natively to the same bits
    await p.evaluate(() => window.forgecad.fc.app_save());
    await t.until(p, (s) => !s.dirty, "the save");
    await p.evaluate(() => window.forgecad.settled());
    const text = fs.readFileSync(path.join(dir, "plate.fg"), "utf8");
    assert.match(text, /shared t = 9/);
    const n1 = t.native([path.join(dir, "plate.fg")])[path.join(dir, "plate.fg")];
    assert.deepEqual(t.differs(n1, st.built), [], `edited: native ${JSON.stringify(n1)} page ${JSON.stringify(st.built)}`);
    console.log("ok - saved to the machine; its native build is the page's, bit for bit");

    // someone else changes the file; the page's next save is refused, not lost
    fs.writeFileSync(path.join(dir, "plate.fg"), text.replace("shared W = 30", "shared W = 31"));
    await p.evaluate(() =>
      window.forgecad.fc.app_apply('{"op":"SetParam","param":"t","value":{"value":7,"dim":"Length"}}'));
    st = await t.until(p, (s) => s.dirty, "the second edit");
    p.on("dialog", (d) => d.dismiss());
    await p.evaluate(() => window.forgecad.fc.app_save());
    await p.evaluate(() => window.forgecad.settled());
    const problems = await p.evaluate(() => window.forgecad.problems);
    assert.ok(problems.some((e) => /StaleBase|changed since it was read/.test(e)), JSON.stringify(problems));
    assert.match(fs.readFileSync(path.join(dir, "plate.fg"), "utf8"), /shared W = 31/, "the other edit kept");
    console.log("ok - a save over another's change is refused (StaleBase), the other change kept");
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
