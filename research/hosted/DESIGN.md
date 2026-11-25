# Hosted, multi-user forgeCAD: design

Status: design only, 2026-11-25, branch `design/hosted`. Phase 1 is built (HOSTED-BUILD); §17
adds phase 2. No product code changed. Every number
comes from `MEASUREMENTS.md` (measured here) or `PRIOR-ART.md` (cited sources, fetched
2026-11-25). Where neither settles a question, it says **UNKNOWN** and how to find out.

## 0. The answer in one page

| choice | decision | the evidence that decides it |
|---|---|---|
| sync model | **server-ordered log of typed ops; a late op is re-applied to the head (rebase) and re-validated; Rename rewrites later ops' names; same-param edits are last-writer-wins** | 251 real documents, ~19k op pairs both orders: 96.9% of different-target pairs commute exactly; every non-commuting case is Rename-before-a-user-of-the-old-name (fixable by substitution) or list position (both apply, server order decides). Lean: SetParam commute and LWW proven (`lean/Merge.lean`). |
| where geometry is computed | **hybrid: every client rebuilds the document locally (as today); compute workers produce heavy, slow-changing results (imports, bend sim, thermoform, CAM, exports, first-open meshes), cached by content hash** | warm rebuild after an edit is 1.5 ms p50 / 38 ms p90 locally, below any network round trip; 0 of 276 documents differ in body or mesh bits across x86-64-v3, v2, one core, wasm/Node and wasm/Chromium (223 that loaded); but the Haas VF-2 takes 13.8 s to open natively and its STEP cannot be meshed on main, and bend sim takes 3.85 s for a 21.7 KB result. |
| op-log store | **Postgres** (op log, documents, permissions in one transactional store; LISTEN/NOTIFY for fan-out between servers); **content-addressed blobs** (snapshots, meshes, STEP, results) on a filesystem or object store | measured on one disk: durable append p50 Postgres 0.24 ms, SQLite 0.19, MongoDB 0.45, FoundationDB 1.02; change feed p50/p99 Postgres 0.40/0.85 ms, MongoDB 1.08/2.27, FoundationDB 2.02/12.3; Postgres is needed anyway for accounts and sharing and lets permission check + append be one transaction |
| fan-out between servers | **in the sequencer's own process** (it holds every connected client of its document); a store's change feed only for other servers' read replicas | Postgres LISTEN/NOTIFY 0.40 ms p50, enough for that |
| determinism | **a requirement, enforced**: every client gets bit-identical geometry from the same op stream; a standing cross-platform determinism gate (§13) | 0 differences measured on every platform this VM can run; sources of divergence listed and closed one by one in §13 |
| desktop app | **a full client of the hosted server** in SERVED mode; LOCAL mode (any folder, plain files + git, no network) unchanged and never slowed (§14–§15) | one protocol crate shared by native forge-render and the wasm build |
| history and git | **op seq = the live history (microversions); git stays the version and interchange layer**: a named version or an idle checkpoint becomes a forge-history commit and tag | forge-history save per edit: 3.4–5.2 ms p50, 17–21 ms p99 = 20–30× an op append; git commit per keystroke-level op would also be the wrong granularity for versions. |
| transport | **WebSocket** for ops and presence; **HTTPS GET** of content-addressed blobs (CDN-cacheable) | WebSocket works in every browser; WebTransport is "Baseline 2026" (Safari 26.4, March 2026). Ops are ~150–250 B. |
| offline / local | **unchanged**: local forgeCAD keeps files, `F.fg.log`, forge-history; the hosted code lives in new crates and is never linked into the gate binaries | the speed gate's binaries (regen1, bench) do not need a line of the sync code. |
| phase 2 | **§17**: Postgres with a fenced, role-checked append (0.40 ms p50); synchronous standby; managed failover; WAL archive + PITR; jobs by SKIP LOCKED; server CPU-bound at 0.8 ms/op after two memory bugs found and fixed | `phase2_pg.py`, `phase2_ha.py`, `server_load.rs`; cited prices in PRIOR-ART-PHASE2 |
| agents | **(1) local Claude Code bridge (user's own plan) talking MCP to the hosted document as that user; (2) bring-your-own API key; (3) operator API key, optional.** Never route users' Pro/Max credentials through our server | Anthropic: third parties may not offer claude.ai login or route plan credentials; the unmodified Claude Code binary signed into by the user is allowed (PRIOR-ART §10). |

The coordinator's hypothesis ("the server orders and stores typed ops and does git; clients
rebuild; compute workers produce heavy shared results cached by content hash") **survives the
tests, with four changes**: (i) git is per version, not per op; (ii) the server must be able to
build any document itself (agents, exports, thumbnails, weak clients, divergence checks), so it
is a client too; (iii) clients report their result hash so divergence is detected, not assumed
away; (iv) Rename needs an op transform, everything else needs only re-validation.

## 1. What exists today (read before designing)

- **Typed ops** (`interface/forge-lang/src/error.rs` `Op`; `edit.rs`, `edit_ops.rs`): SetParam,
  AddLocalParam, MakeShared, SplitInstance, AddFeature, DeleteFeature, ReplaceFeature,
  MoveFeature, LiftLiteral, SetArg, Rename, SetProperty, Suppress, Unsuppress, Respell. Each "is a
  pure function of (document, op): it changes one defined span (or inserts at one point) and
  leaves every other byte as it was". Ops address features and params **by name**, not by byte
  offset, except LiftLiteral and Respell (a `Span`).
- **Versioned refusal**: `fg edit F OPS --base V` refuses with `StaleBase` unless the file's
  version (a 64-bit hash of its text) is still V (`interface/forge-cli/src/fg.rs`). `forgecad web`
  does the same on files with ETags (412). This is optimistic concurrency with **no merge**: the
  second writer re-reads and re-sends.
- **Undo** (`forge-lang/src/undo.rs`): `F.fg.log`, a log of whole texts (before, after) per
  edit; a step is refused if the file is not the text the log ends on. Proven in
  `research/forge-lang/lean/ForgeLang/Undo.lean`. It is a single-writer undo: with collaborators
  it would undo their edits too (it restores a whole text).
- **History** (`interface/forge-history`): every save is a git commit by forge-git (each object
  and ref fsynced), versions are annotated tags `forgecad/v/<path>/<n>`, trailers name the agent.
- **Web**: the same app in wasm; a default build (kernel on the page's thread) and a threaded
  build (builds on Web Workers sharing memory, forge-alloc's eight spin-locked heaps);
  `forgecad web DIR` serves files with ETags, loopback-only by default, token otherwise
  (`docs/web.md`). CI checks page vs native bit for bit on measures (`web/tests/parity.cjs`).
- **Determinism**: `tools/speed/bits.txt` and the callgrind gate; wasm vs native `bits` in
  `tools/speed/wasm.sh`.
- **Proofs about ops**: the ForgeLang Lean model (`research/forge-lang/lean`, 11k lines, no
  `sorry`) proves the meaning of SetParam (`Edits.lean`), Add/Delete/Move (`Struct.lean`,
  `Complete.lean`), Rename (`RenamePart.lean`, `RenameDefs.lean`), Suppress, LiftLiteral, Split,
  the edit closure of ReplaceFeature/SetArg (`EditClosure.lean`).

## 2. Latency budgets

Rule from David: hosted must not make anything slower than local, and local must not get slower
at all. So each budget is "the local number, plus at most what the network adds, and nothing
the network does not force".

Network: not measured from real users here. Lower bounds from the speed of light in fibre
(~200,000 km/s, so ≥ 1 ms of round trip per 100 km of path): same metro ≥ ~0.5 ms; one coast of
the US to the other (~4,000 km) ≥ 40 ms; New York–London (~5,600 km) ≥ 56 ms. Real paths are
longer. **UNKNOWN** for real users: measure with a WebSocket echo from the page to each
candidate region (phase 1 ships that probe).

| # | path | budget | estimate from measurements (p50 / p90-p99) | verdict |
|---|---|---|---|---|
| B1 | edit → own view | **= local** (no network on the path); ≤ 16 ms for the 84% of edits that are ≤ 16 ms locally | the client applies its op optimistically and rebuilds warm, exactly as today: 1.5 ms p50, 38 ms p90 native; ×1.31 median in wasm (Node), ×0.99 in Chromium | meets it by construction: nothing waits on the server |
| B2 | edit → collaborator sees it | **≤ RTT + 2 ms + their local rebuild**; target ≤ 100 ms same region, ≤ 250 ms intercontinental | ½RTT + server validate (0.38 / 3.4–23.7 ms) + durable append (0.20 / 0.36 ms) + ½RTT + collaborator's warm rebuild (1.5 / 38 ms). Same region at 10 ms RTT: ~13 ms p50; NY–London at ≥ 56 ms: ~60 ms p50 | meets it; the p99 term is validation of the largest documents (rack.fg 23.7 ms) — see §4.4 fast path |
| B3 | open a big project | **≤ local open; first picture ≤ 1 s + mesh download** | Haas VF-2: local native open 13.8 s = eval_and_build (the STEP import) 10.2 s + measure 2.2 s + process/IO ~1.4 s. The import is a pure function of the STEP's bytes: a content-hashed server result. Measure (mass properties) is not needed for the first picture: run it after, or serve it cached too. Meshing the raw STEP fails on main; wasm ~18 s estimated. Hosted with cached display meshes: download 4.22 MB (VF-2's shipped meshes); edits need the import's B-rep only if a feature reads it | hosted is **faster** than local for imports; ordinary parts: cold build 3.5 ms p50, 1.2 s p90, 3.9 s p99 → the server's cached meshes give the first picture, the client builds behind it |
| B4 | export, CAM, bend sim | **≤ local; cache hits = download time** | bend sim 3.85 s native for 21.7 KB; thermoform 1–4 s per rebuild; CAM threaded page rerun 0.12 s (CAM-PERF). Compute worker native (≥ wasm speed: wasm is 1.31× native median), result keyed by content hash, so the 2nd collaborator and every reopen get it in one download | meets it; a miss costs the native time + RTT; a hit is ~RTT |
| B5 | local, offline forgeCAD | **0 instructions more** on every gate line | no hosted code is linked into the gate binaries; the edit path's op emission lives in the host (§7) | to be enforced by the gate on every hosted PR |

## 3. Decision: where geometry is computed

Options compared:

| option | edit → own view | edit → collaborator | open | heavy results | server cost | risk |
|---|---|---|---|---|---|---|
| A. server only (Onshape: "geometry servers", the browser receives triangles; PRIOR-ART §1) | RTT + server build + mesh transfer (mesh p50 10.7 KB, p90 486 KB, p99 3.9 MB per edit) | same | fast (cached) | fast | a build per edit per document | **violates B1**: every drag waits on the network |
| B. client only | local | ½RTT×2 + rebuild | slow for imports (VF-2 13.8 s+, cannot mesh) | slow on weak clients, each client recomputes | lowest | determinism across clients must hold; heavy work repeated |
| **C. hybrid (chosen)** | local | ½RTT×2 + rebuild | cached meshes first, local build behind | compute workers + content-hash cache | builds only for misses, agents, exports | needs determinism (measured) + divergence detection |

Why C: B1 forbids A (the measured local warm rebuild, 1.5 ms p50, is below any round trip), and
the measured open and heavy-result costs forbid B for imports and simulations. C's risk is
determinism: two clients must build the same bits from the same ops.

**Determinism evidence** (MEASUREMENTS §a): 0 of 276 documents differ in body or mesh hash across
native x86-64-v3, x86-64-v2, one core vs four, and wasm32 under Node; 0 of 223 in Chromium (53 did
not load their sibling files under the test's minimal WASI shim: a harness limit, no geometry
built on both sides). Why it can hold: wasm scalar f64 arithmetic is correctly rounded by spec;
only NaN payloads and relaxed-SIMD are nondeterministic (PRIOR-ART §6); the repo builds without
relaxed-simd and with its own math functions.

**Not yet shown (UNKNOWN):** Firefox, Safari, ARM (Apple Silicon, phones), the threaded wasm
artifact on this hash. Mitigation in the design, not an assumption: each client sends the
`(seq, body hash)` of every build it finishes (8 bytes per build); the server compares against its
own or other clients'; a mismatch logs the platform and the client switches that document to the
server's meshes until fixed. Cost: one hash per build (the `bits`-style hash over vertices and
edges: not measured separately; UNKNOWN, measure in phase 1; it is a linear pass over the body).

**Content-hash cache key**: (kernel build id, document text hash, hashes of every file it reads
— today only user vendor/material libraries are recorded with path + hash128, on
`feat/declared-libraries` per the coordinator's notes; parts, imports and fonts would need the
same — and the options that change results). The kernel build id is in the key because a kernel change may
change bits (bits.txt changes only for judged fixes; the key makes stale results impossible
rather than unlikely).

## 4. Decision: sync model

### 4.1 Options

| model | fits forgeCAD? | evidence |
|---|---|---|
| **server-ordered op log + rebase/re-validate (chosen)** | ops are already typed, named, pure functions of (text, op), and already refused on conflict (`apply_checked`) | Figma: server authoritative, per-property LWW, "not OT" (PRIOR-ART §2); Replicache: rewind, apply server state, replay pending (§5); Google Docs keeps a revision log and one change in flight per client (§3); Linear: one ordered log per workspace (§5); Onshape: increments applied to the latest microversion, last committer of a feature wins (§1) |
| OT on the text | unneeded: ops are not character edits; the only transform needed is Rename's name substitution | 96.9% commute without any transform (MEASUREMENTS §c) |
| CRDT (Automerge/Yjs/list-move CRDTs) | poor: the document is a program whose validity is global (a name must be bound before it is read, single assignment); a CRDT converges to *some* text, which may not evaluate. Neither Automerge nor Yjs documents a list move (PRIOR-ART §4); move CRDTs exist in papers (Kleppmann 2020, tree move 2021) but converge by LWW anyway | the interleaving anomaly (Kleppmann et al. 2019) shows text CRDTs can produce mixed text; offline-first is the CRDT's strength, and §4.5 covers offline without it |
| locking (check-out a feature) | blocks the 97% of pairs that commute | measured: different-target pairs nearly always commute |

### 4.2 The protocol

- Each document has one **sequencer** (a doc actor on one server at a time; Figma runs one
  process per document, PRIOR-ART §2). It holds the head text, head seq, and the warm `Cache`
  only if it also builds.
- A client sends `{op, base_seq, client_op_id}`. The sequencer:
  1. if `base_seq == head`: apply (fast path §4.4) else **rebase**: transform `op` through every
     Rename committed after `base_seq` (name substitution), then `apply_checked` on the head;
  2. refused → the client gets the typed error (as `fg edit` gives today) and drops its
     optimistic op; nothing is written;
  3. accepted → append `(doc, seq+1, author, op, base_seq, client_op_id)` durably, then ack the
     author and broadcast `(seq, op)` to the others.
- Clients keep pending ops (sent, not acked) and replay them on top of each incoming server op
  (Replicache's rebase). Since ops address names, replay is `apply` again; a replayed op that is
  now refused is dropped with the same typed error the server will send.
- `client_op_id` makes a retried send idempotent (FoundationDB documents the same need for
  `commit_unknown_result`, PRIOR-ART §9).
- **Same-target rules**, as measured: SetParam/SetParam → later wins (`setLit_lww`, proven);
  Delete vs Rename/Replace/Move of the same feature, Rename vs Rename of the same name → the
  later one is refused with its typed error; Add/Add, Add/Move, Move/Move at the same place →
  both apply, server order decides position.
- Ops with byte spans (LiftLiteral, Respell) are valid only at their base text: rebased, they
  are refused unless the text in the span is unchanged (UNKNOWN how often in practice: they were
  not in the pair experiment; add them in phase 1).

**Resend and rebase (added 2026-11-25, from the phase-1 reviews).** A client sends no op before
its connection's first Welcome, then resends every pending op in order. A Resync's snapshot on a
live connection resends nothing. Every op always carries **its real base**, including ops
restored from disk after a restart (`.forgecad/sync/<doc>.pending`, bound to the server it came
from). A base older than the sequencer's in-memory entries is rebased through the kept ops
(read from the store; ops are never deleted, §17.1). Only if those ops are truly gone is it
refused (`RebaseGone`). Never re-base an op to a snapshot's seq: that skips the Rename
transform, and the op lands on the wrong target (reproduced in `reviews/N1-N9-rereview.md`).
Retries are answered by (client, op id), whether committed or refused, so a resend never turns a
refusal into a commit.

### 4.3 Can it be proven?

Yes, in the existing ForgeLang model, op type by op type:
- **Done here** (`lean/Merge.lean`, builds against `research/forge-lang/lean`, no sorry):
  `setLit_comm` (SetParams on different sites commute), `setLit_lww` (same site: last wins),
  `concurrent_setParam_traces` (two unobserved SetParams leave traces, ids and control log as
  they were: a concurrent pair of dimension edits cannot change the feature list).
- **Done** (`lean/Rebase.lean`, same build, no sorry): the Rename transform
  `apply (rename a b S) (rn a b op) = rename a b (apply S op)` for SetParam (by statement name
  and literal position), Add, Delete, Move, Replace/SetArg and Rename (`rebase_rename`, and per
  op `setParam_rn` .. `replace_rn`); Rename/Rename of one name do not commute
  (`rename_race_differ`; the later one is a no-op, `rename_absorbed`, and refused on the head,
  `rename_after_rename_refused`); renames of different names commute (`rename_comm`); two renames
  to one target: the later refused (`rename_same_target_refused`); convergence by construction
  (`converge`, `converge_chunks`) and one-step client replay (`client_replay_rename`). Not in the
  model: nested ids (top-level names only), byte-span ops, Suppress/AddLocalParam/MakeShared/
  SplitInstance/SetProperty (file header).
- **Next** (sizes are guesses, S ≈ 1–2 days of a proof agent, M ≈ a week):
  - Different-statement edits commute: ReplaceFeature/SetArg on different statements, Delete of a
    statement nobody reads, Add at different anchors — from `Struct`, `Complete`, `EditClosure`
    (M).
  - Client replay through several server ops and pending ops (the general rebase, by induction
    over the log, from `rebase_rename` and re-validation) (M).
  - Geometry convergence needs determinism of the kernel across platforms: not a Lean question;
    covered by measurement + runtime hash check (§3).

### 4.4 Validation cost and its fast path

`apply_checked` (apply + evaluate both texts, no kernel build) costs 0.38 ms p50, 3.4 ms p90,
23.7 ms p99 (largest documents). When `base_seq == head`, the author's client already ran the
same `apply_checked` on the same text with the same deterministic code, so the server may
**append first and validate after** (still before the next op is sequenced). An op that fails
validation after the append is followed by a compensating server op (its inverse, §4.6) — UNKNOWN
how often this would fire; measure with the corpus (it should be 0 for honest clients). Rebased
ops always validate before the append.

### 4.5 Offline and long divergence

A client offline for a while is a **branch**: its ops are kept with their base seq; on
reconnect they are rebased one at a time like any late op; refusals are shown as a list the user
resolves (the same typed errors, with their `fixes`). Onshape recommends branches for working
apart (PRIOR-ART §1). A long offline session can instead be pushed as a git branch (forge-history)
and merged as text (§5) — the user chooses.

### 4.6 Undo with collaborators

Today's `F.fg.log` restores whole texts: with collaborators it would erase their edits. Hosted
undo = **the inverse op of the user's own last op, rebased onto the head** (Onshape: "undo is not
a rewind"; PRIOR-ART §1): SetParam → SetParam(old value); Rename → Rename back; DeleteFeature →
AddFeature(old text, old anchor); MoveFeature → MoveFeature(back); ReplaceFeature →
ReplaceFeature(old text); refused with the typed error if a collaborator's later op makes it
invalid. Each op records what its inverse needs (old value/text/anchor) at commit time. The
local single-user undo log stays as it is (B5).

## 5. Decision: storage, history, git

**Measured** (MEASUREMENTS §d, same disk, durable appends): Postgres 0.197 ms, SQLite WAL FULL
0.156 ms, libSQL 0.167 ms, file + fdatasync 0.119 ms, Redis AOF always 0.202 ms, MongoDB 0.449 ms,
FoundationDB 1.023 ms p50 (the last two in a second run, where Postgres measured 0.244 ms);
forge-history git save 3.4–5.2 ms p50. Reads of 200 ops: 0.07–0.7 ms. Open = snapshot + 200
ops: 0.09 ms (SQLite) to 1.15 ms (Postgres). SQLite NORMAL is not durable on power loss
(PRIOR-ART §9) and is excluded.

Since all durable candidates are ≤ 0.2 ms against a ≥ 1 ms network, latency does not choose.
What does:
- **Postgres (chosen)**: one store for the op log, documents, projects, members, shares, and
  permission checks in the same transaction as the append; replication and backups are standard
  (UNKNOWN for us: failover time — test in phase 2 with a streaming replica). Its measured cost
  (0.2 ms append, 9.6k ops/s from a Python client with 16 writers) fits B2.
- SQLite/libSQL per document: fastest reads; ties each document to one machine's disk; fine for a
  single-server phase and for the **local** store, but HA needs extra tooling (UNKNOWN cost).
- Plain append-only file: fastest append; we would re-build indexing, permissions and backup.
- Redis AOF: slower reads (0.7 ms) and memory-bound history.
- FoundationDB (measured, single process): append 1.02 / 2.49 ms p50/p99, 4× Postgres; read of
  200 ops 1.85 ms; watch-based fan-out 2.0 / 12.3 ms; 100 KB value and 10 MB / 5 s transaction
  limits (PRIOR-ART §9) split snapshots. Its strengths (strict serializability across machines,
  versionstamps) answer a scale problem one sequencer per document does not have. Not chosen.
- MongoDB (measured, one-node replica set, `j:true`): append 0.45 / 1.16 ms, read 0.54 ms,
  change-stream fan-out 1.08 / 2.27 ms; 2,829 ops/s with 16 writers (p99 15.6 ms). Works, slower
  than Postgres on every line here, and we would still need a relational store for accounts.
  Not chosen.
- Not measured: either one as a multi-machine cluster (each replicated durable write adds a
  network round trip in any of them, Postgres included).
- Object storage: for content-addressed blobs only (snapshots, meshes, imported files, results);
  latency UNKNOWN (needs an account); Figma keeps checkpoints in S3 and the journal in DynamoDB
  (PRIOR-ART §2), the same split.

**Snapshots**: every N ops (N from measurement: start at 200; open = snapshot + ≤ 200 ops, 1.2 ms
p50 in Postgres) the server stores the document text keyed by hash; the op log stays.

**Git**: the op seq is the live history (Onshape's microversions: immutable, parent-linked,
PRIOR-ART §1). A **named version**, and an automatic checkpoint after an idle period (UNKNOWN
period; start at 5 min), is written as a forge-history save + tag, with `ForgeCAD-Agent`/author
trailers, so a hosted project can be cloned, pushed to GitHub and opened offline with today's
code. Per-op commits are rejected: 20–30× the append cost, p99 ~20 ms, and they would make the
version history unreadable. Importing a git branch (offline work, §4.5) is a text merge: diff
the branch's text against the base as a list of ops (`forge-lang/src/compare.rs` already
compares two documents by features and params) and rebase those ops (phase 4; UNKNOWN how often
the compare produces ops that round-trip — measure on the corpus).

## 6. Decision: transport

- **WebSocket** for ops, acks, presence, build hashes: universal browser support; ops are
  ~150–250 B. WebTransport (HTTP/3) is "Baseline 2026" (Safari 26.4, PRIOR-ART §8): consider
  only if measured head-of-line blocking on lossy links shows in B2 (UNKNOWN; measure in phase 2
  with the probe).
- **HTTPS GET** of content-addressed blobs (`/blob/<hash>`): immutable, so cache headers can be
  permanent and a CDN can serve them.
- Same security model as `forgecad web` today (token, origin checks), plus accounts.

## 7. Local and offline must not get slower (B5)

- New crates (names indicative): `forge-sync` (op envelope, rebase, Rename transform, inverse
  ops), `forge-server` (sequencer, store, workers). The desktop and the gate binaries do not
  depend on them.
- The op is emitted by the **host** after `edit::apply_checked` succeeds (forge-render's
  shell/web host, forge-cli's `fg edit`), behind a "connected" check, not inside forge-lang. The
  gate's `bench app/edit/cli` lines must show 0 instructions more; the coordinator's rules apply
  (exact callgrind gate, ≤ +0.03% band only for judged layout noise).
- Content hashing of inputs for the cache key extends the declared-inputs record (libraries
  only today, `feat/declared-libraries`) to every file a run reads; computing it only when
  connected keeps B5.
- The server's builds run the same native release binary as the gate measures.

## 8. (e) Threaded wasm and the forge-alloc race

What is known (coordinator, 2026-11-25): an **out-of-bounds memory access in forge-alloc's
wasm32+atomics `Alloc::realloc`** while building `rack.fg`, in CI job "browser (threaded) 1" of
PR #56 (Actions run 45502487175, attempt 1; the rerun passed; the same wasm passed on #54 and
#55): intermittent. Suspected origin: train 6's per-thread small-block caches (34d6cad8, from
`cam/perf-rest`). CAM-PERF holds the trace and is hunting the root cause; TRAIN-CONDUCTOR is
turning the thread cache off in release as a safety measure. `alloc/threaded-stress`
(3c10b9f2) runs the allocator natively on 8–16 threads and passes. **UNKNOWN**: the root
cause, and whether the race can also corrupt memory *silently* (wrong bits, no trap) — the
observed failure trapped. Asked CAM-PERF; this section records implications only.

What it means for hosting, independent of its details:
- A heap race corrupts memory: a client could build **different bits** or crash. In a hosted
  product that is a divergence between collaborators, not just one user's crash. The build-hash
  report (§3) detects it; the server (native, glibc's allocator) is not affected.
- Hosted clients do not need shared-memory threads for responsiveness: a build can run in a
  **dedicated Worker with its own (unshared) memory**, so the page never blocks and no allocator
  is shared; the page receives meshes by `postMessage` (copy cost UNKNOWN: mesh p50 10.7 KB,
  p99 3.9 MB; measure). What is lost is the kernel's parallel speed-up inside one build: native
  one core vs four is 1.01× median, but up to ~3× on some documents (p10 0.34).
- Also the threaded build needs COOP/COEP cross-origin isolation (PRIOR-ART §7), which constrains
  embedding the hosted app in other pages.
- Decision: hosted clients default to the unshared-worker build until the race is closed and the
  threaded artifact passes the hash comparison on the corpus; then threaded becomes an opt-in per
  measured speed-up.

## 9. (f) Agents when hosted

David uses a Claude plan, not API keys. Anthropic's current terms (PRIOR-ART §10, quoted there):
third parties may not "offer Claude.ai login into their own applications, or … route requests
through Free, Pro, Max, or Ultra plan credentials on behalf of their users", nor "collect, store,
or intermediate Claude.ai credentials or session tokens"; but this does not prevent "an end user
from signing in to the unmodified Claude Code binary with their own Claude subscription,
including where a platform hosts Claude Code" (with Commercial Terms, binary unmodified, and the
operator may not "pay for, resell, or intermediate Claude usage").

| option | uses David's plan? | works from a phone/Chromebook | cost to operator | allowed? | trade-offs |
|---|---|---|---|---|---|
| **1. local Claude Code + `forgecad mcp` pointed at the hosted document** (the `web/agent-bridge` design: the page drives the user's own Claude Code; here MCP's file access becomes ops as that user, attributed `ForgeCAD-Agent`) | **yes**, through Anthropic's own login on his machine | no (needs a machine running Claude Code) | 0 | yes: unmodified Claude Code, user's own sign-in | best for David; the agent's edits sequence like a collaborator's; needs an MCP transport over HTTPS with the user's hosted session token |
| 2. Remote Control (Claude Code on his machine, steered from claude.ai) | yes | steering yes, execution on his machine | 0 | yes (PRIOR-ART §10: runs locally; API keys not supported) | same as 1 with a phone UI that is Anthropic's, not ours |
| 3. bring-your-own API key per user (Console key, or Bedrock/Vertex) | no (API billing) | yes | 0 | yes | the key must be stored for the user (encrypted) or kept in the browser; metered cost |
| 4. operator API key | no | yes | per token | yes | simplest UX; David pays; needs limits per user |
| 5. host the unmodified Claude Code binary per user, each signing in with their own plan | yes | yes | compute for the sandbox | **UNKNOWN for us**: allowed in principle, needs Commercial Terms; whether a CAD app embedding it fits is for Anthropic to say (contact sales) | heavy: a sandbox per user session |
| ✗ "log in with Claude" in our app / server-side plan tokens (`setup-token`) on behalf of users | — | — | — | **no** | `setup-token` is for one's own CI/scripts |

Decision: build 1 first (it reuses `web/agent-bridge` and costs nothing), offer 3 for users
without a machine, keep 4 optional, ask Anthropic before considering 5.

## 10. Risks

| risk | likelihood (evidence) | impact | mitigation |
|---|---|---|---|
| cross-engine/CPU float divergence (Firefox, Safari, ARM) | low by spec; **untested** | collaborators see different geometry | build-hash reports + server fallback; CI matrix Firefox/WebKit/ARM in phase 1 |
| threaded-wasm allocator race | known to exist (details UNKNOWN) | divergence or crash | unshared-worker build by default (§8) |
| slow assembly edits when modelled threads interfere (5.96 s / 9.26 s in the overlap check's exact screw intersection; MEASUREMENTS §b) | reproduced on 2 files, warm = cold | B1/B2 blown for those edits, locally too | reported with profile; owner to be assigned; hosted adds nothing |
| validation p99 on large docs (24 ms) | measured | B2 p99 | fast path §4.4; profile `apply` (11.5 ms p99 for a text splice is high) |
| span ops (LiftLiteral, Respell) after rebase | untested | spurious refusals | refuse unless span text unchanged; measure in phase 1 |
| Postgres failover time | UNKNOWN | writes stall during failover | measure in phase 2 |
| imported B-rep needed for editing (features reading an import) | Haas VF-2 meshing fails on main | big imports unusable for editing in the browser | server compute for imports; client fetches the cached B-rep only when a feature reads it (size UNKNOWN: measure serialized B-rep of VF-2) |
| determinism of results computed with the process memo (`forge-memo`, geomemo) vs fresh | warm vs cold equal on all 642 edits here | cached results keyed wrongly | key includes kernel build id + inputs; CI compares cached vs fresh |
| scope: accounts, sharing, billing | not measured | large | phases 2–3 |
| terms for agents change | moderate (they were clarified in Feb 2026) | option 5 or even 1 | keep 3/4 working |

## 11. Phased build plan

Sizes: S ≈ ≤ 1 week, M ≈ 2–4 weeks, L ≈ 1–2 months, for one agent with review. Every phase
passes the speed gate with 0 regressions on local lines.

1. **Phase 0 — measurements to close the UNKNOWNs (S).** Firefox/WebKit/ARM hash runs of
   `det.wasm`; fix the WASI shim so the 53 assemblies run in the browser harness; build-hash cost;
   span-op rebase rate; LiftLiteral/Respell/SetArg/SetProperty/Suppress in `merge.rs`; the
   threaded-artifact hash run once the race pointer arrives; 
   WebSocket RTT probe.
2. **Phase 1 — single-server collaboration (M).** `forge-sync` (op envelope, rebase, Rename
   transform, inverse ops), sequencer in `forgecad web` (one process, SQLite-or-Postgres behind a
   trait so the choice is swappable), WebSocket, presence, build-hash reports, hosted undo,
   named versions as forge-history commits. Lean: Rename transform lemma, different-statement
   commutation. Exit: two browsers + an agent editing one part; B1/B2 measured on a LAN.
3. **Phase 2 — hosted service (L).** *Decided in detail in §17 (plan §17.9; users, orgs and auth in
   `USERS-ORGS-AUTH.md`).* Accounts, projects, sharing, Postgres store with replica
   (failover measured), blob store, snapshots, cached first-open meshes, compute workers
   (imports, bend sim, thermoform, exports) keyed by content hash. Exit: B2/B3/B4 measured from
   two regions; Haas VF-2 opens with a first picture in ≤ 1 s + download.
4. **Phase 3 — agents (S–M).** MCP over HTTPS to a hosted document for local Claude Code (option
   1), BYO key (option 3). Exit: an agent's edits arrive as attributed ops; refusals reach it as
   typed errors.
5. **Phase 4 — offline and branches (M).** Offline queue as a branch, git import/export of
   branches via compare → ops, branch merge UI.
6. **Phase 5 — threaded hosted clients (S, after the race).** Threaded artifact opt-in where its
   hash run matches and its speed-up is measured.

## 12. What this design deliberately does not decide

- Pricing, accounts provider, region choice: product decisions with no measurement here.
- Whether to run compute workers on the same machines as sequencers: decide from phase 2 load.
- Assemblies across documents (references to other documents' versions): Onshape pins
  versions (PRIOR-ART §1); forge-lang's `part(FILE)` reads files; the hosted equivalent is
  "a part at a version or at a document's head" — design in phase 2 with the assembly owners.

## 13. Determinism: a requirement, enforced and tested continuously

David (2026-11-25): clients must get **bit-identical geometry everywhere**. So the server-mesh
fallback of §3 is a safety net for a bug, never a mode: a mismatch is a **failed gate** in CI
and a **bug report** in the field.

### 13.1 Every source of divergence, and how each is closed

| source | how it could differ | how it is closed | status / evidence |
|---|---|---|---|
| FMA contraction | `a*b+c` fused on one target, not another | Rust/LLVM never contract without fast-math; `clippy.toml` disallows `f64::mul_add` ("fused results differ from separate ops; breaks native/wasm equality") | closed by toolchain + lint; v2 (no FMA unit used) = v3 = wasm on 276 docs |
| x87 80-bit intermediates | 32-bit x86 builds use x87 | ship no i686 target; x86-64 uses SSE2 scalar (64-bit rounding) | closed by target choice |
| denormals (FTZ/DAZ) | a target flushing subnormals to zero | wasm requires IEEE subnormals (no FTZ); x86-64 Linux/macOS/Windows and aarch64 Linux/macOS start with FTZ off; no code sets MXCSR/FPCR | closed unless some dependency sets it: **UNKNOWN** for every native library linked into the app (GPU drivers, audio): test = the gate on the app binary, not only on `det` |
| libm transcendentals | platform `sin`, `exp`, `pow` differ by an ulp between libcs | `clippy.toml` disallows `f64::sin/cos/sin_cos/asin/acos/powf/...` ("platform libm"); the kernel uses its own `forge_math::dmath` (seen in the profile: `dmath::atan`, `atan2`) | closed by lint; keep the lint list complete (`exp`, `ln`, `hypot`, `cbrt`, `tanh` included? check in phase 0) |
| SIMD | lane order, relaxed ops | wasm built with `+simd128` only, no relaxed-simd (`.cargo/config.toml`); native AVX used only for ops that round per lane as scalars | closed; the gate covers it (v2 vs v3) |
| NaN payload/sign | wasm leaves NaN bits nondeterministic (PRIOR-ART §6) | hashes and comparisons must never read a NaN's bits; a NaN reaching geometry is a typed error | partly: add a debug check that no NaN reaches a hashed body (phase 0) |
| usize width | wasm32 is 32-bit, natives 64-bit: overflow, hashing of `usize` | the forge-git wasm usize overflow was a real bug of this kind (coordinator notes §3); gate covers wasm32 | the gate catches; review rule: hash `u64`, not `usize` |
| hash-map iteration order | std `HashMap` is randomly seeded per process | results must not depend on iteration order; the speed gate pins the seed (`fixedrandom.so`) for counts, which would also **hide** an order dependence — so the determinism gate must run **unpinned, several seeds** | evidence: 5 separate unpinned processes (v3, v2, 1-core, Node, Chromium) gave the same bits on 223–276 docs |
| thread scheduling | parallel reductions in a different order | `forge_math::par` must combine in index order; 1 core vs 4 cores identical on 276 docs | gate runs 1, 2 and N threads |
| allocator / addresses | sorting or hashing by pointer; threaded wasm allocator race | no address-dependent order (review + gate on several allocators: glibc, forge-alloc wasm, threaded forge-alloc) | race: §8; gate catches order effects |
| process-wide memos (`forge-memo`, geomemo, `VOLUMES` in forge-asm) | a memoised result differs from a fresh one | memo keys include every input; gate compares warm-process vs fresh-process bits | 642 warm edits = cold here |
| compiler/toolchain version | a new rustc changes codegen of float code | pinned toolchain (`rust-toolchain.toml` 1.99.1); a toolchain bump is a gate event | closed by pinning |
| engines/CPUs not run here | Firefox, Safari, ARM | **UNKNOWN** — see 13.2 | test plan below |

### 13.2 The cross-platform determinism gate

A CI job, required on every PR that touches kernel, forge-lang, forge-alloc, the toolchain or
`.cargo/config.toml`, and nightly on main:

1. **Corpus**: every `examples/**/*.fg`, the speed gate's parts, the fuzz seeds, and **recorded
   op streams**: real edit sessions (start text + ops) replayed op by op, hashing after each op
   (`det`-style: body bits + mesh bits + error kinds). Assemblies hash each placed part.
2. **Targets**, all producing one file of `doc op# hash` lines:
   - native x86-64-v3 and x86-64-v2 (Linux), 1 thread and all threads, two HashMap seeds;
   - native **aarch64** (Linux runner `ubuntu-24.04-arm`; macOS arm64 runner for Apple Silicon)
     — **UNKNOWN here** (no ARM in this VM);
   - wasm32 in Node, Chromium, **Firefox** and **WebKit** (Playwright), default and threaded
     artifacts — Firefox/WebKit **UNKNOWN here** (only Chromium installed);
   - Windows x86-64 (MSVC) — **UNKNOWN here**.
3. **Rule**: every line equal to the reference (native x86-64-v3); any difference fails, naming
   the document, the op, and the first differing vertex/edge. No tolerance: bit identity.
4. **Harness gap to close first** (phase 0): the browser runner here (`browser/run-wasi-chromium.cjs`)
   could not load 53 assemblies' sibling files; run the gate through the app's own storage
   (as `web/tests/parity.cjs` does) or fix the shim, so every document runs in every browser.
5. **In the field**: the build-hash report of §3 is the same check, live: a mismatch is logged
   with platform, browser, CPU and the doc/op, and the client shows the server's result for that
   document until fixed.

Cost: the native and Node runs took 2.5–3 min each for 276 docs here; the browsers similar.
Sharded like parity.cjs, it fits in the existing web CI time.

## 14. One client library, two hosts: native desktop and wasm

David: the **native desktop app is a full client of the hosted server** (live multi-user,
shared history, same budgets), not only the browser.

```
            forge-lang (edit::apply_checked, run, Cache)   <- unchanged, shared
                         |
   forge-sync  (pure Rust, no I/O):  op envelope {doc, base_seq, client_op_id, op, inverse},
               pending queue, rebase (Rename transform), apply incoming, inverse ops for undo,
               build-hash reports, protocol messages (encode/decode)
         |                     |                      |
   Transport trait        Store trait           Clock/Timer trait
   native: WebSocket      native: files +       native: std
     on its own thread     .forgecad/sync/      wasm: performance.now
   wasm: browser            (pending ops,
     WebSocket (web-sys)    last seq, blob cache)
                          wasm: OPFS/IndexedDB
                            (as web/storage.js)
```

- `forge-sync` holds all protocol logic; both hosts link the same code, so they cannot drift.
  It has no threads of its own and no I/O: the host feeds it messages and asks it what to send
  (a "sans-I/O" state machine), so it runs the same on the page's thread, a Worker, or a native
  thread.
- What differs per host:

| concern | native desktop | wasm (browser) |
|---|---|---|
| threads | build worker thread (today's `plat.rs`), socket on its own thread | build in a dedicated Worker; socket on the page |
| file access | real files: the served project is a plain folder (§15) | `forge_dirs::fs::mem` filled from OPFS/IndexedDB |
| offline | full: edits apply to files and queue in `.forgecad/sync/pending`; reconnect rebases (§4.5) | while the tab lives: queue in IndexedDB; a closed tab keeps its queue |
| caching | content-addressed blobs in `.forgecad/cache/` (meshes, imports, results), shared across projects in the user cache dir | browser Cache API / OPFS, same keys |
| compute | may run heavy jobs itself (native speed) instead of waiting for a worker; the result's hash is the same key, so it can upload it for others (only if the server trusts clients' results: no — it verifies by recomputing or by hash agreement of two clients; decide in phase 2) | asks compute workers |
| auth | token in the OS keychain (today's `keyring` use) | session cookie / token in memory |

- Same budgets B1–B4 apply to both hosts. B5 (local never slower) applies to the desktop in
  LOCAL mode (§15).

## 15. LOCAL and SERVED projects in one app

### 15.1 Identification

- A project is **SERVED** iff its folder holds `.forgecad/remote.toml` (server URL, project id,
  the user's account name, the last seq applied per document). `.forgecad/` is already in the
  starter `.gitignore` (forge-history `STARTER_IGNORE`), so the link is never committed.
- Otherwise it is **LOCAL**: exactly today's behaviour. The app checks for the file **once, when
  the project opens**; in LOCAL mode no sync code runs, no socket opens, no account is needed,
  nothing is read or written beyond today's files. Nothing on the edit path tests the mode
  except the host's "connected?" branch after a successful edit (§7), which the gate measures
  (B5: 0 instructions on `bench app/edit/cli`; if the branch itself costs instructions, move it
  behind a function pointer set at open, so LOCAL mode runs today's exact code).

### 15.2 Moving between modes

| action | what happens | history |
|---|---|---|
| **publish** (LOCAL → SERVED) | the app uploads every project file (content-addressed) and its git history; the server creates the project; the folder gets `.forgecad/remote.toml` | the project's forge-history commits and version tags become the server's first versions; live op history starts after |
| **checkout** (server → new local folder, kept in sync) | the app writes every document's text at head, a git repository whose commits are the server's versions, and `remote.toml` | git in the folder = versions; the live op history stays on the server (fetchable) |
| **sync** (while SERVED) | incoming ops are applied with `edit::apply` and written to the files; local edits go out as ops; versions arrive as new git commits | `git log` in the folder shows the versions |
| **detach** (SERVED → LOCAL) | delete `remote.toml` (with pending ops sent first, or kept as a branch on request) | the folder is a normal local project with its git history |
| **attach** an existing folder to an existing served project | allowed only if the folder's texts equal some server version; otherwise it is published as a branch | |

### 15.3 What a served project looks like on disk

A **plain folder plus git**, readable by any tool: the `.fg` texts at the head the client has
applied, sibling files (STEP, fonts) as files, `.git` with one commit per version
(forge-history format, trailers naming authors and agents), and `.forgecad/` (ignored) holding
`remote.toml`, `sync/` (pending ops, last seq) and `cache/`. Other tools may read and run it;
`git` push/pull to another remote works on the versions.

### 15.4 Hand edits and other tools touching the folder

A text written by something other than the app (an editor, a script, `git checkout`):

1. The desktop notices it: a file watcher while the app runs, and on open a hash compare of
   each document against the last text the sync applied (stored in `sync/`).
2. It turns the change into ops: `forge-lang/src/compare.rs` already lists what B changed from A
   by features and params. If the ops reproduce the new text **exactly** when applied to the
   old (checked: apply, compare bytes), they are sent as ordinary ops, attributed "hand edit".
3. If not exact (formatting, comments, an edit no op expresses), it is sent as one coarse op,
   **`SetText {base_seq, old_hash, new_text}`**: valid only if no op landed on that document
   since `base_seq`; else refused, and the app offers a three-way merge (base text, server head,
   the hand-edited text) — the hand edit is never lost: it is also kept as a branch.
4. A `git checkout` of an old version in a served folder is the same: a hand edit to that text,
   which becomes "restore version N" (forge-history's Restore is already a new save of an old
   state).

How often step 2 is exact: **UNKNOWN**; measure in phase 0 by round-tripping compare → ops on
consecutive versions of the corpus (git history of `examples/`).

## 16. Plan additions

- Phase 0 gains: the determinism gate's harness (13.2 step 4), ARM/Firefox/WebKit/Windows runs,
  the clippy lint list check, unpinned multi-seed runs;  compare → ops exactness rate.
- Phase 1 builds `forge-sync` sans-I/O with **both hosts from day one** (desktop and wasm), and
  LOCAL/SERVED identification, publish and checkout (M → M+, about +1–2 weeks).
- Phase 4 (offline and branches) includes hand-edit reconciliation and `SetText` with
  three-way merge (M).

## 17. Phase 2: the hosted service

Status 2026-11-25. Phase 1 (one forge-server process, file log, dev tokens, loopback) exists on
`hosted/server` and was reviewed (`reviews/N1-N9.md`, `reviews/clients.md`). This section
decides what phase 2 adds, one decision per choice, each from a measurement here
(MEASUREMENTS §p2, scripts in `bench/phase2_*.py`, `bench/server_load.rs`) or a cited source
(`PRIOR-ART-PHASE2.md`, fetched 2026-11-25). Where neither decides, **UNKNOWN** says how to find
out.

### 17.0 Decisions in one table

| choice | decision | evidence |
|---|---|---|
| store | **Postgres 16+**, one database: accounts, projects, members, documents, ops, snapshots, jobs; ops never deleted | §5's measurements; the fenced, role-checked append 0.40 / 0.53 ms p50/p99 |
| one writer per document | **the document's `head_seq` is the fence**: an append commits only if `head_seq = seq − 1` and the author may edit, in one statement | a stale sequencer: 0 of its appends accepted; a viewer: 0 (`phase2_pg.py`) |
| retries | **`unique (doc, client, op_id)` on `ops`** answers a retried op; no in-memory dedup set, nothing in snapshots | lookup 0.12 / 0.18 ms p50/p99; replaces N5/N6 (review M3) |
| replication | **synchronous standby** (`synchronous_commit = on`), never async | async acks before the standby has the commit: it lost 1 acked op of 920 in one run (0 of 11,701 in another), sync 0 in both; commit cost 0.31 ms p50 vs 0.12 ms with no standby (loopback) |
| failover | **managed first (RDS Multi-AZ or Cloud SQL HA)**, measured against documented 60–120 s / ~60 s | promotion itself is 0.21 s here; the managed products' outage is detection + DNS (PRIOR-ART-PHASE2 §2) |
| backups | **continuous WAL archive + daily base backup to object storage; restore drill monthly** | PITR to a named point restored exactly the rows before it and none after; base backup of 663 MB in 4.3 s; restore to a named point in 3.4 s, exact |
| blobs | **content-addressed objects in R2** (S3-compatible, no egress fee) | R2 $0.015/GB-mo, $0 egress; S3 $0.023 + $0.09/GB egress |
| accounts | **email + password (Argon2id m=19 MiB, t=2, p=1) and passkeys (WebAuthn)**; 15-char minimum for password-only | OWASP and NIST SP 800-63B rev 4; hash 17.8 ms p50 here |
| sessions | **opaque 256-bit token in an HttpOnly, Secure, SameSite=Lax cookie; stored as SHA-256** | verify cost is one hash + one index lookup; nothing for page script to steal (client review L) |
| sharing | **per-project roles owner / edit / comment / view; view-only links** | Onshape (view-only links) and Figma (owner/edit/view, edit links) |
| TLS | **terminated in front of each server by Caddy (ACME, automatic)**; ALB only if a cloud LB is wanted for other reasons | 50 µs of crypto per handshake (≈20k/s/core); ALB $16.43/mo + LCUs and no routing by document |
| routing | **a document lives on one server; Hello to any server, `Redirect` to the owner** | the sequencer must be one process per document (§4.2) |
| compute jobs | **a `jobs` table: unique (kind, input hash), claimed with `FOR UPDATE SKIP LOCKED`, leased, woken by NOTIFY** | 1,771 jobs/s with 1 worker (claim 0.30 ms p50), 5,240/s with 4 (0.39 ms), 4,662/s with 16 (1.62 ms; past the 4 vCPUs here); 0 double claims; wakeup 0.30 / 0.50 ms p50/p99 |
| result cache | **key = SHA-256(kind, inputs, FORGE_KERNEL_ID)**; value in R2 | determinism gate (§13) makes a result valid on every client of that kernel |
| capacity unit | **CPU**: 0.8 ms per committed op; 210 KB per open document | `server_load.rs` on 5df9cf74 (17.7); the 3.2 MB/doc first measured was two bugs, now fixed |

### 17.1 Postgres schema and the append

```sql
accounts  (id, email unique, name, created_at)
credentials (account, kind 'password'|'passkey', data, created_at)   -- argon2id PHC string / COSE key
sessions  (token_sha256 primary key, account, created_at, last_seen, expires_at, device text)
projects  (id, owner, name, created_at)
members   (project, account, role smallint 0 view 1 comment 2 edit 3 owner, primary key (project, account))
links     (token_sha256 primary key, project, role 0 only, expires_at null)
documents (id, project, path, head_seq, owner_server, lease_until, unique (project, path))
ops       (doc, seq, author, client, op_id, base, op jsonb, inv jsonb, text_hash, at,
           primary key (doc, seq), unique (doc, client, op_id))
snapshots (doc, seq, text, text_hash, primary key (doc, seq))        -- every N ops; ops are kept
jobs      (id, kind, input bytea, state, worker, lease_until, attempts, result bytea, unique (kind, input))
```

The sequencer's append is one statement (`bench/phase2_pg.py` `APPEND`): update `documents`
setting `head_seq = seq` **where** `head_seq = seq − 1` **and** the author has role ≥ edit on the
document's project; insert the op only if that update returned a row. Three properties follow
from the database, not from code:

1. **One writer.** Two sequencers for one document (a slow one after a failover, a
   misrouted one) cannot both append seq n: the second's update matches no row. Measured: a
   stale sequencer appending at the head, five back and at 1: 0 accepted.
2. **Permission at commit.** A member demoted to view between their op's arrival and its commit
   cannot have it committed. Measured: 0 accepted.
3. **Retries exact for ever.** The unique index answers "already committed at seq s" for any op
   id, after any number of restarts and compactions, with no memory cost (review M3's growth
   disappears). Measured lookup: 0.12 / 0.18 ms p50/p99.

Cost of the checks: 0.40 / 0.53 ms p50/p99 against the bare insert 0.25 / 0.52 ms (1 writer; 16 writers:
2.56 / 7.05 ms p50/p99 at 5,487 ops/s with the checks). Open = latest snapshot + the ops after it: 0.56 / 0.74 ms p50/p99 for 200 ops after a 12 KB
snapshot.

**Ops are never deleted** (review M1). Snapshots are rows beside them; "compaction" in phase 2
means writing a snapshot row, not removing history. Storage: ~250 B of op JSON + index per op;
301 B (203 B row + 96 B for both indexes), i.e. ~0.3 GB per million ops. Retention beyond N years is a product decision (§12).

**Fan-out across servers** stays as §4.2 says: the owner server holds every connection of its
document; NOTIFY is only for other servers' caches (0.40 ms p50, §5).

### 17.2 Replication, failover, and what a client sees

Measured on loopback (`phase2_ha.py`; a real standby in another zone adds that zone's round
trip to every synchronous commit):

| commit mode | append p50 / p99 |
|---|---|
| no standby | 0.12 / 0.22 ms |
| async standby | 0.16 / 0.27 ms |
| sync standby, `on` (flushed there) | 0.31 / 0.49 ms |
| sync standby, `remote_apply` | 0.35 / 0.51 ms |

Failover with the primary killed (no shutdown checkpoint) while a writer appends: promote
returns in 0.21 s and the first write on the new primary succeeds 0.21 s after the kill.
**Async lost 1 of 920 acked ops in the first (smoke) run and 0 of 11,701 in the second; sync lost 0 of 261 and 0 of 5,705.** A loss is a matter of timing (async acks before the standby has the record), so zero in one run proves nothing; one observed loss shows it happens.

Why async is ruled out, beyond losing an edit: an acked op that vanishes leaves its seq free.
The new primary commits a *different* op at that seq, and every client that applied the old one
now holds a different text at the same seq — the divergence §13 exists to prevent, and a
silent one, since both texts hash consistently on their own. So: synchronous replication only.
And the Hello carries the client's last seq; a server that sees a client's seq **above** its head
answers with a snapshot (the client's pending ops replay on it), never with entries.

The managed products' failover is not the promotion (0.21 s here) but detection and DNS:
RDS Multi-AZ "typically 60–120 seconds", Multi-AZ DB cluster "under 35 seconds" (not on
db.m7g); Cloud SQL HA "about sixty seconds" (PRIOR-ART-PHASE2 §2). During it, edits wait as
pending on every client and are resent on the next Welcome (the protocol rule both reviews
arrived at). Nothing is lost; collaborators stop seeing each other for that long.
**UNKNOWN**: whether 60–120 s pauses are acceptable to users. Decide from phase-2 beta;
if not, self-managed Postgres with an automated failover tool (detection seconds) is the
next step, at the cost of running it.

### 17.3 Backups and restore

Continuous WAL archiving plus a base backup; measured: base backup of a 663 MB database (2.16 M rows of op-sized records) in
4.3 s (154 MB/s, compressed tar), restore to a named point in 3.4 s, with exactly
the rows before the point and none after.

Found while measuring: **a restore reaches only what is archived.** The first run lost the last
10,000 rows because the archiver had not shipped the final segment when the primary stopped.
So the recovery point objective is the archive lag, which must be watched: `archive_timeout = 60 s`
bounds it to a minute of ops (still all on the synchronous standby); `pg_stat_archiver` lag is an
alert. A monthly automated restore into a scratch instance, compared row counts and the head's
text hash per document, is the only proof a backup works.

Blobs: R2 objects are content-addressed and immutable; losing one is recoverable by recomputing
it (every blob is a function of ops and inputs) except **imported files**, which are inputs.
Those get a second copy in another provider's bucket.

### 17.4 Accounts, sessions, sharing

- **Passwords** (NIST SP 800-63B rev 4, PRIOR-ART-PHASE2 §5): 15 characters minimum when the
  password is the only factor, 8 with a second; at least 64 allowed; no composition rules, no
  periodic change; checked against a breached-password list; stored as Argon2id m=19 MiB, t=2,
  p=1 (OWASP). Measured: 17.8 ms p50 per hash or verify on this (loaded) CPU, 19 MiB each. A
  semaphore of 4 concurrent hashes caps login memory at 76 MB and login CPU at ~225 logins/s per
  4 cores; per-account and per-IP rate limits keep guessing below that.
- **Passkeys (WebAuthn)** are offered from day one: NIST requires at least one
  phishing-resistant option at AAL2, and syncable authenticators are allowed up to AAL2.
- **Sessions**: a random 256-bit token in an HttpOnly, Secure, SameSite=Lax cookie; the server
  keeps only its SHA-256, with expiry and last-seen. The WebSocket opens same-origin, so the
  browser sends the cookie on the upgrade: **no token is ever visible to page script** (closes
  the client review's stored-`sync_token` finding). The desktop app gets a per-device token by a
  browser sign-in that hands it back over a loopback redirect, stored in the OS keychain, revocable
  in the account page.
- **Sharing** (Onshape and Figma, PRIOR-ART-PHASE2 §6): roles per project — owner, edit, comment,
  view. Only owner and editors share. Links are **view-only** (Onshape's choice; Figma allows edit
  links, which turns a leaked URL into a write credential). Documents inherit the project's
  roles; per-document overrides are left out of phase 2 (Figma lets a file grant more than its
  folder; not needed until asked for).
- The role check is in the append (17.1), so a revoked editor's in-flight ops are refused at
  commit, not just at connect.

### 17.5 TLS, deployment, routing

- **TLS 1.3 at Caddy** in front of each server: automatic ACME certificates (Let's Encrypt, with
  ZeroSSL as fallback), WebSocket passthrough. Cost per handshake on the server is about one
  ECDSA P-256 signature (19 µs) plus one X25519 (28 µs) here, roughly 20,000 handshakes/s per
  core: a reconnect storm of every client at once is not a TLS problem. Let's Encrypt's limits
  (50 certificates per domain per 7 days; renewals via ARI exempt) fit one wildcard or a handful
  of hostnames.
- An AWS ALB costs $16.43/mo plus $5.84 per LCU-month (one LCU per 3,000 active connections per
  minute) and supports WebSockets, but it cannot route a connection to the server that owns a
  document. So routing is in the protocol: **Hello to any server; a non-owner answers
  `Redirect { url }` and closes**. One extra round trip at open, none afterwards. Ownership is
  `documents.owner_server` with `lease_until`; a server renews leases for its open documents
  every few seconds, and a document whose lease expired is taken by the next server a client
  reaches. The `head_seq` fence (17.1) makes a lease race harmless: the loser's appends fail.
- **Deploy**: one binary (forge-server with the Postgres store) per machine, systemd, behind
  Caddy; rolling restart drains a server by redirecting its documents' clients elsewhere (they
  resend pending ops on the new Welcome).

### 17.6 Compute workers and the result cache

- **Queue** in Postgres: `jobs (kind, input)` unique, so the same request from ten collaborators
  is one job. Workers claim with `FOR UPDATE SKIP LOCKED` (the documented use: "avoid lock
  contention with multiple consumers accessing a queue-like table"), hold a lease
  (`lease_until`), and a job whose lease expires is claimable again (`attempts` capped; past the
  cap it is failed with the worker's last error). Measured: 1,771 jobs/s with 1 worker (claim 0.30 ms p50), 5,240/s with 4 (0.39 ms), 4,662/s with 16 (1.62 ms; past the 4 vCPUs here), 0 double claims at
  1/4/16 workers. Idle workers wait on `LISTEN jobs`; enqueue + `NOTIFY` in one transaction wakes
  one in 0.30 / 0.50 ms p50/p99 (NOTIFY is delivered only at commit, so a worker never sees a job that does
  not exist yet).
- **Key** = SHA-256(kind, canonical inputs, `FORGE_KERNEL_ID`). The kernel id is part of the key
  because only same-kernel results are guaranteed bit-identical (§13); a kernel upgrade misses
  the cache once per input.
- **Value** in R2 at `/blob/<hash>`, immutable, served with permanent cache headers.
- **Sizing**: Haas VF-2 open 13.8 s and bend sim 3.85 s are single jobs; at c7i.2xlarge spot
  ($0.163/h, 8 vCPU) a vCPU-second costs $0.0000057, so the VF-2 import is $0.00008 once and a
  download for everyone after. Worker count follows queue wait, not cost.

### 17.7 Capacity and cost

Measured (`bench/server_load.rs`, forge-server as its own process, file store, loopback, this
4-vCPU container, 240 documents with one connection each):

| | value |
|---|---|
| RSS per open document + its connection | 3.2 MB |
| RSS per further connection to an open document | 3.1 MB |
| server CPU per committed op (validate + append + broadcast) | 1.08–1.14 ms (at 240 and 960 ops/s) |
| ack latency on loopback under that load | p50 17–24 ms, p99 ~150 ms |

Both memory costs were found with a heap profile (heaptrack) and are fixable, so the table above
is **phase 1's server, not phase 2's**:

- **3.1 MB per connection** is the outbox: `sync_channel(65536)`, whose slots std allocates and
  initialises up front (~48 B × 65,536). The byte bound (`QUEUE_BYTES`) already limits the
  queue, so ~1,024 slots (~48 KB) do.
- **RSS growth under load** (0.79 → 3.18 GB over 24,000 ops; the same with
  `MALLOC_ARENA_MAX=2`) is not the allocator. Every `forge_lang::run` keeps its sketches on its
  thread (`forge_keep`); a sketch with a named entity is an `Rc` cycle; hosts must release them
  between builds (forge-render and the CLI server do; forge-server did not). Validation runs
  forge-lang two or three times per op on a per-check thread, so every op leaked its sketches.
  Repro `bench/runleak.rs`: 16.2 KB per run of the load test's 3-sketch part as is, **0.0 KB** with
  the release (RSS flat over 3,000 runs).

After both fixes (hosted/server-hardening 5df9cf74), the same test on the same machine:

| | before | after |
|---|---|---|
| 240 open documents | 792 MB | **56 MB** |
| per document + connection | 3.2 MB | **210 KB** |
| per extra connection | 3.1 MB | **43 KB** |
| server CPU per committed op | 1.30 ms | **0.80–0.86 ms** |
| ack p50 / p99 (loopback, shared CPU) | 44–53 / 158–170 ms | **3.6–5.9 / 47–64 ms** |
| RSS after 24,000 ops | 3,185 MB | **154 MB** |

Still to settle: RSS rose 56 → 154 MB over the 24,000 ops (~4.1 KB/op; HOSTED-BUILD's own
120 s soak measured 680 B/op). Either the actors keep every entry in memory (unbounded per
document) or it is warm-up; a longer constant-rate run decides. With the Postgres store an
actor needs only the entries since the oldest base it may rebase from.

So a server is **CPU-bound**: 2 vCPUs commit ~2,400 ops/s at 0.8 ms; 8 GB holds tens of
thousands of open documents.

Cost, from cited on-demand prices (us-east-1, PRIOR-ART-PHASE2 §1–4); users' op rate and
concurrency are **UNKNOWN** (measure in beta), so per open document:

| item | choice | $/month |
|---|---|---|
| sequencer server | m7i.large (2 vCPU / 8 GiB): ~2,400 committed ops/s at 0.8 ms each | 73.58 |
| Postgres | RDS db.m7g.large Multi-AZ | 246.01 + $0.23/GB |
| TLS | Caddy on the server | 0 |
| blobs | R2 | $0.015/GB, egress 0 |
| compute | c7i.2xlarge spot, scaled by queue | $0.163/h while running |

A minimal highly available deployment is about **$320/month**, three quarters of it the
database with its standby (RDS Multi-AZ $246 of ~$320). The servers are cheap: at 1 op per
active editor per minute (**UNKNOWN**, a guess until beta telemetry), one m7i.large commits for
~140,000 concurrently active editors. The database, not the servers, is the cost floor until
far beyond that.

### 17.8 Risks and UNKNOWNs added by phase 2

| item | how to find out |
|---|---|
| the RSS growth under load | fixed (unreleased `forge_keep` sketches): 3,185 → 154 MB after 24k ops; the remaining ~4.1 KB/op to explain |
| 3.1 MB per connection | fixed (the outbox's 65,536 preallocated slots): 43 KB now |
| users' op rate and concurrency | phase-2 beta telemetry (ops/min per active editor) |
| managed failover pauses of 60–120 s | beta; then self-managed with automated failover if needed |
| a standby in another zone's commit cost | rerun `phase2_ha.py` across zones once a cloud account exists |
| RDS Multi-AZ DB cluster price (under-35-s failover) | price page for an eligible class |
| GCP us-east prices | the pricing page with us-east1 selected |
| threaded wasm (`THREAD_CACHE`) | the realloc out-of-bounds crash has no root cause yet (CAM-PERF, `research/handoff/CAM-PERF.md` on alloc/threaded-stress); hosted browser clients stay on the unthreaded build (§8) until it has one |

### 17.9 Phase 2 plan (replaces §11's phase 2 line)

1. Fix the review highs first (N1-N9 H1–H3, clients H1–H3) and add "resend pending ops on every
   Welcome" to the protocol (§4.2). **S**
2. Release forge-lang's kept sketches after every validation and hash build; shrink the outbox's
   slot count; soak test with `server_load.rs` (17.7). **S**
3. `PgStore` behind the existing `Store` trait: the 17.1 schema, the fenced append, the
   unique-index dedup, snapshots as rows, ops kept. Run `phase2_pg.py`'s checks as tests. **M**
4. Accounts, sessions, passkeys, roles, view links (17.4). **M**
5. Caddy + Redirect routing + leases (17.5). **S**
6. Jobs, workers, result cache in R2 (17.6). **M**
7. Replica, managed failover drill, backups and the monthly restore drill (17.2–17.3). **S**
Exit: two regions' B2/B3/B4 measured; a failover drill under load with 0 acked ops lost and every
client converged; a restore drill matching every document's head hash.
