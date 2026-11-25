// Runs forge-det's wasm32-wasip1 build in a browser engine through Playwright (or, with
// --engine shim, in Node through the same WASI shim, to check the shim against node:wasi), with
// the files under the given directories in memory at their real absolute paths, and prints its
// stdout. The program runs in a dedicated Worker (its own memory, as a hosted client builds).
//
//   node run.cjs --engine chromium|firefox|webkit|shim --wasm FORGE_DET.wasm --root REPO \
//        --dirs examples,verify/forge-det/streams [--threads N] -- ARGS..
// ARGS are forge-det's (the runner adds --root REPO). Exit: the program's exit code; 3 if the
// program called a WASI function the shim lacks (listed on stderr).
const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");

const argv = process.argv.slice(2);
const dash = argv.indexOf("--");
const opts = {};
for (let i = 0; i < (dash < 0 ? argv.length : dash); i += 2) opts[argv[i].replace(/^--/, "")] = argv[i + 1];
const progArgs = dash < 0 ? [] : argv.slice(dash + 1);
const root = path.resolve(opts.root || ".");
const files = [];
for (const d of (opts.dirs || "examples").split(",")) {
  const walk = (p) => {
    for (const e of fs.readdirSync(p, { withFileTypes: true })) {
      const q = path.join(p, e.name);
      if (e.isDirectory()) walk(q);
      else if (e.isFile()) files.push(q);
    }
  };
  walk(path.join(root, d));
}
// POSIX paths inside the program, whatever the host OS
const posix = (p) => "/" + path.relative("/", p).split(path.sep).join("/");
const progRoot = process.platform === "win32" ? "/repo" : root;
const inProg = (f) => (process.platform === "win32" ? "/repo/" + path.relative(root, f).split(path.sep).join("/") : posix(f));
const env = { FORGECAD_CACHE: "off" };
if (opts.threads) env.FORGECAD_THREADS = opts.threads;
const args = ["forge-det", "--root", progRoot, ...progArgs];

async function shim() {
  const { makeWasi } = await import(path.join(__dirname, "wasi.mjs"));
  const map = new Map(files.map((f) => [inProg(f), new Uint8Array(fs.readFileSync(f))]));
  const w = makeWasi({ args, env, files: map, stdout: (l) => console.log(l), stderr: (l) => console.error(l) });
  const mod = await WebAssembly.compile(fs.readFileSync(opts.wasm));
  const inst = await WebAssembly.instantiate(mod, w.imports);
  const code = w.start(inst);
  if (w.missing.size) { console.error("# missing WASI calls:", [...w.missing].join(" ")); process.exit(3); }
  process.exit(code);
}

const WORKER = `
import { makeWasi } from "/wasi.mjs";
onmessage = async (e) => {
  const { args, env } = e.data;
  const list = await (await fetch("/files.json")).json();
  const map = new Map();
  await Promise.all(list.map(async (f) => map.set(f, new Uint8Array(await (await fetch("/f" + f)).arrayBuffer()))));
  const w = makeWasi({ args, env, files: map, stdout: (l) => postMessage({ out: l }), stderr: (l) => postMessage({ err: l }) });
  const mod = await WebAssembly.compileStreaming(fetch("/prog.wasm"));
  const inst = await WebAssembly.instantiate(mod, w.imports);
  let code;
  try { code = w.start(inst); } catch (x) { postMessage({ err: "trap: " + x }); code = 101; }
  postMessage({ done: code, missing: [...w.missing] });
};`;

async function browser(engine) {
  const pw = require("playwright");
  const byPath = new Map(files.map((f) => [inProg(f), f]));
  const srv = http.createServer((req, res) => {
    const u = decodeURIComponent(req.url.split("?")[0]);
    const send = (type, body) => { res.setHeader("content-type", type); res.end(body); };
    if (u === "/") return send("text/html", "<!doctype html><title>forge-det</title>");
    if (u === "/worker.mjs") return send("text/javascript", WORKER);
    if (u === "/wasi.mjs") return send("text/javascript", fs.readFileSync(path.join(__dirname, "wasi.mjs")));
    if (u === "/files.json") return send("application/json", JSON.stringify([...byPath.keys()]));
    if (u === "/prog.wasm") return send("application/wasm", fs.readFileSync(opts.wasm));
    if (u.startsWith("/f/") && byPath.has(u.slice(2))) return send("application/octet-stream", fs.readFileSync(byPath.get(u.slice(2))));
    res.statusCode = 404; res.end();
  });
  await new Promise((r) => srv.listen(0, "127.0.0.1", r));
  const b = await pw[engine].launch();
  const page = await b.newPage();
  let code = 1, missing = [];
  const done = new Promise((resolve) => {
    page.exposeFunction("emit", (m) => {
      if (m.out !== undefined) console.log(m.out);
      else if (m.err !== undefined) console.error(m.err);
      else if (m.done !== undefined) { code = m.done; missing = m.missing; resolve(); }
    });
  });
  page.on("console", (m) => { if (m.type() === "error") console.error("page:", m.text()); });
  page.on("pageerror", (e) => console.error("pageerror:", e));
  await page.goto(`http://127.0.0.1:${srv.address().port}/`);
  console.error("# " + (await page.evaluate(() => navigator.userAgent)));
  await page.evaluate(({ args, env }) => {
    const w = new Worker("/worker.mjs", { type: "module" });
    w.onmessage = (e) => window.emit(e.data);
    w.onerror = (e) => window.emit({ err: "worker error: " + e.message });
    w.postMessage({ args, env });
  }, { args, env });
  await done;
  await b.close();
  srv.close();
  if (missing.length) { console.error("# missing WASI calls:", missing.join(" ")); process.exit(3); }
  process.exit(code);
}

(opts.engine === "shim" ? shim() : browser(opts.engine || "chromium")).catch((e) => { console.error(e); process.exit(1); });
