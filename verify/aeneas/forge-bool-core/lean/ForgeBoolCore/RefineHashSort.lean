import ForgeBoolCore.RefineCells
import ForgeBoolCore.SortModel
open Aeneas Aeneas.Std Result

/-!
# `sort_hashed`: the merge sort of `(hash, id)` pairs

The same bottom-up merge sort as `sort_u32` and `sort_triples` (group S's proof, `RefineSplitOk.lean`,
reused with the order `le2`): a permutation of its input, ascending by hash then id. Bounded by
`2·n ≤ usize::MAX`.
-/

namespace forge_bool_core

open ForgeBoolCore.Sort

abbrev HPr := U64 × U32

/-- `le2`, on values: by hash, then by id. -/
def HRt (a b : HPr) : Prop := a.1.val < b.1.val ∨ (a.1.val = b.1.val ∧ a.2.val ≤ b.2.val)

instance (a b : HPr) : Decidable (HRt a b) := by unfold HRt; infer_instance

theorem h_rt_trans : ∀ a b c, HRt a b → HRt b c → HRt a c := by
  intro a b c h1 h2; unfold HRt at *; omega

theorem h_rt_total (a b : HPr) (h : ¬ HRt a b) : HRt b a := by
  unfold HRt at *; omega

theorem h_get_some {l : List HPr} {i : Nat} (h : i < l.length) : l[i]? = some l[i] :=
  List.getElem?_eq_getElem h

theorem h_le2_spec (a b : HPr) : sort.le2 a b ⦃ r => r = decide (HRt a b) ⦄ := by
  obtain ⟨a1, a2⟩ := a
  obtain ⟨b1, b2⟩ := b
  show (if a1 < b1 then ok true else if a1 = b1 then ok (decide (a2 ≤ b2)) else ok false) ⦃ r => _ ⦄
  have e2 : (a1 = b1) ↔ a1.val = b1.val := by scalar_tac
  simp only [UScalar.lt_equiv, UScalar.le_equiv, e2, HRt]
  by_cases h1 : a1.val < b1.val <;> by_cases h2 : a1.val = b1.val <;> simp [h1, h2] <;> omega

/-- No condition on the values (the merge reads no cells). -/
def h_IdsIn (_ : List HPr) : Prop := True

theorem h_merge_spec (src D0 dst : alloc.vec.Vec HPr)
    (lo mid hi i j k : Usize) (hm : MInv HRt src.val D0.val dst.val lo.val mid.val hi.val i.val j.val k.val)
    (hsl : (seg src.val lo.val mid.val).Pairwise HRt)
    (hsr : (seg src.val mid.val hi.val).Pairwise HRt) (hids : h_IdsIn src.val) :
    sort.sort_hashed_loop0_loop0_loop0 src dst mid hi i j k ⦃ r =>
      ∃ out, r.val = D0.val.take lo.val ++ out ++ D0.val.drop hi.val ∧
        out.Perm (seg src.val lo.val hi.val) ∧ out.Pairwise HRt ⦄ := by
  unfold sort.sort_hashed_loop0_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec HPr × Usize × Usize × Usize) => hi.val - q.2.2.2.val)
    (fun q => MInv HRt src.val D0.val q.1.val lo.val mid.val hi.val q.2.1.val q.2.2.1.val q.2.2.2.val)
  · rintro ⟨d, a, b, c⟩ hinv
    unfold sort.sort_hashed_loop0_loop0_loop0.body
    simp only at hinv ⊢
    have hinv' := hinv
    obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, hperm, _, _⟩ := hinv'
    have hdl : d.val.length = src.val.length := by
      have := hperm.length_eq
      rw [List.length_append, seg_length _ hlo (by omega), seg_length _ hmj (by omega)] at this
      rw [heq]; simp; omega
    split
    · rename_i hkh
      have hc : c.val < hi.val := by scalar_tac
      have hcd : c.val < d.val.length := by omega
      split
      · rename_i hamid
        have ha : a.val < mid.val := by scalar_tac
        split
        · rename_i hbhi
          have hb : hi.val ≤ b.val := by scalar_tac
          step as ⟨y, hy⟩
          step as ⟨w, hw, hback⟩
          obtain ⟨w1, back⟩ := w
          simp only at hw hback ⊢
          step as ⟨a1, ha1⟩
          step as ⟨c1, hc1⟩
          refine ⟨?_, by scalar_tac⟩
          rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hy]
          exact minv_left h_rt_trans hinv hsl ha (h_get_some (by omega)) (fun h => absurd h (by omega))
        · rename_i hbhi
          have hb : b.val < hi.val := by scalar_tac
          step as ⟨x1, hx1⟩
          step as ⟨y1, hy1⟩
          step with h_le2_spec as ⟨bb, hbb⟩
          split
          · rename_i hle
            have hle' : HRt src.val[a.val] src.val[b.val] := by
              rw [← hx1, ← hy1]; subst hbb; simpa using hle
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨a1, ha1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hx1]
            refine minv_left h_rt_trans hinv hsl ha (h_get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[b.val] := by rw [h_get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'; exact hle'
          · rename_i hgt
            have hgt' : HRt src.val[b.val] src.val[a.val] := by
              rw [← hx1, ← hy1]; subst hbb; exact h_rt_total _ _ (by simpa using hgt)
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨b1, hb1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy1]
            refine minv_right h_rt_trans hinv hsr hb (h_get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[a.val] := by rw [h_get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'; exact hgt'
      · rename_i hamid
        have ha : mid.val ≤ a.val := by scalar_tac
        have hb : b.val < hi.val := by omega
        step as ⟨y, hy⟩
        step as ⟨w, hw, hback⟩
        obtain ⟨w1, back⟩ := w
        simp only at hw hback ⊢
        step as ⟨b1, hb1⟩
        step as ⟨c1, hc1⟩
        refine ⟨?_, by scalar_tac⟩
        rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy]
        exact minv_right h_rt_trans hinv hsr hb (h_get_some (by omega)) (fun h => absurd h (by omega))
    · rename_i hkh
      simp only [WP.spec_ok]
      exact minv_done hinv (by scalar_tac)
  · exact hm

/-- The middle loop: blocks of width `2w` merged up to `lo`. -/
def H_MidInv (src D0 dst : List HPr) (w lo : Nat) : Prop :=
  lo ≤ src.length ∧ ((∃ b, lo = b * (2 * w)) ∨ lo = src.length) ∧
  ∃ out, dst = out ++ D0.drop lo ∧ out.length = lo ∧ out.Perm (src.take lo) ∧ Blocks (HRt) (2 * w) out

theorem h_drop_of_out {out D0 : List HPr} {lo hi : Nat} (hl : out.length = lo) (h : lo ≤ hi) :
    (out ++ D0.drop lo).drop hi = D0.drop hi := by
  rw [List.drop_append, List.drop_of_length_le (by omega), List.nil_append, List.drop_drop, hl]
  congr 1; omega

/-- One block's merge, from the middle loop's invariant to the next. -/
theorem h_mid_step (n : Usize) (src D0 d : alloc.vec.Vec HPr)
    (w l mid hi : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hb : Blocks (HRt) w.val src.val) (hids : h_IdsIn src.val)
    (hlt : l.val < n.val) (b : Nat) (hbl : l.val = b * (2 * w.val))
    (hmidv : mid.val = min (l.val + w.val) n.val) (hhiv : hi.val = min (l.val + 2 * w.val) n.val)
    (out : List HPr) (hdeq : d.val = out ++ D0.val.drop l.val) (hol : out.length = l.val)
    (hperm : out.Perm (src.val.take l.val)) (hblk : Blocks (HRt) (2 * w.val) out) :
    (do
      let dst1 ← sort.sort_hashed_loop0_loop0_loop0 src d mid hi l mid l
      ok (ControlFlow.cont (dst1, hi))) ⦃ r =>
      match r with
      | ControlFlow.done (y : alloc.vec.Vec HPr) =>
        y.val.Perm src.val ∧ Blocks (HRt) (2 * w.val) y.val ∧ y.val.length = n.val
      | ControlFlow.cont (x' : alloc.vec.Vec HPr × Usize) =>
        H_MidInv src.val D0.val x'.1.val w.val x'.2.val ∧ n.val - x'.2.val < n.val - l.val ⦄ := by
  have hdl : d.val.length = src.val.length := by rw [hdeq]; simp; omega
  have hlm : l.val ≤ mid.val := by rw [hmidv]; omega
  have hmh : mid.val ≤ hi.val := by rw [hmidv, hhiv]; omega
  have hhn : hi.val ≤ src.val.length := by rw [hhiv]; omega
  -- the two runs
  have hrl : (seg src.val l.val mid.val).Pairwise (HRt) := by
    have := run_of_blocks hb (2 * b)
    rw [show 2 * b * w.val = l.val by rw [hbl]; rw [Nat.mul_comm 2 b, Nat.mul_assoc],
      show (2 * b + 1) * w.val = l.val + w.val by rw [hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc],
      ← hn, ← hmidv] at this
    exact this
  have hrr : (seg src.val mid.val hi.val).Pairwise (HRt) := by
    by_cases hc : l.val + w.val < n.val
    · have hm' : mid.val = (2 * b + 1) * w.val := by
        rw [hmidv, Nat.min_eq_left (by omega), hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]
      have := run_of_blocks hb (2 * b + 1)
      rw [← hm', show (2 * b + 1 + 1) * w.val = l.val + 2 * w.val by
        rw [hbl, Nat.add_mul, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]; omega, ← hn, ← hhiv] at this
      exact this
    · have : mid.val = hi.val := by rw [hmidv, hhiv]; omega
      rw [this, seg_self]; exact List.Pairwise.nil
  have hstart := minv_start (R := HRt) src.val d.val hlm hmh hhn hdl
  apply WP.spec_bind (h_merge_spec src d d l mid hi l mid l hstart hrl hrr hids)
  intro d1 hd1
  obtain ⟨m, hd1, hmp, hms⟩ := hd1
  simp only [WP.spec_ok]
  refine ⟨⟨hhn, ?_, out ++ m, ?_, ?_, ?_, ?_⟩, ?_⟩
  · by_cases hc : l.val + 2 * w.val < n.val
    · left; exact ⟨b + 1, by rw [hhiv, Nat.min_eq_left (by omega), hbl, Nat.add_mul, Nat.one_mul]⟩
    · right; rw [hhiv, ← hn, Nat.min_eq_right (by omega)]
  · rw [hd1, hdeq]
    have e1 : (out ++ D0.val.drop l.val).take l.val = out := by
      rw [List.take_append_of_le_length (by omega), List.take_of_length_le (by omega)]
    rw [e1, h_drop_of_out (D0 := D0.val) hol (show l.val ≤ hi.val by omega)]
  · have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; simp; omega
  · rw [take_eq_seg, seg_append src.val (Nat.zero_le _) (by omega : l.val ≤ hi.val), ← take_eq_seg]
    exact hperm.append hmp
  · apply blocks_append (by omega) hblk b (by rw [hol, hbl]) hms
    have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; rw [this, hhiv]; omega
  · rw [hhiv]; omega

theorem h_mid_loop_spec (n : Usize) (src D0 dst : alloc.vec.Vec HPr)
    (w lo : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hwn : w.val < n.val) (hmax : 2 * n.val ≤ Usize.max) (hb : Blocks (HRt) w.val src.val)
    (hids : h_IdsIn src.val) (hinv : H_MidInv src.val D0.val dst.val w.val lo.val) :
    sort.sort_hashed_loop0_loop0 n src dst w lo ⦃ r =>
      r.val.Perm src.val ∧ Blocks (HRt) (2 * w.val) r.val ∧ r.val.length = n.val ⦄ := by
  unfold sort.sort_hashed_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec HPr × Usize) => n.val - q.2.val)
    (fun q => H_MidInv src.val D0.val q.1.val w.val q.2.val)
  · rintro ⟨d, l⟩ hm
    unfold sort.sort_hashed_loop0_loop0.body
    simp only at hm ⊢
    obtain ⟨hln, hal, out, hdeq, hol, hperm, hblk⟩ := hm
    split
    · rename_i hlt
      have hlt' : l.val < n.val := by scalar_tac
      obtain ⟨b, hbl⟩ : ∃ b, l.val = b * (2 * w.val) := by
        rcases hal with h | h
        · exact h
        · omega
      have h1 : l.val + w.val ≤ Usize.max := by omega
      step as ⟨i, hi⟩
      have h2 : 2 * w.val ≤ Usize.max := by omega
      have hl2 : l.val = 0 ∨ 2 * w.val ≤ l.val := by
        cases b with
        | zero => left; rw [hbl]; simp
        | succ b => right; rw [hbl]; exact Nat.le_mul_of_pos_left _ (Nat.succ_pos _)
      have h3 : l.val + 2 * w.val ≤ Usize.max := by omega
      by_cases hin : i < n
      · simp only [hin, ite_true, Std.bind_ok]
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hmv : i.val = min (l.val + w.val) n.val := by
          have : i.val < n.val := by scalar_tac
          rw [Nat.min_eq_left (by omega)]; omega
        by_cases hin2 : i2 < n
        · simp only [hin2, ite_true]
          step as ⟨i3, hi3⟩
          refine WP.spec_mono (h_mid_step n src D0 d w l i i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (h_mid_step n src D0 d w l i n hn hD0 hw hb hids hlt' b hbl hmv
            (by have : ¬i2.val < n.val := hin2
                rw [Nat.min_eq_right (by omega)]) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
      · simp only [hin, ite_false, Std.bind_ok]
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hmv : n.val = min (l.val + w.val) n.val := by
          have : ¬i.val < n.val := by scalar_tac
          rw [Nat.min_eq_right (by omega)]
        by_cases hin2 : i2 < n
        · simp only [hin2, ite_true]
          step as ⟨i3, hi3⟩
          refine WP.spec_mono (h_mid_step n src D0 d w l n i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (h_mid_step n src D0 d w l n n hn hD0 hw hb hids hlt' b hbl hmv
            (by have : ¬i2.val < n.val := hin2
                rw [Nat.min_eq_right (by omega)]) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
    · rename_i hge
      simp only [WP.spec_ok]
      have hl : l.val = n.val := by scalar_tac
      rw [hl] at hdeq hol hperm
      have hdrop : D0.val.drop n.val = [] := List.drop_of_length_le (by omega)
      rw [hdeq, hdrop, List.append_nil]
      refine ⟨?_, hblk, by omega⟩
      rw [hn, List.take_of_length_le (Nat.le_refl _)] at hperm; exact hperm
  · exact hinv

theorem h_idsIn_perm {l l' : List HPr} (h : l.Perm l') (hl : h_IdsIn l') :
    h_IdsIn l := trivial

theorem h_outer_loop_spec (n : Usize) (v0 : List HPr)
    (src dst : alloc.vec.Vec HPr) (w : Usize) (hn : n.val = src.val.length) (hdn : dst.val.length = n.val)
    (hw : 0 < w.val) (hmax : 2 * n.val ≤ Usize.max) (hp : src.val.Perm v0)
    (hb : Blocks (HRt) w.val src.val) (hids : h_IdsIn v0) :
    sort.sort_hashed_loop0 n src dst w ⦃ r => r.val.Perm v0 ∧ r.val.Pairwise (HRt) ⦄ := by
  unfold sort.sort_hashed_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec HPr × alloc.vec.Vec HPr × Usize) => n.val - q.2.2.val)
    (fun q => q.1.val.length = n.val ∧ q.2.1.val.length = n.val ∧ 0 < q.2.2.val ∧
      q.1.val.Perm v0 ∧ Blocks (HRt) q.2.2.val q.1.val)
  · rintro ⟨s, d, ww⟩ ⟨hsn, hdn', hww, hsp, hsb⟩
    unfold sort.sort_hashed_loop0.body
    simp only at hsn hdn' hww hsp hsb ⊢
    split
    · rename_i hlt
      have hlt' : ww.val < n.val := by scalar_tac
      have hstart : H_MidInv s.val d.val d.val ww.val 0 :=
        ⟨Nat.zero_le _, Or.inl ⟨0, by simp⟩, [], by simp, rfl, by simp, fun b => by simp [seg]⟩
      step with h_mid_loop_spec n s d d ww 0#usize (by omega) (by omega) hww hlt' hmax hsb
        (h_idsIn_perm hsp hids) hstart as ⟨d1, hd1p, hd1b, hd1l⟩
      step as ⟨w1, hw1⟩
      refine ⟨hd1l, by omega, by omega, hd1p.trans hsp, by rw [hw1, Nat.mul_comm]; exact hd1b, by omega⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : n.val ≤ ww.val := by scalar_tac
      exact ⟨hsp, blocks_full hsb (by omega)⟩
  · exact ⟨hn.symm, hdn, hw, hp, hb⟩


theorem h_clone_spec (v : alloc.vec.Vec HPr) :
    alloc.vec.CloneVec.clone (BuiltinClone HPr) v ⦃ r => r.val = v.val ⦄ := by
  unfold alloc.vec.CloneVec.clone
  have hc : ∀ x ∈ v.slice.val, (BuiltinClone HPr).clone x = ok x := fun _ _ => rfl
  step as ⟨s, hs⟩
  rw [← slice_val v, ← hs]; rfl

/-- **`sort_hashed` sorts**: a permutation of its input, ascending. -/
theorem h_sort_hashed_spec (v : alloc.vec.Vec HPr) (hmax : 2 * v.val.length ≤ Usize.max) :
    sort.sort_hashed v ⦃ r => r.val.Perm v.val ∧ r.val.Pairwise HRt ⦄ := by
  unfold sort.sort_hashed
  simp only
  split
  · rename_i h
    simp only [WP.spec_ok]
    exact ⟨List.Perm.refl _, pairwise_of_len_le_one (by scalar_tac)⟩
  · rename_i h
    step with h_clone_spec v as ⟨src, hsrc⟩
    apply h_outer_loop_spec _ v.val src src 1#usize (by simp [hsrc]) (by simp [hsrc]) (by simp)
      (by simpa using hmax) (by rw [hsrc]) (by simp; exact blocks_one _) trivial

end forge_bool_core
