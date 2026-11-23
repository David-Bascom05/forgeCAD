# Work streams: status and how to resume

Last updated 2026-11-23 (after PR #6, main ec328182). Each row names the branch (`claude/repo-review-mbdrlj-<suffix>`), its head when the stream stopped or was last reported, and where its own notes live. A paused stream is resumed by starting a session on its branch and reading the notes file first. Branch heads are on origin and are kept after a session ends.

Owner priorities that paused or narrowed streams (2026-11-23):
- Proofs only for code an active feature is changing. Whole-kernel proving (Aeneas sweeps, validity-check L4, peripheral L2/L5) is de-prioritised.
- Sketch text and surfaces/thin: finish current work as features, then stop.
- Mass import of other kernels' CAD programs (DeepCAD, CADFS, Fusion reconstruction) is paused. New file types (3MF etc.) and STEP-reader bugs already found continue.
- Speed rule (owner, 2026-11-23): user-facing wall-clock time at the same thread count must be ≤ base; instruction counts are diagnostic only. "If flat bool can be made faster with the same number of threads I don't really care how many instructions it is. If it's just faster because of better threading in that algo it should win." The wall-clock gate (tools/speed/latency.sh) is being built by session_01Q19LzzwaZhmXMt4M3NiseR; until it is validated the callgrind instruction gate is the interim check. Once it lands, its baseline is re-saved on main and merges are judged by it.
- Dropped: DWG decompressor for 4 LibreCAD files, Google Fonts scale run, IGES, sketch-text "cut letters out of regions", certified-tracing pipes (deferred).

## Active

| Stream | Branch suffix | Head | Status / next |
|---|---|---|---|
| New flat boolean (decide-once / A8), Rust | `decide-once` | 25b4d52 | R1 types, R2 core done; R3 direct face builder step 1 done (facet counts match). Next: rename `v2` module (Aeneas name clash), builder speed numbers vs 0.92M/1000-block budget, then R3 steps 1–5, R4 gate. Design: research/decide-once/face-arrangement.md |
| New flat boolean, Lean | `decide-once-lean` | 2d03a8b | L1 L2 L4 L5 and coverageMI done. Next: EndToEndF over the Rust v2 types, R3 wedge lemma |
| Boolean speed + write path | `bool-speed` | 6596d98 | Switch-off speed-ups (94/97 lines lower) to be cherry-picked to a `bool-speed-off` branch by the merge session. Next: seed-29 lineage fix in forge-lang build.rs, stitch skeleton on v2 types, edge-name parity |
| Curved boolean, Lean exact reference | `dol-curved` | 4e2ef57 | Integrator for q1/LemmaD/tier0 (all ended, branches below). Proposers, sweep, sample, ledger, bounding sphere done. Next: qsampler_covers (facets), edges/verts via checked route-B R1 certificate |
| Curved boolean, Rust | `dol-curved-rust` | 072f3cf | Exact proposers mirror Lean; forge-alg crate (Knuth D, Lehmer); L2 reference under test. Next: curved builder stages a–f on v2 types |
| Curved boolean fixes | `fix-curved` | ecf4db5 | Pole, lug/drill fixes; gates running. Next: kernel causes from scale-programs (InconsistentClassification, ZeroThickness gaps) |
| Touch decisions | `fix-touch` | 20afa5b | Final. 19 scoreboard rows ok → reject (each non-manifold per the reference; list in its report), suite fuzz seed 55, centre_on_rim error → ok, drill_from_tangent_plane NotClosed → builds. With fix-curved: 19e1b6c's uv_chord + this branch's uv_dir, singular_way, pole_frame |
| Exact curved reference (fuzz/oracle) | `exact-curved` | 8a04dcb | Root completeness done; torus band running. Repros: research/kernel-audit/curved-repros/ |
| Certified tracing | `trace` | fa705fb | 1.2–2.9× cost, off by default. Next: counted 1D match, then Aeneas L4 of interval.rs/certify.rs. Pipes deferred |
| IEEE 754 / predicates | `ieee` | 093a50b | All 12 decide-once screens, interval sin/cos, Big sign/eq done. Next: Q/divrem/gcd specs for forge-alg |
| Aeneas | `aeneas` | b68b72b | dmath done; sum_into/scale_into refine list models; SURVEY.md closed (topology core deferred). Next: move v2 translation into verify/aeneas/forge-bool-core |
| Fillet fixes | `fix-fillet` | d4f2095 | Rim-and-back rounds build right (p00010, p00133, p00164, p00188; p00094, p00128, p00199 used to fail in later booleans); 0 worse, fuzz 150/150, n-ary 167/167, bits and wasm identical. In the merge queue. Next: CADFS 00154651 stack overflow, then the 4 refused notched rounds |
| Fillet census | `fillets` | 5acd69f | Relabel and pipe OCCT check finishing; census.md update |
| Sheet metal fixes | `fix-sheet` | a6aee8e | Round 6 merged in. Arcs with stored end points (Curve2::Arc from/to; forge-brake updated), wired fold zone, exact signs via forge_sheet_core::exact, FoldedUndecided, zone touch detection; 62 sheet parts' DXF/DWG byte-identical, 1206 tests, SHEET_LEAN pass, 0 worse, fuzz 150/150, bits identical. In the merge queue; correctness items continue |
| Draft | `fix-draft` | 7e8f3fa | Final, merge-ready: curved fuzz 300/300, convex fuzz 348/349 exact (base: 117 Invalid, 1 panic), every user-facing line below base, bits identical, 0 worse, fuzz 150/150 |
| Sweep / revolve | `sweep-spec` | 805c67a | Pinch, frame, exact selfx. Next: n-ary shape-only selfx check, exact ToNext; Aeneas item skipped |
| Tessellation L4 (Aeneas cdt.rs) | `cdt-l4` | f91b25d | In progress |
| CAM | `cam` | 3e8a91d | Cones/tori, section pockets. Next: G0-plunge printer fix (top), vises + custom soft jaws, per-check speed, ramps/engrave/thread mill. Design: research/cam/design.md |
| CAM Lean | `cam-lean` | 31c09c0 | verify/forge-cam/RESUME.md. Vise proofs in; PAUSED until the vise/soft-jaw code lands, then corrected G2 against it |
| Import robustness | `import-robust` | e27953b | Next: speed fix (per-call nesting count), ZIP64 (HIGH), FCStd SideType, wrong-side faces, OFFSET_SURFACE |
| Web | `web` | b8d879d3 | Final, merged with round 6: HiDPI and first-frame fixes, sharded forge-alloc heaps for the threaded build (rack 32 s → 2.2 s), parity fail-fast; web CI 21.7 → 10.5 min. forge-alloc joins the hidden-inputs scan |
| UI | `ui` | 2c03a34 | Vendor/DFM polish 33db3ea is local only (push to `ui-vendor` needs owner approval). Waiting on surfaces/sketch-text round-6 merges |
| Validity check (fundamentals) | `fundamentals` | 9692711 | research/fundamentals/RESUME.md. Shell faces, edges, loops at L4. Ending after the PROOF-STANDARD note on dropping post-build checks for proven ops |

## Finishing current work, then stopping

| Stream | Branch suffix | Head | What's left (notes) |
|---|---|---|---|
| Sketch text | `sketch-text` | 5557c8f1 | Final (round 6 merged): CFF reader at L4, bezier-margin soundness fix, TTF lsb fix; bits identical, 0 worse, fuzz 150/150, n-ary 167/167; manifold seq/nary1 diagnostic lines ~+0.2% (latency gate to check). In the merge queue. Resume: research/sketch-text/whats-left.md |
| Surfaces / thin | `surfaces` | 3d11b00 | Round-6 merge, spline extension past ends. Left: forge-sheetops, L1 thicken spec, L6 fuzz kind, sheets.py in CI |
| Conic fillets | `conic-fillet` | ee31e8a | Final (round 6 merged, phase 2 in): no line over base, bits identical, 0 worse, fuzz 150/150, n-ary 167/167, native = wasm. In the merge queue. Resume: research/fillets/conic/LEFT.md |

## Paused or ended (resume from the notes)

| Stream | Branch suffix | Head | Notes / what's left |
|---|---|---|---|
| CAD programs → .fg at scale | `scale-programs` | 512e3ab | research/import/RESUME.md; 5k report in research/import/scale.md. Kernel-cause repros sent to fix-curved and fix-fillet. Language gaps and arc rule not started |
| Importer coverage | `import` | 4246cb4 | research/import/paused.md: scale counts, ranked cancelled queue, arc-rule findings |
| STEP real parts at scale | `scale-step` | ee56795 | research/step/RESUME.md; research/step/scale.md (5k report, top-10 causes, repros in research/step/repro/). Paused; kernel fixes in forge-step read.rs (closed-spline rim edge range, near-flat cone) in the merge queue |
| Spline spec | `spline-spec` | e40f675 | research/spline-spec/DESIGN.md "What is left" (L4 for through/through_ends/from_nurbs, L5 bounds, past-the-ends) |
| Ops spec (shell/draft/hole/rib/chamfer) | `ops-spec` | f6d4d77 | research/ops-spec/unfinished.md |
| Curved Q1 / Lemma D / Tier0 | `dol-curved-q1`, `dol-curved-lemmad`, `dol-curved-tier0` | 14e9e84, a69987c, 123b299 | Merged into `dol-curved`; dol-curved owns the curved Lean |
| Sheet metal spec | `sheet-spec` | 48112df6 | Ended. disc and line_gap proven on the shipped forge_sheet_core::exact (L1–L6), no kernel change; research/expansion-theory/README.md (the false Shewchuk theorems, the W2 plan). Its verify/ieee754 files are being wired in by the IEEE session. forge-sheet-core's overflow-checked translation (cbbab144) still to fold in |
| Leftover topology | `fix-tidy` | 8215e66 | Ended, merge-ready (code 13aaa36): unmerged faces 10450 → 0, strict gate ≤ base, bits differ only on 16 justified lines, 0 worse, fuzz 150/150. In an early post-PR-#6 batch |
| Tessellation | `fix-mesh-asm` | bdde473c | Ended. Sphere/cone/torus sag rules, a torus-through-axis hole fixed, STEP reader fixes d08ce5b/ba4fba0 (NURBS projection, heal-walk: spool.step). Gates: callgrind no line over, bits identical, 1900 tests, wasm 35/35; changed meshes only out-of-tolerance ones (list in the commit message). Take bdde473c whole (510c3ffa alone breaks cargo). Resume: research/tessellation-proof/README.md |
| Formats corpus | `formats-corpus` | 7846c3b | Ended (on import-robust ad8c3d7; merge with or after it). Kernel fixes: far origins 8164af8, STEP assembly placements 4687a3d, tori and whole faces 512ddac, DWG pages ec20825; corpora and judges. Resume: research/formats/RESUME.md; report research/formats/real-files.md |
| Point classification (index) | `fix-index` | ebf6504 | research/kernel-audit/index-fix-per-line.txt; waits for a merge batch |
| .fg speed vs .fc | `speed` | 43c39aa | Ended; reparse work |
| Fillet spec | `fillet-spec` | fc90a16 | Fillet obstacle definition was an owner decision; idle |
| Boolean prior art | `bool-research` | baa5b47 | research/booleans/ (prior-art.md, small-booleans.md, a8-review.md) |
| Certificate checker | `certify` | 901f15dd | Ended. Lean-proven boolean certificate checker (verify/forge-certify) with a Rust harness crate; kernel hooks behind forge-bool's `certify` feature (off by default, byte-identical to base); CI and on-demand only. verify/forge-certify is its own Cargo workspace (outside the root, like verify/forge-exact) with its own CI job, so the workspace test gate tests the shipped, certify-off kernel. Resume: verify/forge-certify/DESIGN.md §9 |
| Weldments, vendors, bendsim, mesh export | `weldments`/`weld-spec`, `sheet-vendors`, `bendsim`, `mesh-export` | 8249f49/efe7379, fd5f5e2, 1d04882, 4e36762 | In main (PR #6, ec328182) |

## Merge queue (after PR #6)

Small batches of 3–5, each through the full gates: bool-speed-off (cherry-picked), web, ui-vendor (if approved), fix-index, ops-spec, spline-spec, import, aa88eaf (hidden-inputs audit), fundamentals, then fix-fillet, fix-tidy, fix-draft, sweep-spec, fix-curved + fix-touch, fix-sheet, sheet-spec, surfaces, sketch-text, conic, decide-once (own PR, after bool-speed-off).
