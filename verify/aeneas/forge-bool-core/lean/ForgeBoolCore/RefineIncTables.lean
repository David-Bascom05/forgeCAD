import ForgeBoolCore.IncSpecs

/-!
# inc refinement, group T: the tables (`rows`, `row`, `body_of`, `cls_of`)

SPLIT-inc.md. Each Rust reader is the Lean one: `rows` is `nrowsL`, a row's bounds are its two
`start` entries, and `body_of` / `cls_of` are `IncLedger.bodyOf` / `clsOf` of `toLF`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

theorem rows_spec (start : Slice U32) : rows start ⦃ r => r.val = start.val.length - 1 ⦄ := by
  unfold rows
  step as ⟨b, hb⟩
  split
  · rename_i h
    have : start.val = [] := by simpa using hb.mp h
    simp [this]
  · rename_i h
    have hne : start.val.length ≠ 0 := by
      intro e; apply h; apply hb.mpr; simpa using e
    step as ⟨r, hr⟩
    scalar_tac

/-- A row's bounds: `start[k]` and `start[k+1]`. -/
theorem row_spec (c : Csr) (k : Usize) (hk : k.val + 1 < c.start.val.length) :
    row c k ⦃ r => r.1.val = (c.start.val.getD k.val 0#u32).val ∧
      r.2.val = (c.start.val.getD (k.val + 1) 0#u32).val ⦄ := by
  unfold row
  have hk0 : k.val < c.start.val.length := by omega
  step as ⟨i, hi⟩
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  have hk2 : i2.val < c.start.val.length := by scalar_tac
  step as ⟨i3, hi3⟩
  step as ⟨i4, hi4⟩
  have e2 : i2.val = k.val + 1 := by scalar_tac
  have g1 : c.start.val.getD k.val 0#u32 = i := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk0, hi]; rfl
  have g2 : c.start.val.getD (k.val + 1) 0#u32 = i3 := by
    rw [← e2, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk2, hi3]; rfl
  rw [g1, g2]
  refine ⟨?_, ?_⟩ <;> scalar_tac

theorem body_of_spec (l : Ledger) (f : FaceId) :
    body_of l f ⦃ r => r.val = (toLF l).bodyOf (f : U32).val ⦄ := by
  unfold body_of
  step as ⟨i, hi⟩
  have hiv : i.val = (f : U32).val := by rw [hi]; scalar_tac
  split
  · rename_i hlt
    have hl : (f : U32).val < l.face_body.val.length := by scalar_tac
    step as ⟨i2, hi2⟩
    have : i2.val = (f : U32).val := by rw [hi2]; scalar_tac
    step as ⟨r, hr⟩
    rw [hr]
    simp [ForgeExact.DecideOnce.IncLedger.bodyOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, this]
  · rename_i hge
    have hl : l.face_body.val.length ≤ (f : U32).val := by scalar_tac
    simp only [WP.spec_ok]
    simp [ForgeExact.DecideOnce.IncLedger.bodyOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_eq_none hl]

theorem cls_of_spec (l : Ledger) (f : FaceId) :
    cls_of l f ⦃ r => r.val = (toLF l).clsOf (f : U32).val ⦄ := by
  unfold cls_of
  step as ⟨i, hi⟩
  have hiv : i.val = (f : U32).val := by rw [hi]; scalar_tac
  split
  · rename_i hlt
    have hl : (f : U32).val < l.face_class.val.length := by scalar_tac
    step as ⟨i2, hi2⟩
    have : i2.val = (f : U32).val := by rw [hi2]; scalar_tac
    step as ⟨r, hr⟩
    rw [hr]
    simp [ForgeExact.DecideOnce.IncLedger.clsOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, this]
  · rename_i hge
    have hl : l.face_class.val.length ≤ (f : U32).val := by scalar_tac
    simp only [WP.spec_ok]
    simp [ForgeExact.DecideOnce.IncLedger.clsOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_eq_none hl]

end inc

end forge_bool_core
