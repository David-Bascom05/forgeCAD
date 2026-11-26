# CAM-PERF handoff

CAM-PERF's brief was to find the CAM's most expensive time costs and cut them without weakening
any check. Everything below is as of main 10d87778 (train 8).

## Landed in main

- **Memos.**
  - forge-memo crate (`kernel/forge-memo`, in-process, full Debug-text keys).
  - Adaptive memo (`adaptive/1`): rerun 6.0 s -> 0.013 s.
  - Turning certify memo (`turn-certify/1`): hub_turn rerun 0.37 s -> 25 ms.
  - Quality memo over `CamView` (`quality/1`).
- **Other CAM speed-ups.**
  - Parallel post, with `memo::context` carried into par closures.
  - Engrave frames prepared once.
  - Builtin machines parsed once.
- **Threaded wasm allocator.**
  - Heaps quick-look first (c0c6143f).
  - Per-thread small-block caches (34d6cad8), now OFF: see below.
- **Ledger.** `research/cam/speed-ledger.md` has the identical-output rows and the "Temporary
  safety exceptions" row for the caches being off.

The page rerun went from about 0.45 s to about 0.126 s.

## Unfinished: the threaded-page crash, and the per-thread caches

**State.** `THREAD_CACHE = false` in `kernel/forge-alloc/src/lib.rs` (on main since #59). Every
small block goes through the eight spin-locked heaps again, which costs about 20 ms of a threaded
manifold rerun. No output depends on it.

**The fault.**
- Two workers died with "memory access out of bounds" in `GlobalAlloc::realloc`, at the same
  build (#28), with memory at 62-78 MiB.
- Seen twice in CI, both with the caches on:
  - #56, parity shard 1/3, rack.fg;
  - #58, parity shard 2/3, rack_nopanels.fg.
- Never reproduced locally in about 20 runs:
  - shards 1 and 2;
  - 1, 2 and 4 CPUs;
  - a 256 KiB worker stack;
  - caches on and off;
  - Playwright 1.63.0 / Chromium 158, the same as CI.
- Since #59, TRAIN-CONDUCTOR has been watching CI with the caches off. As of this handoff no
  recurrence has been reported.

**Ruled out, with the reason for each:**
- **Worker stack overflow.** wasm-bindgen 0.2.130 gives workers 2 MiB
  (`DEFAULT_THREAD_STACK_SIZE = 1 << 21`). The single-threaded build runs the same builds, par
  jobs inline, on a 1 MiB main stack, and passes.
- **TLS before it exists.** `cache()` returns None while `__tls_base` is 0. A new worker's stack
  and TLS block come from the heaps directly. `__wasm_init_tls` copies the whole 2656-byte
  template.
- **Thread teardown.** `__wbindgen_thread_destroy` is never called, so no cache is orphaned.
- **forge-par / the pool.**
  - `scope`'s live count falls only after everything `'scope` is dropped.
  - The pool's jobs are popped under its Mutex.
  - No other `static mut`, `unsafe impl Sync`, or unsafely shared state was found in the
    workspace outside forge-alloc.
- **Stale per-worker memory size after another worker's `memory.grow`.** This would show only in
  V8's bounds-checked mode. Parity shard 1/3 with `--js-flags=--no-wasm-trap-handler`, cache-on
  build: 97/97 clean. Rerun time rose from about 12 s to 17.6 s, so the flag did take effect.
  This is not disproved, only unreproduced: the window may be too narrow to hit.
- **Heap logic, read twice.**
  - Every heap op runs under its shard lock.
  - A page's owner is set before any block of it is handed out, and never changes.
  - Big and small blocks keep to their heaps in `realloc`; a cross-heap move goes through
    alloc + copy + dealloc.

**On main since train 8 (from `alloc/threaded-stress`).**
- The threaded allocator takes its memory through `Mem { base, grow }`, so it runs natively over
  an arena.
- `tests::threaded_stress` is a native multi-threaded alloc/realloc/free stress.
  - Env vars: `FORGE_ALLOC_STRESS_THREADS`, `FORGE_ALLOC_STRESS_STEPS`.
  - It is gated off wasi, which has no threads.
  - It fails if `owner_of` is mutated to read without the lock.
- Base for any further hunting. The wasm build is unchanged.

**Next steps, if someone picks this up.**
1. If CI stays clean with the caches off for many runs, the caches are implicated. Extend
   `threaded_stress` to drive the cache paths natively: batch fill (`small`), cross-thread frees
   and `shed`, and small->small realloc across classes. That means letting `cache()` work off
   wasm, behind the test. Then look for a reproduction.
2. If it recurs with the caches off, the shared heaps or the engine are implicated. Get the
   failing address from CI: add a `console.error` of the pointer and size in a realloc wrapper on
   the threaded build. Also check whether it lies beyond the faulting thread's `memory.size`.
3. Turn the caches back on (`THREAD_CACHE = true`) only with a root-cause fix plus a test that
   reproduced the fault. That is the ledger row's condition.

**Risks.** While the caches stay off, the threaded page is about 20 ms slower per rerun, and that
is the only cost. If the fault is in the shared heaps (step 2), it can still happen with the caches
off. The CI watch exists to find that out.
