import ForgeSheetCore.Funs
import ForgeSheetCore.UFModel
open Aeneas Aeneas.Std Result

namespace forge_sheet_core

open ForgeSheetCore.UF

/-- A parent vector as the model's list. -/
def vals (v : alloc.vec.Vec Std.U32) : List Nat := v.val.map (·.val)

theorem vals_length (v : alloc.vec.Vec Std.U32) : (vals v).length = v.val.length := by simp [vals]

theorem par_vals (v : alloc.vec.Vec Std.U32) {x : Nat} (hx : x < v.val.length) :
    par (vals v) x = v.val[x].val := by
  simp [par, vals, hx]

theorem find_loop_spec (s : union_find.Uf) (x : Usize) (hx : x.val < s.parent.val.length)
    (hg : Good (vals s.parent)) :
    union_find.Uf.find_loop s x ⦃ r =>
      r.2.val = ufRoot (vals s.parent) x.val ∧ r.1.val.length = s.parent.val.length ∧
      Good (vals r.1) ∧ ∀ y, ufRoot (vals r.1) y = ufRoot (vals s.parent) y ⦄ := by
  unfold union_find.Uf.find_loop
  apply loop.spec_decr_nat (fun (q : union_find.Uf × Usize) => q.2.val)
    (fun q => q.2.val < s.parent.val.length ∧ q.1.parent.val.length = s.parent.val.length ∧
      Good (vals q.1.parent) ∧ (∀ y, ufRoot (vals q.1.parent) y = ufRoot (vals s.parent) y) ∧
      ufRoot (vals q.1.parent) q.2.val = ufRoot (vals s.parent) x.val)
  · rintro ⟨t, j⟩ ⟨hj, hlen, hgt, hroots, hroot⟩
    unfold union_find.Uf.find_loop.body
    simp only at hj hlen hgt hroots hroot ⊢
    step as ⟨i, hi⟩
    step as ⟨i1, hi1⟩
    have hpj : par (vals t.parent) j.val = i.val := by rw [par_vals _ (by omega), hi]
    split
    · rename_i hne
      step as ⟨p, hp⟩
      have hpv : p.val = par (vals t.parent) j.val := by rw [hpj]; scalar_tac
      have hpl : p.val < t.parent.val.length := by
        have := par_le hgt j.val; omega
      step as ⟨g, hg'⟩
      step as ⟨w, back, hw⟩
      step as ⟨x1, hx1⟩
      rename_i hback
      have hi1v : i1.val = i.val := by rw [hi1]; scalar_tac
      have hne' : par (vals t.parent) j.val ≠ j.val := by
        rw [hpj, ← hi1v]; intro h; apply (bne_iff_ne.mp hne); exact UScalar.eq_of_val_eq h
      have hlt : par (vals t.parent) j.val < j.val := by have := par_le hgt j.val; omega
      have hgv : g.val = par (vals t.parent) (par (vals t.parent) j.val) := by
        rw [← hpv, par_vals _ hpl, hg']
      have hx1v : x1.val = g.val := by rw [hx1]; scalar_tac
      have hset : vals (back g) = (vals t.parent).set j.val (par (vals t.parent) (par (vals t.parent) j.val)) := by
        rw [hback, ← hgv]; simp [vals, List.map_set]
      have hlen' : (back g).val.length = t.parent.val.length := by rw [hback]; simp
      have hle2 := par_le hgt (par (vals t.parent) j.val)
      refine ⟨by omega, by omega, ?_, ?_, ?_, by omega⟩
      · rw [hset]; exact halve_good hgt j.val
      · intro y; rw [hset, halve_root hgt, hroots]
      · rw [hset, halve_root hgt, hx1v, hgv, root_par hgt, root_par hgt, hroot]
    · rename_i heq
      have hi1v : i1.val = i.val := by rw [hi1]; scalar_tac
      have hj' : par (vals t.parent) j.val = j.val := by
        rw [hpj, ← hi1v]; simp at heq; rw [heq]
      simp only [WP.spec_ok]
      refine ⟨?_, hlen, hgt, hroots⟩
      rw [← hroot, ufRoot_eq, hj']; simp
  · exact ⟨hx, rfl, hg, fun _ => rfl, rfl⟩

theorem find_spec (s : union_find.Uf) (x : Usize) (hx : x.val < s.parent.val.length)
    (hg : Good (vals s.parent)) :
    union_find.Uf.find s x ⦃ r =>
      r.1.val = ufRoot (vals s.parent) x.val ∧ r.2.parent.val.length = s.parent.val.length ∧
      Good (vals r.2.parent) ∧ ∀ y, ufRoot (vals r.2.parent) y = ufRoot (vals s.parent) y ⦄ := by
  unfold union_find.Uf.find
  step with find_loop_spec s x hx hg as ⟨v, r, h1, h2, h3, h4⟩
  exact ⟨h1, h2, h3, h4⟩

/-- The invariant on a Rust union–find. -/
def UInv (E : List (Nat × Nat)) (n : Nat) (s : union_find.Uf) : Prop := Inv E n (vals s.parent)

theorem uinv_len {E : List (Nat × Nat)} {n : Nat} {s : union_find.Uf} (h : UInv E n s) :
    s.parent.val.length = n := by
  have := h.2.1; rwa [vals_length] at this

/-- **`union` keeps the invariant**, for one more pair; it needs the array to fit `u32` ids. -/
theorem union_spec {E : List (Nat × Nat)} {n : Nat} (s : union_find.Uf) (hi : UInv E n s)
    (hE : ∀ e ∈ E, e.1 < n ∧ e.2 < n) (hn : n ≤ U32.max) (a b : Usize) (ha : a.val < n)
    (hb : b.val < n) :
    union_find.Uf.union s a b ⦃ s' => UInv (E ++ [(a.val, b.val)]) n s' ⦄ := by
  have hl := uinv_len hi
  obtain ⟨hg, _, hle⟩ := hi
  unfold union_find.Uf.union
  step with find_spec s a (by omega) hg as ⟨ra, s1, hra, hl1, hg1, hr1⟩
  step with find_spec s1 b (by omega) hg1 as ⟨rb, s2, hrb, hl2, hg2, hr2⟩
  -- the second array has the first's roots
  have hroots : ∀ y, ufRoot (vals s2.parent) y = ufRoot (vals s.parent) y := fun y => by rw [hr2, hr1]
  have hinv2 : Inv E n (vals s2.parent) := by
    refine ⟨hg2, by rw [vals_length]; omega, ?_⟩
    rw [funext hroots]; exact hle
  have hlink := inv_link hinv2 hE ha hb
  have hra' : ra.val = ufRoot (vals s2.parent) a.val := by rw [hroots, hra]
  have hrb' : rb.val = ufRoot (vals s2.parent) b.val := by rw [hrb, hr2]
  have hlen2 : (vals s2.parent).length = n := by rw [vals_length]; omega
  have hra_lt : ra.val < n := by
    have := root_lt hg2 (x := a.val) (by omega); rw [hra']; omega
  have hrb_lt : rb.val < n := by
    have := root_lt hg2 (x := b.val) (by omega); rw [hrb']; omega
  unfold link at hlink
  simp only [← hra', ← hrb'] at hlink
  split
  · rename_i h
    have h' : ra.val < rb.val := by scalar_tac
    step as ⟨w, back, hw⟩
    rename_i hback
    step as ⟨c, hc⟩
    have hcv : c.val = ra.val := by rw [hc]; scalar_tac
    have : vals (back c) = (vals s2.parent).set rb.val ra.val := by
      rw [hback, ← hcv]; simp [vals, List.map_set]
    unfold UInv; rw [this]; rw [if_pos h'] at hlink; exact hlink
  · rename_i h
    split
    · rename_i h2
      have h' : ¬ra.val < rb.val := by scalar_tac
      have h2' : rb.val < ra.val := by scalar_tac
      step as ⟨w, back, hw⟩
      rename_i hback
      step as ⟨c, hc⟩
      have hcv : c.val = rb.val := by rw [hc]; scalar_tac
      have : vals (back c) = (vals s2.parent).set ra.val rb.val := by
        rw [hback, ← hcv]; simp [vals, List.map_set]
      unfold UInv; rw [this]; rw [if_neg h', if_pos h2'] at hlink; exact hlink
    · rename_i h2
      have h' : ¬ra.val < rb.val := by scalar_tac
      have h2' : ¬rb.val < ra.val := by scalar_tac
      simp only [WP.spec_ok]
      unfold UInv; rw [if_neg h', if_neg h2'] at hlink; exact hlink

theorem new_loop_spec (n : Usize) (hn : n.val ≤ U32.max + 1) (v : alloc.vec.Vec U32) (i : Usize)
    (hv : vals v = List.range i.val) (hi : i.val ≤ n.val) :
    union_find.Uf.new_loop n v i ⦃ r => vals r = List.range n.val ⦄ := by
  unfold union_find.Uf.new_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => n.val - q.2.val)
    (fun q => vals q.1 = List.range q.2.val ∧ q.2.val ≤ n.val)
  · rintro ⟨w, j⟩ ⟨hw, hj⟩
    unfold union_find.Uf.new_loop.body
    simp only at hw hj ⊢
    split
    · rename_i h
      have hjl : j.val < n.val := by scalar_tac
      step as ⟨c, hc⟩
      have hcv : c.val = j.val := by rw [hc]; scalar_tac
      have hwl : w.val.length = j.val := by rw [← vals_length, hw]; simp
      step as ⟨w1, hw1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨?_, by omega, by omega⟩
      rw [hj1v]; simp only [vals] at hw ⊢
      rw [hw1, List.map_append, hw, List.range_succ]; simp [hcv]
    · rename_i h
      have : j.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hw, this]
  · exact ⟨hv, hi⟩

/-- **`Uf::new n` is the identity**, for `n` within `u32` ids. -/
theorem new_spec (n : Usize) (hn : n.val ≤ U32.max + 1) :
    union_find.Uf.new n ⦃ u => vals u.parent = List.range n.val ⦄ := by
  unfold union_find.Uf.new
  step with new_loop_spec n hn (alloc.vec.Vec.with_capacity U32 n) 0#usize
    (by simp [vals, alloc.vec.Vec.with_capacity]) (by simp) as ⟨v, hv⟩
  exact hv

/-! ## The flood -/

end forge_sheet_core
