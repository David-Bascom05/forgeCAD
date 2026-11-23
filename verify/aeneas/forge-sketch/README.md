# kernel/forge-sketch in Lean (Aeneas)

`translate.conf` names what is translated: the line crossing of the winding count
(`winding::line` in kernel/forge-sketch/src/winding.rs), unchanged. `lean/` requires the float
bridge (verify/aeneas/bridge) and the specs (verify/forge-text).
`tools/aeneas/translate.sh forge-sketch [--check]` writes or checks `lean/ForgeSketch/{Types,Funs}.lean`,
then run `lake build` in `lean/`.

The translation has 2 definitions (`above`, `line`) and leaves 3 items opaque, each defined in
`lean/ForgeSketch/FunsExternal.lean`:
- `f64::min` and `f64::max` follow std's documentation: the other argument when one is NaN.
  Of `+0` and `-0` std may return either; these return the first, and no comparison in `line`
  tells them apart.
- `orient2d` is forge-math's. It is an `opaque` constant (not an axiom). What `line` needs of it,
  a finite result with the sign of the exact determinant of the input doubles, is the theorems'
  hypothesis `OrientOk`. That is forge-math's claim for `orient2d`. Its fast path is proven at
  L4 on the aeneas branch (verify/aeneas/forge-math: `orient2d_fast`); the expansion fallback is
  not yet.

## What is proven

`lean/ForgeSketch/Line.lean` uses standard axioms only (`lake env lean Axioms.lean`) and has no
`sorry`.
- `line_spec`: for finite points and under `OrientOk`, `winding::line a b p` is ForgeText's
  `lineCrossQ` on the doubles' exact (rational) values. That is the half-open crossing rule,
  `None` exactly when p is on the side.
- `line_int`: on points whose values are integers (a glyph on its grid), it is ForgeText's
  integer `lineCross`. That is the test `winding_is_topological` is proven about, through
  `lineCross_toQ`.

## L5, started: the hull subdivision's rounding (`lean/ForgeSketch/Hull.lean`, on verify/ieee754)

- `mid_err`: one de Casteljau midpoint `(a ⊕ b) ⊘ 2` against the exact midpoint of what `a` and
  `b` stand for (each within `E`). The error is at most E + 2⁻⁵³|result| + 2⁻⁵⁴|sum| + 2⁻¹⁰⁷⁴.
- `proj_err`: one projection `x̃ ⊘ w̃` against the exact `x / v`, with weights at least `wmin`. The
  error is one rounding, plus E/wmin, plus |x/v|·E/wmin.

The induction over the depth is not written yet, because the margin it would prove is not the
one `bezier` uses: research/sketch-text/hull-margin.md. `bezier` sizes the margin by the current
piece; the error a piece carries is sized by its ancestors.
