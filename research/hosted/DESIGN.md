# Hosted, multi-user forgeCAD: design

Status: design only, 2026-11-25, branch `design/hosted`. No product code changed. Every number
comes from `MEASUREMENTS.md` (measured here) or `PRIOR-ART.md` (cited sources, fetched
2026-11-25). Where neither settles a question, it says **UNKNOWN** and how to find out.

## 0. The answer in one page

| choice | decision | the evidence that decides it |
|---|---|---|
| sync model | **server-ordered log of typed ops; a late op is re-applied to the head (rebase) and re-validated; Rename rewrites later ops' names; same-param edits are last-writer-wins** | 251 real documents, ~19k op pairs both orders: 96.9% of different-target pairs commute exactly; every non-commuting case is Rename-before-a-user-of-the-old-name (fixable by substitution) or list position (both apply, server order decides). Lean: SetParam commute and LWW proven (`lean/Merge.lean`). |
| where geometry is computed | **hybrid: every client rebuilds the document locally (as today); compute workers produce heavy, slow-changing results (imports, bend sim, thermoform, CAM, exports, first-open meshes), cached by content hash** | warm rebuild after an edit is 1.5 ms p50 / 38 ms p90 locally, below any network round trip; 0 of 276 documents differ in body or mesh bits across x86-64-v3, v2, one core, wasm/Node and wasm/Chromium (223 that loaded); but the Haas VF-2 takes 13.8 s to open natively and its STEP cannot be meshed on main, and bend sim takes 3.85 s for a 21.7 KB result. |
| op-log store | **Postgres** (op log, documents, permissions in one transactional store); **content-addressed blobs** (snapshots, meshes, STEP, results) on a filesystem or object store | every durable candidate appends in 0.12–0.20 ms p50 on the same disk, so latency does not decide; Postgres is needed anyway for accounts and sharing, and lets permission check + append be one transaction. FoundationDB and MongoDB: UNKNOWN (not runnable here). |
| history and git | **op seq = the live history (microversions); git stays the version and interchange layer**: a named version or an idle checkpoint becomes a forge-history commit and tag | forge-history save per edit: 3.4–5.2 ms p50, 17–21 ms p99 = 20–30× an op append; git commit per keystroke-level op would also be the wrong granularity for versions. |
| transport | **WebSocket** for ops and presence; **HTTPS GET** of content-addressed blobs (CDN-cacheable) | WebSocket works in every browser; WebTransport is "Baseline 2026" (Safari 26.4, March 2026). Ops are ~150–250 B. |
| offline / local | **unchanged**: local forgeCAD keeps files, `F.fg.log`, forge-history; the hosted code lives in new crates and is never linked into the gate binaries | the speed gate's binaries (regen1, bench) do not need a line of the sync code. |
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
| B3 | open a big project | **≤ local open; first picture ≤ 1 s + mesh download** | Haas VF-2: local native open 13.8 s (and no mesh: meshing the raw STEP fails on main); wasm ~18 s estimated. Hosted with cached display meshes: download 4.22 MB (VF-2's shipped meshes); edits need the import's B-rep only if a feature reads it | hosted is **faster** than local for imports; ordinary parts: cold build 3.5 ms p50, 1.2 s p90, 3.9 s p99 → the server's cached meshes give the first picture, the client builds behind it |
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

### 4.3 Can it be proven?

Yes, in the existing ForgeLang model, op type by op type:
- **Done here** (`lean/Merge.lean`, builds against `research/forge-lang/lean`, no sorry):
  `setLit_comm` (SetParams on different sites commute), `setLit_lww` (same site: last wins),
  `concurrent_setParam_traces` (two unobserved SetParams leave traces, ids and control log as
  they were: a concurrent pair of dimension edits cannot change the feature list).
- **Next** (sizes are guesses, S ≈ 1–2 days of a proof agent, M ≈ a week):
  - Rename transform: `apply (rename a b S) (rn a b op) = rename a b (apply S op)` for each op
    kind — from `RenamePart`/`RenameDefs` (M).
  - Different-statement edits commute: ReplaceFeature/SetArg on different statements, Delete of a
    statement nobody reads, Add at different anchors — from `Struct`, `Complete`, `EditClosure`
    (M).
  - Convergence of the protocol: every client that applies the server's sequence ends at the
    server's text — this is by construction (a total order, a deterministic `apply`); stating it
    is S; proving client replay equals server application needs the rebase lemma per op (M).
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
0.156 ms, libSQL 0.167 ms, file + fdatasync 0.119 ms, Redis AOF always 0.202 ms p50;
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
- FoundationDB: strict serializability, versionstamps for ordering, but 10 MB transaction / 100 KB
  value / 5 s limits (PRIOR-ART §9) and its latency **UNKNOWN** here (its downloaded server binary
  could not be run in this sandbox). MongoDB: **UNKNOWN**, same reason. Measure both with
  `bench/oplog_bench.py` on a machine that may run them before phase 2 if Postgres shows a
  problem.
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

What I could establish: the threaded browser build uses forge-alloc's eight spin-locked heaps
(`docs/web.md`); per-thread small-block caches (34d6cad8) and a quick look before flushing
(c0c6143f) are on `cam/perf-rest`, not main; `alloc/threaded-stress` (3c10b9f2) runs the
allocator natively on 8–16 threads and passes, and "fails at once with owner_of taking no
lock". The specific "known race" was not found written up; I asked the coordinator for a
pointer. **UNKNOWN** until then: its trigger and frequency.

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
| warm-cache assembly pathology (4–6 s warm vs 10–30 ms fresh, MEASUREMENTS §b) | reproduced 3× on 2 files | B1/B2 blown for those edits, locally too | report to the owner (done); hosted adds nothing |
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
   threaded-artifact hash run once the race pointer arrives; FDB/MongoDB where they may run;
   WebSocket RTT probe.
2. **Phase 1 — single-server collaboration (M).** `forge-sync` (op envelope, rebase, Rename
   transform, inverse ops), sequencer in `forgecad web` (one process, SQLite-or-Postgres behind a
   trait so the choice is swappable), WebSocket, presence, build-hash reports, hosted undo,
   named versions as forge-history commits. Lean: Rename transform lemma, different-statement
   commutation. Exit: two browsers + an agent editing one part; B1/B2 measured on a LAN.
3. **Phase 2 — hosted service (L).** Accounts, projects, sharing, Postgres store with replica
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
