// The bench parts and the examples built in the page and natively: every measure the same, to
// the bit (volume and area bits, faces, edges, bodies, problems, whether it built).
//   node web/tests/parity.cjs OUT [--examples N]   (OUT: web/build.sh's output, served here)
// A hang fails fast: a file whose build takes longer than PARITY_FILE_MS (30 s) or a whole run
// longer than PARITY_RUN_MS (10 min) stops the test, saying the file and the worker pool's state.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const k = process.argv.indexOf("--examples");
  const most = k > 0 ? Number(process.argv[k + 1]) : Infinity;
  const dir = t.tmpdir("parity");
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  let bad = 0;
  let widest = 1;
  let n = 0;
  const fileMs = Number(process.env.PARITY_FILE_MS ?? 30000);
  const runMs = Number(process.env.PARITY_RUN_MS ?? 600000);
  let stage = "starting";
  let page = null;
  let slowest = [0, ""];
  let memory = "?";
  // the whole run's cap: a stage that never ends (the native run, the page's load) is said too
  const cap = setTimeout(async () => {
    const pool = page ? await page.evaluate(() => window.forgecad?.fc?.pool_state?.()).catch((e) => `unread: ${e.message}`) : null;
    console.error(`not ok - the run took over ${runMs / 1000} s, stopped at: ${stage}\n  workers: ${pool}`);
    process.exit(1);
  }, runMs);
  try {
    stage = "the page's load";
    const p = (page = await t.page(b, `${url}?storage=memory&noapp=1`));
    await p.waitForFunction(() => window.forgecad?.loaded, null, { timeout: 120000 });
    await p.evaluate(() => window.forgecad.loaded);
    // THREADED=1: the threaded artifact's kernel (std with atomics, the spin-locked heap) is the one
    const threaded = await p.evaluate(() => window.forgecad.threaded);
    if (process.env.THREADED) assert.equal(threaded, true, "the threaded build isn't the one running");
    console.log(`the ${threaded ? "threaded" : "single-threaded"} build`);
    // the bench parts: natively written to a folder, in the page to its filesystem
    stage = "the native builds";
    const benchDir = path.join(dir, "bench");
    const nat = t.native(["--bench", benchDir]);
    const pageFiles = await p.evaluate(() => window.forgecad.fc.bench_load("/bench"));
    const examples = fs.readFileSync(path.join(app, "examples/list.txt"), "utf8").trim().split("\n")
      .filter((f) => f.endsWith(".fg")).slice(0, most);
    Object.assign(nat, t.native(examples.map((f) => path.join(t.repo, "examples", f))));
    const pairs = [
      ...pageFiles.map((f) => [f, path.join(benchDir, path.basename(f))]),
      ...examples.map((f) => [`/project/examples/${f}`, path.join(t.repo, "examples", f)]),
    ];
    for (const [pf, nf] of pairs) {
      stage = `${pf} in the page`;
      const t0 = Date.now();
      // on a worker (the threaded build: the kernel's parallel paths on more workers)
      const r = await p.evaluate(async ([f, ms]) => {
        const fc = window.forgecad.fc;
        const id = fc.build_start(f);
        const end = performance.now() + ms;
        while (performance.now() < end) {
          const r = fc.build_result(id);
          if (r !== undefined) return { r };
          await new Promise((ok) => setTimeout(ok, 5));
        }
        return { timeout: fc.pool_state() };
      }, [pf, fileMs]);
      if (r.timeout) {
        throw new Error(`${pf}: not built in ${fileMs / 1000} s (a hang?)\n  workers: ${r.timeout}\n  console: ${p.problems.join(" | ")}`);
      }
      const ms = Date.now() - t0;
      if (ms > slowest[0]) slowest = [ms, pf];
      const got = JSON.parse(r.r);
      widest = Math.max(widest, got.threads ?? 1);
      t.check(p);
      const d = t.differs(nat[nf], got);
      n += 1;
      if (d.length) {
        bad += 1;
        console.log(`not ok - ${nf}: ${d.join(", ")}\n  native ${JSON.stringify(nat[nf])}\n  page   ${JSON.stringify(got)}`);
      } else {
        console.log(`ok - ${path.relative(t.repo, nf)}: ${got.ok ? `${got.faces} faces` : "refused alike"}`);
      }
    }
    // the memory's size at the end (the threaded build's 4 GiB is the most it may have)
    memory = JSON.parse(await p.evaluate(() => window.forgecad.fc.pool_state())).memory_mib;
  } finally {
    clearTimeout(cap);
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
  console.log(`${n - bad} of ${n} the same (the kernel's parallel paths on up to ${widest} threads; slowest ${slowest[1]}, ${(slowest[0] / 1000).toFixed(1)} s; memory ${memory} MiB)`);
  if (process.env.THREADED) assert.ok(widest > 1, "the threaded build's kernel ran on one thread");
  assert.equal(bad, 0);
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
