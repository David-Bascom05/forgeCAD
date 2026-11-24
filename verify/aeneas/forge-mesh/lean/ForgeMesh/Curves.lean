/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs

/-!
# The space-filling curves return

`morton` (BRIO's Z-order key) and `hilbert` (the refinement's insertion order) return `ok`:
`hilbert` for coordinates on its 65536-step grid, its sum bounded by `d + 4 s² ≤ 2^32` as the
step halves (each step adds at most `3 s²`, and `4 (s/2)² ≤ s²`).
-/

open Aeneas Aeneas.Std Result
namespace forge_mesh

@[step] theorem spread_spec (v : Std.U32) : cdt.morton.spread v ⦃ _ => True ⦄ := by
  unfold cdt.morton.spread
  simp only [lift, bind_ok]
  step*

theorem morton_spec (x y : Std.U32) : cdt.morton x y ⦃ _ => True ⦄ := by
  unfold cdt.morton
  step*

theorem quarter (s : Nat) : 4 * (s / 2) * (s / 2) ≤ s * s := by
  have h : 2 * (s / 2) ≤ s := Nat.mul_div_le s 2
  calc 4 * (s / 2) * (s / 2) = (2 * (s / 2)) * (2 * (s / 2)) := by ring
    _ ≤ s * s := Nat.mul_le_mul h h

theorem from_u32_val (s : Std.U32) : (core.convert.num.FromU64U32.from s).val = s.val :=
  core.convert.num.FromU64U32.from_val_eq s

theorem from_bool_le (b : Bool) : (core.convert.num.FromU32Bool.from b).val ≤ 1 := by
  cases b <;> simp [core.convert.num.FromU32Bool.from]

theorem xor_le (a b : Std.U32) (ha : a.val ≤ 3) (hb : b.val ≤ 3) : (a ^^^ b).val ≤ 3 := by
  have ha' : a.val < 2 ^ 2 := by omega
  have hb' : b.val < 2 ^ 2 := by omega
  have := Nat.xor_lt_two_pow ha' hb'
  rw [UScalar.val_xor]
  omega

@[step] theorem hilbert_loop_spec (x y : Std.U32) (d : Std.U64) (s : Std.U32)
    (hx : x.val ≤ 65535) (hy : y.val ≤ 65535) (hd : d.val + 4 * s.val * s.val ≤ 2 ^ 32) :
    cdt.hilbert_loop x y d s ⦃ _ => True ⦄ := by
  unfold cdt.hilbert_loop
  apply loop.spec_decr_nat
    (fun (a : Std.U32 × Std.U32 × Std.U64 × Std.U32) => a.2.2.2.val)
    (fun a => a.1.val ≤ 65535 ∧ a.2.1.val ≤ 65535 ∧ a.2.2.1.val + 4 * a.2.2.2.val * a.2.2.2.val ≤ 2 ^ 32)
  · rintro ⟨x, y, d, s⟩ ⟨g1, g2, g3⟩
    simp only at g1 g2 g3 ⊢
    clear hx hy hd
    have hq := quarter s.val
    unfold cdt.hilbert_loop.body
    simp only [lift, bind_ok]
    split
    · have r1 := from_bool_le (decide (x &&& s > 0#u32))
      have r2 := from_bool_le (decide (y &&& s > 0#u32))
      have hs : s.val * s.val ≤ 2 ^ 30 := by nlinarith [g3]
      step*
      all_goals
        have ht : (core.convert.num.FromU64U32.from
            (i5 ^^^ core.convert.num.FromU32Bool.from (decide (y &&& s > 0#u32)))).val ≤ 3 := by
          rw [from_u32_val]; apply xor_le <;> omega
        have hi4 : i4.val = s.val * s.val := by rw [i4_post, from_u32_val]
      · rw [hi4]; scalar_tac_preprocess; nlinarith
      · have : i8.val ≤ 3 * 2 ^ 30 := by rw [i8_post, hi4]; nlinarith
        scalar_tac
      · have hd1 : d1.val ≤ d.val + 3 * (s.val * s.val) := by
          rw [d1_post, i8_post, hi4]; nlinarith
        have hs0 : 0 < s.val := by scalar_tac
        split <;> (try split) <;> step* <;> simp only [Nat.shiftRight_eq_div_pow, pow_one] at * <;>
          refine ⟨by omega, by omega, ?_, by omega⟩ <;> rw [s1_post] <;> nlinarith
    · simp_all
  · simp_all

/-- `hilbert` returns, for coordinates on the 65536-step grid. -/
@[step] theorem hilbert_spec (x y : Std.U32) (hx : x.val ≤ 65535) (hy : y.val ≤ 65535) :
    cdt.hilbert x y ⦃ _ => True ⦄ := by
  unfold cdt.hilbert
  step*

end forge_mesh
