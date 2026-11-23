import ForgeExact.QRef.Decide
import ForgeExact.DecideOnce.Curved

/-!
# Q1 L2, first brick: the exact covector of a sample on a rational line

The curved enumerator's facet samples are points of a rational line `P + t D` at an algebraic
parameter `t` (design §2.2): a root of one carrier's restriction to the line, which is a
polynomial in `t` (`QRef.alongLine`). Every other carrier's sign there is the sign of its own
restriction at that root (`Alg.Num.sign`, proven by `sign_spec`).

* `Quadric.evK`: a quadric's value at a point of `K³`, for any model `K` of the reals;
* `alongLine_ev`: the restriction is right: its value at `t` is the quadric's value at
  `P + t D`;
* `covOn qs P D n fuel`: the covector of the point the number `n` names, decided exactly, or
  `none` (fuel spent: never a wrong sign);
* **`covOn_spec`**: if `n` denotes `t` in the model, `covOn` is the point's covector over the
  quadrics' values, `covOfK` (Curved.lean). The decision is the geometry's, in every model.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- A point of `K³`. -/
abbrev PK (K : Type) := K × K × K

/-- A quadric's value at a point of `K³`. -/
def Quadric.evK (q : Quadric) (p : PK K) : K :=
  M.q q.xx * p.1 * p.1 + M.q q.yy * p.2.1 * p.2.1 + M.q q.zz * p.2.2 * p.2.2 +
    M.q q.xy * p.1 * p.2.1 + M.q q.xz * p.1 * p.2.2 + M.q q.yz * p.2.1 * p.2.2 +
    M.q q.lx * p.1 + M.q q.ly * p.2.1 + M.q q.lz * p.2.2 + M.q q.c

/-- The point `P + t D`. -/
def linePt (P D : V3) (t : K) : PK K :=
  (M.q P.x + t * M.q D.x, M.q P.y + t * M.q D.y, M.q P.z + t * M.q D.z)

/-- **The restriction is right.** -/
theorem alongLine_ev (q : Quadric) (P D : V3) (t : K) :
    M.ev (alongLine q P D) t = Quadric.evK M q (linePt M P D t) := by
  simp only [alongLine, Model.ev, evalWith_cons, evalWith_nil, Quadric.evK, linePt,
    Quadric.eval, Quadric.grad, Quadric.quad, V3.dot]
  simp only [Model.q, M.ι.map_add, M.ι.map_mul]
  have h2 : M.ι.f 2 = 1 + 1 := by
    rw [show (2 : Rat) = 1 + 1 by grind, M.ι.map_add, M.ι.map_one]
  rw [h2]
  grind

/-- An exact sign, as a side. -/
def sgnOfInt (s : Int) : Sgn := if s = 1 then .pos else if s = -1 then .neg else .zero

/-- The covector of the point `n` names on `P + t D`, decided exactly (`none`: fuel spent). -/
def covOn (qs : List Quadric) (P D : V3) (n : Num) (fuel : Nat) : Option Cov :=
  qs.mapM fun q => (n.sign (alongLine q P D) fuel).map sgnOfInt

theorem sgnK_of_signIs [DecidableRel (α := K) (· < ·)] {y : K} {s : Int} (h : SignIs y s) :
    sgnK y = sgnOfInt s := by
  unfold sgnK sgnOfInt
  rcases h with ⟨rfl, h⟩ | ⟨rfl, rfl⟩ | ⟨rfl, h⟩
  · simp [h]
  · have : ¬(0 : K) < 0 := Std.lt_irrefl
    simp [this]
  · have : ¬(0 : K) < y := fun h' => Std.lt_irrefl (Std.lt_trans h h')
    simp [this, h]

/-- **The decided covector is the geometry's**, in every model of the reals. -/
theorem covOn_spec [DecidableRel (α := K) (· < ·)] (P D : V3) {n : Num} {t : K}
    (ht : Denotes M n t) (fuel : Nat) :
    ∀ (qs : List Quadric) (X : Cov), covOn qs P D n fuel = some X →
      X = covOfK (qs.map fun q => Quadric.evK M q) (linePt M P D t)
  | [], X, h => by simp [covOn] at h; subst h; simp [covOfK]
  | q :: qs, X, h => by
    simp only [covOn, List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.map_eq_some_iff, Option.pure_def, Option.some.injEq] at h
    obtain ⟨s, ⟨i, hi, rfl⟩, rest, hrest, rfl⟩ := h
    have hq := sign_spec M (alongLine q P D) fuel ht hi
    rw [alongLine_ev] at hq
    have ih := covOn_spec P D ht fuel qs rest (by simpa [covOn] using hrest)
    simp only [covOfK, List.map_cons, List.map_map] at ih ⊢
    rw [sgnK_of_signIs hq, ih]

/-! ## Any sound sign oracle (for the Rust's filters)

The bridge decides a sign by a float filter first (interval evaluation on an enclosure of the
sample) and falls back to `Num.sign`. The filter is a *partial oracle*: when it answers, the
answer is the true sign. `covWith` builds a covector from any oracle, and `covWith_spec` needs only
that soundness, so the filtered decision (`signF`) is covered by the same theorem as the exact one.
-/

/-- A covector from a sign oracle on the carriers. -/
def covWith (sg : Quadric → Option Int) (qs : List Quadric) : Option Cov :=
  qs.mapM fun q => (sg q).map sgnOfInt

theorem covWith_spec [DecidableRel (α := K) (· < ·)] (sg : Quadric → Option Int) (pt : PK K)
    (hs : ∀ q s, sg q = some s → SignIs (Quadric.evK M q pt) s) :
    ∀ (qs : List Quadric) (X : Cov), covWith sg qs = some X →
      X = covOfK (qs.map fun q => Quadric.evK M q) pt
  | [], X, h => by simp [covWith] at h; subst h; simp [covOfK]
  | q :: qs, X, h => by
    simp only [covWith, List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.map_eq_some_iff, Option.pure_def, Option.some.injEq] at h
    obtain ⟨s, ⟨i, hi, rfl⟩, rest, hrest, rfl⟩ := h
    have ih := covWith_spec sg pt hs qs rest (by simpa [covWith] using hrest)
    simp only [covOfK, List.map_cons, List.map_map] at ih ⊢
    rw [sgnK_of_signIs (hs q i hi), ih]

/-- The filtered sign: the filter's answer if it has one, else the exact one. -/
def signF (flt : Quadric → Option Int) (exact : Quadric → Option Int) (q : Quadric) : Option Int :=
  match flt q with
  | some s => some s
  | none => exact q

/-- A filter and an exact decider, both sound, make a sound filtered sign. -/
theorem signF_sound {pt : PK K} {flt exact : Quadric → Option Int}
    (hf : ∀ q s, flt q = some s → SignIs (Quadric.evK M q pt) s)
    (he : ∀ q s, exact q = some s → SignIs (Quadric.evK M q pt) s) :
    ∀ q s, signF flt exact q = some s → SignIs (Quadric.evK M q pt) s := by
  intro q s h
  unfold signF at h
  split at h
  · rename_i s' hs'; simp only [Option.some.injEq] at h; subst h; exact hf q s' hs'
  · exact he q s h

/-- The line sample's exact decider is sound (`sign_spec`), so `covOn` is `covWith` of it. -/
theorem covOn_exact_sound (P D : V3) {n : Num} {t : K} (ht : Denotes M n t) (fuel : Nat) :
    ∀ q s, n.sign (alongLine q P D) fuel = some s → SignIs (Quadric.evK M q (linePt M P D t)) s := by
  intro q s h
  have := sign_spec M (alongLine q P D) fuel ht h
  rwa [alongLine_ev] at this

end ForgeExact.DecideOnce.Curved
