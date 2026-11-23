# forge-sheet's exact signs: proof status (L4)

region.rs decides `near`, `within` and the arc-disc test (verify/sheet Region.lean `near`,
`within`, `inDisc`) by exact signs: a float filter (16 ε, Filter16.lean `disc_filter_sound`,
`line_gap_filter_sound`), else the value recomputed exactly as a floating-point expansion.

## Correction to fd6971c's message

fd6971c ("exact.rs: built on forge-math predicates.rs's own expansion helpers") says the
existing proofs of those helpers cover exact.rs. They do not yet. `sum_into`, `scale_into` and
`estimate` are opaque in the aeneas forge-math translation. The proven parts of `orient2d` are
its fast paths only (verify/aeneas/forge-math/lean/ForgeMath/Predicates.lean).

**L4 pending:** sum_into/scale_into/estimate un-opaqued by aeneas, proved against
Fast-Expansion-Sum/Scale-Expansion by sheet-spec (that theory is in progress in verify/ieee754).
Until it lands, the exact fallbacks are not described as proven.

## Where the code is now

- fd6971c: forge-sheet's exact.rs called predicates.rs's helpers (choice (a)).
- 5fdaa63: region.rs calls `forge_sheet_core::exact::{disc, line_gap}` instead. That is
  sheet-spec's Aeneas-style translation (e6ddfc1 on -sheet-spec), checked bit-for-bit against
  forge-sheet's former exact.rs by kernel/forge-sheet-core/tests/core.rs. forge-sheet's own
  exact.rs is removed and the predicates.rs helpers are private again.
- Its expansion steps (two-sum, two-product, grow-expansion, the sign of the top component) are
  translated. Their proofs are sheet-spec's to report. The same pending note applies until they
  are done.
- The master has ruled that choice (a) stands. Moving back to the predicates.rs helpers, if
  wanted, is that ruling's to settle against 5fdaa63.
