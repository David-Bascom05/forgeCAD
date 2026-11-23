# The hull subdivision's rounding margin (winding.rs `bezier`): it did not cover the rounding (fixed)

Found while setting up the L5 proof (the hull subdivision's rounding bound against
verify/ieee754).

## What the code claims

`bezier` splits a curve piece at its midpoint in f64, and decides a piece by its control hull's
box widened by `m = 4·(depth+2)·ε·big/wmin`. Here `big` is the largest |coordinate| (weights
included) of the piece being decided. A piece decided this way is wholly left of p, right of it,
or above or below its height, with certainty.

## Why it does not hold

At each level, the rounding of a midpoint is relative to the size of its operands. The errors
it carries in come from the ancestors and are sized by the ancestors' coordinates. A small piece
of a large curve has a small `big` (about 1, the weight). It still carries errors of the order
ε·(the ancestors' size) per level. Cancellation in `(a + b)/2` does not cancel the error: the
value shrinks, its error does not.

`hull_margin_probe.py` follows the halving path towards a point the curve passes through. It
compares the computed control points (f64, as `bezier` computes them) with the exact ones
(rationals):

```
worst error/m: 22.0 (trial 141, control points within 10000, depth 14: error 3.28e-13, m 1.49e-14, big 1.05)
```

A point p within about 1e-12 of such a piece can therefore be classified with certainty, and
wrongly. The count is then wrong where it should be unknown. This needs a curve with control
points far from the origin and a point of it near the origin, and a test point that close to the
curve. Real parts are unlikely to meet that, but the claim "certain or unknown, never a guess"
does not hold.

## The fix, and what it costs

Size the margin by the root piece, not the current one: `m = 4·(depth+2)·ε·max(big, big0)/wmin`,
with `big0` the root's `big`, passed down the recursion. The de Casteljau points are convex
combinations of the root's, so every operand at every level stays within about `big0`. With this
change the probe's worst error/m is about 0.03.

`m` only grows, so a decision can only become "split again" or "unknown". A count that is
certain today stays the same or becomes unknown. Containment would then try another point.

The cost is more splits where a small piece of a large curve passes near p. That is rare, but
it changes instructions on any gate part whose sketches nest curve loops. Bits change only if
some decision moves.

With the fix, the L5 bound is provable. Each level adds at most ε·big0 per coordinate, the
halving is exact, and the projection adds ε relative error and divides the carried error by
`wmin`. One hypothesis remains: no subnormal intermediate (ieee754's `round_abs_err`; the
subnormal range needs an absolute term `m` does not have).

## Status

The fix is applied: `bezier` takes `big0`, the largest coordinate of the levels above, and sizes
`m` by `max(big, big0)`. With it, the bound is provable on verify/ieee754.
- Each de Casteljau level adds at most one rounding of a sum and one of a halving.
  verify/aeneas/forge-sketch's `mid_err` proves it: E + 2⁻⁵³|result| + 2⁻⁵⁴|sum| + 2⁻¹⁰⁷⁴, at
  most 2⁻⁵² · big0 per level outside the subnormal range.
- A cubic's split is 3 such levels, against the 4·ε·big0 per depth that `m` allows.
- The projection adds one rounding and the carried error over `wmin` (`proj_err`). The `+2` in
  `(depth+2)` covers it.

The induction over the depth is not written: by the owner's ruling, L5 here stops at the note
and the two per-level lemmas.
