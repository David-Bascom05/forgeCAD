/-
Commutation of a part's boolean features, as point sets. No regularisation, no geometry: these hold
for any sets, which is the point. The hypotheses are the ones left after trying to remove them.
-/
import Mathlib.Data.Set.Basic
import Mathlib.Tactic.Tauto

namespace Rebuild
variable {α : Type*}
open Set

/-- The three ways a tool combines with a part. -/
def add (B T : Set α) : Set α := B ∪ T
def cut (B T : Set α) : Set α := B \ T
def keep (B T : Set α) : Set α := B ∩ T

/-- Symmetric difference: where two parts differ. -/
def diff (A B : Set α) : Set α := (A \ B) ∪ (B \ A)

theorem add_add (B S T : Set α) : add (add B S) T = add (add B T) S := by
  ext x; simp [add]; tauto
theorem cut_cut (B S T : Set α) : cut (cut B S) T = cut (cut B T) S := by
  ext x; simp [cut]; tauto
theorem keep_keep (B S T : Set α) : keep (keep B S) T = keep (keep B T) S := by
  ext x; simp [keep]; tauto
theorem cut_keep (B S T : Set α) : keep (cut B S) T = cut (keep B T) S := by
  ext x; simp [cut, keep]; tauto

/-- Add then cut differ exactly on what the tools share; the part is not involved. -/
theorem add_cut_diff (B S T : Set α) : diff (cut (add B S) T) (add (cut B T) S) = S ∩ T := by
  ext x; simp [diff, add, cut]; tauto

/-- So they commute exactly when the tools are disjoint, whatever the part. -/
theorem add_cut_comm_iff (B S T : Set α) :
    cut (add B S) T = add (cut B T) S ↔ S ∩ T = ∅ := by
  constructor
  · intro h
    have := add_cut_diff B S T
    rw [h] at this
    rw [← this]
    ext x
    simp [diff]
  · intro h
    ext x
    have hx : ¬ (x ∈ S ∧ x ∈ T) := fun hh => by
      have : x ∈ S ∩ T := hh
      rw [h] at this; exact this
    simp [add, cut]; tauto

/-- Add then keep differ exactly on the first tool outside the second. -/
theorem add_keep_diff (B S T : Set α) : diff (keep (add B S) T) (add (keep B T) S) = S \ T := by
  ext x; simp [diff, add, keep]; tauto

/-- A boolean with an unmoved tool can only shrink where two parts differ. -/
theorem add_shrinks (A B T : Set α) : diff (add A T) (add B T) = diff A B \ T := by
  ext x; simp [diff, add]; tauto
theorem cut_shrinks (A B T : Set α) : diff (cut A T) (cut B T) = diff A B \ T := by
  ext x; simp [diff, cut]; tauto
theorem keep_shrinks (A B T : Set α) : diff (keep A T) (keep B T) = diff A B ∩ T := by
  ext x; simp [diff, keep]; tauto

/-- A cached add or cut applies to a changed part with no condition on the change at all, as point
sets: inside the tool the answer is known, outside it the new part stands. So as sets nothing needs
to be disjoint; what needs it is the boundary (see `Local`). -/
theorem cut_patch (B T : Set α) : cut B T = (B \ T) ∪ (∅ ∩ T) := by
  ext x; simp [cut]
theorem add_patch (B T : Set α) : add B T = (B \ T) ∪ T := by
  ext x; simp [add]; tauto

end Rebuild
