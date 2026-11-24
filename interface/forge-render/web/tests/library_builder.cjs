// The Library Builder in the browser (docs/ux/STANDARD.md §10/§11): in its environment over a
// part kept by `forgecad web`, each kind's record shown as the native test shows it (a cutting
// tool with its helix, a holder, a vise and a chuck with what their records don't give as
// ghosts, a punch, a die with its section inset, a press brake), at 1400×860 and at 1024×700.
//   node web/tests/library_builder.cjs OUT   (OUT: web/build.sh's output)
//   FORGE_WEB_SHOTS=DIR keeps the pictures.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const t = require("./lib.cjs");

const PART = `shared L = 60
shared W = 30
shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
`;

async function machine(p, cmd) {
  await p.evaluate((c) => window.forgecad.fc.app_machine(c), cmd);
  const st = await t.until(p, (s) => (s.status ?? "").startsWith(`machine: [${cmd}] `), `machine ${cmd}`, 120000);
  await Promise.race([t.frames(p, st, 3).catch(() => null), new Promise((r) => setTimeout(r, 3000))]);
  return st.status.slice(`machine: [${cmd}] `.length);
}

const RECORDS = [
  ["tool", "harvey-792208", "tool-helix"],
  ["tool", "flat-10", "tool-flat"],
  ["holder", null, "holder"],
  ["workholding", "haas-vise-05-0404", "vise"],
  ["workholding", "kitagawa-b208", "chuck"],
  ["punch", "trumpf/OW223-K", "punch"],
  ["die", "trumpf/EV-S-W4-30deg", "die"],
  ["press", "trumpf-trubend-5085", "press"],
  ["machine", "generic-trunnion-bc", "machine"],
];

(async () => {
  const app = path.resolve(process.argv[2] ?? "web/out");
  const dir = t.tmpdir("library-builder");
  fs.writeFileSync(path.join(dir, "plate.fg"), PART);
  const { url, proc } = await t.serve(dir, app);
  const b = await t.browser();
  try {
    for (const size of ["1400x860", "1024x700"]) {
      const p = await t.page(b, `${url}?open=plate.fg&offscreen=${size}`);
      await t.until(p, (s) => s.built?.checked, "the first build");
      await t.loaded(p);
      let s = await machine(p, "open");
      for (const [lib, rec, name] of RECORDS) {
        s = await machine(p, `lib ${lib}`);
        if (rec) s = await machine(p, `rec ${rec}`);
        if (rec) assert.match(s, new RegExp(`^${rec.replace(/[/.]/g, "\\$&")}`), s);
        const f = await t.pixels(p);
        console.log(`ok - ${lib} ${rec ?? "(first)"} at ${size} (${t.shot(`web-${size}-${name}.png`, f)})`);
      }
      t.check(p);
      await p.close();
    }
  } finally {
    await b.close();
    proc.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
