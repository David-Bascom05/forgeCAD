/-!
# G9: a kept `FaceInfo` answers as a fresh one (`CacheOK`)

kernel/forge-bool/src/classify.rs. A face's point-in-face index (`FaceInfo`) is built once and
kept on the face across booleans; a loop added in place extends it (`extend_query`) and a vertex
whose tolerance grows in place raises it (`grow_query_reach`). What a kept index can differ in
from a fresh one is its `reach`: `point_in_face` looks for the nearest boundary use only among
edges whose box is within `reach` of the point. The rest it reads (the uses in index order, the
band rows, the drawn loops) is the face's own, the same kept or fresh.

`point_in_face_past`'s nearest-edge search, as written: `try_one` over the band rows round `p`
(`Bands::near`, sorted by index and deduplicated) or over every use, in index order; a use is
skipped when its box is farther than `min(reach, best)`, else its distance `d` is solved and kept
when strictly less than the best so far. The result is read only where `d ≤ thr(e)`: the end
vertex's tolerance or `LINEAR` (the `On` answer), or `sag_bound(e)` (the decision by side); past
that the answer comes from the parity and drawn-loop tests, which do not read it. So with `R0` the
largest of `LINEAR`, the face's loops' vertex tolerances and its edges' sag bounds, every reading
is gated by `d ≤ R0`. (The band rows `near` returns for a reach hold every use whose box is
within that reach, and `try_one` drops the rest, so they are modelled as the full index order.)

This file models that search and proves **`cacheOK`**: for any two reaches at least `R0`, the
search's result read through the gate `d ≤ R0` is the same. Hence a kept index, whose reach is at
least `R0`, answers every point as a fresh one does. Distances are any type with the laws of a
linear order (`Laws`); f64 distances, never NaN here, qualify.

What stays open for G9 (ledger): that the code is this model (the Aeneas translation of
`point_in_face_past`, E.shape); the reach invariant `R0 ≤ reach` for every kept index (asserted in
debug builds by `Index::new`/`Index::info`; kept by `extend_query` and `grow_query_reach`, every
write of a vertex's tolerance going through `Body::grow_vertex_tol`, which forge-topo's
`vertex_tol_writes` test enforces: PR proof/g9-vertex-ball); `bd ≤ d` (a use's box holds its
edge: the edge's box grown by `LINEAR`); and `SCRATCH` (boolean.rs's thread-local scratch, taken
and put back per boolean). Core Lean only.
-/

namespace ForgeBoolFallback.Cache

theorem filter_filter_of_imp {ι : Type} (p q : ι → Bool) (h : ∀ e, q e = true → p e = true) (l : List ι) :
    (l.filter p).filter q = l.filter q := by
  induction l with
  | nil => rfl
  | cons e l ih =>
    by_cases hq : q e = true
    · simp [h e hq, hq, ih]
    · by_cases hp : p e = true <;> simp [hp, hq, ih]

/-- The laws of a linear order the proof uses. -/
structure Laws (α : Type) [LE α] [LT α] : Prop where
  le_trans : ∀ {a b c : α}, a ≤ b → b ≤ c → a ≤ c
  lt_of_lt_of_le : ∀ {a b c : α}, a < b → b ≤ c → a < c
  le_of_lt : ∀ {a b : α}, a < b → a ≤ b
  not_le : ∀ {a b : α}, ¬ a ≤ b ↔ b < a
  not_lt : ∀ {a b : α}, ¬ a < b ↔ b ≤ a

variable {α ι : Type} [LE α] [LT α] [DecidableRel (α := α) (· ≤ ·)]
  [DecidableRel (α := α) (· < ·)]

/-- One candidate, unpruned: kept when strictly nearer than the best so far. -/
def step (d : ι → α) (best : Option (α × ι)) (e : ι) : Option (α × ι) :=
  match best with
  | none => some (d e, e)
  | some b => if d e < b.1 then some (d e, e) else some b

/-- One candidate as `try_one` has it: skipped when its box distance `bd e` is past the reach or
past the best so far, else solved and kept when strictly nearer. -/
def tryOne (bd d : ι → α) (reach : α) (best : Option (α × ι)) (e : ι) : Option (α × ι) :=
  match best with
  | none => if bd e ≤ reach then step d none e else none
  | some b => if bd e ≤ reach ∧ bd e ≤ b.1 then step d (some b) e else some b

/-- The nearest-edge search over the candidates in index order. -/
def nearest (bd d : ι → α) (reach : α) (l : List ι) : Option (α × ι) :=
  l.foldl (tryOne bd d reach) none

/-- The search's result as `point_in_face` reads it: a nearest use within `R0`, or nothing. -/
def gate (R0 : α) : Option (α × ι) → Option (α × ι)
  | some b => if b.1 ≤ R0 then some b else none
  | none => none

/-- **CacheOK** (G9): the gated result is the same for any two reaches at least `R0`. -/
def CacheOK (bd d : ι → α) (R0 : α) (l : List ι) : Prop :=
  ∀ r₁ r₂, R0 ≤ r₁ → R0 ≤ r₂ → gate R0 (nearest bd d r₁ l) = gate R0 (nearest bd d r₂ l)

omit [LT α] [DecidableRel (α := α) (· < ·)] in
theorem gate_gate (R0 : α) (x : Option (α × ι)) : gate R0 (gate R0 x) = gate R0 x := by
  cases x with
  | none => rfl
  | some b => by_cases h : b.1 ≤ R0 <;> simp [gate, h]

section
variable (L : Laws α)
include L

/-- The prune on the best so far drops nothing `step` would keep: a box past the best holds an
edge past it. -/
theorem tryOne_eq {bd d : ι → α} (hbd : ∀ e, bd e ≤ d e) (r : α) (best : Option (α × ι))
    (e : ι) : tryOne bd d r best e = if bd e ≤ r then step d best e else best := by
  cases best with
  | none => by_cases h : bd e ≤ r <;> simp [tryOne, h]
  | some b =>
    by_cases h : bd e ≤ r
    · by_cases hb : bd e ≤ b.1
      · simp [tryOne, h, hb]
      · have hfar : b.1 < d e := L.lt_of_lt_of_le (L.not_le.mp hb) (hbd e)
        have hn : ¬ d e < b.1 := fun hl => (L.not_le.mpr hfar) (L.le_of_lt hl)
        simp [tryOne, step, h, hb, hn]
    · simp [tryOne, h]

/-- A best past `R0` is as good as none to the gated result. -/
theorem gate_step (d : ι → α) (R0 : α) (acc : Option (α × ι)) (e : ι) :
    gate R0 (step d acc e) = gate R0 (step d (gate R0 acc) e) := by
  cases acc with
  | none => rfl
  | some b =>
    by_cases hb : b.1 ≤ R0
    · simp [gate, hb]
    · by_cases hl : d e < b.1
      · simp [gate, step, hb, hl]
      · have hfar : R0 < d e := L.lt_of_lt_of_le (L.not_le.mp hb) (L.not_lt.mp hl)
        simp [gate, step, hb, hl, L.not_le.mpr hfar]

/-- A use past `R0` changes nothing the gate lets through. -/
theorem gate_step_far (d : ι → α) (R0 : α) (acc : Option (α × ι)) (e : ι)
    (he : ¬ d e ≤ R0) : gate R0 (step d acc e) = gate R0 acc := by
  cases acc with
  | none => simp [gate, step, he]
  | some b =>
    by_cases hl : d e < b.1
    · have hb : ¬ b.1 ≤ R0 := L.not_le.mpr
        (L.lt_of_lt_of_le (L.not_le.mp he) (L.le_of_lt hl))
      simp [gate, step, hl, he, hb]
    · simp [step, hl]

theorem gate_fold (d : ι → α) (R0 : α) (l : List ι) :
    ∀ acc, gate R0 (l.foldl (step d) acc) = gate R0 (l.foldl (step d) (gate R0 acc)) := by
  induction l with
  | nil => intro acc; simp [gate_gate]
  | cons e l ih =>
    intro acc
    simp only [List.foldl_cons]
    rw [ih (step d acc e), gate_step L d R0 acc e, ← ih]

theorem gate_fold_filter (d : ι → α) (R0 : α) (l : List ι) :
    ∀ acc, gate R0 (l.foldl (step d) acc)
      = gate R0 ((l.filter fun e => decide (d e ≤ R0)).foldl (step d) acc) := by
  induction l with
  | nil => intro acc; rfl
  | cons e l ih =>
    intro acc
    by_cases he : d e ≤ R0
    · simp only [List.foldl_cons, List.filter_cons, he, decide_true, ite_true]
      exact ih _
    · simp only [List.foldl_cons, List.filter_cons, he, decide_false]
      rw [gate_fold L d R0 l (step d acc e), gate_step_far L d R0 acc e he, ← gate_fold L, ih]
      rfl

theorem nearest_eq {bd d : ι → α} (hbd : ∀ e, bd e ≤ d e) (r : α) (l : List ι) :
    ∀ acc, l.foldl (tryOne bd d r) acc
      = (l.filter fun e => decide (bd e ≤ r)).foldl (step d) acc := by
  induction l with
  | nil => intro acc; rfl
  | cons e l ih =>
    intro acc
    by_cases h : bd e ≤ r
    · simp only [List.foldl_cons, List.filter_cons, h, decide_true, ite_true, tryOne_eq L hbd]
      exact ih _
    · simp only [List.foldl_cons, List.filter_cons, h, decide_false, tryOne_eq L hbd]
      simpa using ih acc

/-- **G9, `cacheOK`**: where each use's box distance is at most its distance, the nearest-edge
search read through `d ≤ R0` answers alike for every reach at least `R0`. -/
theorem cacheOK {bd d : ι → α} (hbd : ∀ e, bd e ≤ d e) (R0 : α) (l : List ι) :
    CacheOK bd d R0 l := by
  have one : ∀ r, R0 ≤ r → gate R0 (nearest bd d r l)
      = gate R0 ((l.filter fun e => decide (d e ≤ R0)).foldl (step d) none) := by
    intro r hr
    unfold nearest
    rw [nearest_eq L hbd r l none, gate_fold_filter L d R0,
      filter_filter_of_imp (fun e => decide (bd e ≤ r)) (fun e => decide (d e ≤ R0))]
    intro e he
    simp only [decide_eq_true_eq] at he ⊢
    exact L.le_trans (L.le_trans (hbd e) he) hr
  intro r₁ r₂ h₁ h₂
  rw [one r₁ h₁, one r₂ h₂]

end

end ForgeBoolFallback.Cache
