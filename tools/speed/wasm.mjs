// `n` regens of a part in one wasm module under Node's WASI: wasm.sh counts the difference
// between n=5 and n=1, so start-up and compile drop out.
// usage: node --no-turbo-fast-api-calls --no-liftoff --no-wasm-tier-up --no-wasm-lazy-compilation tools/speed/wasm.mjs <regen1.wasm> <n> <part> <path>
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const [file, n, ...args] = process.argv.slice(2);
const module = await WebAssembly.compile(await readFile(file));
for (let i = 0; i < Number(n); i++) {
  const wasi = new WASI({ version: "preview1", args: ["regen1", ...args], env: process.env, returnOnExit: true });
  if (wasi.start(new WebAssembly.Instance(module, wasi.getImportObject())) !== 0) process.exit(1);
}
