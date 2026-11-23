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

**Status:** exact signs via forge_sheet_core::exact; disc proven (sheet-spec), line_gap filter
proven, exact branch in progress.

- 5fdaa63: region.rs calls `forge_sheet_core::exact::{disc, line_gap}`. That is sheet-spec's
  Aeneas-style translation (e6ddfc1 on -sheet-spec), checked bit-for-bit against forge-sheet's
  former exact.rs by kernel/forge-sheet-core/tests/core.rs. forge-sheet's own exact.rs is removed.
  The master accepted this as (C)'s outcome, so the earlier predicates.rs route (fd6971c) no
  longer applies, and the predicates.rs helpers are private again.
- The L4 note above concerns predicates.rs's helpers. Those are no longer on the sheet's path,
  but the note stays until sheet-spec's expansion theory lands.
