# Names and arena order

The skip path's differential (research/rebuild) showed that names on main followed the order in a
body's arenas: a fillet's vertices its edge order, which is the selector's, which is the arena's; new
vertices' ties `part_frame`, from the arena's first face, which a fillet reorders; a chamfer's corner
its first corner in the arena. Reversing a stage's arenas renamed entities on 7 of 482 bench lines
(with sorted edges and the frame fix 2; with the chamfer's corner too, jack's text only).

To the bit: reversed arenas moved points by ulps on many lines (28 of 160 on five parts: `settle` sums in the
order it is given, a cut cluster's point is its first cut's). A boolean now meets faces in order of name,
and every feature that changes the part but a fillet, and a pattern's pass, work on `Body::into_canonical`:
0 of 482 bench lines differ in any point's bits, jack's text included.

The fix: `apply_fillet` and the chamfer take edges sorted by name; `part_frame` takes the least-named
face; a chamfer's corner starts from its least-named face. `tests/arena_order.rs` holds it.

Rerun: copy `nameprobe.rs` to interface/forge-agent/examples/ on two checkouts, `nameprobe dump > F` on
each, `python3 renames.py F_OLD F_NEW`; `cargo run --release -p forge-model --example namehunt -- 200 SEED`,
also with `--inserts` and `--big`.
