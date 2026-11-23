# krawczyk: the step lemma of the certified tracer, in Lean

`Krawczyk/Step.lean` proves what kernel/forge-geom/src/certify.rs relies on
(research/kernel-audit/certified-tracing.md §3.2, §5):

- `unique_zero`: F has exactly one zero in a closed box X when two things hold. First,
  x ↦ x − C F x maps X into X. Second, every difference F u − F v over X is A(u − v) for some A
  with ‖I − C A‖ ≤ q < 1. The proof is Banach's fixed-point theorem on X. C is one to one, because
  C A is a unit.
- `maps_into`: the Krawczyk inclusion y − C F y + (I − C A)(X − y) ⊆ X gives the first hypothesis.
- `nearest_between`, `add_encloses`: the interval side. A bound rounded to nearest and pushed out
  one float on each side (`next_down`, `next_up`) encloses the exact bound.

`Krawczyk/Trig.lean` proves forge_math::Interval's sin/cos range rule over ℝ:

- `sin_le_ends`, `ends_le_sin`: sin on [a, b] stays between its end values, unless a maximum (k + ½)π with k even lies inside (for the upper bound) or a minimum with k odd does (for the lower). The proof uses the extreme value theorem and Fermat's theorem.
- `cos_le_ends`, `ends_le_cos`: the same for cos at kπ.
- `crit_enclosed`, `crit_quotients`: the kernel's outward test (now the quotient form, k + ½ between a/PI, a/PI_UP and b/PI, b/PI_UP rounded out) never misses an extreme.

The endpoint values are dmath's, within an absolute 2^-52 for |x| ≤ 2048. That bound is proven on verify/ieee754's model (`Dmath.sin_err`, `Dmath.cos_err`, IEEE branch), and forge-math's `trig_within_the_contract` measures it against a double-double reference (worst 8.4e-17).

The kernel's test is stronger than the hypotheses. It checks the sup-norm row sums of the interval
I − C 𝐉, which bound ‖I − C A‖ for every A in 𝐉. And K is computed in outward-rounded intervals,
which contain the exact K.

What is not yet joined: `nearest_between`'s set S is the binary64 values, and its `f` is
verify/ieee754's `round`, whose nearest property is proven there. That project is on another branch.
The two use the same Mathlib pin, so one can require the other.

Build: `lake exe cache get && lake build`. It needs Mathlib v4.36.0's cache. The theorems depend
only on propext, Classical.choice and Quot.sound.
