import ForgeBoolCore.RefineIncTouch
import ForgeBoolCore.FanBridgeI

/-!
# Group K: the touch rule at verts

`inc::around_v l a v` lists the sectors round vert `v` whose facet is in the result (by index);
`inc::fans` unites the fan-related ones (sharing a bounding edge); `inc::bad_vert` asks whether
they fall in more than one class. Here that is `badVert` of the keep table, through `fan_bad`
(FanBridgeI).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

instance : Inhabited Sector := ⟨⟨0#u32, 0#u32, 0#u32⟩⟩

def secAt (l : Ledger) (k : Nat) : Sector := l.vert_sectors.sec.val.getD k default

/-- Two sectors share a bounding edge. -/
def fanB (s t : Sector) : Bool :=
  decide (s.a.val = t.a.val) || decide (s.a.val = t.b.val) || decide (s.b.val = t.a.val) ||
    decide (s.b.val = t.b.val)

theorem u32_eq_iff (x y : U32) : x = y ↔ x.val = y.val :=
  ⟨fun h => h ▸ rfl, fun h => UScalar.eq_of_val_eq h⟩

theorem fan_rel_spec (s t : Sector) : fan_rel s t ⦃ b => b = fanB s t ⦄ := by
  unfold fan_rel fanB
  split
  · rename_i h; simp [(u32_eq_iff _ _).mp h]
  · rename_i h1
    split
    · rename_i h; simp [(u32_eq_iff _ _).mp h]
    · rename_i h2
      split
      · rename_i h; simp [(u32_eq_iff _ _).mp h]
      · rename_i h3
        simp only [WP.spec_ok]
        rw [u32_eq_iff] at h1 h2 h3
        simp [h1, h2, h3, u32_eq_iff]

/-! ### Sector rows -/

def sstAt (l : Ledger) (k : Nat) : Nat := (l.vert_sectors.start.val.getD k 0#u32).val

theorem sec_mono (l : Ledger) (hw : LedgerWF2 l) {k : Nat} :
    ∀ d, k + d < l.vert_sectors.start.val.length → sstAt l k ≤ sstAt l (k + d)
  | 0, _ => Nat.le_refl _
  | d + 1, hd => by
    have ih := sec_mono l hw (k := k) d (by omega)
    have := hw.sectors_csr.mono (k + d) (by omega)
    have e : k + (d + 1) = k + d + 1 := by omega
    rw [e]; exact Nat.le_trans ih this

theorem sec_le_len (l : Ledger) (hw : LedgerWF2 l) {k : Nat} (hk : k < l.vert_sectors.start.val.length) :
    sstAt l k ≤ l.vert_sectors.sec.val.length := by
  have := sec_mono l hw (k := k) (l.vert_sectors.start.val.length - 1 - k) (by omega)
  rw [show k + (l.vert_sectors.start.val.length - 1 - k) = l.vert_sectors.start.val.length - 1 by omega] at this
  have hl := hw.sectors_csr.last
  unfold sstAt at *; omega

theorem take_drop_eq_map {β : Type} [Inhabited β] (S : List β) (lo hi : Nat) (h : hi ≤ S.length) :
    (S.drop lo).take (hi - lo) = (List.range' lo (hi - lo)).map (fun k => S.getD k default) := by
  apply List.ext_getElem
  · simp; omega
  · intro i h1 h2
    simp only [List.getElem_take, List.getElem_drop, List.getElem_map, List.getElem_range', Nat.one_mul]
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp at h1; omega)]
    rfl

theorem secRowL_eq (l : Ledger) {v : Nat} (hv : sstAt l (v + 1) ≤ l.vert_sectors.sec.val.length) :
    secRowL l.vert_sectors v = (List.range' (sstAt l v) (sstAt l (v + 1) - sstAt l v)).map (secAt l) := by
  show (l.vert_sectors.sec.val.drop (sstAt l v)).take (sstAt l (v + 1) - sstAt l v) = _
  exact take_drop_eq_map _ _ _ hv

/-! ### `around_v` -/

/-- The sector indices round a vert, from `lo` up to `k`, whose facet is in the result. -/
def avL (l : Ledger) (a : Assembled) (lo k : Nat) : List Nat :=
  (List.range' lo (k - lo)).filter fun j => rpA a (secAt l j).facet.val != U32.max

theorem avL_succ (l : Ledger) (a : Assembled) {lo k : Nat} (h : lo ≤ k) :
    avL l a lo (k + 1) = avL l a lo k ++ (if rpA a (secAt l k).facet.val != U32.max then [k] else []) := by
  unfold avL
  rw [show k + 1 - lo = (k - lo) + 1 by omega, List.range'_1_concat, List.filter_append,
    show lo + (k - lo) = k by omega]
  simp only [List.filter_cons, List.filter_nil]

theorem around_v_loop_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : nfacets l ≤ a.rpos.val.length)
    (v : Nat) (hv : v < nrowsL l.vert_edges) (lo hi : Usize) (hlo : lo.val = sstAt l v) (hhi : hi.val = sstAt l (v + 1))
    (out : alloc.vec.Vec Usize) (k : Usize) (hk : lo.val ≤ k.val ∧ k.val ≤ hi.val)
    (ho : out.val.map (·.val) = avL l a lo.val k.val) :
    around_v_loop l.vert_sectors.sec a hi out k ⦃ r => r.val.map (·.val) = avL l a lo.val hi.val ⦄ := by
  have hsr := hw.sectors_rows
  have hvl : v + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hmono := sec_mono l hw (k := v) 1 hvl
  have hlen := sec_le_len l hw hvl
  have hrow : secRowL l.vert_sectors v = (List.range' lo.val (hi.val - lo.val)).map (secAt l) := by
    rw [hlo, hhi]; exact secRowL_eq l hlen
  unfold around_v_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Usize × Usize) => hi.val - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧ q.1.val.map (·.val) = avL l a lo.val q.2.val)
  · rintro ⟨o, j⟩ ⟨hj1, hj2, ho'⟩
    unfold around_v_loop.body
    simp only at hj1 hj2 ho' ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.vert_sectors.sec.val.length := by scalar_tac
      have hmem : secAt l j.val ∈ secRowL l.vert_sectors v := by
        rw [hrow, List.mem_map]; exact ⟨j.val, List.mem_range'_1.mpr ⟨hj1, by scalar_tac⟩, rfl⟩
      have hfac := (hw.sectors v hv _ hmem).1
      step as ⟨sc, hsc⟩
      have hscv : sc = secAt l j.val := by
        rw [hsc, secAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; rfl
      step as ⟨i, hi'⟩
      have hiv : i.val = (secAt l j.val).facet.val := by rw [hi', hscv]; scalar_tac
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = rpA a (secAt l j.val).facet.val := by
        rw [rpA, ← hiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, hi1]
      have hol : o.val.length ≤ j.val := by
        have := congrArg List.length ho'
        simp only [List.length_map, avL] at this
        have h2 := List.length_filter_le (fun x => rpA a (secAt l x).facet.val != U32.max) (List.range' lo.val (j.val - lo.val))
        simp only [List.length_range'] at h2
        omega
      split
      · rename_i hne
        have hne' : (rpA a (secAt l j.val).facet.val != U32.max) = true := by
          rw [← hi1v]; have : i1 ≠ NONE := by simpa using hne
          simp only [bne_iff_ne, ne_eq]; intro e; apply this; apply UScalar.eq_of_val_eq; rw [e, inc_none_val]
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1'⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have : j1.val = j.val + 1 := by scalar_tac
        rw [this, avL_succ l a hj1, if_pos hne', ho1, List.map_append, ho']
        simp
      · rename_i hne
        have hne' : (rpA a (secAt l j.val).facet.val != U32.max) = false := by
          rw [← hi1v]; have : i1.val = NONE.val := by simpa using hne
          rw [this, inc_none_val]; simp
        step as ⟨j1, hj1'⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have : j1.val = j.val + 1 := by scalar_tac
        rw [this, avL_succ l a hj1, hne']; simpa using ho'
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = hi.val := by scalar_tac
      rw [← this]; exact ho'
  · exact ⟨hk.1, hk.2, ho⟩

theorem around_v_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : nfacets l ≤ a.rpos.val.length)
    (v : Usize) (hv : v.val < nrowsL l.vert_edges) :
    around_v l a v ⦃ r => r.val.map (·.val) = avL l a (sstAt l v.val) (sstAt l (v.val + 1)) ⦄ := by
  have hsr := hw.sectors_rows
  have hvl : v.val + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hmono := sec_mono l hw (k := v.val) 1 hvl
  have hlen := sec_le_len l hw hvl
  unfold around_v
  step as ⟨i, hi⟩
  step as ⟨lo, hlo⟩
  have hlov : lo.val = sstAt l v.val := by
    rw [hlo, sstAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, ← hi]; scalar_tac
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  step as ⟨hi', hhi⟩
  have hhiv : hi'.val = sstAt l (v.val + 1) := by
    have : i1.val = v.val + 1 := by scalar_tac
    rw [hhi, sstAt, ← this, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, ← hi2]
    scalar_tac
  rw [← hlov, ← hhiv]
  exact around_v_loop_spec l hw a hrl v.val hv lo hi' hlov hhiv (alloc.vec.Vec.new Usize) lo
    ⟨Nat.le_refl _, by rw [hlov, hhiv]; exact hmono⟩ (by simp [avL])

/-! ### `fans` -/

/-- Fan-related, by position in `ar`. -/
def relA (l : Ledger) (ar : List Nat) (p q : Nat) : Bool := fanB (secAt l (ar.getD p 0)) (secAt l (ar.getD q 0))

theorem fanE_succ {rel : Nat → Nat → Bool} {n p q : Nat} (h : p + 1 ≤ q) :
    fanE rel n p (q + 1) = fanE rel n p q ++ (if rel p q then [(p, q)] else []) := by
  unfold fanE
  rw [show q + 1 - (p + 1) = (q - (p + 1)) + 1 by omega, List.range'_1_concat, List.filter_append,
    show p + 1 + (q - (p + 1)) = q by omega, List.map_append, List.append_assoc]
  congr 2
  simp only [List.filter_cons, List.filter_nil]
  split <;> simp

theorem fanE_row {rel : Nat → Nat → Bool} {n p : Nat} : fanE rel n (p + 1) (p + 2) = fanE rel n p n := by
  unfold fanE
  simp [List.range_succ, List.flatMap_append]

theorem fanE_in {rel : Nat → Nat → Bool} {n p q : Nat} (hp : p ≤ n) (hq : q ≤ n ∨ q ≤ p + 1) :
    ∀ e ∈ fanE rel n p q, e.1 < n ∧ e.2 < n := by
  rintro ⟨x, y⟩ he
  unfold fanE at he
  simp only [List.mem_append, List.mem_flatMap, List.mem_map, List.mem_filter, List.mem_range'_1,
    List.mem_range, Prod.mk.injEq] at he
  rcases he with ⟨p', hp', q', ⟨⟨h1, h2⟩, _⟩, rfl, rfl⟩ | ⟨q', ⟨⟨h1, h2⟩, _⟩, rfl, rfl⟩ <;> omega

theorem fans_inner_spec (l : Ledger) (ar : Slice Usize) (hn : ar.val.length ≤ U32.max)
    (har : ∀ x ∈ ar.val, x.val < l.vert_sectors.sec.val.length)
    (p : Usize) (hp : p.val < ar.val.length) (uf : union_find.Uf) (q : Usize)
    (hq : p.val + 1 ≤ q.val ∧ q.val ≤ ar.val.length)
    (hu : UInv (fanE (relA l (ar.val.map (·.val))) ar.val.length p.val q.val) ar.val.length uf) :
    fans_loop0_loop0 l ar uf p q ⦃ r => r.1 = l ∧
      UInv (fanE (relA l (ar.val.map (·.val))) ar.val.length p.val ar.val.length) ar.val.length r.2 ⦄ := by
  unfold fans_loop0_loop0
  apply loop.spec_decr_nat (fun (x : union_find.Uf × Usize) => ar.val.length - x.2.val)
    (fun x => p.val + 1 ≤ x.2.val ∧ x.2.val ≤ ar.val.length ∧
      UInv (fanE (relA l (ar.val.map (·.val))) ar.val.length p.val x.2.val) ar.val.length x.1)
  · rintro ⟨u, j⟩ ⟨hja, hj2, hu'⟩
    unfold fans_loop0_loop0.body
    simp only at hja hj2 hu' ⊢
    split
    · rename_i hlt
      have hjl : j.val < ar.val.length := by scalar_tac
      step as ⟨i1, hi1⟩
      have hi1l : i1.val < l.vert_sectors.sec.val.length := by rw [hi1]; exact har _ (List.getElem_mem _)
      step as ⟨sc, hsc⟩
      step as ⟨i2, hi2⟩
      have hi2l : i2.val < l.vert_sectors.sec.val.length := by rw [hi2]; exact har _ (List.getElem_mem _)
      step as ⟨sc1, hsc1⟩
      step with fan_rel_spec sc sc1 as ⟨b, hb⟩
      have hbv : b = relA l (ar.val.map (·.val)) p.val j.val := by
        rw [hb, relA, hsc, hsc1, secAt, secAt]
        simp only [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_eq_getElem hp,
          List.getElem?_eq_getElem hjl, Option.map_some, Option.getD_some]
        rw [List.getElem?_eq_getElem (by rw [← hi1]; exact hi1l), List.getElem?_eq_getElem (by rw [← hi2]; exact hi2l)]
        simp [hi1, hi2]
      have hrange := fanE_in (rel := relA l (ar.val.map (·.val))) (n := ar.val.length) (p := p.val) (q := j.val)
        (by omega) (Or.inl (by omega))
      split
      · rename_i hb1
        step with union_spec u hu' hrange (by omega) p j hp hjl as ⟨u1, hu1⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, by omega, ?_, by omega⟩
        rw [hj1v, fanE_succ hja, ← hbv, hb1]; simpa using hu1
      · rename_i hb1
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, by omega, ?_, by omega⟩
        have : b = false := by simpa using hb1
        rw [hj1v, fanE_succ hja, ← hbv, this]; simpa using hu'
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ar.val.length := by scalar_tac
      refine ⟨by simp, ?_⟩
      have e := this
      rw [e] at hu'; exact hu'
  · exact ⟨hq.1, hq.2, hu⟩

theorem fans_spec (l : Ledger) (ar : Slice Usize) (hn : ar.val.length ≤ U32.max)
    (har : ∀ x ∈ ar.val, x.val < l.vert_sectors.sec.val.length) :
    fans l ar ⦃ r => UInv (fanAll (relA l (ar.val.map (·.val))) ar.val.length) ar.val.length r ⦄ := by
  unfold fans
  have hn' : (Slice.len ar).val ≤ U32.max + 1 := by simp; omega
  step with new_spec _ hn' as ⟨u, hu⟩
  have hlen : (Slice.len ar).val = ar.val.length := by simp
  unfold fans_loop0
  apply loop.spec_decr_nat (fun (x : Ledger × union_find.Uf × Usize) => ar.val.length - x.2.2.val)
    (fun x => x.1 = l ∧ x.2.2.val ≤ ar.val.length ∧
      UInv (fanE (relA l (ar.val.map (·.val))) ar.val.length x.2.2.val (x.2.2.val + 1)) ar.val.length x.2.1)
  · rintro ⟨l', u', p⟩ ⟨rfl, hp, hu'⟩
    unfold fans_loop0.body
    simp only at hp hu' ⊢
    split
    · rename_i hlt
      have hpl : p.val < ar.val.length := by scalar_tac
      step as ⟨q, hq⟩
      have hqv : q.val = p.val + 1 := by scalar_tac
      step with fans_inner_spec l' ar hn har p hpl u' q ⟨by omega, by omega⟩ (by rw [hqv]; exact hu')
        as ⟨l1, u1, hl1, hu1⟩
      step as ⟨p1, hp1⟩
      have hp1v : p1.val = p.val + 1 := by scalar_tac
      refine ⟨hl1, by omega, ?_, by omega⟩
      rw [hp1v, fanE_row]; exact hu1
    · rename_i hge
      simp only [WP.spec_ok]
      have : p.val = ar.val.length := by scalar_tac
      rw [this] at hu'; exact hu'
  · refine ⟨by simp, by simp, ?_⟩
    unfold UInv; rw [hu, hlen]
    have : fanE (relA l (ar.val.map (·.val))) ar.val.length 0 1 = [] := by simp [fanE]
    rw [show (0#usize : Usize).val = 0 by simp, this]; exact inv_range _

/-! ### `bad_vert` -/

theorem any_range_true {f : Nat → Bool} {p n : Nat} (hp : p ≤ n) (h : (List.range p).any f = true) :
    (List.range n).any f = true := by
  rw [List.any_eq_true] at h ⊢
  obtain ⟨x, hx, hfx⟩ := h
  exact ⟨x, List.mem_range.mpr (by have := List.mem_range.mp hx; omega), hfx⟩

theorem bad_vert_loop_spec (ar : alloc.vec.Vec Usize) (E : List (Nat × Nat)) (uf : union_find.Uf)
    (hu : UInv E ar.val.length uf) :
    bad_vert_loop ar uf false 1#usize ⦃ b =>
      b = (List.range ar.val.length).any fun q => ufRoot (vals uf.parent) q != ufRoot (vals uf.parent) 0 ⦄ := by
  have hl := uinv_len hu
  obtain ⟨hg0, _, _⟩ := hu
  generalize hR : ufRoot (vals uf.parent) = R
  generalize hn : ar.val.length = n at hl
  unfold bad_vert_loop
  apply loop.spec_decr_nat (fun (x : union_find.Uf × Bool × Usize) => n + 1 - x.2.2.val)
    (fun x => 1 ≤ x.2.2.val ∧ x.2.2.val ≤ max n 1 ∧ Good (vals x.1.parent) ∧ x.1.parent.val.length = n ∧
      (∀ y, ufRoot (vals x.1.parent) y = R y) ∧ x.2.1 = (List.range x.2.2.val).any fun q => R q != R 0)
  · rintro ⟨u, b, p⟩ ⟨hp1, hp2, hg, hlu, hr, hb⟩
    unfold bad_vert_loop.body
    simp only at hp1 hp2 hg hlu hr hb ⊢
    split
    · rename_i hbt
      simp only [WP.spec_ok]
      have hbt' : b = true := by simpa using hbt
      rw [hbt'] at hb
      rcases Nat.lt_or_ge 0 n with hn0 | hn0
      · rw [hbt']; exact (any_range_true (p := p.val) (n := n) (by omega) hb.symm).symm
      · exfalso
        have : p.val = 1 := by omega
        rw [this] at hb; simp at hb
    · rename_i hbf
      have hbf' : b = false := by simpa using hbf
      subst hbf'
      clear hbf
      split
      · rename_i hlt
        have hpl : p.val < n := by rw [← hn]; scalar_tac
        step with find_spec u p (by omega) hg as ⟨i1, u1, hi1, hl1, hg1, hr1⟩
        step with find_spec u1 0#usize (by simp; omega) hg1 as ⟨i2, u2, hi2, hl2, hg2, hr2⟩
        step as ⟨p1, hp1'⟩
        have hp1v : p1.val = p.val + 1 := by scalar_tac
        refine ⟨by omega, by omega, hg2, by omega, fun y => by rw [hr2, hr1]; exact hr y, ?_, by omega⟩
        rw [hp1v, List.range_succ, List.any_append, ← hb]
        simp only [List.any_cons, List.any_nil, Bool.or_false, Bool.false_or]
        rw [Bool.eq_iff_iff, bne_iff_ne, bne_iff_ne, ne_eq, ne_eq, ← hr p.val, ← hr 0, ← hi1]
        have : i2.val = ufRoot (vals u.parent) 0 := by rw [hi2, hr1]
        rw [← this]
        constructor
        · intro h e; apply h; apply UScalar.eq_of_val_eq; exact e
        · intro h e; apply h; rw [e]
      · rename_i hge
        simp only [WP.spec_ok]
        have hpn : p.val ≥ n := by rw [← hn]; scalar_tac
        rw [hb]
        rcases Nat.lt_or_ge 0 n with hn0 | hn0
        · have : p.val = n := by omega
          rw [this]
        · have h1 : p.val = 1 := by omega
          have h0 : n = 0 := by omega
          rw [h1, h0]; simp
  · refine ⟨by simp, by simp, hg0, hl, fun y => by rw [hR], by simp⟩

/-! ### The Lean side -/

def toSec (s : Sector) : ForgeExact.DecideOnce.Sector FCell := ⟨.facet s.facet.val, [.edge s.a.val, .edge s.b.val]⟩

theorem toSec_inj {s t : Sector} (h : toSec s = toSec t) : s = t := by
  obtain ⟨f, a, b⟩ := s
  obtain ⟨f', a', b'⟩ := t
  simp only [toSec, ForgeExact.DecideOnce.Sector.mk.injEq, FCell.facet.injEq, List.cons.injEq, FCell.edge.injEq,
    and_true] at h
  obtain ⟨h1, h2, h3⟩ := h
  rw [UScalar.eq_of_val_eq h1, UScalar.eq_of_val_eq h2, UScalar.eq_of_val_eq h3]

theorem fanRel_toSec (s t : Sector) : IncLedger.fanRel (toSec s) (toSec t) = fanB s t := by
  rw [Bool.eq_iff_iff]
  simp [IncLedger.fanRel, toSec, fanB, or_assoc]

theorem fanRel_symm (x y : ForgeExact.DecideOnce.Sector FCell) : IncLedger.fanRel x y = IncLedger.fanRel y x := by
  rw [Bool.eq_iff_iff]
  simp only [IncLedger.fanRel, List.any_eq_true, List.contains_iff_mem]
  constructor <;> rintro ⟨e, h1, h2⟩ <;> exact ⟨e, h2, h1⟩

theorem mem_avL {l : Ledger} {a : Assembled} {lo hi x : Nat} (h : x ∈ avL l a lo hi) : lo ≤ x ∧ x < hi := by
  simp only [avL, List.mem_filter, List.mem_range'_1] at h; omega

/-- The Lean's result sectors round a vert are `around_v`'s, read as sectors. -/
theorem aroundV_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (a : Assembled) (h : AInv l s (keepVec l s rule) a.result.val a.rpos.val a.orient.val (nfacets l))
    {v : Nat} (hv : v < nrowsL l.vert_edges) :
    (toLF l).aroundV ((toLF l).keepTab (ruleL l.nb.val rule)) (FCell.vert v) =
      (avL l a (sstAt l v) (sstAt l (v + 1))).map (fun k => toSec (secAt l k)) := by
  have hsr := hw.sectors_rows
  have hvl : v + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hlen := sec_le_len l hw hvl
  have hrow := secRowL_eq l hlen
  unfold IncLedger.aroundV
  have hV : (toLF l).vertSectors (FCell.vert v) = (secRowL l.vert_sectors v).map toSec := rfl
  rw [hV, hrow, List.map_map, List.filter_map, avL]
  congr 1
  apply List.filter_congr
  intro k hk
  have hmem : secAt l k ∈ secRowL l.vert_sectors v := by rw [hrow]; exact List.mem_map.mpr ⟨k, hk, rfl⟩
  have hfl := (hw.sectors v hv _ hmem).1
  simp only [Function.comp, toSec]
  rw [rpA_ne l hw s _ a h hfl, result_eq l hw s hs rule, Bool.eq_iff_iff, List.contains_iff_mem, List.mem_map]
  constructor
  · rintro ⟨g, hg, he⟩; cases he; exact (mem_resL l s _ hfl).mp hg
  · intro hi; exact ⟨_, (mem_resL l s _ hfl).mpr hi, rfl⟩

theorem aroundV_nodup (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) {v : Nat} (hv : v < nrowsL l.vert_edges) :
    ((avL l a (sstAt l v) (sstAt l (v + 1))).map (fun k => toSec (secAt l k))).Nodup := by
  have hsr := hw.sectors_rows
  have hvl : v + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hlen := sec_le_len l hw hvl
  have hnd := hw.sectors_nodup v hv
  rw [secRowL_eq l hlen] at hnd
  have e : (avL l a (sstAt l v) (sstAt l (v + 1))).map (fun k => toSec (secAt l k)) =
      (((List.range' (sstAt l v) (sstAt l (v + 1) - sstAt l v)).map (secAt l)).filter
        (fun sc => rpA a sc.facet.val != U32.max)).map toSec := by
    rw [List.filter_map, List.map_map, avL]; rfl
  rw [e]
  exact (hnd.filter _).map (fun _ _ h => toSec_inj h)

/-- **`bad_vert` is the Lean's `badVert`.** -/
theorem bad_vert_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (a : Assembled) (h : AInv l s (keepVec l s rule) a.result.val a.rpos.val a.orient.val (nfacets l))
    (v : Usize) (hv : v.val < nrowsL l.vert_edges) :
    bad_vert l a v ⦃ b => b = (toLF l).badVert ((toLF l).keepTab (ruleL l.nb.val rule)) (FCell.vert v.val) ⦄ := by
  have hsm := hw.small
  have hsl : l.vert_sectors.sec.val.length < U32.max := hsm.2.2.2.2.2.2.2.2
  have hsr := hw.sectors_rows
  have hvl : v.val + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hlen := sec_le_len l hw hvl
  have hrl : nfacets l ≤ a.rpos.val.length := by rw [h.rpos_len]
  unfold bad_vert
  step with around_v_spec l hw a hrl v hv as ⟨ar, har⟩
  have hidx : ∀ x ∈ ar.val, x.val < l.vert_sectors.sec.val.length := by
    intro x hx
    have : x.val ∈ avL l a (sstAt l v.val) (sstAt l (v.val + 1)) := by
      rw [← har]; exact List.mem_map.mpr ⟨x, hx, rfl⟩
    have := mem_avL this; omega
  have harl : ar.val.length ≤ U32.max := by
    have := congrArg List.length har
    simp only [List.length_map, avL] at this
    have h2 := List.length_filter_le (fun j => rpA a (secAt l j).facet.val != U32.max)
      (List.range' (sstAt l v.val) (sstAt l (v.val + 1) - sstAt l v.val))
    simp only [List.length_range'] at h2
    omega
  step with fans_spec l (alloc.vec.Vec.deref ar) (by simpa [alloc.vec.Vec.deref] using harl)
    (by simpa [alloc.vec.Vec.deref] using hidx) as ⟨uf, hu⟩
  have hu' : UInv (fanAll (relA l (ar.val.map (·.val))) ar.val.length) ar.val.length uf := by
    simpa [alloc.vec.Vec.deref] using hu
  step with bad_vert_loop_spec ar _ uf hu' as ⟨b, hb⟩
  rw [hb]
  unfold IncLedger.badVert
  rw [aroundV_eq l hw s hs rule a h hv]
  set L := (avL l a (sstAt l v.val) (sstAt l (v.val + 1))).map (fun k => toSec (secAt l k)) with hL
  have hLl : L.length = ar.val.length := by
    rw [hL, List.length_map, ← har, List.length_map]
  have hnd := aroundV_nodup l hw a hv
  rw [← hL] at hnd
  have hrel : ∀ x y (hx : x < L.length) (hy : y < L.length),
      relA l (ar.val.map (·.val)) x y = IncLedger.fanRel L[x] L[y] := by
    intro x y hx hy
    simp only [hL, List.getElem_map, fanRel_toSec, relA]
    rw [har, List.getD_eq_getElem?_getD, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by rw [hL, List.length_map] at hx; exact hx),
      List.getElem?_eq_getElem (by rw [hL, List.length_map] at hy; exact hy)]
    rfl
  have hl : Least (fanAll (relA l (ar.val.map (·.val))) L.length) L.length (ufRoot (vals uf.parent)) := by
    rw [hLl]; exact hu'.2.2
  rw [← hLl]
  have key := (fan_bad fanRel_symm hnd hrel hl).symm
  clear_value L
  cases L with
  | nil => simp
  | cons c t =>
    simp only at key ⊢
    convert key using 4

/-! ### The vert scan -/

def bVL (l : Ledger) (rule : Rule) (v : Nat) : Bool :=
  (toLF l).badVert ((toLF l).keepTab (ruleL l.nb.val rule)) (FCell.vert v)

/-- What the scan has found over the first `k` verts. -/
def VScan (l : Ledger) (rule : Rule) (bad0 : Outcome) (k : Nat) (bad : Outcome) (found : Bool) : Prop :=
  match (List.range k).find? (bVL l rule) with
  | some v => found = true ∧ ∃ x : U32, x.val = v ∧ bad = .Refused (.ZeroThicknessVert x)
  | none => found = false ∧ bad = bad0

theorem core_k_loop2_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (a : Assembled) (h : AInv l s (keepVec l s rule) a.result.val a.rpos.val a.orient.val (nfacets l))
    (nv : Usize) (hnv : nv.val = nrowsL l.vert_edges) (bad0 bad : Outcome) (found : Bool) (v : Usize)
    (hv : v.val ≤ nv.val) (hI : VScan l rule bad0 v.val bad found) :
    core_k_loop2 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient nv bad found v ⦃ r => VScan l rule bad0 (nrowsL l.vert_edges) r.1 r.2 ⦄ := by
  have hsm := hw.small
  have hnvr : nrowsL l.vert_edges < U32.max := hsm.2.2.2.2.2.2.2.1
  unfold core_k_loop2
  apply loop.spec_decr_nat (fun (q : Outcome × Bool × Usize) => nv.val - q.2.2.val)
    (fun q => q.2.2.val ≤ nv.val ∧ VScan l rule bad0 q.2.2.val q.1 q.2.1)
  · rintro ⟨b, fd, j⟩ ⟨hj, hE⟩
    unfold core_k_loop2.body
    simp only at hj hE ⊢
    split
    · rename_i hfd
      simp only [WP.spec_ok]
      unfold VScan at hE ⊢
      split at hE
      · rename_i e0 he0
        rw [find_range_mono (by omega) he0]; exact hE
      · exfalso; rw [hE.1] at hfd; simp at hfd
    · rename_i hfd
      have hfd' : fd = false := by simpa using hfd
      have hnone : (List.range j.val).find? (bVL l rule) = none := by
        unfold VScan at hE; split at hE
        · exfalso; rw [hE.1] at hfd'; simp at hfd'
        · assumption
      have hb0 : b = bad0 := by unfold VScan at hE; rw [hnone] at hE; exact hE.2
      clear hfd hE
      split
      · rename_i hlt
        have hjl : j.val < nrowsL l.vert_edges := by scalar_tac
        step with bad_vert_spec l hw s hs rule a h j hjl as ⟨bb, hbb⟩
        split
        · rename_i hgt
          step as ⟨i2, hi2⟩
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          unfold VScan
          have hb : bVL l rule j.val = true := by rw [bVL, ← hbb]; exact hgt
          rw [hj1v, find_range_succ, hnone, if_pos hb]
          exact ⟨rfl, i2, by rw [hi2]; scalar_tac, rfl⟩
        · rename_i hgt
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          unfold VScan
          have hb : bVL l rule j.val = false := by rw [bVL, ← hbb]; simpa using hgt
          rw [hj1v, find_range_succ, hnone, hb]
          exact ⟨hfd', hb0⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nrowsL l.vert_edges := by scalar_tac
        rw [← this]; unfold VScan; rw [hnone]; exact ⟨hfd', hb0⟩
  · exact ⟨hv, hI⟩

/-- Once something is found, the vert scan returns it. -/
theorem core_k_loop2_found (l : Ledger) (a : Assembled) (nv : Usize) (bad : Outcome) (v : Usize) :
    core_k_loop2 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient nv bad true v ⦃ r => r = (bad, true) ⦄ := by
  unfold core_k_loop2
  apply loop.spec_decr_nat (fun (_ : Outcome × Bool × Usize) => 0)
    (fun q => q.1 = bad ∧ q.2.1 = true)
  · rintro ⟨b, fd, j⟩ ⟨hb, hfd⟩
    simp only at hb hfd
    subst hb hfd
    unfold core_k_loop2.body
    simp
  · exact ⟨rfl, rfl⟩

theorem find_range_verts (l : Ledger) (p : FCell → Bool) :
    (toLF l).verts.find? p = ((List.range (nrowsL l.vert_edges)).find? (p ∘ FCell.vert)).map FCell.vert := by
  simp [toLF, List.find?_map]

end inc

end forge_bool_core
