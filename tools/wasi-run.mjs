#!/usr/bin/env -S node --no-turbo-fast-api-calls
// Runs a wasm32-wasip1 test binary under Node's WASI, so `cargo test --target
// wasm32-wasip1` runs the same tests in the same arithmetic a browser uses.
// Node 22's fast fd_write can start a GC mid-call and segfault; the flag keeps it on the slow path.
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";
import process from "node:process";

const [file, ...args] = process.argv.slice(2);
const wasi = new WASI({
  version: "preview1",
  args: [file, ...args],
  env: process.env,
  preopens: { "/": "/" },
  returnOnExit: true,
});
const module = await WebAssembly.compile(await readFile(file));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
