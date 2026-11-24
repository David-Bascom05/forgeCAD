// The UI audit in the page (tests/ui_audit is the native crawl and has the judge): every ribbon
// command the app's own registry lists (`audit.tools` in the state, `app_audit`), on every tab of
// the same documents, clicked with `app_click`. Each click is classed as natively (a popup, a
// panel, an environment, the page's file picker or download, a status line, a change in the view,
// or nothing visible, a panic) so research/ui-audit/report.md can put the two side by side.
//   node web/tests/ui_audit.cjs OUT [RECORDS_DIR]   (OUT: web/build.sh's, served static)
// UI_AUDIT_ONLY=plate,cam narrows the documents. Writes RECORDS_DIR/records-web.tsv.
const path = require("node:path"), http = require("node:http"), fs = require("node:fs");
const t = require("./lib.cjs");
const root = process.argv[2];
const outDir = process.argv[3] ?? t.tmpdir("ui-audit-web");
if (!root) throw new Error("usage: node web/tests/ui_audit.cjs OUT [RECORDS_DIR]");

const PLATE = `shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
plate = extrude(plan, t)
`;
// the native crawl's documents (tests/ui_audit/main.rs FIXTURES), by the same names
const FIXTURES = [
  { name: "plate", file: null, tabs: [] },
  { name: "part", file: "demo/part_tools.fg", tabs: ["3D Model"] },
  { name: "assembly", file: "sheetmetal/tray_asm.fg", tabs: [] },
  { name: "sheet", file: "sheetmetal/load_tray.fg", tabs: ["Sheet Metal"] },
  { name: "cam", file: "machined/bracket_cam.fg", tabs: ["Manufacture"] },
  { name: "weld", file: "weldments/table_frame.fg", tabs: ["Weldments"] },
  { name: "harness", file: "harness/loom.fg", tabs: ["Harness"] },
];
const COMMON = new Set(["View", "Inspect", "Manage"]);

const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
const server = http.createServer((q, r) => {
  const u = decodeURIComponent(new URL(q.url, "http://x").pathname);
  const f = path.join(root, u === "/" ? "index.html" : u);
  if (!fs.existsSync(f) || fs.statSync(f).isDirectory()) {
    r.writeHead(404);
    return r.end();
  }
  r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" });
  fs.createReadStream(f).pipe(r);
});

const W = 1400, H = 860;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

/// The frame's pixels changed since the last read, by region (ribbon, view, the rest).
async function changed(p) {
  return p.evaluate(async ({ W, H }) => {
    const f = await window.forgecad.pixels();
    const a = f.rgba, b = window.audit_last;
    window.audit_last = a;
    const n = { ribbon: 0, view: 0, rest: 0 };
    if (!b || b.length !== a.length) return n;
    for (let y = 0; y < f.height; y += 2) {
      for (let x = 0; x < f.width; x += 2) {
        const i = (y * f.width + x) * 4;
        const d = Math.max(Math.abs(a[i] - b[i]), Math.abs(a[i + 1] - b[i + 1]), Math.abs(a[i + 2] - b[i + 2]));
        if (d > 24) {
          if (y < 150) n.ribbon += 4;
          else if (y < f.height - 28 && x > 270) n.view += 4;
          else n.rest += 4;
        }
      }
    }
    return n;
  }, { W, H });
}

async function state(p) {
  return p.evaluate(() => window.forgecad.state());
}

/// A state's frames, and the worker's builds, settled.
async function settle(p, st0) {
  const s1 = await t.until(p, (s) => s.frames >= (st0?.frames ?? 0) + 6, "frames", 30000);
  return t.until(p, (s) => (s.done ?? 0) >= (s.seq ?? 0) && s.frames >= s1.frames + 4, "settled", 60000).catch(() => s1);
}

const layersOf = (s) =>
  (s.audit?.layers ?? []).filter((l) => !["Tooltip", "Debug", "Background"].includes(l[0]) && l[2][2] * l[2][3] > 16);
const sig = (s) =>
  JSON.stringify([s.env, s.tab, s.seq, s.dirty, layersOf(s).map((l) => l[1]).sort(), (s.audit?.places ?? []).map((x) => x[0]).filter((k) => !k.startsWith("view:")).sort()]);

async function open(b, base, fx) {
  const p = await t.page(b, base);
  await t.until(p, (s) => s.ready, "ready");
  await p.evaluate(() => window.forgecad.loaded);
  await p.evaluate(() => {
    window.forgecad.fc.app_audit(true);
    // the page's host: its picker and downloads, counted
    window.audit_host = [];
    for (const k of ["forge_pick", "forge_download", "forge_save_as", "forge_export"]) {
      const was = window[k];
      window[k] = (...a) => {
        window.audit_host.push(k);
        return was?.(...a);
      };
    }
  });
  await p.evaluate(({ file, text }) => {
    if (text) {
      window.forgecad.fc.fs_write("/project/audit/plate.fg", new TextEncoder().encode(text));
      window.forgecad.fc.app_open("/project/audit/plate.fg");
    } else {
      window.forgecad.fc.app_open(`/project/examples/${file}`);
    }
  }, { file: fx.file, text: fx.file ? null : PLATE });
  const want = fx.file ? path.basename(fx.file) : "plate.fg";
  await t.until(p, (s) => s.path?.endsWith(want) && s.done === s.seq && s.built?.checked, `${want} built`, 300000);
  await settle(p, await state(p));
  return p;
}

const tabs = (s) =>
  (s.audit?.places ?? []).filter((x) => x[0].startsWith("tab:")).sort((a, b) => a[1][0] - b[1][0]).map((x) => x[0].slice(4));

async function goTab(p, tab) {
  const s0 = await state(p);
  await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), `tab:${tab}`);
  await settle(p, s0);
}

function classify(b, a, host, px, problems) {
  if (problems.length) return ["CRASH", problems[0].slice(0, 160)];
  if (host.length) return ["HOST", host.join(", ")];
  if (a.env !== b.env) return ["WORKSPACE", `into ${a.env ?? "the design"}`];
  const bl = new Set(layersOf(b).map((l) => l[1]));
  const nl = layersOf(a).filter((l) => !bl.has(l[1]));
  if (nl.length) return ["DIALOG", `${nl.length} popup(s)`];
  if (a.tab !== b.tab) return ["PANEL", `switches to tab ${a.tab}`];
  const bp = new Set((b.audit?.places ?? []).map((x) => x[0]));
  const np = (a.audit?.places ?? []).filter((x) => !bp.has(x[0]) && x[1][2] * x[1][3] > 4000);
  if (np.length) return ["PANEL", `panel ${np.slice(0, 3).map((x) => x[0]).join(", ")}`];
  if (a.seq !== b.seq || a.dirty !== b.dirty) return ["ACTION", "the document changed"];
  if (a.status !== b.status && a.status.trim()) return ["ACTION", `status: "${a.status.slice(0, 60)}"`];
  if (px.view > 200) return ["ACTION", `the view changed (${px.view} px)`];
  if (px.ribbon > 30) return ["MODE", "the button stays on"];
  return ["NOTHING VISIBLE", "no popup, panel, status line or change in the view"];
}

server.listen(0, async () => {
  const base = `http://127.0.0.1:${server.address().port}/?offscreen=${W}x${H}&storage=memory`;
  const b = await t.browser();
  const only = process.env.UI_AUDIT_ONLY?.split(",");
  const rows = ["fixture\tenv\ttab\tgroup\ttool\tclass\tdetail\tfindings\tshot\tthumb\tms"];
  const seen = new Set();
  for (const fx of FIXTURES.filter((f) => !only || only.includes(f.name))) {
    console.log(`== ${fx.name}`);
    let p = await open(b, base, fx);
    for (const tab of tabs(await state(p))) {
      if (fx.tabs.length && !fx.tabs.includes(tab)) continue;
      if (tab === "Environments") continue; // entered by name natively; here the tiles are judged there
      const key = COMMON.has(tab) ? `common:${tab}` : `${fx.name}:${tab}`;
      if (seen.has(key)) continue;
      seen.add(key);
      await goTab(p, tab);
      const tools = (await state(p)).audit?.tools ?? [];
      for (const [group, tool, live] of tools) {
        const T0 = Date.now();
        let cls, detail;
        if (!live) {
          [cls, detail] = ["DISABLED", ""];
        } else {
          await changed(p);
          const before = await state(p);
          p.problems.length = 0;
          await p.evaluate(() => (window.audit_host.length = 0));
          await p.evaluate((n) => window.forgecad.fc.app_click(n, 0, false), `tool:${tool}`);
          let after = before;
          try {
            after = await settle(p, before);
          } catch (e) {
            p.problems.push(String(e.message).slice(0, 200));
          }
          const host = await p.evaluate(() => window.audit_host.slice()).catch(() => []);
          const px = p.problems.length ? { view: 0, ribbon: 0 } : await changed(p);
          [cls, detail] = classify(before, after, host, px, p.problems.slice());
          // back where it began, or the document again
          if (cls === "CRASH" || sig(after) !== sig(before)) {
            await p.evaluate(() => window.forgecad.fc.app_env("exit")).catch(() => {});
            await pause(300);
            const now = await state(p).catch(() => null);
            if (!now || sig(now) !== sig(before)) {
              await p.close();
              p = await open(b, base, fx);
              await goTab(p, tab);
            }
          }
        }
        const line = [fx.name, "", tab, group, tool, cls, detail, "", "", "", Date.now() - T0].map((x) => String(x).replace(/[\t\n]/g, " "));
        rows.push(line.join("\t"));
        console.log(`  ${fx.name.padEnd(10)} ${`${tab}/${tool}`.padEnd(32)} ${cls.padEnd(16)} ${detail}`);
      }
    }
    await p.close();
  }
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, "records-web.tsv"), rows.join("\n") + "\n");
  console.log(`${rows.length - 1} commands; ${path.join(outDir, "records-web.tsv")}`);
  await b.close();
  server.close();
});
