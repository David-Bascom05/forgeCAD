// The bench parts and the examples built in the page and natively: every measure the same, to
// the bit (volume and area bits, faces, edges, bodies, problems, whether it built).
//   node web/tests/parity.cjs OUT [--examples N]   (OUT: web/build.sh's output, served here)
// PARITY_SHARD=I/N (I from 0) tests every N-th file from the I-th, so N runs of I = 0..N-1 test
// each file once between them (the CI runs them at once).
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const k = process.argv.indexOf("--examples");
  const most = k > 0 ? Number(process.argv[k + 1]) : Infinity;
  const [shard, shards] = (process.env.PARITY_SHARD ?? "0/1").split("/").map(Number);
  assert.ok(Number.isInteger(shard) && Number.isInteger(shards) && shard >= 0 && shard < shards,
    `PARITY_SHARD=${process.env.PARITY_SHARD}: I/N with 0 <= I < N`);
  const mine = (_, i) => i % shards === shard;
  const dir = t.tmpdir("parity");
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  let bad = 0;
  let widest = 1;
  let n = 0;
  try {
    const p = await t.page(b, `${url}?storage=memory&noapp=1`);
    await p.waitForFunction(() => window.forgecad?.fc, null, { timeout: 120000 });
    // THREADED=1: the threaded artifact's kernel (std with atomics, the spin-locked heap) is the one
    const threaded = await p.evaluate(() => window.forgecad.threaded);
    if (process.env.THREADED) assert.equal(threaded, true, "the threaded build isn't the one running");
    console.log(`the ${threaded ? "threaded" : "single-threaded"} build`);
    // the bench parts: natively written to a folder, in the page to its filesystem
    const benchDir = path.join(dir, "bench");
    const nat = t.native(["--bench", benchDir]);
    const pageFiles = await p.evaluate(() => window.forgecad.fc.bench_load("/bench"));
    const examples = fs.readFileSync(path.join(app, "examples/list.txt"), "utf8").trim().split("\n")
      .filter((f) => f.endsWith(".fg")).slice(0, most);
    const all = [
      ...pageFiles.map((f) => [f, path.join(benchDir, path.basename(f))]),
      ...examples.map((f) => [`/project/examples/${f}`, path.join(t.repo, "examples", f)]),
    ];
    const pairs = all.filter(mine);
    const rest = pairs.map(([, nf]) => nf).filter((nf) => !(nf in nat));
    if (rest.length) Object.assign(nat, t.native(rest));
    if (shards > 1) console.log(`# shard ${shard}/${shards}: ${pairs.length} of ${all.length} files`);
    for (const [pf, nf] of pairs) {
      // on a worker (the threaded build: the kernel's parallel paths on more workers)
      const got = JSON.parse(
        await p.evaluate(async (f) => {
          const fc = window.forgecad.fc;
          const id = fc.build_start(f);
          for (;;) {
            const r = fc.build_result(id);
            if (r !== undefined) return r;
            await new Promise((ok) => setTimeout(ok, 5));
          }
        }, pf),
      );
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
  } finally {
    await b.close();
    proc.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  }
  console.log(`${n - bad} of ${n} the same (the kernel's parallel paths on up to ${widest} threads)`);
  if (process.env.THREADED) assert.ok(widest > 1, "the threaded build's kernel ran on one thread");
  assert.equal(bad, 0);
})().catch((e) => {
  console.error(`not ok - ${e.stack ?? e}`);
  process.exit(1);
});
