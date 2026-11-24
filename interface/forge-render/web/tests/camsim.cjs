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
  for (const f of ["flange.fg", "flange_cam.fg"]) fs.copyFileSync(path.join(machined, f), path.join(dir, f));
  fs.writeFileSync(path.join(dir, "refused_cam.fg"), REFUSED);
  const { proc, url } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    const p = await t.page(b, `${url}?open=flange_cam.fg&offscreen=1440x900`);
    let st = await t.until(p, (s) => s.camsim && s.camsim.total > 0, "the simulation", 900000);
    t.check(p);
    assert.equal(st.camsim.ok, true, JSON.stringify(st));
    assert.equal(st.camsim.ops, 5);
    assert.equal(st.camsim.refused, 0);
    assert.equal(st.camsim.markers, 0);
    console.log(`ok - flange_cam.fg simulated: ${st.camsim.moves} moves, ${st.camsim.total.toFixed(1)} s, ${st.camsim.certification}`);

    const shots = [
      ["sim-01-start.png", "t=0;part=ghost"],
      ["sim-02-facing.png", "at=0.12"],
      ["sim-03-clearing.png", "at=0.45"],
      ["sim-04-contour.png", "op=2;at=0.66"],
      ["sim-05-end-compare.png", "at=1;colour=compare;part=hidden"],
      ["sim-06-moves.png", "colour=material;part=ghost;op=3;locate=3,0,40,0.5;tab=moves"],
      ["sim-07-display.png", "tab=display;at=0.8;played=1"],
    ];
    let last = -1;
    for (const [name, cmd] of shots) {
      st = await sim(p, cmd);
      t.check(p);
      const f = st.frame;
      assert.ok(f.colours > 40, `${name}: ${f.colours} colours`);
      if (last >= 0) assert.notEqual(f.changed, 0, `${name}: nothing changed`);
      last = f.colours;
      console.log(`ok - ${cmd}: t ${st.camsim.t.toFixed(2)} op ${st.camsim.op} (${t.shot(name, f)})`);
    }
    // played: time moves on its own
    st = await sim(p, "played=0;tab=program;at=0.2;speed=25;play");
    const t0 = st.camsim.t;
    st = await t.until(p, (s) => s.camsim.t > t0 + 1, "play to move the time");
    await sim(p, "pause");
    console.log(`ok - played from ${t0.toFixed(2)} to ${st.camsim.t.toFixed(2)} s`);

    // the posted program on its machine: the machine's time, its bodies about the part, what
    // the machine run found as markers
    // (refused while forge-machine reads `G91 G28 Z0.` before any absolute position: said)
    if (st.camsim.machine_errors.length) {
      assert.match(st.camsim.machine_errors[0], /unknown position/);
      console.log(`ok - machine run refused, said: ${st.camsim.machine_errors[0]}`);
    } else {
      st = await sim(p, "source=machine;tab=program;at=0.3;colour=material;part=ghost;fit=machine");
      assert.equal(st.camsim.source, "generic-vmc3", JSON.stringify(st.camsim));
      assert.ok(st.camsim.markers > 0, "the machine run's contacts are markers");
      let f9 = st.frame;
      console.log(`ok - posted code on ${st.camsim.source}: ${st.camsim.total.toFixed(1)} s, ${st.camsim.markers} markers (${t.shot("sim-09-machine.png", f9)})`);
      st = await sim(p, "marker=0;tab=checks");
      f9 = st.frame;
      console.log(`ok - at the machine's first issue: t ${st.camsim.t.toFixed(2)} (${t.shot("sim-10-machine-issue.png", f9)})`);
      await sim(p, "source=toolpath;fit=stock");
    }

    // a refused op: its checks are markers; jumping to one stops on it
    await p.evaluate(() => window.forgecad.fc.app_open("/project/refused_cam.fg"));
    st = await t.until(p, (s) => s.camsim && s.camsim.refused > 0, "the refused document");
    assert.ok(st.camsim.markers > 0);
    st = await sim(p, "marker=0;tab=checks");
    const f = st.frame;
    console.log(`ok - refused: ${st.camsim.refused} op, ${st.camsim.markers} markers, at the first: t ${st.camsim.t.toFixed(2)} (${t.shot("sim-08-refused-checks.png", f)})`);
    t.check(p);
  } finally {
    await b.close();
    proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
