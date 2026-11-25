# Prior-art fact sheet: server-hosted, real-time collaborative parametric CAD

Compiled 2026-11-25. Every URL below was fetched on that date; quotes are verbatim from the fetched page
(ellipses mark trims). "UNKNOWN" means not verified from a primary source, and the entry says what would settle it.
Secondary sources (press) are labelled as such.

---

## 1. Onshape

**Kernel / where geometry is computed (server).** Onshape describes itself as servers plus services, with dedicated "geometry servers". It licenses Parasolid and D-Cubed.
- "They make use of the venerable geometric modeling kernel Parasolid and constraint solver D-Cubed" (licensed from Siemens PLM). Source: M. Lauer, "How Does Onshape Really Work?", 2018-06-28. https://www.onshape.com/en/blog/how-does-onshape-really-work
- "Onshape, on the other hand, is like any other large-scale modern cloud application – a collection of servers and services." / "Others know what is “inside” a document, and keep modeling sessions in memory" / "Others, the admittedly unimaginatively named “geometry servers,” do all the math". (same URL)

**What the client gets (tessellation, not images).**
- "The client doesn’t receive pre-rendered images" … "its graphics are all in the form of triangles, rendered with WebGL (in browsers) or OpenGL (for mobile apps)". (same URL)
- Clients use "both ordinary HTTPS/ReST calls and websocket connections, using our own wire protocol." (same URL)

**Storage and document model (definition vs. derived results).**
- "Onshape stores its data not in files on a disk or in a PDM system vault, but in a database." (Lauer 2018, same URL)
- "The Part Studio definition is what we store in the database and that is where collaborative editing happens." / "The regeneration results are cached, but they can always be rebuilt from the definition." (The regeneration results include "triangles for display".) Source: I. Baran, "Under The Hood: How Collaboration Works", 2015-06-22. https://www.onshape.com/en/blog/under-the-hood-how-collaboration-works

**Microversions (immutable, parent-linked increments).**
- Each definition is "stored as an eternal, immutable object that we internally call a microversion." / "The new microversion stores a reference to the previous (parent) microversion and the actual definition change." (Baran 2015, same URL)
- API docs: "With every edit made in the document, a microversion of every document is automatically created with every edit." / "POST requests are always made to a workspace, since versions and microversions are immutable." / "Work within an Onshape document is organized into branches; each branch contains a workspace and may contain versions." https://onshape-public.github.io/docs/api-intro/
- Undo is not a rewind. The inverse of the user's change is applied to the latest microversion, which keeps collaborators' later edits (Baran 2015, paraphrase of the post).

**Feature Studio / FeatureScript (regeneration = running the feature list as code).**
- "Onshape's standard features are defined in the Feature Studios of a public document, where their source code may be viewed." / "Part Studio regeneration, triggered when features are updated, is execution of the build function." / "The rollback bar is a return statement in the build function, causing subsequent features not to run." https://cad.onshape.com/FsDoc/intro.html

**Concurrent edits to the same Part Studio.**
- Increments from several collaborators "can simply be applied to the latest microversion". When branches are merged, "Merge conflicts should never happen", and the "from" workspace wins a conflicting dimension (Baran 2015, same URL).
- Tech tip, 2021-10-12: same sketch/feature edited at once is "simultaneous changes that could cause a conflict so this should be avoided"; "whoever commits the sketch (or feature) last would have their changes stay"; "Users should consider using Branches when working in the same Part Studio to work more independently." https://www.onshape.com/en/resource-center/tech-tips/tech-tip-why-onshapes-live-collaboration-is-nothing-to-fear
- Upshot: Onshape resolves conflicts per feature at commit time, last committer wins. A feature dialog is a local draft until the user commits it, and presence cues are meant to keep people out of each other's features. (This is our reading of the two sources above.)

**Large assemblies / parallelism.**
- "Onshape computes all element tabs within a document in parallel." / "Versions are immutable and static, so there is less regeneration time involved and no automatic updating." / "Rigid subassembly mates are not required to be solved in higher-level assemblies." / Graphics culling: "For assemblies with fewer than 5,000 bodies, the threshold is 2 pixel screen size." "For larger assemblies, the threshold is 10 pixels." https://cad.onshape.com/help/Content/Home/performance_considerations.htm

**Latency / regen numbers.** UNKNOWN. None of the fetched Onshape pages publish server regeneration latency or edit round-trip figures; the Lauer post "contains no latency figures". To settle it, find an Onshape engineering talk or post with measured p50/p95 regeneration or round-trip times. The "parallelization-now-in-cloud-cad" blog page (https://www.onshape.com/en/blog/parallelization-now-in-cloud-cad) returned no article body when fetched.

---

## 2. Figma multiplayer

**Architecture (Evan Wallace, 2019-10-16).** https://www.figma.com/blog/how-figmas-multiplayer-technology-works/
- "We use a client/server architecture where Figma clients are web pages that talk with a cluster of servers over WebSockets."
- "Our servers currently spin up a separate process for each multiplayer document"
- Not OT: "OTs were unnecessarily complex for our problem space." CRDT-inspired: "Figma's tech is instead inspired by something called CRDTs." / "Figma isn't using true CRDTs though" / "Since Figma is centralized (our server is the central authority), we can simplify our system by removing this extra overhead"
- Property-level LWW: "A conflict happens when two clients change the same property on the same object." … "the document will just end up with the last value that was sent to the server"
- Local echo: the client will "discard incoming changes from the server that conflict with unacknowledged property changes."
- IDs: "assigning every client a unique client ID and including that client ID as part of newly-created object IDs."
- Tree moves: the server rejects parent changes that would create a cycle, and clients temporarily hide objects caught in a transient cycle (paraphrase). Child order uses fractional indexing: "The order of an object's children is determined by sorting them by their positions."
- Persistence of non-document data: "that is stored in Postgres, not our multiplayer system".

**Per-document Rust process (2018-05-02, Evan Wallace).** https://figma.com/blog/rust-in-production-at-figma/
- "The node.js process creates a separate Rust child process per document and communicates with it using a message-based protocol over stdin and stdout."
- "The main problem with the old server was the unpredictable latency spikes during syncing."

**Storage: checkpoints in S3 + journal (WAL) in DynamoDB (Darren Tsung, 2022-10-20).** https://www.figma.com/blog/making-multiplayer-more-reliable/
- The journal is "a durable datastore that is asynchronously written to as multiplayer accepts incoming changes." / "We used DynamoDB as our backing datastore."
- "multiplayer periodically writes the state of the file to storage every 30 to 60 seconds" … "the entire file encoded into a binary format, compressed, and uploaded to S3."
- "Each change is assigned a sequence number, which is an incrementing integer associated with the file."
- Recovery: "multiplayer queries for all entries that are newer (i.e. have a higher sequence number) than the checkpoint." Goal: "<1s of data loss." Metric: "95% of edits to a Figma file get saved within 600ms."

**LiveGraph (2024-05-17, Arushi Bandi).** https://www.figma.com/blog/livegraph-real-time-data-at-scale/
- LiveGraph is "Figma's real-time data-fetching service"; it tails Postgres: "The PostgreSQL replication stream writes all primary database row mutations to a write-ahead log (WAL)."
- UNKNOWN: whether a 2024-era rewrite replaced the DynamoDB journal. No such post was found. A Figma engineering post naming the current journal store would settle it.

---

## 3. Google Docs (OT) and Jupiter

**Google Docs 2010 series (John Day-Richter).**
- Part 2, "Conflict resolution", 2010-09-22: "We save your document as a revision log consisting of a list of these changes." / "The algorithm that we use to handle these shifts is called operational transformation (OT). If OT is implemented correctly, it guarantees that once all editors have received all changes, everyone will be looking at the same version of the document." https://drive.googleblog.com/2010/09/whats-different-about-new-google-docs_22.html
- Part 3, "Making collaboration fast", 2010-09-23: the client tracks "The number of the most recent revision sent from the server to the client", unsent local changes, and changes "sent to the server, but not yet acknowledged". The server keeps "The complete history of all processed changes (called the revision log)". "we never send more than one pending change at a time." "The server will use OT to transform John’s change so that it can be stored as Revision 3." https://drive.googleblog.com/2010/09/whats-different-about-new-google-docs.html

**Jupiter (Nichols, Curtis, Dixon, Lamping, Xerox PARC, UIST 1995; DOI 10.1145/215585.215706).** Abstract: https://uist.acm.org/uist1995/abstracts/Nichols.html. Full text (third-party mirror, matches the abstract): https://lively-kernel.org/repository/webwerkstatt2011/projects/Collaboration/paper/Jupiter.pdf
- "Our algorithm is derived from a fully distributed, optimistic algorithm developed by Ellis and Gibbs [12]. Jupiter’s centralized architecture allows us to substantially simplify their algorithm."
- "Instead, Jupiter uses the optimistic protocol only for the individual server-client links. To the server, each client appears to be operating synchronously with respect to the server’s actions."
- "When messages cross on the wire, each receiver fixes up the incoming message so that it makes sense relative to the receiver’s current state."

---

## 4. CRDT literature (ordered lists, moves, trees)

- **Tree move.** Kleppmann, Mulligan, Gomes, Beresford, "A highly-available move operation for replicated trees", IEEE TPDS 33(7):1711–1724, DOI 10.1109/TPDS.2021.3118603. Sources disagree on the year (2021 or 2022); see https://researchr.org/publication/KleppmannMGB22/bibtex. PDF: https://martin.kleppmann.com/papers/move-op.pdf
  - "we present a CRDT algorithm that handles arbitrary concurrent modifications on trees, while ensuring that the tree structure remains valid (in particular, no cycles are introduced)". The abstract also reports bugs in Google Drive and Dropbox under concurrent moves. Mechanism: a replica keeps a log of moves in timestamp order and undoes and redoes them when an out-of-order op arrives ("undo op, redo op").
- **List move.** Kleppmann, "Moving Elements in List CRDTs", PaPoC '20, DOI 10.1145/3380787.3393677. https://martin.kleppmann.com/papers/list-move-papoc20.pdf
  - Delete+reinsert duplicates under concurrency: "We argue that such duplication in the face of concurrency is undesirable". The fix: "we can use a long-established CRDT: a last-writer wins (LWW) register … We need one such register for each list element, containing the position of that element." It works "in conjunction with any existing list CRDT algorithm"; moving ranges is left as an "open problem".
- **Interleaving anomaly.** Kleppmann, Gomes, Mulligan, Beresford, "Interleaving anomalies in collaborative text editors", PaPoC 2019. https://martin.kleppmann.com/papers/interleaving-papoc19.pdf
  - "concurrently inserted portions of text with a well-defined order may be randomly interleaved on a character-by-character basis". The paper finds that Logoot and LSEQ suffer from this, and that RGA shows "a lesser variant of the anomaly".
- **JSON CRDT move.** Da & Kleppmann, "Extending JSON CRDTs with Move Operations", PaPoC 2024: "We plan to integrate this algorithm into the Automerge CRDT library." https://martin.kleppmann.com/2024/04/22/json-crdt-move.html
- **Automerge lists.** Supported operations are "push, unshift, insertAt, deleteAt, splice"; the page lists no move. https://automerge.org/docs/reference/documents/lists/ UNKNOWN: whether native move has since shipped. The Automerge changelog would settle it.
- **Yjs Y.Array.** Only insert/delete/push/unshift are documented; there is no move. https://docs.yjs.dev/api/shared-types/y.array
- **Local-first software.** Kleppmann, Wiggins, van Hardenberg, McGranaghan, Ink & Switch, 2019 (Onward! 2019). https://www.inkandswitch.com/essay/local-first/
  - "CRDTs have the potential to be a foundational technology for realizing local-first software." Servers play a role "in a supporting role, not the source of truth." The seven ideals: no spinners; not trapped on one device; network optional; seamless collaboration; the long now; security and privacy by default; user ownership and control.

---

## 5. Server-ordered / rebase sync engines

**Replicache (Rocicorp).** https://doc.replicache.dev/concepts/how-it-works
- "In Replicache, the server is authoritative." / "local changes to the space in a client are immediately (optimistically) visible". Local changes are "pending (optimistic)" until pushed.
- "changes (mutator invocations aka mutations) that have run locally against Replicache are re-run on the server when pushed."
- Rebase: Replicache "rewinds" the Client View "to the last version it got from the server", applies the patch, then "replays any pending mutations on top." "It’s possible and common for mutations to calculate a different effect when they run during rebase."
- "Replicache therefore discards any pending mutations it has for each client with id ≤ lastMutationID."
- Status: rocicorp.dev lists "Reflect (Deprecated)" and Zero as "General‑purpose sync for the whole web". https://rocicorp.dev/

**Linear.**
- "Each workspace has its own immutable ordered sync action log where new sync actions are appended." / "the client sends a checkpoint containing the ID of the last change it applied." / "Postgres is the right place to commit and retain our client-facing WAL." (2026-08-18) https://linear.app/blog/rebuilding-delta-sync-read-path
- The talk page (2023-06-29) gives no protocol details: https://linear.app/blog/scaling-the-linear-sync-engine. Secondary source (Liveblocks, 2025-12-17): "The server assigns each change a monotonically increasing sync ID, and conflicts resolve to the highest ID (effectively last-write-wins, but server-ordered)." https://liveblocks.io/blog/understanding-sync-engines-how-figma-linear-and-google-docs-work

**Liveblocks Storage.** "LiveObject is a CRDT-like data structure that allows these changes to be merged automatically." / "Each property is independent, and changes to different properties never conflict." (same Liveblocks URL)

**Pattern summary (synthesis of the sources above).**
1. The client applies a mutation locally right away and queues it as pending.
2. The server is the single sequencer. It assigns a total order (a Figma per-file sequence number, a Linear sync ID, a Google Docs revision number, a Replicache lastMutationID) and persists the op.
3. The server broadcasts the ordered ops; the sender gets an ack.
4. On receiving ops, the client rewinds to the last confirmed state, applies the server ops, and replays its still-pending ops on top (Replicache). Google Docs/Jupiter instead OT-transform the incoming op against pending ones; Figma drops incoming values that conflict with its own unacked property writes.
5. Conflicts resolve by server order (LWW per property or field) or by re-running mutators against the new state.

---

## 6. WebAssembly floating-point determinism (spec: "WebAssembly 3.0 (2026-11-24)")

Source: https://webassembly.github.io/spec/core/exec/numerics.html
- "Floating-point arithmetic follows the IEEE 754 standard, with the following qualifications: All operators use round-to-nearest ties-to-even, except where otherwise specified. Non-default directed rounding attributes are not supported."
- Scalar add/sub/mul/div/sqrt are defined as the exact real result rounded once, for example fadd_N(z1,z2) = float_N(z1 + z2) and fsqrt_N(+q) = float_N(√q). So f32/f64 add/mul/div/sqrt are correctly rounded per IEEE 754. The rounding rule: "Rounding always is round-to-nearest ties-to-even, in correspondence with IEEE 754 (Section 4.3.1)."
- NaN: "its sign is non-deterministic" … "If the payload of all NaN inputs to the operator is canonical (including the case that there are no NaN inputs), then the payload of the output is canonical as well. Otherwise the payload is picked non-deterministically among all arithmetic NaNs" … "In the deterministic profile, only positive canonical NaN outputs are produced."
- Relaxed ops: "The result of relaxed operators are implementation-dependent, because the set of possible results may depend on properties of the host environment, such as its hardware." … "These choices are fixed, that is, parameters are constant during the execution of any given program." … "In the deterministic profile, every parameter is prescribed to be 0."
- relaxed_madd: "Relaxed multiply-add allows for fused or unfused results, which leads to implementation-dependent rounding behaviour. In the deterministic profile, the unfused behaviour is used."

**Deterministic profile.** https://webassembly.github.io/spec/core/appendix/profiles.html
- "It defines a sub-language that does not exhibit any incidental non-deterministic behaviour: All NaN values generated by floating-point instructions are canonical and positive. All relaxed vector instructions have a fixed behaviour that does not depend on the implementation. Even under this profile, the memory.grow and table.grow instructions technically remain non-deterministic".
- "producers should not generate code that depends on specific profiles". Browsers implement the full profile.

**Relaxed-SIMD proposal overview.** https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md
- "This proposal adds a set of useful SIMD instructions that introduce local non-determinism (where the results of the instructions may vary based on hardware support)." madd may be computed with "a total of 2 roundings" or "rounded once (if supported by hardware)."

**Design rationale.** https://github.com/WebAssembly/design/blob/main/Rationale.md and Nondeterminism.md
- "at a high-level most operators follow IEEE-754 semantics, it is therefore not necessary to specify WebAssembly's floating-point operators differently from IEEE-754."
- NaN bits are observable only via `reinterpret`, a store followed by a differently typed load, or the host boundary. Shared-memory threads are also listed as nondeterministic.
- x87 / implicit FMA contraction: UNKNOWN as an explicit statement. No fetched page mentions x87. It follows from the per-operation single-rounding definitions above that extended-precision intermediates and implicit fusion are not permitted for scalar ops (this is an inference). Note also that transcendental functions such as sin/exp/pow are not wasm instructions; they come from the compiled libm, so they are deterministic only if every client and server runs the same wasm libm.

---

## 7. Browser wasm threads and memory limits

- MDN SharedArrayBuffer: "To use shared memory your document must be in a secure context and cross-origin isolated." / "When cross-origin isolated, postMessage() no longer throws for SharedArrayBuffer objects". https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer
- web.dev headers: `Cross-Origin-Embedder-Policy: require-corp` and `Cross-Origin-Opener-Policy: same-origin`; "SharedArrayBuffer on Chrome desktop requires cross-origin isolation starting from Chrome 92." The article also covers COEP `credentialless`, from Chrome 96. https://web.dev/articles/coop-coep
- Memory limits (JS API spec): "The maximum size of a 32-bit memory is 65,536 pages (4 GiB)." / "The maximum size of a 64-bit memory is 262,144 pages (16 GiB)." https://webassembly.github.io/spec/js-api/
- Feature status (raw data behind https://webassembly.org/features/: https://github.com/WebAssembly/website/blob/main/features.json, fetched 2026-11-25):
  - Memory64: Chrome 133, Firefox 134, Safari "flag" ("Supported in Safari Technical Preview 251"). So Safari stable does not ship memory64 as of this date.
  - Threads: Chrome 74, Firefox 79, Safari 14.1 ("iOS Safari since 14.5").
  - Relaxed SIMD: Chrome 114, Firefox 145, Safari "flag" ("Requires JavaScriptCore flag `useWebAssemblyRelaxedSIMD`").

---

## 8. Transport

- WebSocket (MDN browser-compat-data): Chrome 5, Firefox 11, Safari 5, so it is universal. https://github.com/mdn/browser-compat-data/blob/main/api/WebSocket.json
- WebTransport on MDN: "Baseline 2026 / Newly available … Since March 2026, this feature works across the latest devices and browser versions." "Some parts of this feature may have varying levels of support." https://developer.mozilla.org/en-US/docs/Web/API/WebTransport. BCD: Chrome 97, Firefox 114, Safari 26.4 (iOS mirrors). https://github.com/mdn/browser-compat-data/blob/main/api/WebTransport.json
- WebKit blog, 2026-03-24: "Safari 26.4 adds support for WebTransport, a modern API for low-latency, bidirectional communication" / "It runs over HTTP/3 and QUIC, providing multiple streams that won’t block each other." / "WebTransport can run over HTTP/2 and TCP as a fallback with the same API." https://webkit.org/blog/17862/webkit-features-for-safari-26-4/
- Implication: WebTransport now works in current major browsers, but users on Safari before 26.4 need a WebSocket fallback.

---

## 9. Durability semantics

**PostgreSQL (docs for v19).** https://www.postgresql.org/docs/current/runtime-config-wal.html
- synchronous_commit: "The local behavior of all non-off modes is to wait for local flush of WAL to disk. In off mode, there is no waiting … (The maximum delay is three times wal_writer_delay.) Unlike fsync, setting this parameter to off does not create any risk of database inconsistency: an operating system or database crash might result in some recent allegedly-committed transactions being lost". It can be set per transaction: "SET LOCAL synchronous_commit TO OFF".
- With standbys: on = standby "flushed it to durable storage"; remote_write = standby written to its file system, not durable across a standby OS crash; remote_apply = applied and visible on the standby.
- fsync: "While turning off fsync is often a performance benefit, this can result in unrecoverable data corruption in the event of a power failure or system crash."

**SQLite.** https://www.sqlite.org/pragma.html#pragma_synchronous and https://www.sqlite.org/wal.html
- FULL: "FULL is atomic, consistent, isolated, and durable (ACID) in WAL mode".
- NORMAL: "WAL mode is always consistent with synchronous=NORMAL, but WAL mode does lose durability. A transaction committed in WAL mode with synchronous=NORMAL might roll back following a power loss or system crash. Transactions are durable across application crashes regardless of the synchronous setting or journal mode."
- wal.html: "Writers sync the WAL on every transaction commit if PRAGMA synchronous is set to FULL but omit this sync if PRAGMA synchronous is set to NORMAL." / "All processes using a database must be on the same host computer; WAL does not work over a network filesystem."

**FoundationDB.** https://apple.github.io/foundationdb/known-limitations.html (page "Last updated on Nov 17, 2026")
- "Transaction size cannot exceed 10,000,000 bytes of affected data." / "If any single transaction exceeds one megabyte of affected data, you should modify your design."
- "Keys cannot exceed 10,000 bytes in size. Values cannot exceed 100,000 bytes in size."
- "FoundationDB currently does not support transactions running for over five seconds." After 5 s from the first read, a commit raises "transaction_too_old or not_committed".
- Guarantees: "FoundationDB provides global, ACID transactions with strict serializability using optimistic concurrency." / On commit_unknown_result, "retry loops that don’t check for commit_unknown_result could execute the transaction twice. In these cases, you must consider the idempotence of the transaction." https://apple.github.io/foundationdb/developer-guide.html
- Versionstamps: "The versionstamp guarantees uniqueness and monotonically increasing values for the entire lifetime of a single FDB cluster." https://apple.github.io/foundationdb/data-modeling.html

---

## 10. Anthropic: can a third-party hosted web app use an end user's Claude subscription?

**Answer from Anthropic's own docs: No, unless Anthropic has approved it.** Third-party products must use API-key authentication (or a cloud provider). There is one carve-out: a platform may host the unmodified Claude Code binary, and the end user signs into it with their own subscription.

- Agent SDK overview: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK. Use the API key authentication methods described in the Quickstart instead." https://code.claude.com/docs/en/agent-sdk/overview
- Claude Code Legal & compliance, "Authentication and credential use" (no date shown on the page; fetched 2026-11-25): https://code.claude.com/docs/en/legal-and-compliance
  - "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications."
  - "Developers building products or services that interact with Claude's capabilities, including those using the Agent SDK, should use API key authentication through Claude Console or a supported cloud provider. Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow."
  - Carve-out: "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code as described under Can customers offer Claude Code in their products?"
  - Hosting Claude Code requires the Commercial Terms, and "The Claude Code binary must not be modified" … "customers may not remove, disable, or restrict any authentication method built into it". Also: "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key, Claude subscription plan credentials, or 3P inference provider credential".
  - "Advertised usage limits for Pro, Max, and Ultra plans assume ordinary, individual usage of Claude Code and the Agent SDK." / "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice."
- Consumer Terms (Effective Oct 8, 2025). https://www.anthropic.com/legal/consumer-terms
  - Prohibited: "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it, to access the Services through automated or non-human means, whether through a bot, script, or otherwise."
  - "You may not share your Account login information, Anthropic API key, or Account credentials with anyone else. You also may not make your Account available to anyone else."
- `claude setup-token` / CLAUDE_CODE_OAUTH_TOKEN (first-party CI/scripts only). https://code.claude.com/docs/en/authentication
  - "For CI pipelines, scripts, or other environments where interactive browser login isn't available, generate a one-year OAuth token with `claude setup-token`". "This token authenticates with your Claude subscription and requires a Pro, Max, Ultra, Team, or Enterprise plan. It can only make model requests, so it can't establish Remote Control sessions or fetch claude.ai connectors."
  - GitHub Actions: `CLAUDE_CODE_OAUTH_TOKEN` is "an OAuth token that authenticates with your Claude subscription". For a secret shared across repositories, Anthropic recommends a Console API key "since an OAuth token is tied to the subscription of the person who ran `claude setup-token`." https://code.claude.com/docs/en/github-actions
- Remote Control: Claude "keeps running locally the entire time". Requirements: "available on Pro, Max, Ultra, Team, and Enterprise plans. API keys are not supported." https://code.claude.com/docs/en/remote-control
- Timeline (secondary, The Register, 2026-02-20): Anthropic "revised its legal terms to make clear that subscription OAuth tokens may be used only in Claude Code and Claude.ai". The article quotes the compliance page wording of the time: using such OAuth tokens "in any other product, tool, or service … is not permitted and constitutes a violation of the Consumer Terms of Service". https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access. The current page wording is the one quoted above, not this February text.
- **Design implication.** A hosted CAD web app that wants Claude features must use (a) its own API key, billed to the operator, (b) a bring-your-own API key per user, or (c) a cloud provider such as Bedrock, Vertex or Foundry. "Log in with Claude" for subscription quota is not allowed without explicit approval from Anthropic sales. The only subscription-backed route is hosting the unmodified Claude Code binary, where the user signs in through Anthropic's own flow. Even then the operator may not intermediate or resell usage, and whether a CAD app embedding Claude Code fits that route is UNKNOWN. A written answer from Anthropic (https://www.anthropic.com/contact-sales) would settle it.
