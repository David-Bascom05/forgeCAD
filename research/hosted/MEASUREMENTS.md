# Hosted forgeCAD: measurements

Everything here was measured on 2026-11-25 in one cloud VM (4 vCPU, 15 GB, ext4 on a virtual
disk `/dev/vda`, Linux 6.18), from `origin/main` at 50af9c42. Scripts and raw outputs are in this
directory; each section names them. Nothing measured here touches the product: the Rust
prototype `proto/` is its own Cargo workspace (copied lockfile) and is not linked into any gate
binary.

**Caveat for every timing:** one VM, one run per configuration unless stated, other jobs
sometimes running on other cores (each timed run pinned with `taskset` to its own core where
noted). Ratios between configurations are more trustworthy than absolute values. The disk is
virtual: its `fdatasync` (0.125 ms p50) is far faster than typical network block storage and
may be acknowledged from the host's cache, so absolute durability latencies are a **lower
bound**; the ranking of the candidates is what this section shows.

## (a) Bit-identical geometry across builds

Tool: `proto/src/bin/det.rs`. For each of the 276 `.fg` files under `examples/`, one cold
`forge_lang::run` with default options. Then two hashes:
- **body**: every vertex point, every edge range, and the face count of the finished body, plus
  the kinds of the errors (the `bits` example's hash extended with errors);
- **mesh**: every position and triangle of `tessellate` at the default quality (chord 0.01 mm,
  15°).

| build | where it ran | files | body differs | mesh differs | cold time vs native v3 (median, files > 20 ms, n=82) |
|---|---|---|---|---|---|
| native x86-64-v3 (AVX2; the shipped native build) | VM, all cores | 276 | reference | reference | 1.00 |
| native x86-64-v2 (no AVX) | VM, 1 core | 276 | **0** | **0** | 1.28 (p10 0.96, p90 2.30) |
| native v3, pinned to one core (`forge_math::par` runs every job in turn) | VM, 1 core | 276 | **0** | **0** | 1.01 (p10 0.34, p90 1.52) |
| wasm32-wasip1, `--profile wasm` (+simd128), Node 22.23.5 (V8) | VM | 276 | **0** | **0** | 1.31 (p10 1.05, p90 2.84) |
| the same `.wasm` in Chromium (HeadlessChrome 158.0.8127.36, Playwright's), in-memory WASI (`browser/run-wasi-chromium.cjs`) | VM | 276 | **0 of the 223 that loaded**; 53 did not load their files | 0 of 223 | 0.99 (p90 1.88) |

The 53 Chromium mismatches are all one cause, shown by their error texts
(`data/det-53-errors-native.txt` vs `data/det-53-errors-chromium.txt`): under the minimal
in-memory WASI shim the document's sibling files were not read ("File X: not found beside the
part file", "PartFile ../x.fg is not a Forge file this reads"): 4 parts that import a STEP or a
font, and 49 assemblies. The shim resolves `..` itself; the loader's other calls (UNKNOWN which:
probably canonicalize/readlink, which the shim lacks) fail. Not an arithmetic difference: in
none of the 53 did both sides build. Those files are covered in the real page, with the app's
own storage, by `web/tests/parity.cjs` in CI (measures, not hashes).

Data: `data/det-native-x86-64-v3.txt`, `data/det-native-x86-64-v2.txt`,
`data/det-native-v3-1core.txt`, `data/det-wasm-node.txt`, `data/det-wasm-chromium.txt`;
comparison `bench/compare_det.py`.

The 276 files include 73 that end in an error or are assemblies (no single body); their hash
covers the error kinds only. Assemblies' placed parts are not hashed by this tool.

What this settles and what it does not:
- **Settled here:** CPU feature level (v2 vs v3), the thread pool (one core vs four), and
  native vs wasm (V8) give the same bits for every example. This agrees with the repo's own
  checks: `tools/speed/wasm.sh` (`bits` example, native vs wasm under Node) and
  `web.yml`'s `parity.cjs` (page vs native measures, default and threaded artifacts).
- **Why it can hold by construction:** wasm scalar `f64` add, sub, mul, div and sqrt are
  correctly rounded (round to nearest, ties to even) by the spec, and only NaN bit patterns and
  relaxed-SIMD ops are nondeterministic (PRIOR-ART.md §6). The repo builds wasm with
  `+simd128` and without relaxed-simd (`.cargo/config.toml`), Rust never fuses `mul_add` on its
  own, and the kernel's float functions (sin, cos, ...) are its own code (`dmath`), not the host's.
- **Not tested here (UNKNOWN):** Firefox (SpiderMonkey), Safari (JavaScriptCore), ARM CPUs
  (Apple Silicon, phones), the threaded wasm artifact on this tool. How to find out: run
  `det.wasm` with `browser/run-wasi-chromium.cjs` ported to Playwright's `firefox` and `webkit`
  (needs those browsers installed, which this VM may not do), and on an ARM machine (native
  aarch64 `det` and wasm in Safari). Add it to CI as a matrix. The risk lies in NaN payloads
  reaching a hash or a branch; the spec allows those to differ between engines.

## (b) Rebuild per edit, and opening

Same tool, same files. Native x86-64-v3 (`data/det-native-x86-64-v3.txt`); for each file up
to three SetParam edits (each handle × 1.05), timed **warm** (the previous run's `Cache`, as the
app rebuilds) and then **cold-in-process** (an empty `Cache`, but the process's memos warm).

| quantity | n | p50 | p90 | p99 | max |
|---|---|---|---|---|---|
| cold build of a document (first run in the process) | 276 | 3.5 ms | 1,164 ms | 3,885 ms | 8,203 ms (rack18u/asm/rack.fg) |
| warm rebuild after one SetParam | 642 | 1.5 ms | 38 ms | 3,251 ms | 6,003 ms |
| tessellation of the finished body | 276 | 0.2 ms | 7.7 ms | 57 ms | 149 ms |
| mesh size (f32 position + normal per vertex, u32×3 per triangle) | 276 | 10.7 KB | 486 KB | 3.9 MB | 13.8 MB |

- 540 of 642 warm edits (84%) rebuilt within one 60 Hz frame (16 ms); 593 (92%) within 100 ms.
- warm-vs-cold bits: every warm rebuild's body hash equals the cold rebuild's of the same text
  (no `WARM_COLD_DIFFER` line). Determinism across the cache holds on this corpus.
- **Anomaly (a performance bug on the local path; not hosting-specific):** in some assemblies a
  warm rebuild is far slower than a fresh one. `examples/rack18u/hw/tests/hinge_pin.fg`,
  SetParam `size_m6` 6 → 6.3: warm 5.9–6.0 s, fresh 10 ms, the same 7 `ExpectFailed` errors
  both ways; `examples/rack18u/asm/post_kit.fg`, SetParam `m6` 6 → 6.3: warm 4.3 s, fresh 30 ms.
  Reproduced 3 times (`DET_WHY=1 det`). Both change a thread size to a non-standard one.
  Reported to the coordinator; root cause UNKNOWN (profile `asm.rs`'s warm path).
- Thermoform documents rebuild their wall simulation on every edit (cover.fg 1.8–2.1 s): heavy
  results, compute-worker candidates.

**The big open (Haas VF-2, the stress case).** Haas's own model `VF-2_STEP__1_2023.step` from
branch `haas-3d-models` (`vf-2_solid_models_1_2023.zip`): 59.4 MB STEP, 13.5 MB gzipped,
15,070 `ADVANCED_FACE`s, 2,638 `B_SPLINE_SURFACE`s.
- `forgecad build` of `m = import("vf2.step")`, native: **13.8 s wall**, 170 MB peak RSS
  (eval and build 10.2 s, measure 2.2 s). With `det`: 16.2 s cold, 7,984 faces.
- Meshing that import fails on main (`forgecad export vf2.fg stl`: "meshing failed: face";
  whole-body `tessellate_tolerant` also errs). The Haas import branches ship
  **decimated display meshes** instead (`forgecad machine model ship`): VF-2 = 4.22 MB of STL
  in six bodies (branch `machines/haas-models-vmc`, LFS pointer sizes); 13 machines 1.8–6.1 MB.
- wasm open of the VF-2: not run (UNKNOWN; at the measured 1.3× median it would be ~18 s plus
  the 13.5 MB download).

**Heavy results** (native, `forgecad` CLI, pinned to one core):
- bend simulation `query examples/sheetmetal/load_tray.fg bendsim`: 3.85 s, result 21.7 KB JSON;
  `rack_face.fg`: 0.48 s, 1.8 KB.
- CAM: not re-measured here (the CAM documents run through a different CLI path than
  `forge_lang::run`). CAM-PERF's numbers for the threaded page, rerun of `manifold_cam` after
  a cold run: worker 104 ms, page 0.123 s (commit 34d6cad8 on `cam/perf-rest`).

## (c) Concurrent ops: what order changes

Tool: `proto/src/bin/merge.rs`, data `data/merge-pairs.txt`. For each example that evaluates
cleanly (251 documents), the candidate ops a user could send against it: SetParam ×1.1 and
×0.9 on every handle; Rename (two different new names), DeleteFeature, MoveFeature (to the
end), and ReplaceFeature (the same statement plus a comment) on every top-level feature;
two AddFeatures at the end. Only ops valid alone on the base text S are kept. Then 60 random
pairs (a, b) per document, each applied **in both orders** with `apply_checked` (the op applied
to the text and the result evaluated, as `fg edit` does: exactly what a server re-validating
a late op against the new head would run).

Classes: **commute** = both orders apply and give the same text; **diverge** = both apply, the
texts differ; **refuse** = the second op is refused in that order.

Different targets (the ops name different features or params):

| pair | commute | diverge | refused in one order | refused both | total |
|---|---|---|---|---|---|
| SetParam + SetParam | 2,342 | 0 | 0 | 1 | 2,343 |
| Rename + SetParam | 2,703 | 0 | 32 | 0 | 2,735 |
| Rename + ReplaceFeature | 825 | 0 | 280 | 0 | 1,105 |
| Rename + Rename | 1,065 | 0 | 0 | 0 | 1,065 |
| ReplaceFeature + SetParam | 1,316 | 1 | 0 | 0 | 1,317 |
| AddFeature + Rename / ReplaceFeature / SetParam / DeleteFeature | 1,946 | 0 | 0 | 0 | 1,946 |
| DeleteFeature + Delete / Move / Rename / Replace / SetParam | 1,208 | 0 | 0 | 0 | 1,208 |
| MoveFeature + Rename / Replace / SetParam | 1,016 | 0 | 0 | 0 | 1,016 |
| ReplaceFeature + ReplaceFeature | 281 | 0 | 0 | 0 | 281 |
| AddFeature + AddFeature | 0 | 102 | 0 | 0 | 102 |
| AddFeature + MoveFeature | 0 | 132 | 0 | 0 | 132 |
| MoveFeature + MoveFeature | 0 | 32 | 0 | 0 | 32 |

- 96.9% of different-target pairs commute exactly.
- Every refusal is one pattern: a **Rename lands first and the other op still names the old
  name** (`UnknownParam`, `UnknownName`, `UnknownFeature`). The other order applies. The fix is
  to rewrite the late op through the Rename (substitute the name in its id, param key and text),
  as OT does for positions. That substitution is the same token-level renaming `edit_ops.rs`
  already does to the document, and its meaning is proven in `ForgeLang/RenamePart.lean` and
  `RenameDefs.lean`.
- Every divergence is a **position** question: two statements added or moved to the same place.
  Both apply; the order the server picks decides which comes first. Neither result is wrong, and
  every collaborator sees the same order (server order).
- The 1 ReplaceFeature+SetParam divergence: ReplaceFeature re-writes the statement's own params,
  which the SetParam also sets, so the two ops share a target in fact.

Same target (the ops name the same feature or param):

| pair | commute | diverge | refused in one order | refused both | total |
|---|---|---|---|---|---|
| SetParam + SetParam (different values) | 0 | 168 | 0 | 0 | 168 |
| Rename + Rename (different new names) | 0 | 0 | 0 | 171 | 171 |
| DeleteFeature + Rename | 0 | 0 | 0 | 136 | 136 |
| Rename + ReplaceFeature | 0 | 0 | 381 | 0 | 381 |
| MoveFeature + Rename | 0 | 0 | 116 | 0 | 116 |
| DeleteFeature + ReplaceFeature / MoveFeature | 0 | 0 | 132 | 0 | 132 |
| MoveFeature + ReplaceFeature | 0 | 51 | 0 | 0 | 51 |

Same-target SetParams are last-writer-wins (both apply, the later value stays). The others are
true conflicts: the second op is refused, with a typed error, nothing written.

"Refused both" for Rename+Rename and Delete+Rename means `apply_checked` refused the second op
in **both** orders: it names something the first removed. Under server order, the first is
kept and the second author is told.

**Proofs** (`lean/Merge.lean`, checked against `research/forge-lang/lean` with Lean
v4.36.0-rc2; axioms propext, Quot.sound, Classical.choice; no sorry):
- `setLit_comm`: SetParams on different sites commute;
- `setLit_lww`: on the same site the later wins and the earlier leaves nothing;
- `concurrent_setParam_traces`: if neither site is observed (no structural param), the merged
  document has the base run's traces, ids and control log: a concurrent pair of dimension edits
  can never change the feature list.

## (d) Op-log storage

Script `bench/oplog_bench.py`, results `bench/oplog-results.jsonl`. Workload from real op
shapes (SetParam, AddFeature, Rename, ReplaceFeature, MoveFeature JSON, ~150–250 B with author
and base version). Each append is durable before it returns. Python client (GIL: the concurrent
numbers are client-bound and understate the servers).

| candidate | append p50 / p99 (one writer) | 16 writers: ops/s, p99 | read 200 ops since seq (p50) | write 200 KB snapshot | open = latest snapshot + 200 ops (p50 / p99) |
|---|---|---|---|---|---|
| disk floor: `fdatasync` of a 150 B append | 0.125 / 0.295 ms | | | | |
| Postgres 16.15 (synchronous_commit=on, fsync=on), unix socket | 0.197 / 0.356 ms | 9,588, 4.2 ms | 0.225 ms | 3.9 ms | 1.15 / 2.64 ms |
| SQLite 3.53.1 WAL, synchronous=FULL (durable) | 0.156 / 0.271 ms | 6,801, 4.8 ms | 0.074 ms | 0.9 ms | 0.092 / 0.214 ms |
| SQLite WAL, synchronous=NORMAL (**not durable** on power loss) | 0.008 / 0.032 ms | 18,167, 7.3 ms | 0.075 ms | 0.14 ms | 0.091 / 0.139 ms |
| libSQL embedded (libsql-experimental wheel), WAL FULL | 0.167 / 0.262 ms | 6,779, 3.8 ms | 0.087 ms | 1.1 ms | 0.112 / 0.207 ms |
| append-only file per doc, fdatasync per append | 0.119 / 0.171 ms | 15,192, 8.1 ms | 0.175 ms | 0.71 ms | 0.197 / 0.289 ms |
| append-only file per doc, group commit (one flusher) | 0.128 / 0.241 ms | 7,015, 3.4 ms | 0.174 ms | 0.55 ms | 0.185 / 0.246 ms |
| Redis 7.0.15 streams, AOF appendfsync=always | 0.202 / 0.410 ms | 5,647, 7.8 ms | 0.708 ms | 1.3 ms | 0.867 / 1.42 ms |
| forge-history git save (today's per-edit history), `proto/src/bin/gitsave.rs` | **3.4 / 17.1 ms** (640 B doc); **5.2 / 21.3 ms** (85 KB doc) | | | | |
| FoundationDB 7.3 | UNKNOWN: running its downloaded server binary was refused in this sandbox | | | | |
| MongoDB 8.0 | UNKNOWN: same | | | | |
| object storage (S3 and kin) | UNKNOWN: needs an account; not measured | | | | |

How to settle the UNKNOWNs: run `oplog_bench.py` with an FDB and a MongoDB class (each a
local single-node install) on a machine where those servers may run; object storage only as
the snapshot tier, measured from the target region.

Reading: on this disk every durable choice costs ≤ 0.2 ms p50 per op, well below network
round-trip times (§ budgets in DESIGN.md), so storage choice does not decide latency. Git per
edit costs 20–30× more and its p99 is ~20 ms.

## Server CPU per op (validation)

Tool `proto/src/bin/valcost.rs`, data `data/valcost-native.txt` (231 documents with a handle,
median of up to 5 SetParams each, native, one core):

| step | p50 | p90 | p99 | max |
|---|---|---|---|---|
| `edit::apply` (the op on the text) | 0.21 ms | 1.7 ms | 11.5 ms | 13.2 ms |
| `edit::apply_checked` (apply + evaluate both texts, no kernel build) | 0.38 ms | 3.4 ms | 23.7 ms | 26.0 ms |

The worst are the rack's large documents (rack.fg, 85 KB: 23.7 ms).

## (e) Threaded wasm and forge-alloc

Read, not measured. See DESIGN.md §(e).
