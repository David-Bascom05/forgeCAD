import Mathlib.Topology.MetricSpace.Basic
import Mathlib.Analysis.Normed.Group.Basic

/-!
# `on_edge`, as written: what it guarantees, and where it guesses

forge-bool's `on_edge(body, e, p)` (boolean.rs) asks the edge's curve for its closest parameter
`s` to `p`, measures `d = |curve(s) - p|`, and answers "on" when `d ≤ edge.tol` (or, past an end
of the range, when `d ≤` that end vertex's tolerance). Its callers in `trim` ask it about two
kinds of hit (curved-sites rates, research/decide-once/curved-fallback-rates.md):

* **B:2520, own carrier**: a hit made on the edge's own line or circle (`plane_meet`);
* **B:2830, across**: a hit made on `S_f ∩ A` (the face's surface and the surface across the
  edge), by `curve_surface` on the face pair's curve. The edge's carrier is one component (a
  branch) of `S_f ∩ A` too.

So at B:2830 the hit's *exact* counterpart `p` lies on `S_f ∩ A` by provenance, and the only
facts left are (a) which branch of `S_f ∩ A` it is on and (b) whether it is inside the edge's
range. This file proves (a) from a separation of the branches, in any metric space, with the
float errors as named bounds:
* `branch_of_sep`: exact form: a point of `K ∪ L` within `ε < sep` of `K` is in `K`;
* `branch_of_sep_delta`: the F1 form: the code's `d` measured between a computed hit `h`
  (within `η` of the exact `p`) and a computed curve point `c` (within `η'` of a point of `K`);
  `d + η + η' < sep` proves `p ∈ K`.

What it does *not* prove, kept as stated contracts (`def … : Prop`, ledger rows C.cuts):
* `ClosestGlobal`: `closest` answers a global nearest parameter (it is a local search);
* `OnEdgeRange`: the range test agrees with the exact arc between the end vertices;
* the separation `sep` per pair kind (`SepTheorem`), and the IEEE bounds `η`, `η'`.

`on_edge`'s "on" at B:2830 is therefore proven exactly where the pair kind has a separation
theorem whose `sep` exceeds `edge.tol + η + η'`; "off" needs `ClosestGlobal`.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.OnEdge

variable {α : Type*} [PseudoMetricSpace α]

/-- Two sets at least `sep` apart. -/
def Separated (K L : Set α) (sep : ℝ) : Prop :=
  ∀ k ∈ K, ∀ l ∈ L, sep ≤ dist k l

/-- **The branch, exactly.** A point of `K ∪ L` within `ε` of a point of `K`, with `K` and `L`
at least `sep > ε` apart, is in `K`. -/
theorem branch_of_sep {K L : Set α} {sep ε : ℝ} (hs : Separated K L sep) (hε : ε < sep)
    {p q : α} (hp : p ∈ K ∪ L) (hq : q ∈ K) (hpq : dist p q ≤ ε) : p ∈ K := by
  rcases hp with hK | hL
  · exact hK
  · exfalso
    have h1 := hs q hq p hL
    have h2 : dist q p ≤ ε := by rw [dist_comm]; exact hpq
    linarith

/-- **The branch, as the code measures it (F1).** `h` is the computed hit, within `η` of the exact
meet `p ∈ K ∪ L`; `c` is the computed curve point, within `η'` of `q ∈ K`; the code's distance
is `d = dist h c`. Then `d + η + η' < sep` proves `p ∈ K`. -/
theorem branch_of_sep_delta {K L : Set α} {sep d η η' : ℝ} (hs : Separated K L sep)
    (hlt : d + η + η' < sep) {p q h c : α} (hp : p ∈ K ∪ L) (hq : q ∈ K)
    (hh : dist h p ≤ η) (hc : dist c q ≤ η') (hd : dist h c ≤ d) : p ∈ K := by
  refine branch_of_sep hs hlt hp hq ?_
  calc dist p q ≤ dist p h + dist h c + dist c q := dist_triangle4 p h c q
    _ ≤ η + d + η' := by
        rw [dist_comm p h]
        linarith
    _ = d + η + η' := by ring

/-- **One branch.** Where `S_f ∩ A` is connected (plane–plane, plane–sphere, sphere–sphere,
a plane not parallel to a cylinder's axis), `L = ∅` and every point is on the edge's carrier. -/
theorem branch_of_empty {K : Set α} {p : α} (hp : p ∈ K ∪ (∅ : Set α)) : p ∈ K := by
  simpa using hp

/-! ## Stated contracts (open; ledger C.cuts) -/

/-- `closest` is global: the parameter it returns is a nearest point of the curve over the
range, within `η` (today: a local search; a gap wherever a curve comes back near `p`). -/
def ClosestGlobal (curve : ℝ → α) (lo hi : ℝ) (closest : α → ℝ) (η : ℝ) : Prop :=
  ∀ p, closest p ∈ Set.Icc lo hi ∧
    ∀ t ∈ Set.Icc lo hi, dist (curve (closest p)) p ≤ dist (curve t) p + η

/-- A separation theorem for a pair kind: the components of `S ∩ A` other than the edge's carrier
`K` lie at least `sep` from it. Per kind: two lines `2√(r² − h²)` apart (a plane parallel to a
cylinder's axis), the chord (parallel cylinders), `2a` (a plane and a cone, hyperbola),
`2√(r² − h²)` (a torus and a plane square to its axis), `2(R − r)` (through its axis). -/
def SepTheorem (K L : Set α) (sep : ℝ) : Prop :=
  0 < sep ∧ Separated K L sep

/-- The range part of `on_edge`: a point of the carrier is on the edge (the closed arc between its
end vertices) iff the code's parameter test says so, outside the vertices' tolerance balls. -/
def OnEdgeRange (arc carrier : Set α) (v0 v1 : α) (tol : ℝ) (inRange : α → Bool) : Prop :=
  ∀ p ∈ carrier, tol < dist p v0 → tol < dist p v1 → (inRange p = true ↔ p ∈ arc)

end ForgeCurved.OnEdge
