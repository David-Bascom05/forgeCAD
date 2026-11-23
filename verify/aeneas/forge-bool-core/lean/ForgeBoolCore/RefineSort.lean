import ForgeBoolCore.RefineIndex
import ForgeBoolCore.SortModel
import ForgeBoolCore.RefineHash
open Aeneas Aeneas.Std Result

/-!
# `sort_by_cov` is a sorted permutation, so `Index::new` is `IndexOK`

The relation is `Rk cells`: id `a` before id `b` when `a`'s covector is not after `b`'s.
-/

namespace forge_bool_core

open ForgeBoolCore.Sort

def Rk (cells : Slice (alloc.vec.Vec Sgn)) (a b : U32) : Prop :=
  leL (cellAt cells a.val) (cellAt cells b.val)

theorem leL_trans {a b c : List Sgn} (h1 : leL a b) (h2 : leL b c) : leL a c := by
  rcases leL_cases h1 with rfl | h
  · exact h2
  · have := lt_le_trans h h2; unfold leL; rw [this]; decide

theorem rk_trans (cells : Slice (alloc.vec.Vec Sgn)) : ∀ a b c, Rk cells a b → Rk cells b c → Rk cells a c :=
  fun _ _ _ h1 h2 => leL_trans h1 h2

/-- Every id in `l` names a cell. -/
def IdsIn (cells : Slice (alloc.vec.Vec Sgn)) (l : List U32) : Prop := ∀ x ∈ l, x.val < cells.val.length

theorem get_some {l : List U32} {i : Nat} (h : i < l.length) : l[i]? = some l[i] := List.getElem?_eq_getElem h

theorem merge_loop_spec (cells : Slice (alloc.vec.Vec Sgn)) (src D0 dst : alloc.vec.Vec U32)
    (lo mid hi i j k : Usize) (hm : MInv (Rk cells) src.val D0.val dst.val lo.val mid.val hi.val i.val j.val k.val)
    (hsl : (seg src.val lo.val mid.val).Pairwise (Rk cells))
    (hsr : (seg src.val mid.val hi.val).Pairwise (Rk cells)) (hids : IdsIn cells src.val) :
    sort.sort_by_cov_loop0_loop0_loop0 cells src dst mid hi i j k ⦃ r =>
      ∃ out, r.val = D0.val.take lo.val ++ out ++ D0.val.drop hi.val ∧
        out.Perm (seg src.val lo.val hi.val) ∧ out.Pairwise (Rk cells) ⦄ := by
  unfold sort.sort_by_cov_loop0_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize × Usize × Usize) => hi.val - q.2.2.2.val)
    (fun q => MInv (Rk cells) src.val D0.val q.1.val lo.val mid.val hi.val q.2.1.val q.2.2.1.val q.2.2.2.val)
  · rintro ⟨d, a, b, c⟩ hinv
    unfold sort.sort_by_cov_loop0_loop0_loop0.body
    simp only at hinv ⊢
    have hinv' := hinv
    obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, _, _, _⟩ := hinv'
    have hdl : d.val.length = src.val.length := by
      have := hperm_len hinv; omega
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
          simp only [Std.bind_ok, ite_true]
          step as ⟨y, hy⟩
          step as ⟨w, hw, hback⟩
          obtain ⟨w1, back⟩ := w
          simp only at hw hback ⊢
          step as ⟨a1, ha1⟩
          step as ⟨c1, hc1⟩
          refine ⟨?_, by scalar_tac⟩
          rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hy]
          exact minv_left (rk_trans cells) hinv hsl ha (get_some (by omega)) (fun h => absurd h (by omega))
        · rename_i hbhi
          have hb : b.val < hi.val := by scalar_tac
          step as ⟨x1, hx1⟩
          step as ⟨x2, hx2⟩
          have hx1l : x1.val < cells.val.length := by rw [hx1]; exact hids _ (List.getElem_mem _)
          have hx2v : x2.val = x1.val := by rw [hx2]; scalar_tac
          step as ⟨v, hv⟩
          step as ⟨y1, hy1⟩
          step as ⟨y2, hy2⟩
          have hy1l : y1.val < cells.val.length := by rw [hy1]; exact hids _ (List.getElem_mem _)
          have hy2v : y2.val = y1.val := by rw [hy2]; scalar_tac
          step as ⟨v1, hv1⟩
          step with cmp_cov_spec (alloc.vec.Vec.deref v) (alloc.vec.Vec.deref v1) as ⟨o, ho⟩
          have hcv : (alloc.vec.Vec.deref v).val = cellAt cells x1.val := by
            rw [cellAt_eq hx1l, hv]; simp [alloc.vec.Vec.deref, hx2v]
          have hcv1 : (alloc.vec.Vec.deref v1).val = cellAt cells y1.val := by
            rw [cellAt_eq hy1l, hv1]; simp [alloc.vec.Vec.deref, hy2v]
          rw [hcv, hcv1] at ho
          by_cases hne : o = 1#i8
          · -- take right: src[j] strictly before src[i]
            have hgt : cmpL (cellAt cells x1.val) (cellAt cells y1.val) = 1 := by rw [← ho, hne]; rfl
            simp only [hne, bne_self_eq_false, Std.bind_ok, Bool.false_eq_true, ite_false]
            step as ⟨y, hy⟩
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨b1, hb1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy]
            refine minv_right (rk_trans cells) hinv hsr hb (get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[a.val] := by rw [get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'
            unfold Rk leL
            rw [← hy1, ← hx1, cmpL_swap, hgt]; decide
          · have hle : Rk cells src.val[a.val] src.val[b.val] := by
              unfold Rk leL; rw [← hx1, ← hy1, ← ho]; intro e; apply hne; scalar_tac
            have : (o != 1#i8) = true := by simpa using hne
            simp only [this, Std.bind_ok, ite_true]
            step as ⟨y, hy⟩
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨a1, ha1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hy]
            refine minv_left (rk_trans cells) hinv hsl ha (get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[b.val] := by rw [get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'; exact hle
      · rename_i hamid
        have ha : mid.val ≤ a.val := by scalar_tac
        have hb : b.val < hi.val := by omega
        simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
        step as ⟨y, hy⟩
        step as ⟨w, hw, hback⟩
        obtain ⟨w1, back⟩ := w
        simp only at hw hback ⊢
        step as ⟨b1, hb1⟩
        step as ⟨c1, hc1⟩
        refine ⟨?_, by scalar_tac⟩
        rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy]
        exact minv_right (rk_trans cells) hinv hsr hb (get_some (by omega)) (fun h => absurd h (by omega))
    · rename_i hkh
      simp only [WP.spec_ok]
      exact minv_done hinv (by scalar_tac)
  · exact hm
where
  hperm_len {src D0 dst : List U32} {lo mid hi i j k : Nat}
      (h : MInv (Rk cells) src D0 dst lo mid hi i j k) : dst.length = src.length := by
    obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, hperm, _, _⟩ := h
    have := hperm.length_eq
    rw [List.length_append, seg_length _ hlo (by omega), seg_length _ hmj (by omega)] at this
    rw [heq]; simp; omega

/-- The middle loop: blocks of width `2w` merged up to `lo`. -/
def MidInv (cells : Slice (alloc.vec.Vec Sgn)) (src D0 dst : List U32) (w lo : Nat) : Prop :=
  lo ≤ src.length ∧ ((∃ b, lo = b * (2 * w)) ∨ lo = src.length) ∧
  ∃ out, dst = out ++ D0.drop lo ∧ out.length = lo ∧ out.Perm (src.take lo) ∧ Blocks (Rk cells) (2 * w) out

theorem drop_of_out {out D0 : List U32} {lo hi : Nat} (hl : out.length = lo) (h : lo ≤ hi) :
    (out ++ D0.drop lo).drop hi = D0.drop hi := by
  rw [List.drop_append, List.drop_of_length_le (by omega), List.nil_append, List.drop_drop, hl]
  congr 1; omega

/-- One block's merge, from the middle loop's invariant to the next. -/
theorem mid_step (cells : Slice (alloc.vec.Vec Sgn)) (n : Usize) (src D0 d : alloc.vec.Vec U32)
    (w l mid hi : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hb : Blocks (Rk cells) w.val src.val) (hids : IdsIn cells src.val)
    (hlt : l.val < n.val) (b : Nat) (hbl : l.val = b * (2 * w.val))
    (hmidv : mid.val = min (l.val + w.val) n.val) (hhiv : hi.val = min (l.val + 2 * w.val) n.val)
    (out : List U32) (hdeq : d.val = out ++ D0.val.drop l.val) (hol : out.length = l.val)
    (hperm : out.Perm (src.val.take l.val)) (hblk : Blocks (Rk cells) (2 * w.val) out) :
    (do
      let dst1 ← sort.sort_by_cov_loop0_loop0_loop0 cells src d mid hi l mid l
      ok (ControlFlow.cont (dst1, hi))) ⦃ r =>
      match r with
      | ControlFlow.done (y : alloc.vec.Vec U32) =>
        y.val.Perm src.val ∧ Blocks (Rk cells) (2 * w.val) y.val ∧ y.val.length = n.val
      | ControlFlow.cont (x' : alloc.vec.Vec U32 × Usize) =>
        MidInv cells src.val D0.val x'.1.val w.val x'.2.val ∧ n.val - x'.2.val < n.val - l.val ⦄ := by
  have hdl : d.val.length = src.val.length := by rw [hdeq]; simp; omega
  have hlm : l.val ≤ mid.val := by rw [hmidv]; omega
  have hmh : mid.val ≤ hi.val := by rw [hmidv, hhiv]; omega
  have hhn : hi.val ≤ src.val.length := by rw [hhiv]; omega
  -- the two runs
  have hrl : (seg src.val l.val mid.val).Pairwise (Rk cells) := by
    have := run_of_blocks hb (2 * b)
    rw [show 2 * b * w.val = l.val by rw [hbl]; rw [Nat.mul_comm 2 b, Nat.mul_assoc],
      show (2 * b + 1) * w.val = l.val + w.val by rw [hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc],
      ← hn, ← hmidv] at this
    exact this
  have hrr : (seg src.val mid.val hi.val).Pairwise (Rk cells) := by
    by_cases hc : l.val + w.val < n.val
    · have hm' : mid.val = (2 * b + 1) * w.val := by
        rw [hmidv, Nat.min_eq_left (by omega), hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]
      have := run_of_blocks hb (2 * b + 1)
      rw [← hm', show (2 * b + 1 + 1) * w.val = l.val + 2 * w.val by
        rw [hbl, Nat.add_mul, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]; omega, ← hn, ← hhiv] at this
      exact this
    · have : mid.val = hi.val := by rw [hmidv, hhiv]; omega
      rw [this, seg_self]; exact List.Pairwise.nil
  have hstart := minv_start (R := Rk cells) src.val d.val hlm hmh hhn hdl
  apply WP.spec_bind (merge_loop_spec cells src d d l mid hi l mid l hstart hrl hrr hids)
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
    rw [e1, drop_of_out (D0 := D0.val) hol (show l.val ≤ hi.val by omega)]
  · have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; simp; omega
  · rw [take_eq_seg, seg_append src.val (Nat.zero_le _) (by omega : l.val ≤ hi.val), ← take_eq_seg]
    exact hperm.append hmp
  · apply blocks_append (by omega) hblk b (by rw [hol, hbl]) hms
    have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; rw [this, hhiv]; omega
  · rw [hhiv]; omega

theorem mid_loop_spec (cells : Slice (alloc.vec.Vec Sgn)) (n : Usize) (src D0 dst : alloc.vec.Vec U32)
    (w lo : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hwn : w.val < n.val) (hmax : 3 * n.val ≤ Usize.max) (hb : Blocks (Rk cells) w.val src.val)
    (hids : IdsIn cells src.val) (hinv : MidInv cells src.val D0.val dst.val w.val lo.val) :
    sort.sort_by_cov_loop0_loop0 cells n src dst w lo ⦃ r =>
      r.val.Perm src.val ∧ Blocks (Rk cells) (2 * w.val) r.val ∧ r.val.length = n.val ⦄ := by
  unfold sort.sort_by_cov_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => n.val - q.2.val)
    (fun q => MidInv cells src.val D0.val q.1.val w.val q.2.val)
  · rintro ⟨d, l⟩ hm
    unfold sort.sort_by_cov_loop0_loop0.body
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
          refine WP.spec_mono (mid_step cells n src D0 d w l i i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (mid_step cells n src D0 d w l i n hn hD0 hw hb hids hlt' b hbl hmv
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
          refine WP.spec_mono (mid_step cells n src D0 d w l n i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (mid_step cells n src D0 d w l n n hn hD0 hw hb hids hlt' b hbl hmv
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

theorem idsIn_perm {cells : Slice (alloc.vec.Vec Sgn)} {l l' : List U32} (h : l.Perm l') (hl : IdsIn cells l') :
    IdsIn cells l := fun x hx => hl x (h.mem_iff.mp hx)

theorem outer_loop_spec (cells : Slice (alloc.vec.Vec Sgn)) (n : Usize) (v0 : List U32)
    (src dst : alloc.vec.Vec U32) (w : Usize) (hn : n.val = src.val.length) (hdn : dst.val.length = n.val)
    (hw : 0 < w.val) (hmax : 3 * n.val ≤ Usize.max) (hp : src.val.Perm v0)
    (hb : Blocks (Rk cells) w.val src.val) (hids : IdsIn cells v0) :
    sort.sort_by_cov_loop0 cells n src dst w ⦃ r => r.val.Perm v0 ∧ r.val.Pairwise (Rk cells) ⦄ := by
  unfold sort.sort_by_cov_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => n.val - q.2.2.val)
    (fun q => q.1.val.length = n.val ∧ q.2.1.val.length = n.val ∧ 0 < q.2.2.val ∧
      q.1.val.Perm v0 ∧ Blocks (Rk cells) q.2.2.val q.1.val)
  · rintro ⟨s, d, ww⟩ ⟨hsn, hdn', hww, hsp, hsb⟩
    unfold sort.sort_by_cov_loop0.body
    simp only at hsn hdn' hww hsp hsb ⊢
    split
    · rename_i hlt
      have hlt' : ww.val < n.val := by scalar_tac
      have hstart : MidInv cells s.val d.val d.val ww.val 0 :=
        ⟨Nat.zero_le _, Or.inl ⟨0, by simp⟩, [], by simp, rfl, by simp, fun b => by simp [seg]⟩
      step with mid_loop_spec cells n s d d ww 0#usize (by omega) (by omega) hww hlt' hmax hsb
        (idsIn_perm hsp hids) hstart as ⟨d1, hd1p, hd1b, hd1l⟩
      step as ⟨w1, hw1⟩
      refine ⟨hd1l, by omega, by omega, hd1p.trans hsp, by rw [hw1, Nat.mul_comm]; exact hd1b, by omega⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : n.val ≤ ww.val := by scalar_tac
      exact ⟨hsp, blocks_full hsb (by omega)⟩
  · exact ⟨hn.symm, hdn, hw, hp, hb⟩

theorem clone_u32_spec (v : alloc.vec.Vec U32) :
    alloc.vec.CloneVec.clone core.clone.CloneU32 v ⦃ r => r.val = v.val ⦄ := by
  unfold alloc.vec.CloneVec.clone
  have hc : ∀ x ∈ v.slice.val, core.clone.CloneU32.clone x = ok x := fun _ _ => rfl
  step as ⟨s, hs⟩
  rw [← slice_val v, ← hs]; rfl

/-- **`sort_by_cov` sorts**: a permutation of its input, in covector order. -/
theorem sort_by_cov_spec (v : alloc.vec.Vec U32) (cells : Slice (alloc.vec.Vec Sgn))
    (hids : IdsIn cells v.val) (hmax : 3 * v.val.length ≤ Usize.max) :
    sort.sort_by_cov v cells ⦃ r => r.val.Perm v.val ∧ r.val.Pairwise (Rk cells) ⦄ := by
  unfold sort.sort_by_cov
  simp only
  split
  · rename_i h
    simp only [WP.spec_ok]
    exact ⟨List.Perm.refl _, pairwise_of_len_le_one (by scalar_tac)⟩
  · rename_i h
    step with clone_u32_spec v as ⟨src, hsrc⟩
    apply outer_loop_spec cells _ v.val src src 1#usize (by simp [hsrc]) (by simp [hsrc]) (by simp)
      (by simpa using hmax) (by rw [hsrc]) (by simp; exact blocks_one _) hids

theorem index_new_loop_spec (cells : Slice (alloc.vec.Vec Sgn)) (hsmall : cells.val.length < U32.max)
    (o : alloc.vec.Vec U32) (i : Usize) (ho : o.val.map (·.val) = List.range i.val)
    (hi : i.val ≤ cells.val.length) :
    index.Index.new_loop cells o i ⦃ r => r.val.map (·.val) = List.range cells.val.length ⦄ := by
  unfold index.Index.new_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => cells.val.length - q.2.val)
    (fun q => q.1.val.map (·.val) = List.range q.2.val ∧ q.2.val ≤ cells.val.length)
  · rintro ⟨w, j⟩ ⟨hw, hj⟩
    unfold index.Index.new_loop.body
    simp only at hw hj ⊢
    split
    · rename_i h
      have hjl : j.val < cells.val.length := by scalar_tac
      step as ⟨c, hc⟩
      have hcv : c.val = j.val := by rw [hc]; scalar_tac
      have hwl : w.val.length = j.val := by
        have := congrArg List.length hw; simpa using this
      step as ⟨w1, hw1⟩
      step as ⟨j1, hj1⟩
      refine ⟨?_, by scalar_tac, by scalar_tac⟩
      rw [hj1, hw1, List.map_append, hw, List.range_succ]; simp [hcv]
    · rename_i h
      have : j.val = cells.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [hw, this]
  · exact ⟨ho, hi⟩

/-- **`Index::new` builds an index `Index::find` can use**, for at most `u32::MAX - 1` cells (and
`3·n` within `usize`). -/
theorem index_new_spec (cells : Slice (alloc.vec.Vec Sgn)) (hsmall : cells.val.length < U32.max)
    (hmax : 3 * cells.val.length ≤ Usize.max) :
    index.Index.new cells ⦃ ix => IndexOK ix.order cells ∧ HashOK ix.hs cells ⦄ := by
  unfold index.Index.new
  step with index_new_loop_spec cells hsmall (alloc.vec.Vec.with_capacity U32 (Slice.len cells)) 0#usize
    (by simp [alloc.vec.Vec.with_capacity]) (by simp) as ⟨o, ho⟩
  have hol : o.val.length = cells.val.length := by have := congrArg List.length ho; simpa using this
  have hids : IdsIn cells o.val := by
    intro x hx
    have : x.val ∈ o.val.map (·.val) := List.mem_map.mpr ⟨x, hx, rfl⟩
    rw [ho] at this; simpa using this
  step with sort_by_cov_spec o cells hids (by omega) as ⟨r, hrp, hrs⟩
  have hrl : r.val.length = cells.val.length := by rw [hrp.length_eq, hol]
  step with hash_table_spec cells hsmall (by omega) as ⟨hs, hhs⟩
  refine ⟨⟨fun p hp => ?_, fun p q hpq hq => ?_, fun id hid => ?_, hsmall⟩, hhs⟩
  · rw [ordAt_eq hp]; exact idsIn_perm hrp hids _ (List.getElem_mem _)
  · rw [ordAt_eq (by omega), ordAt_eq hq]
    exact List.pairwise_iff_getElem.mp hrs p q (by omega) hq hpq
  · have : id ∈ o.val.map (·.val) := by rw [ho]; simpa using hid
    obtain ⟨x, hx, hxv⟩ := List.mem_map.mp this
    obtain ⟨p, hp, hpx⟩ := List.mem_iff_getElem.mp (hrp.mem_iff.mpr hx)
    exact ⟨p, hp, by rw [ordAt_eq hp, hpx, hxv]⟩

end forge_bool_core
