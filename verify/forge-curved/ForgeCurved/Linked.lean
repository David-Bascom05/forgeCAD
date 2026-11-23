import ForgeCurved.Faces

/-!
# `FacesLinked`, decided on the list of realised covectors

The enumerator lists the realised covectors (`R`, with `∀ X, RealisedK cs X ↔ X ∈ R` its
covering theorem). On that list `FacesLinked` is a finite graph search: **`linkedB R Y`** grows the
set of proper faces of `Y` reachable from the first by incidence, and asks that every proper face
was reached. **`facesLinked_of_linkedB`**: sound.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

/-- Incidence, either way. -/
def adjB (X X' : Cov) : Bool := Cov.le X X' || Cov.le X' X

/-- The proper faces of `Y` in the list. -/
def facesB (R : List Cov) (Y : Cov) : List Cov := R.filter fun X => X != Y && Cov.le X Y

/-- `n` rounds of growing `S` by incidence inside `F`. -/
def reachB (F : List Cov) : Nat → List Cov → List Cov
  | 0, S => S
  | n + 1, S => reachB F n (S ++ F.filter fun X => S.any fun Z => adjB Z X)

/-- **The faces of `Y` are linked**, on the list. -/
def linkedB (R : List Cov) (Y : Cov) : Bool :=
  match facesB R Y with
  | [] => true
  | X0 :: F => (X0 :: F).all fun X => (reachB (X0 :: F) (F.length + 1) [X0]).contains X

variable {P : Type} [TopologicalSpace P]

theorem mem_facesB {cs : List (Carrier P ℝ)} {R : List Cov} (hR : ∀ X, RealisedK cs X ↔ X ∈ R)
    {Y X : Cov} : X ∈ facesB R Y ↔ ProperFace cs Y X := by
  simp only [facesB, List.mem_filter, Bool.and_eq_true, bne_iff_ne, ne_eq, ProperFace, hR]

theorem reachB_inv {F : List Cov} {Q : Cov → Prop}
    (hF : ∀ X ∈ F, ∀ Z, Q Z → adjB Z X = true → Q X) :
    ∀ n S, (∀ s ∈ S, Q s) → ∀ Z ∈ reachB F n S, Q Z
  | 0, S, hS => hS
  | n + 1, S, hS => by
    refine reachB_inv hF n _ fun s hs => ?_
    rcases List.mem_append.mp hs with h | h
    · exact hS s h
    · obtain ⟨hsF, hany⟩ := List.mem_filter.mp h
      obtain ⟨Z, hZ, hadj⟩ := List.any_eq_true.mp hany
      exact hF s hsF Z (hS Z hZ) hadj

/-- **`linkedB` is sound.** -/
theorem facesLinked_of_linkedB {cs : List (Carrier P ℝ)} {R : List Cov}
    (hR : ∀ X, RealisedK cs X ↔ X ∈ R) {Y : Cov} (h : linkedB R Y = true) :
    FacesLinked cs Y := by
  intro A hA X X' hX hX' hAX
  have hmem : ∀ Z, Z ∈ facesB R Y ↔ ProperFace cs Y Z := fun Z => mem_facesB hR
  unfold linkedB at h
  cases hG : facesB R Y with
  | nil => exact absurd ((hmem X).mpr hX) (by rw [hG]; simp)
  | cons X0 F =>
    rw [hG] at h
    simp only [hG] at hmem
    have hinv : ∀ Z ∈ reachB (X0 :: F) (F.length + 1) [X0], Z ∈ X0 :: F ∧ (A Z ↔ A X0) := by
      refine reachB_inv (Q := fun Z => Z ∈ X0 :: F ∧ (A Z ↔ A X0)) ?_ _ _ ?_
      · intro W hW Z ⟨hZm, hZA⟩ hadj
        refine ⟨hW, ?_⟩
        have hZp := (hmem Z).mp hZm
        have hWp := (hmem W).mp hW
        rcases Bool.or_eq_true_iff.mp hadj with hle | hle
        · exact (hA Z W hZp hWp hle).symm.trans hZA
        · exact (hA W Z hWp hZp hle).trans hZA
      · intro s hs
        rw [List.mem_singleton] at hs
        subst hs
        exact ⟨List.mem_cons_self .., Iff.rfl⟩
    have hall := List.all_eq_true.mp h
    have h1 := (hinv X (by simpa using hall X ((hmem X).mpr hX))).2
    have h2 := (hinv X' (by simpa using hall X' ((hmem X').mpr hX'))).2
    exact h2.mpr (h1.mp hAX)

end ForgeCurved
