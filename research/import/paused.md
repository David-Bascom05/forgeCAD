# Importing other kernels' programs: paused

The owner paused importing other kernels' CAD programs into .fg on 2026-11-23. This note keeps what was left and
the counts it was ranked by, so the work can be picked up again. coverage.md and truth.md hold what was done.

## The scale counts

From the scale session's CADFS run: the first 2,256 parts (shards 1 and 2 of 200), made with cadfs.py at 1892d2d.

| class | parts |
|---|---:|
| match | 1327 |
| mismatch | 120 |
| uncertain | 6 |
| refused | 137 |
| fail | 11 |
| untranslated | 646 |

What left parts untranslated, by part. Multiply by about 170 for the whole set.

| cause | kind | parts |
|---|---|---:|
| a region the query does not settle | reference | 158 |
| an imprint edge that bounds nothing | reference | 66 |
| a reference to an entity the sketch no longer has | reference | 47 |
| an unequal chamfer | form | 35 |
| a pattern of many bodies | form | 25 |
| an imprint edge's source | reference | 24 |
| feature deleteBodies | form | 23 |
| a face the query does not settle | reference | 20 |
| no sketch record | reference | 19 |
| sketch text beside other curves | form | 12 |
| each of the rest | | 10 or fewer |

Sketch text inside a region is one of the rest, at 8 parts (about 1,400 at full scale).

The likely causes of the mismatches, by part:

| likely cause | parts |
|---|---:|
| with a fillet | 32 |
| plain extrude | 31 |
| extrude with an approximated curve | 11 |
| the program makes fewer bodies than the source | 11 |
| revolve | 8 |
| chamfer | 7 |
| hole | 6 |

The scale session has example ids for each cause. It was also committing per-shard results to
research/import/scale-results/ on its branch.

## What was left, largest first

The master ranked these across corpora. None of them was started.

1. **The arc rule.** Today `arc(center, from, to)` is refused when `to` is off the circle. That refusal hits Fusion
   5 of 739 parts unprompted and DeepCAD 30 of 400. DeepCAD keeps 8 decimals of a metre, so its ends can lie up to
   1e-5 mm off the circle.

   The asked-for semantics:
   - the radius comes from `from`;
   - `to` is projected radially onto the circle;
   - the arc is refused only beyond a stated relative tolerance, with a lint and a fix;
   - research/forge-lang/lean is extended to match, checked with `FG_LEAN=1` parse_diff;
   - the IEEE session reviews the Lean.

   What was found before the pause:
   - The geometry already works this way. `profile.rs` takes the radius from the start and only the angle from the
     end.
   - The refusal is the solver's. `arc_residual` (forge-sketch `sketch.rs`) is a row that has to meet `SOLVED`, an
     absolute 1e-10 (`solve.rs`). With fixed literal ends, nothing can move.
   - Open question: the stated tolerance has to keep the gap from the projected end to a line ending at `to` within
     `tol::LINEAR` (1e-5 mm absolute). Profile nodes join only within that distance, so a purely relative
     tolerance would open chains on large radii. The gap's effect on the kernel downstream was not checked.
   - Open question: no edit op fits a fix to a sketch-level argument. `SetArg` addresses features only.
2. **Language gaps.**
   - CADFS: the region query 158, the imprint edge 66, the stale entity 47, the unequal chamfer 35 (a new .fg form,
     with Lean) and the pattern of many bodies 25.
   - DeepCAD: an extrude from a face, 48.
   - deleteBodies, 23, belongs to the scale-motion session.
3. **DeepCAD "more bodies than source", 29.** It is not yet decided whether this is the translator's
   (fusion.py) or .fg's semantics.
