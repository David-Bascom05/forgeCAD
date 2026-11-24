# Boolean speed ledger: correctness exceptions

One place for every kernel boolean change let in over the speed band because it fixes a wrong
answer. The rule (the exact callgrind gate, tools/speed/gate.sh, band 0.03%): zero slowdown, or a
judged exception backed by a per-function table. Started 2026-11-24, as research/cam/speed-ledger.md
does for CAM.

**A row says:**
- **what**: the change and where, and the wrong answer it stops;
- **costs**: the gate against main on one machine (bits, total, the lines over band);
- **where**: the per-function table, and why the cost cannot be cut further;
- **granted**: by whom, when.

## proof/open-shell-refuse: an open shell the boolean makes is refused (FLAT-SITES)

- **What.** `kernel/forge-bool/src/boolean.rs` `opened` / `open_shell`: after `apply`, every edge
  of a face the boolean took off, or of a loop it took off a face, left alive and used once
  refuses the op (`Lost::OpenShell`), where it was a wrong Ok (`check`: EdgeUses; drift.md §8;
  KERNEL-STRESS K032-K034 now refuse).
- **Costs.** Against ded2d224: bits identical, total +0.0091%. Over band: coplanar seq +0.194%,
  housing seq +0.148%, patterns seq +0.095%, upto-flat seq +0.061%, plate and torture seq +0.047%.
- **Where.** Coplanar seq (172.87M): +334,120 in `forge_bool::boolean::general` (`opened`
  inlined), nothing else above ±1.2k. It is the scan itself, one read per coedge of the faces
  taken off. Collecting the edges inside `apply` cost three times as much. An O(1) pre-filter
  (live coedges = 2 × live edges) needs a counter kept by forge-topo's `drop_use` and `add_loop`,
  exact only through those paths (unify and others write `alive` and `coedges` directly), so it
  was not built.
- **Granted.** Coordinator, 2026-11-24 22:35 UTC.

## proof/flush-per-plane: the n-ary pass judges a plane flush with all of another plane's faces or none (FLAT-SITES)

- **What.** `kernel/forge-bool/src/nary.rs` `plane_boxes`: two planes within 1e-6 of parallel
  (not to the bit) get one flush verdict over where the whole of each plane's faces meet; it was
  per face pair, not transitive, and left a slit (FaceWrongSide on the next boolean; drift.md §9).
- **Costs.** Against ded2d224: bits identical, total -0.0003%. Over band: plate and torture
  nary1 +0.137%, patterns nary1 +0.088%, upto nary1 +0.059%, hole-grid nary1 +0.056%.
- **Where.** Plate nary1 (121.37M, +165.8k): +116k memcpy inside glibc realloc, with the same
  number of reallocs as main (6,512 vs 6,513), so heap layout (grow in place or copy) and not more
  work; +62k/-61k a codegen rename; real work about +45k (the pair loop +21k, malloc +20k).
- **Granted.** Coordinator, 2026-11-24 22:16 UTC.
