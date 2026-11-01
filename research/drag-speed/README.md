# What a frame of a drag costs

David's measure: a number dragged on a real part, `Model::update` per value. `dragcost.rs` (copy to `interface/forge-agent/examples/`, build, then
`sh research/drag-speed/run.sh BIN OUT`) gives instructions a frame for each line of `drags.txt`: (played N values - played 0) / N, the edits that make the N sources
counted in both. Regen only, not mesh or upload.

```
As it went, each on the main of its day (the last two with boxes over the whole domain, taken out below). As shipped, against main 3cc62503, instructions a frame in M:
```
                     main    memo-1
conrod bolt.4+tap.4   40.9    40.9
crank len            278.0   232.4   -16%
frame reach          409.8   249.6   -39%
gearbox W            231.5   229.1    -1%
heatsink L            97.8    97.7
manifold pitch       227.0   160.8   -29%
volute core.166      713.0   327.6   -54%
sum                 1998.0  1338.1   -33%
```
Cold regen, gate sum flat (-0.03%): the near match gone costs manifold +5.5%, upto seq +3.1%, upto nary1 +0.7%, mixed seq +0.7%, and it had to go (a result that hung on load, below).

                     pipe-6   walls   whole boxes  searched memo  tool memo
conrod bolt.4+tap.4    41.0    41.0    41.0          41.0          41.0
crank len             269.6   269.6   213.4         212.9         167.8
frame reach           407.8   407.8   407.9         296.8         247.3
gearbox W             314.4   308.0   308.0         293.6         293.7
heatsink L            481.7   385.0   385.0         382.9         382.9
manifold pitch        230.4   230.4   185.0         119.9         118.3
volute core.166       709.2   709.1   709.2         607.0         323.0
sum                  2454.1                         1954.1        1574.0
```

## Whole boxes: tried, measured, taken out

A procedural face's box over the whole domain, not the columns its loops span, was 22% of manifold's frame and 40% of crank's, most of it drawing loops. Bench bits did not move and the gate sum fell 3.4%, and drags crank -21%, manifold -20%.
It did not survive the proof: history seed 9450 takes over 160 s against 1 s on main, 92% in `Hyperbola::closest` under `subdivide_pieces`, a loose pipe face's box making a hyperbola's search range very long. On 60 fresh histories the only such part, and seed 4 took a new way. A bound on that search, or the tight box for pipes alone, would have to come first.

## Remembering `searched`

`ssi::searched` kept the last 32 answers; a drag asks hundreds, most about pairs it did not move. Now up to 4096 in a map (a plain list of 4096 made heatsink +5.8%). A stale hit would be a silently wrong
part, so the key was made to hold every input, to the bit:
- Analytic surfaces by every field (`Surface::bits`, whole-struct patterns, so a field added does not compile until it is in), a zero's sign told apart, where `==` would not. Procedural ones by
  address, the entry holding the `Arc` so the address cannot be reused.
- The box's six bounds, to the bit. `searched` before took a box within 1e-11 of one it knew, which makes a warm answer depend on what was asked before, so that went. Its price: a box a rounding off is traced again, upto +3.5%, mixed +0.8% (1.3M and 12M, cold).
  Putting the box on a grid before asking (2^-33, nearest, a cell out) kept that speed and was exact too, and was dropped: the search itself moved, and of 200 fresh fillet fuzz parts 2 began to fail (`inconsistent classification`) and 5 changed.
- The map compares whole keys: the hash only picks where to look, so a collision cannot answer falsely.

## Remembering tools

A sweep, coil or loft that reads only its own numbers and id is a function of them (`swept::tool_of`, the last 64, by feature id and the feature's debug text, which prints every field and tells any two floats apart). Ends and sizes `To` a target read the history and are not remembered. A drag rebuilds many whose numbers did not move: volute's loft
is 20% of its frame by itself. A remembered tool also keeps its procedural surfaces the same `Arc`s, which is what `searched` compares, so its pairs now hit there too without a value key: volute's frame halves.

## Why a hit is the answer

`lean/Memo.lean` (`lake build`, `lake env lean Axioms.lean`; no sorry): if the work's answer is a function of the key (`Covers`), a hit is the answer (`hit_is_answer`), an asking returns the work's answer and keeps the table sound
(`ask_answer`, `ask_sound`), forgetting any entries keeps it sound (`forget_sound`), and work done on a form of the input, as the box on its grid, is covered by a key of that form (`covers_of_form`). `covers_is_needed`: with an input left out, a hit is wrong.

`Covers` is the assumption, and the model cannot check the Rust. What backs it:
- Read through: `search` reads `a`, `b` and the box. State beneath it: `screw::remembered` (last value by surface id and exact point, the same answer), `corner::rational`'s table and cache (constants, keyed by bits), `OnceLock`s on surfaces and curves
  (each set once from the surface's own data), `ALONGDBG` (a debug print). None changes an answer. `build_tool` reads the feature and its id; `i` goes only in errors.
- Tested, `kernel/forge-bool/src/ssi.rs`: a surface a last digit off, every field in the bits, a zero's sign, a colliding hash, a box a last digit off, keeping 0, 1, 2, 5 answers, 200 surfaces at freed addresses.
- Tested, `interface/forge-agent/tests/drag_warm.rs`, each step against the same source built on a new thread, on vertices, tolerances, names, edges and their curves (chain, tangents, sag, both surfaces, values), coedges, loops, faces, shells and
  surfaces (every field, or values and derivatives on a grid): numbers dragged there and back, a line deleted and restored, on bench parts and random histories; keeping little; every fifth number of two parts moved by its last digit and back.
  `DRAG_WARM_ALL=1` for every number of every part, `DRAG_WARM_SEEDS=first,count` for fresh histories.

## A result that hung on load

`bits` on main gave different hashes run to run under load, on n-ary lines only (5 repeats at load 70: plate, torture, fuzz1, fuzz8, housing, mixed), the same with `FORGE_BATCH=cpu`. Cause: the near match in `searched`, added with the Illinois step. `nary::par_map` starts helper threads once 200 microseconds have passed, so which thread
takes which pair hangs on load; the memory is per thread; and a box within 1e-11 of one a thread knew took that one's answer. So which answer depended on the thread. With the exact key an answer is the same on any thread: 12 repeats alike at the same load. `kernel/forge-model/tests/nary_threads.rs` holds 1, 2 and 4 threads to the same bits under 8 spinning threads, but it passes on old main too, so it guards the outcome and does not catch this cause. `ssi.rs`' `a_box_a_little_off_is_another_question` does: on main a01cf374 a box 1e-13 off took the other's answer, where its own differs.
