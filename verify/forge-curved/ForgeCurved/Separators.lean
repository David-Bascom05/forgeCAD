import ForgeCurved.Sides

/-!
# The separator rule (decision D3) and what it must give

Design §2.3 and decision D3: the separators of a boolean are the Tier-0 seams of its near
carriers; if and only if the exact Tier-0 check on the resulting arrangement fails, the Tier-1
sweep walls are added. Both are functions of the near carriers; nothing is retried.

* `SepRule`: the three components (seams, check, walls), each a function of the carriers, and
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
def FaithfulAll (cs : List (Carrier P ℝ)) : Prop :=
  Faithful cs (fun _ => True) ∧ VWing cs (fun _ => True)

/-- Faithful for all cells is faithful for the listed ones. -/
theorem FaithfulAll.listed {cs : List (Carrier P ℝ)} (h : FaithfulAll cs) (Listed : Cov → Prop) :
    Faithful cs Listed ∧ VWing cs Listed :=
  ⟨⟨fun X _ => h.1.connected X trivial, fun X Y _ _ => h.1.incidence X Y trivial trivial,
    fun φ _ => h.1.sides φ trivial, fun e φ _ _ => h.1.wing e φ trivial trivial⟩,
    fun v φ _ _ => h.2 v φ trivial trivial⟩

/-- The separator rule's components, all functions of the near carriers. -/
structure SepRule (P : Type) where
  /-- Tier 0: the seams of §2.3's table (axial planes, the cone's nappe plane, contact planes) -/
  seams : List (Carrier P ℝ) → List (Carrier P ℝ)
  /-- the exact combinatorial Tier-0 check, on the arrangement with the seams -/
  check : List (Carrier P ℝ) → Bool
  /-- Tier 1: the sweep walls at rational midpoints between the events -/
  walls : List (Carrier P ℝ) → List (Carrier P ℝ)

/-- **D3, the one rule**: the seams, and the walls iff the Tier-0 check fails. -/
def SepRule.separators (r : SepRule P) (cs : List (Carrier P ℝ)) : List (Carrier P ℝ) :=
  if r.check (cs ++ r.seams cs) then r.seams cs else r.seams cs ++ r.walls cs

/-- **Tier 0**, the statement: on the domain, a passed check makes the seamed arrangement faithful. -/
def Tier0Sound (r : SepRule P) (Dom : List (Carrier P ℝ) → Prop) : Prop :=
  ∀ cs, Dom cs → r.check (cs ++ r.seams cs) = true → FaithfulAll (cs ++ r.seams cs)

/-- **Lemma D**, the statement: on the domain, seams and walls make the arrangement faithful (each
open slab holds at most one event). -/
def LemmaD (r : SepRule P) (Dom : List (Carrier P ℝ) → Prop) : Prop :=
  ∀ cs, Dom cs → FaithfulAll (cs ++ (r.seams cs ++ r.walls cs))

/-- **The rule gives faithful cells**, from the two tiers' theorems. -/
theorem faithful_of_rule {r : SepRule P} {Dom : List (Carrier P ℝ) → Prop} (h0 : Tier0Sound r Dom)
    (hD : LemmaD r Dom) (cs : List (Carrier P ℝ)) (hc : Dom cs) :
    FaithfulAll (cs ++ r.separators cs) := by
  unfold SepRule.separators
  split
  · exact h0 cs hc (by assumption)
  · exact hD cs hc

end ForgeCurved
