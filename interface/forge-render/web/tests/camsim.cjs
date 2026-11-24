// The toolpath and stock simulation in the browser: a machining document (the flange's) opens in
// its simulation, the CAM run made in the page; scrubbed, played, coloured against the part, its
// panel's tabs drawn; then a document with a refused op, its checks as markers to jump to.
//   node web/tests/camsim.cjs OUT   (OUT: web/build.sh's output)
//   FORGE_WEB_SHOTS=DIR keeps each frame as a PNG.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const REFUSED = `machining
f = part("flange.fg", group: "N")
m = machine("generic-vmc3")
t1 = tool("flat-10", t: 1, stickout: 32, holder: "bt40-er32")
op1 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: "top-center", tol: 0.01)
skim = facing(op1, tool: t1, to: 18, step: 1)
bolts = drill(op1, f.bolts, plan: "auto")
prog = program(op1)
`;

async function sim(p, cmd) {
  await p.evaluate((c) => window.forgecad.fc.app_camsim(c), cmd);
  // the words are taken on the app's next turn; a frame read after them draws them
  const frame = await t.pixels(p);
  const st = await p.evaluate(() => window.forgecad.state());
  st.frame = frame;
  return st;
}

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("camsim");
  const machined = path.join(t.repo, "examples/machined");
  for (const f of ["flange.fg", "flange_cam.fg", "shaft.fg", "shaft_turn.fg"]) {
    fs.copyFileSync(path.join(machined, f), path.join(dir, f));
  }
  fs.writeFileSync(path.join(dir, "refused_cam.fg"), REFUSED);
  const { proc, url } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    // the review set's sizes (docs/ux/STANDARD.md §10), the full run at the first
    for (const [w, h] of [[1400, 860], [1600, 1000], [1024, 700]]) {
      const tag = `${w}x${h}`;
      const p = await t.page(b, `${url}?open=flange_cam.fg&offscreen=${tag}`);
      let st = await t.until(p, (s) => s.camsim && s.camsim.total > 0, "the simulation", 900000);
      t.check(p);
      assert.equal(st.camsim.ok, true, JSON.stringify(st));
      assert.equal(st.camsim.ops, 5);
      assert.equal(st.camsim.refused, 0);
      assert.equal(st.camsim.markers, 0);
      console.log(`ok - ${tag}: flange_cam.fg simulated: ${st.camsim.moves} moves, ${st.camsim.total.toFixed(1)} s, ${st.camsim.certification}`);
      const shots = [
        ["web-sim-01-start", "t=0;part=ghost"],
        ["web-sim-02-contour", "op=2;at=0.5"],
        ["web-sim-03-end-compare", "at=1;colour=compare;part=hidden"],
        ["web-sim-04-moves", "colour=material;part=ghost;op=3;locate=3,0,40,0.5;tab=moves"],
      ];
      for (const [name, cmd] of shots) {
        st = await sim(p, cmd);
        t.check(p);
        const f = st.frame;
        assert.ok(f.colours > 40, `${name}: ${f.colours} colours`);
        console.log(`ok - ${cmd}: t ${st.camsim.t.toFixed(2)} op ${st.camsim.op} (${t.shot(`${name}-${tag}.png`, f)})`);
      }
      // the posted program on its machine: placed (at:) and clear of the table, so no issue
      st = await sim(p, "tab=program;source=machine;at=0.3;colour=material;part=ghost");
      assert.deepEqual(st.camsim.machine_errors, []);
      assert.equal(st.camsim.source, "generic-vmc3", JSON.stringify(st.camsim));
      assert.equal(st.camsim.markers, 0, "the placed flange's posted code: no collision or limit");
      console.log(`ok - posted code on ${st.camsim.source}: ${st.camsim.total.toFixed(1)} s (${t.shot(`web-sim-05-machine-${tag}.png`, st.frame)})`);
      await sim(p, "source=toolpath");
      if (w === 1400) {
        // played: time moves on its own
        st = await sim(p, "tab=program;at=0.2;speed=25;play");
        const t0 = st.camsim.t;
        st = await t.until(p, (s) => s.camsim.t > t0 + 1, "play to move the time");
        await sim(p, "pause");
        console.log(`ok - played from ${t0.toFixed(2)} to ${st.camsim.t.toFixed(2)} s`);

        // a refused op: its checks are markers; jumping to one stops on it
        await p.evaluate(() => window.forgecad.fc.app_open("/project/refused_cam.fg"));
        st = await t.until(p, (s) => s.camsim && s.camsim.refused > 0, "the refused document");
        assert.ok(st.camsim.markers > 0);
        st = await sim(p, "marker=0;tab=checks");
        console.log(`ok - refused: ${st.camsim.refused} op, ${st.camsim.markers} markers, at the first: t ${st.camsim.t.toFixed(2)} (${t.shot(`web-sim-07-refused-checks-${tag}.png`, st.frame)})`);
        t.check(p);
      }
      await p.close();
    }

    // a turning document: the lathe's run, no milling simulation and no milling errors
    const p = await t.page(b, `${url}?open=shaft_turn.fg&offscreen=1400x860`);
    let st = await t.until(p, (s) => s.cam && s.cam.generated && !s.cam.generating, "the turning run", 900000);
    for (let k = 0; k < 10; k++) st = { ...(await p.evaluate(() => window.forgecad.state())), frame: await t.pixels(p) };
    assert.equal(st.camsim, null, "a milling simulation of a turning document");
    console.log(`ok - shaft_turn.fg: ${st.cam.certification}, no milling simulation (${t.shot("web-sim-06-turning-no-sim-1400x860.png", st.frame)})`);
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
