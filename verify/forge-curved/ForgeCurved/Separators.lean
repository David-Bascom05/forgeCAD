import ForgeCurved.Sides
import ForgeExact.DecideOnce.QSep

/-!
# The separator rule (decision D3) and what it must give

Design §2.3 and decision D3: the separators of a boolean are the Tier-0 seams of its near
carriers; if and only if the exact Tier-0 check on the resulting arrangement fails, the Tier-1
sweep walls are added. Both are functions of the near carriers; nothing is retried.

* `SepRule ι`: the carriers and the three components (seams, check, walls), each a function of
  the input descriptors `ι` (the faces: an axis or apex cannot be read off a carrier function), and
  `SepRule.separators`, the one rule;
* `FaithfulAll`: R1–R5 for every cell (so for the listed cells of any ledger: `FaithfulAll.listed`);
* the two obligations, stated: **`Tier0Sound`** (a passed check gives `FaithfulAll` with the
  seams) and **`LemmaD`** (the walls give it always);
* **`faithful_of_rule`**: with both, the rule's arrangement is faithful, whichever tier it took.
  So the answer does not depend on the tier (`sep_invariant_reg` for the set), and the rule is one
  decided rule.

R3 is already discharged for both tiers on quadric carriers with seamed singular points
(`quadric_sides`), since walls and seams are planes. `Tier0Sound` and `LemmaD` are open here: they
are the 2D Jordan–Schoenflies statement for piecewise-conic curves on simply connected quadric
pieces, and the slab Morse argument, design §4 Q1 L3 (L–XL).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

variable {P : Type} [TopologicalSpace P]

/-- R1–R5 for every cell of the arrangement. -/
def FaithfulAll (cs : List (Carrier P ℝ)) : Prop := Faithful cs (fun _ => True)

/-- Faithful for all cells is faithful for the listed ones. -/
theorem FaithfulAll.listed {cs : List (Carrier P ℝ)} (h : FaithfulAll cs) (Listed : Cov → Prop) :
    Faithful cs Listed :=
  ⟨fun X _ => h.connected X trivial, fun X Y _ _ => h.incidence X Y trivial trivial,
    fun φ _ => h.sides φ trivial, fun e φ _ _ => h.wing e φ trivial trivial,
    fun v φ _ _ => h.vwing v φ trivial trivial⟩

/-! ## The rule, over input descriptors -/

/-- A rational quadric (`Quad.Quadric`, exact-curved), as a real carrier. -/
def qR (q : ForgeExact.Quad.Quadric) : Carrier R3 ℝ := fun p =>
  (q.xx : ℝ) * p.1 * p.1 + q.yy * p.2.1 * p.2.1 + q.zz * p.2.2 * p.2.2 + q.xy * p.1 * p.2.1 +
    q.xz * p.1 * p.2.2 + q.yz * p.2.1 * p.2.2 + q.lx * p.1 + q.ly * p.2.1 + q.lz * p.2.2 + q.c

/-- A rational plane (`Ref.Plane`), as a real carrier. -/
def planeR (π : Ref.Plane) : Carrier R3 ℝ := fun p =>
  (π.n.x : ℝ) * p.1 + π.n.y * p.2.1 + π.n.z * p.2.2 - π.c

theorem qR_cont (q : ForgeExact.Quad.Quadric) : Continuous (qR q) := by unfold qR; fun_prop
theorem planeR_cont (π : Ref.Plane) : Continuous (planeR π) := by unfold planeR; fun_prop

/-- The separator rule's components, all functions of the input descriptors `ι` (for Q1, the
faces' `QFace`s: kind, axis, apex, centre), from which the carriers are read by `eval`. -/
structure SepRule (ι : Type) where
  /-- the carrier of a descriptor -/
  eval : ι → Carrier R3 ℝ
  /-- Tier 0: the seams of §2.3's table (axial planes, the cone's nappe plane, contact planes) -/
  seams : List ι → List (Carrier R3 ℝ)
  /-- the exact combinatorial Tier-0 check, on the arrangement with the seams -/
  check : List ι → Bool
  /-- Tier 1: the sweep walls at rational midpoints between the events -/
  walls : List ι → List (Carrier R3 ℝ)

variable {ι : Type}

/-- The input carriers. -/
def SepRule.carriers (r : SepRule ι) (fs : List ι) : List (Carrier R3 ℝ) := fs.map r.eval

/-- **D3, the one rule**: the seams, and the walls iff the Tier-0 check fails. -/
def SepRule.separators (r : SepRule ι) (fs : List ι) : List (Carrier R3 ℝ) :=
  if r.check fs then r.seams fs else r.seams fs ++ r.walls fs

/-- **Tier 0**, the statement: on the domain, a passed check makes the seamed arrangement
faithful. (Owner: q1.) -/
def Tier0Sound (r : SepRule ι) (Dom : List ι → Prop) : Prop :=
  ∀ fs, Dom fs → r.check fs = true → FaithfulAll (r.carriers fs ++ r.seams fs)

/-- **Lemma D**, the statement: on the domain, seams and walls make the arrangement faithful (each
open slab holds at most one event). (Owner: the Lemma D session.) -/
def LemmaD (r : SepRule ι) (Dom : List ι → Prop) : Prop :=
  ∀ fs, Dom fs → FaithfulAll (r.carriers fs ++ (r.seams fs ++ r.walls fs))

/-- **The rule gives faithful cells**, from the two tiers' theorems. -/
theorem faithful_of_rule {r : SepRule ι} {Dom : List ι → Prop} (h0 : Tier0Sound r Dom)
    (hD : LemmaD r Dom) (fs : List ι) (hc : Dom fs) :
    FaithfulAll (r.carriers fs ++ r.separators fs) := by
  unfold SepRule.separators
  split
  · exact h0 fs hc (by assumption)
  · exact hD fs hc

/-- Every carrier of the rule's arrangement is continuous, when the parts are. -/
theorem SepRule.cont (r : SepRule ι) (fs : List ι) (he : ∀ f ∈ fs, Continuous (r.eval f))
    (hs : ∀ c ∈ r.seams fs, Continuous c) (hw : ∀ c ∈ r.walls fs, Continuous c) :
    ∀ c ∈ r.carriers fs ++ r.separators fs, Continuous c := by
  intro c hc
  rcases List.mem_append.mp hc with h | h
  · obtain ⟨f, hf, rfl⟩ := List.mem_map.mp h; exact he f hf
  · unfold SepRule.separators at h
    split at h
    · exact hs c h
    · rcases List.mem_append.mp h with h | h
      · exact hs c h
      · exact hw c h

/-! ## Q1: quadric faces, with dol-curved's exact Tier-0 seams -/

open ForgeExact.QRef in
/-- **The Q1 rule**: the faces' quadrics (`QFace.quadric`), the exact Tier-0 seams
(`Curved.seams`, QSep.lean: axial planes, nappe planes, centre planes, one class per point set),
and the given Tier-0 check and Tier-1 walls (the check on q1's side, the walls on Lemma D's). -/
def qRule (check : List QFace → Bool) (walls : List QFace → List (Carrier R3 ℝ)) : SepRule QFace where
  eval f := qR f.quadric
  seams fs := (ForgeExact.DecideOnce.Curved.seams fs).map planeR
  check := check
  walls := walls

end ForgeCurved
