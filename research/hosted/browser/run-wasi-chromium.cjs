// Runs a wasm32-wasip1 program inside Chromium (the preinstalled Playwright) with an in-memory
// WASI filesystem holding the given directories at their real absolute paths, and prints its
// stdout. The same .wasm under Node (tools/wasi-run.mjs) and here must print the same lines.
//   node run-wasi-chromium.cjs PROG.wasm DIR[,DIR..] ARGS..
const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("playwright");

const [wasm, dirs, ...args] = process.argv.slice(2);
const files = [];
for (const d of dirs.split(",")) {
  const walk = (p) => {
    for (const e of fs.readdirSync(p, { withFileTypes: true })) {
      const q = path.join(p, e.name);
      if (e.isDirectory()) walk(q);
      else if (e.isFile()) files.push(q);
    }
  };
  if (fs.statSync(d).isDirectory()) walk(d); else files.push(d);
}
const shim = path.join(__dirname, "node_modules/@bjorn3/browser_wasi_shim/dist");
const srv = http.createServer((req, res) => {
  const u = decodeURIComponent(req.url.split("?")[0]);
  let f;
  if (u === "/") { res.setHeader("content-type", "text/html"); return res.end("<html><body>wasi</body></html>"); }
  if (u === "/prog.wasm") f = wasm;
  else if (u === "/files.json") { res.setHeader("content-type", "application/json"); return res.end(JSON.stringify(files)); }
  else if (u.startsWith("/shim/")) { f = path.join(shim, u.slice(6)); res.setHeader("content-type", "text/javascript"); }
  else if (u.startsWith("/f/")) f = u.slice(2);
  if (!f || !fs.existsSync(f)) { res.statusCode = 404; return res.end(); }
  if (f.endsWith(".wasm")) res.setHeader("content-type", "application/wasm");
  fs.createReadStream(f).pipe(res);
});
srv.listen(0, "127.0.0.1", async () => {
  const url = `http://127.0.0.1:${srv.address().port}/`;
  const b = await chromium.launch();
  const p = await b.newPage();
  p.on("console", (m) => { if (m.type() === "error") console.error("page:", m.text()); });
  await p.goto(url);
  const out = await p.evaluate(async ({ args }) => {
    const S = await import("/shim/index.js");
    const list = await (await fetch("/files.json")).json();
    const root = new Map();
    for (const f of list) {
      const parts = f.split("/").filter(Boolean);
      let m = root;
      for (const d of parts.slice(0, -1)) {
        if (!m.has(d)) m.set(d, new S.Directory(new Map()));
        m = m.get(d).contents;
      }
      const buf = new Uint8Array(await (await fetch("/f" + f)).arrayBuffer());
      m.set(parts[parts.length - 1], new S.File(buf));
    }
    const lines = [];
    let errs = "";
    const fds = [
      new S.OpenFile(new S.File([])),
      S.ConsoleStdout.lineBuffered((l) => lines.push(l)),
      S.ConsoleStdout.lineBuffered((l) => (errs += l + "\n")),
      new S.PreopenDirectory("/", root),
    ];
    const w = new S.WASI(["prog", ...args], [], fds);
    const mod = await WebAssembly.compileStreaming(fetch("/prog.wasm"));
    const inst = await WebAssembly.instantiate(mod, { wasi_snapshot_preview1: w.wasiImport });
    let code;
    try { code = w.start(inst); } catch (e) { errs += String(e); }
    return { lines, errs, code, ua: navigator.userAgent };
  }, { args });
  console.error("# " + out.ua + " exit=" + out.code);
  if (out.errs) console.error(out.errs.slice(0, 2000));
  for (const l of out.lines) console.log(l);
  await b.close();
  srv.close();
});
