import ForgeBoolCore.RefineBetween

/-!
# `bad_vert`: the fans round a vertex

`bad_vert` unites two facets round `v` (by their index in `around v`) when they share an edge cell
between `v` and both; each edge remembers the first facet that had it (`seen`). The classes it ends
with are the closure of "share an edge" (`fan_classes`), so it answers whether some facet is out of
the first one's fan (`bad_vert_spec`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- The lookup in `seen`: the facet the edge `es[q]` was first seen on, or `NONE`. -/
theorem seen_find_spec (seen : alloc.vec.Vec (U32 × U32)) (es : alloc.vec.Vec U32) (q : Usize)
    (hq : q.val < es.val.length) (hsn : ∀ p ∈ seen.val, p.2 ≠ NONE) (found : U32) (r : Usize)
    (hr : r.val ≤ seen.val.length)
    (hf1 : found = NONE → ∀ p ∈ seen.val.take r.val, p.1 ≠ es.val[q.val])
    (hf2 : found ≠ NONE → (es.val[q.val], found) ∈ seen.val) :
    bad_vert_loop0_loop0_loop0 seen es q found r ⦃ f =>
      (f = NONE → ∀ p ∈ seen.val, p.1 ≠ es.val[q.val]) ∧ (f ≠ NONE → (es.val[q.val], f) ∈ seen.val) ⦄ := by
  unfold bad_vert_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (st : U32 × Usize) => seen.val.length - st.2.val)
    (fun st => st.2.val ≤ seen.val.length ∧
      (st.1 = NONE → ∀ p ∈ seen.val.take st.2.val, p.1 ≠ es.val[q.val]) ∧
      (st.1 ≠ NONE → (es.val[q.val], st.1) ∈ seen.val))
  · rintro ⟨f0, i⟩ ⟨hi, h1, h2⟩
    unfold bad_vert_loop0_loop0_loop0.body
    simp only at hi h1 h2 ⊢
    split
    · rename_i hnone
      split
      · rename_i hlt
        have hil : i.val < seen.val.length := by scalar_tac
        have ht : seen.val.take (i.val + 1) = seen.val.take i.val ++ [seen.val[i.val]] :=
          List.take_succ_eq_append_getElem hil
        step as ⟨k1, k2, hpr⟩
        step as ⟨x, hx⟩
        split
        · rename_i heq
          step as ⟨r1, hr1⟩
          have hmem : seen.val[i.val] ∈ seen.val := List.getElem_mem _
          have hk2 : k2 ≠ NONE := by
            have := hsn _ hmem; rw [← hpr] at this; exact this
          refine ⟨by scalar_tac, fun e => absurd e hk2, fun _ => ?_, by scalar_tac⟩
          have : (es.val[q.val], k2) = seen.val[i.val] := by rw [← hpr, heq, hx]
          rw [this]; exact hmem
        · rename_i hne
          step as ⟨r1, hr1⟩
          refine ⟨by scalar_tac, fun _ => ?_, fun e => absurd hnone e, by scalar_tac⟩
          rw [hr1, ht]
          intro p hp
          simp only [List.mem_append, List.mem_singleton] at hp
          rcases hp with hp | hp
          · exact h1 hnone p hp
          · rw [hp, ← hpr]; simp only; rw [← hx]; exact hne
      · rename_i hge
        simp only [WP.spec_ok]
        have : i.val = seen.val.length := by scalar_tac
        refine ⟨fun e => ?_, h2⟩
        have := h1 e; rwa [List.take_of_length_le (by omega)] at this
    · rename_i hsome
      simp only [WP.spec_ok]
      exact ⟨fun e => absurd e hsome, h2⟩
  · exact ⟨hr, hf1, hf2⟩

/-- `seen` as numbers. -/
def sv (S : List (U32 × U32)) : List (Nat × Nat) := S.map fun p => (p.1.val, p.2.val)

/-- What `bad_vert`'s loops keep, for facets `0 .. j-1` done and the edges `D` of facet `j` done. -/
structure FanInv (eb : Nat → Nat → Bool) (n : Nat) (j : Nat) (D : List Nat) (S : List (U32 × U32))
    (E : List (Nat × Nat)) (uf : union_find.Uf) : Prop where
  keys : ((sv S).map (·.1)).Nodup
  owner : ∀ p ∈ sv S, p.2 ≤ j ∧ p.2 < n ∧ eb p.2 p.1 = true
  done : ∀ j', j' < j → ∀ e, eb j' e = true → ∃ j0, (e, j0) ∈ sv S ∧ Eqv E j0 j'
  cur : ∀ e ∈ D, ∃ j0, (e, j0) ∈ sv S ∧ Eqv E j0 j
  pairs : ∀ p ∈ E, p.1 < n ∧ p.2 < n ∧ ∃ e, eb p.1 e = true ∧ eb p.2 e = true
  uf : UInv E n uf

theorem mem_sv {S : List (U32 × U32)} {p : U32 × U32} (h : p ∈ S) : (p.1.val, p.2.val) ∈ sv S :=
  List.mem_map_of_mem h

theorem sv_length (S : List (U32 × U32)) : (sv S).length = S.length := by simp [sv]

theorem fan_inner_spec (eb : Nat → Nat → Bool) (n N : Nat) (hn : n < U32.max) (hN : N < U32.max)
    (heN : ∀ j e, eb j e = true → e < N)
    (j : Usize) (hj : j.val < n) (es : alloc.vec.Vec U32) (hes : ∀ e ∈ es.val, eb j.val e.val = true)
    (uf : union_find.Uf) (S : alloc.vec.Vec (U32 × U32)) (E : List (Nat × Nat)) (q : Usize)
    (hq : q.val ≤ es.val.length) (hinv : FanInv eb n j.val ((es.val.take q.val).map (·.val)) S.val E uf) :
    bad_vert_loop0_loop0 uf S j es q ⦃ r => ∃ E', (∀ p ∈ E, p ∈ E') ∧
      FanInv eb n j.val (es.val.map (·.val)) r.2.val E' r.1 ⦄ := by
  unfold bad_vert_loop0_loop0
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × alloc.vec.Vec (U32 × U32) × Usize) => es.val.length - st.2.2.val)
    (fun st => st.2.2.val ≤ es.val.length ∧ ∃ E', (∀ p ∈ E, p ∈ E') ∧
      FanInv eb n j.val ((es.val.take st.2.2.val).map (·.val)) st.2.1.val E' st.1)
  · rintro ⟨u0, S0, i⟩ ⟨hi, E0, hE0, hI⟩
    unfold bad_vert_loop0_loop0.body
    simp only at hi hE0 hI ⊢
    split
    · rename_i hlt
      have hil : i.val < es.val.length := by scalar_tac
      have ht : (es.val.take (i.val + 1)).map (·.val) = (es.val.take i.val).map (·.val) ++ [(es.val[i.val]).val] := by
        rw [List.take_succ_eq_append_getElem hil, List.map_append, List.map_singleton]
      have hsn : ∀ p ∈ S0.val, p.2 ≠ NONE := by
        intro p hp e
        have := (hI.owner _ (mem_sv hp)).2.1
        rw [e, none_val] at this; omega
      have hei := hes _ (List.getElem_mem hil)
      step with seen_find_spec S0 es i hil hsn NONE 0#usize (by simp) (by simp) (fun h => absurd rfl h) as ⟨f, hf1, hf2⟩
      split
      · rename_i hfn
        have hfn' := hf1 hfn
        step as ⟨x, hx⟩
        step as ⟨c, hc⟩
        have hcv : c.val = j.val := by rw [hc]; scalar_tac
        have hkeysN : ((sv S0.val).map (·.1)) ⊆ List.range N := by
          intro e he
          simp only [List.mem_map] at he
          obtain ⟨p, hp, rfl⟩ := he
          simp only [List.mem_range]; exact heN _ _ (hI.owner p hp).2.2
        have hSl : S0.val.length ≤ N := by
          have := List.Nodup.length_le_of_subset hI.keys hkeysN
          simp [sv] at this; exact this
        step as ⟨S1, hS1⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ⟨E0, hE0, ?_⟩, by scalar_tac⟩
        have hsv : sv S1.val = sv S0.val ++ [(x.val, j.val)] := by rw [hS1]; simp [sv, hcv]
        have hxn : x.val ∉ (sv S0.val).map (·.1) := by
          intro h; simp only [sv, List.map_map, List.mem_map, Function.comp] at h
          obtain ⟨p, hp, e⟩ := h
          exact hfn' p hp (by rw [← hx]; exact UScalar.eq_of_val_eq e)
        have hmono : ∀ p ∈ sv S0.val, p ∈ sv S1.val := fun p hp => by rw [hsv]; exact List.mem_append_left _ hp
        refine ⟨?_, ?_, ?_, ?_, hI.pairs, hI.uf⟩
        · rw [hsv, List.map_append]
          exact List.nodup_append.mpr ⟨hI.keys, by simp, fun a ha b hb => by simp at hb; rintro rfl; exact hxn (hb ▸ ha)⟩
        · intro p hp
          rw [hsv] at hp
          rcases List.mem_append.mp hp with hp | hp
          · exact hI.owner p hp
          · simp at hp; rw [hp]; exact ⟨le_refl _, hj, by rw [hx]; exact hei⟩
        · intro j' hj' e he
          obtain ⟨j0, h1, h2⟩ := hI.done j' hj' e he
          exact ⟨j0, hmono _ h1, h2⟩
        · intro e he
          rw [hi1, ht] at he
          rcases List.mem_append.mp he with he | he
          · obtain ⟨j0, h1, h2⟩ := hI.cur e he
            exact ⟨j0, hmono _ h1, h2⟩
          · simp at he
            refine ⟨j.val, ?_, Eqv.refl _⟩
            rw [hsv, he, ← hx]; simp
      · rename_i hfs
        have hmem := hf2 hfs
        have how := hI.owner _ (mem_sv hmem)
        simp only at how
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = f.val := by rw [hi1]; scalar_tac
        have hl := uinv_len hI.uf
        step with union_spec u0 hI.uf (fun e he => ⟨(hI.pairs e he).1, (hI.pairs e he).2.1⟩) (by omega) i1 j
          (by omega) hj as ⟨u1, hu1⟩
        step as ⟨i2, hi2⟩
        refine ⟨by scalar_tac, ⟨E0 ++ [(i1.val, j.val)], fun p hp => List.mem_append_left _ (hE0 p hp), ?_⟩, by scalar_tac⟩
        have hmono : ∀ {a b}, Eqv E0 a b → Eqv (E0 ++ [(i1.val, j.val)]) a b :=
          fun h => Eqv.mono (fun e he => List.mem_append_left _ he) h
        refine ⟨hI.keys, fun p hp => hI.owner p hp, ?_, ?_, ?_, hu1⟩
        · intro j' hj' e he
          obtain ⟨j0, h1, h2⟩ := hI.done j' hj' e he
          exact ⟨j0, h1, hmono h2⟩
        · intro e he
          rw [hi2, ht] at he
          rcases List.mem_append.mp he with he | he
          · obtain ⟨j0, h1, h2⟩ := hI.cur e he
            exact ⟨j0, h1, hmono h2⟩
          · simp at he
            refine ⟨f.val, by rw [he]; exact mem_sv hmem, ?_⟩
            rw [← hi1v]
            exact Eqv.edge (e := (i1.val, j.val)) (by simp)
        · intro p hp
          rcases List.mem_append.mp hp with hp | hp
          · exact hI.pairs p hp
          · simp at hp; rw [hp]
            exact ⟨by omega, hj, es.val[i.val].val, by rw [hi1v]; exact how.2.2, hei⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = es.val.length := by scalar_tac
      rw [this, List.take_length] at hI
      exact ⟨E0, hE0, hI⟩
  · exact ⟨hq, E, fun p hp => hp, hinv⟩

/-- Edge cell `e` lies between `vc` and `cc` (`EdgeBetween` as a test). -/
def ebB (l : Ledger) (ext : Ext) (isE : List Bool) (vc cc : List Sgn) (e : Nat) : Bool :=
  decide (e < isE.length) && isE.getD e false &&
    Cov.le (vc.map toS) ((covAt l ext e).map toS) && Cov.le ((covAt l ext e).map toS) (cc.map toS)

theorem ebB_iff (l : Ledger) (ext : Ext) (isE : List Bool) (vc cc : List Sgn) (e : Nat) :
    ebB l ext isE vc cc e = true ↔ EdgeBetween l ext isE vc cc e := by
  simp only [ebB, Bool.and_eq_true, decide_eq_true_eq, leS_iff, EdgeBetween]; tauto

/-- The facet at index `j` of `around v`. -/
def csAt (l : Ledger) (ext : Ext) (a : Assembled) (ar : List Nat) (j : Nat) : List Sgn :=
  covAt l ext (resAt a (ar.getD j 0))

theorem fan_outer_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (isE : Slice Bool) (hN : isE.val.length = (allCells l ext).length) (v : U32) (hv : v.val < (allCells l ext).length)
    (ar : Slice U32) (har : ar.val.map (·.val) = aroundL l ext a (covAt l ext v.val))
    (z : alloc.vec.Vec Usize) (hz : z.val.map (·.val) = zerosL (covAt l ext v.val))
    (uf : union_find.Uf) (S : alloc.vec.Vec (U32 × U32)) (E : List (Nat × Nat)) (j : Usize)
    (hj : j.val ≤ ar.val.length)
    (hinv : FanInv (fun j e => ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e)
      ar.val.length j.val [] S.val E uf) :
    bad_vert_loop0 l ext a isE v ar z uf S j ⦃ r => ∃ E' S',
      FanInv (fun j e => ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e)
        ar.val.length ar.val.length [] S' E' r ⦄ := by
  have hsmall := hok.small
  have hrs := h.small
  have harl : ar.val.length ≤ a.result.val.length := by
    have := congrArg List.length har; simp only [List.length_map, aroundL] at this; rw [this]
    exact (List.length_filter_le _ _).trans (by simp)
  have harlt : ∀ x ∈ ar.val, x.val < a.result.val.length := by
    intro x hx
    have : x.val ∈ aroundL l ext a (covAt l ext v.val) := by rw [← har]; exact List.mem_map_of_mem hx
    simp only [aroundL, List.mem_filter, List.mem_range] at this; exact this.1
  have harle : ∀ x ∈ ar.val, LeS (covAt l ext v.val) (covAt l ext (resAt a x.val)) := by
    intro x hx
    have : x.val ∈ aroundL l ext a (covAt l ext v.val) := by rw [← har]; exact List.mem_map_of_mem hx
    simp only [aroundL, List.mem_filter, List.mem_range] at this; exact (leS_iff _ _).mp this.2
  unfold bad_vert_loop0
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × alloc.vec.Vec (U32 × U32) × Usize) => ar.val.length - st.2.2.val)
    (fun st => st.2.2.val ≤ ar.val.length ∧ ∃ E',
      FanInv (fun j e => ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e)
        ar.val.length st.2.2.val [] st.2.1.val E' st.1)
  · rintro ⟨u0, S0, i⟩ ⟨hi, E0, hI⟩
    unfold bad_vert_loop0.body
    simp only at hi hI ⊢
    split
    · rename_i hlt
      have hil : i.val < ar.val.length := by scalar_tac
      have hx := harlt _ (List.getElem_mem hil)
      step as ⟨x, hxv⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = x.val := by rw [hi2]; scalar_tac
      step as ⟨phi, hphi⟩
      have hphiv : phi.val = resAt a x.val := by
        unfold resAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp; rw [hxv]; omega)]
        simp [hphi, hi2v]
      have hcs : csAt l ext a (ar.val.map (·.val)) i.val = covAt l ext phi.val := by
        unfold csAt; rw [hphiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp; omega)]; simp [hxv]
      have hphil : phi.val < (allCells l ext).length := by rw [hphiv]; exact resAt_lt h (by rw [hxv]; omega)
      have hfac : (covAt l ext phi.val).count .Zero = 1 := by rw [hphiv]; exact resAt_facet h (by rw [hxv]; omega)
      have hle : LeS (covAt l ext v.val) (covAt l ext phi.val) := by
        rw [hphiv, hxv]; exact harle _ (List.getElem_mem hil)
      step with edges_between_spec l ext hok isE v phi (alloc.vec.Vec.deref z) hN hv hphil
        (by simpa [alloc.vec.Vec.deref] using hz) hfac hle as ⟨es, hes⟩
      have hes' : ∀ e ∈ es.val, ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) i.val) e.val = true := by
        intro e he; rw [hcs, ebB_iff]; exact (hes e.val).mp (List.mem_map_of_mem he)
      have hI0 : FanInv (fun j e => ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e)
          ar.val.length i.val ((es.val.take 0).map (·.val)) S0.val E0 u0 := by
        simpa using hI
      step with fan_inner_spec _ ar.val.length (allCells l ext).length (by omega) hsmall
        (fun j e he => by rw [ebB_iff] at he; rw [← hN]; exact he.1) i hil es hes' u0 S0 E0 0#usize (by simp) hI0
        as ⟨u1, S1, E1, hE1, hI1⟩
      step as ⟨i1, hi1⟩
      refine ⟨by scalar_tac, ⟨E1, ?_⟩, by scalar_tac⟩
      rw [hi1]
      refine ⟨hI1.keys, fun p hp => ⟨by have := (hI1.owner p hp).1; omega, (hI1.owner p hp).2⟩, ?_, by simp,
        hI1.pairs, hI1.uf⟩
      intro j' hj' e he
      by_cases hjj : j' < i.val
      · exact hI1.done j' hjj e he
      · have : j' = i.val := by omega
        subst this
        apply hI1.cur
        have : EdgeBetween l ext isE.val (covAt l ext v.val) (covAt l ext phi.val) e := by
          rw [← ebB_iff, ← hcs]; exact he
        exact (hes e).mpr this
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = ar.val.length := by scalar_tac
      rw [this] at hI
      exact ⟨E0, S0.val, hI⟩
  · exact ⟨hj, E, hinv⟩

/-- Two facets round `v` sharing an edge cell (below `N`). -/
def shareR (eb : Nat → Nat → Bool) (N : Nat) (a b : Nat) : Bool := (List.range N).any fun e => eb a e && eb b e

theorem shareR_symm (eb : Nat → Nat → Bool) (N : Nat) (a b : Nat) : shareR eb N a b = shareR eb N b a := by
  unfold shareR; congr 1; funext e; rw [Bool.and_comm]

theorem key_unique : ∀ {L : List (Nat × Nat)} {e a b : Nat}, (L.map (·.1)).Nodup → (e, a) ∈ L → (e, b) ∈ L → a = b
  | [], _, _, _, _, h, _ => by simp at h
  | p :: t, e, a, b, hn, ha, hb => by
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hn
    simp only [List.mem_cons] at ha hb
    rcases ha with ha | ha <;> rcases hb with hb | hb
    · rw [← ha] at hb; simp at hb; exact hb.symm
    · exact absurd ⟨(e, b), hb, by rw [← ha]⟩ hn.1
    · exact absurd ⟨(e, a), ha, by rw [← hb]⟩ hn.1
    · exact key_unique hn.2 ha hb

/-- **The union–find's classes are the closure of "share an edge".** -/
theorem fan_classes {eb : Nat → Nat → Bool} {n N : Nat} (heN : ∀ j e, eb j e = true → e < N)
    {S : List (U32 × U32)} {E : List (Nat × Nat)} {uf : union_find.Uf} (hI : FanInv eb n n [] S E uf)
    {a b : Nat} (ha : a < n) (hb : b < n) :
    Eqv E a b ↔ Conn (shareR eb N) (List.range n) a b := by
  have hE : ∀ p ∈ E, p.1 < n ∧ p.2 < n := fun p hp => ⟨(hI.pairs p hp).1, (hI.pairs p hp).2.1⟩
  constructor
  · intro hab
    suffices H : ∀ y z, Eqv E y z → y < n → Conn (shareR eb N) (List.range n) y z from H a b hab ha
    intro y z hyz
    induction hyz with
    | refl _ => exact fun _ => Conn.refl
    | @edge p hp =>
      intro _
      obtain ⟨h1, h2, e, he1, he2⟩ := hI.pairs p hp
      refine Conn.step Conn.refl (List.mem_range.mpr h2) ?_
      unfold shareR; rw [List.any_eq_true]
      exact ⟨e, List.mem_range.mpr (heN _ _ he1), by simp [he1, he2]⟩
    | @symm y z h ih =>
      intro hz
      rcases Eqv.range hE h with e | ⟨hy, _⟩
      · subst e; exact Conn.refl
      · exact Conn.symm (shareR_symm eb N) (List.mem_range.mpr hy) (ih hy)
    | @trans x y z h1 h2 ih1 ih2 =>
      intro hx
      have c1 := ih1 hx
      have hy : y < n := List.mem_range.mp (Conn.mem (List.mem_range.mpr hx) c1)
      exact c1.trans (ih2 hy)
  · intro hab
    clear hb
    induction hab with
    | refl => exact Eqv.refl _
    | @step c d hac hd hcd ih =>
      unfold shareR at hcd; rw [List.any_eq_true] at hcd
      obtain ⟨e, _, he⟩ := hcd
      simp only [Bool.and_eq_true] at he
      have hc : c < n := List.mem_range.mp (Conn.mem (List.mem_range.mpr ha) hac)
      obtain ⟨j0, h0, e0⟩ := hI.done c hc e he.1
      obtain ⟨j1, h1, e1⟩ := hI.done d (List.mem_range.mp hd) e he.2
      have := key_unique hI.keys h0 h1
      subst this
      exact ih.trans (e0.symm.trans e1)

theorem fan_check_spec (ar : Slice U32) (uf : union_find.Uf) (R : Nat → Nat)
    (hl : uf.parent.val.length = ar.val.length) (hg : Good (vals uf.parent))
    (hR : ∀ y, ufRoot (vals uf.parent) y = R y) (bad : Bool) (j : Usize) (hj1 : 1 ≤ j.val)
    (hj : j.val ≤ max ar.val.length 1)
    (hb1 : bad = true → ∃ k, 1 ≤ k ∧ k < j.val ∧ k < ar.val.length ∧ R k ≠ R 0)
    (hb2 : bad = false → ∀ k, 1 ≤ k → k < j.val → R k = R 0) :
    bad_vert_loop1 ar uf bad j ⦃ r => r = true ↔ ∃ k, 1 ≤ k ∧ k < ar.val.length ∧ R k ≠ R 0 ⦄ := by
  unfold bad_vert_loop1
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × Bool × Usize) => ar.val.length + 1 - st.2.2.val)
    (fun st => st.1.parent.val.length = ar.val.length ∧ Good (vals st.1.parent) ∧
      (∀ y, ufRoot (vals st.1.parent) y = R y) ∧ 1 ≤ st.2.2.val ∧ st.2.2.val ≤ max ar.val.length 1 ∧
      (st.2.1 = true → ∃ k, 1 ≤ k ∧ k < st.2.2.val ∧ k < ar.val.length ∧ R k ≠ R 0) ∧
      (st.2.1 = false → ∀ k, 1 ≤ k → k < st.2.2.val → R k = R 0))
  · rintro ⟨u0, b0, i⟩ ⟨hl0, hg0, hR0, hi1, hi, h1, h2⟩
    unfold bad_vert_loop1.body
    simp only at hl0 hg0 hR0 hi1 hi h1 h2 ⊢
    split
    · rename_i hbt
      simp only [WP.spec_ok]
      obtain ⟨k, hk1, _, hk3, hk4⟩ := h1 hbt
      exact ⟨fun _ => ⟨k, hk1, hk3, hk4⟩, fun _ => hbt⟩
    · rename_i hbf
      have hbf' : b0 = false := by simpa using hbf
      split
      · rename_i hlt
        have hil : i.val < ar.val.length := by scalar_tac
        step with find_spec u0 i (by omega) hg0 as ⟨r1, u1, hr1, hl1, hg1, hroot1⟩
        step with find_spec u1 0#usize (by simp; omega) hg1 as ⟨r2, u2, hr2, hl2, hg2, hroot2⟩
        step as ⟨i1, hi1'⟩
        have hr1v : r1.val = R i.val := by rw [hr1, hR0]
        have hr2v : r2.val = R 0 := by rw [hr2, hroot1, hR0]
        refine ⟨by omega, hg2, fun y => by rw [hroot2, hroot1, hR0], by scalar_tac, by scalar_tac, ?_, ?_, by scalar_tac⟩
        · intro hne
          refine ⟨i.val, hi1, by scalar_tac, hil, ?_⟩
          rw [← hr1v, ← hr2v]; intro e; apply (by simpa using hne : r1 ≠ r2); exact UScalar.eq_of_val_eq e
        · intro heq k hk1 hk2
          have heq' : r1 = r2 := by simpa using heq
          by_cases hk : k < i.val
          · exact h2 hbf' k hk1 hk
          · have : k = i.val := by scalar_tac
            rw [this, ← hr1v, ← hr2v, heq']
      · rename_i hge
        simp only [WP.spec_ok]
        have hge' : ar.val.length ≤ i.val := by scalar_tac
        constructor
        · intro e; rw [e] at hbf'; cases hbf'
        · rintro ⟨k, hk1, hk2, hk3⟩
          exact absurd (h2 hbf' k hk1 (by omega)) hk3
  · exact ⟨hl, hg, hR, hj1, hj, hb1, hb2⟩

/-- The fan relation on the indices of `around v`. -/
def fanR (l : Ledger) (ext : Ext) (a : Assembled) (isE : List Bool) (vc : List Sgn) (ar : List Nat) : Nat → Nat → Bool :=
  shareR (fun j e => ebB l ext isE vc (csAt l ext a ar j) e) isE.length

/-- **`bad_vert` answers whether some facet round `v` is out of the first one's fan.** -/
theorem bad_vert_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (isE : Slice Bool) (hN : isE.val.length = (allCells l ext).length) (v : U32) (hv : v.val < (allCells l ext).length)
    (ar : Slice U32) (har : ar.val.map (·.val) = aroundL l ext a (covAt l ext v.val)) :
    bad_vert l ext a isE v ar ⦃ r => r = true ↔ ∃ k, 1 ≤ k ∧ k < ar.val.length ∧
      ¬ Conn (fanR l ext a isE.val (covAt l ext v.val) (ar.val.map (·.val))) (List.range ar.val.length) 0 k ⦄ := by
  have hsmall := hok.small
  have harl : ar.val.length ≤ a.result.val.length := by
    have := congrArg List.length har; simp only [List.length_map, aroundL] at this; rw [this]
    exact (List.length_filter_le _ _).trans (by simp)
  have hrs := h.small
  unfold bad_vert
  step with cov_spec l ext v hv as ⟨s, hs⟩
  step with zeros_spec s as ⟨z, hz⟩
  rw [hs] at hz
  step with new_spec (Slice.len ar) (by simp; omega) as ⟨uf, huf⟩
  have hinv0 : FanInv (fun j e => ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e)
      ar.val.length 0 [] (alloc.vec.Vec.new (U32 × U32)).val [] uf := by
    refine ⟨by simp [sv], fun p hp => by simp [sv] at hp, fun j' hj' => absurd hj' (Nat.not_lt_zero _),
      fun e he => by simp at he, fun p hp => by simp at hp, ?_⟩
    unfold UInv; rw [huf]; simpa using inv_range ar.val.length
  step with fan_outer_spec l ext hok a W h isE hN v hv ar har z hz uf _ [] 0#usize (by simp) hinv0
    as ⟨uf1, E1, S1, hI1⟩
  have hU := hI1.uf
  obtain ⟨hg, hlen, hleast⟩ := hU
  have heN : ∀ j e, ebB l ext isE.val (covAt l ext v.val) (csAt l ext a (ar.val.map (·.val)) j) e = true →
      e < isE.val.length := fun j e he => ((ebB_iff _ _ _ _ _ _).mp he).1
  apply WP.spec_mono (fan_check_spec ar uf1 (ufRoot (vals uf1.parent)) (by rw [← vals_length, hlen])
    hg (fun _ => rfl) false 1#usize (by simp) (by simp) (fun e => by cases e) (fun _ k h1 h2 => by simp at h2; omega))
  intro r hr
  rw [hr]
  constructor
  · rintro ⟨k, hk1, hk2, hk3⟩
    refine ⟨k, hk1, hk2, fun hc => hk3 ?_⟩
    have := (fan_classes heN hI1 (by omega) hk2).mpr hc
    exact ((least_iff hleast (by omega) hk2).mp this).symm
  · rintro ⟨k, hk1, hk2, hk3⟩
    refine ⟨k, hk1, hk2, fun he => hk3 ?_⟩
    exact (fan_classes heN hI1 (by omega) hk2).mp ((least_iff hleast (by omega) hk2).mpr he.symm)

/-- A vertex (by id) whose round facets form more than one fan: `bad_vert`'s answer, decided. -/
def badV (l : Ledger) (ext : Ext) (a : Assembled) (isE : List Bool) (v : Nat) : Bool :=
  let ar := aroundL l ext a (covAt l ext v)
  (List.range ar.length).any fun k => !(cls (fanR l ext a isE (covAt l ext v) ar) (List.range ar.length) 0).contains k

theorem badV_iff (l : Ledger) (ext : Ext) (a : Assembled) (isE : List Bool) (v : Nat) :
    badV l ext a isE v = true ↔ ∃ k, 1 ≤ k ∧ k < (aroundL l ext a (covAt l ext v)).length ∧
      ¬ Conn (fanR l ext a isE (covAt l ext v) (aroundL l ext a (covAt l ext v)))
        (List.range (aroundL l ext a (covAt l ext v)).length) 0 k := by
  unfold badV
  simp only [List.any_eq_true, List.mem_range, Bool.not_eq_true', List.contains_eq_mem, decide_eq_false_iff_not]
  constructor
  · rintro ⟨k, hk, hn⟩
    have h0 : 0 < (aroundL l ext a (covAt l ext v)).length := by omega
    rw [cls_spec _ List.nodup_range (List.mem_range.mpr h0)] at hn
    refine ⟨k, ?_, hk, hn⟩
    by_contra h1; have : k = 0 := by omega
    subst this; exact hn Conn.refl
  · rintro ⟨k, _, hk, hn⟩
    refine ⟨k, hk, ?_⟩
    rw [cls_spec _ List.nodup_range (List.mem_range.mpr (by omega))]; exact hn

/-- What the vertex scan leaves: no bad vertex and every vertex's `around`, or the first bad one. -/
def ScanOutV (l : Ledger) (ext : Ext) (a : Assembled) (isE : List Bool) (vs : List Nat) (bad : U32)
    (av : List (List Nat)) : Prop :=
  (bad = NONE → vs.find? (badV l ext a isE) = none ∧ av = vs.map fun v => aroundL l ext a (covAt l ext v)) ∧
  (bad ≠ NONE → vs.find? (badV l ext a isE) = some bad.val)

theorem vert_scan_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (isE : alloc.vec.Vec Bool) (hN : isE.val.length = (allCells l ext).length)
    (hV : ∀ x ∈ l.verts.val, (x : U32).val < (allCells l ext).length) (hVs : l.verts.val.length < U32.max)
    (b0 : U32) :
    keep_on_loop4 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts ext
      a.result a.orient a.rpos isE b0 (alloc.vec.Vec.new (alloc.vec.Vec U32)) 0#usize ⦃ r =>
      (b0 ≠ NONE → r.1 = b0) ∧
      (b0 = NONE → ScanOutV l ext a isE.val (idsOf l.verts) r.1 (r.2.val.map fun v => v.val.map (·.val))) ⦄ := by
  unfold keep_on_loop4
  apply loop.spec_decr_nat
    (fun (q : U32 × alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => l.verts.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ l.verts.val.length ∧ q.2.1.val.length = q.2.2.val ∧
      (q.1 = NONE → b0 = NONE ∧ ((idsOf l.verts).take q.2.2.val).find? (badV l ext a isE.val) = none ∧
        q.2.1.val.map (fun v => v.val.map (·.val)) =
          ((idsOf l.verts).take q.2.2.val).map fun e => aroundL l ext a (covAt l ext e)) ∧
      (q.1 ≠ NONE → (b0 ≠ NONE → q.1 = b0) ∧ (b0 = NONE → (idsOf l.verts).find? (badV l ext a isE.val) = some q.1.val)))
  · rintro ⟨bd, av0, j⟩ ⟨hj, havl, hb1, hb2⟩
    unfold keep_on_loop4.body
    simp only at hj havl hb1 hb2 ⊢
    split
    · rename_i hnone
      obtain ⟨hb0, hfind, hmap⟩ := hb1 hnone
      split
      · rename_i hlt
        have hjl : j.val < l.verts.val.length := by scalar_tac
        have hil : j.val < (idsOf l.verts).length := by simp [idsOf]; omega
        have hget : (idsOf l.verts)[j.val]'hil = (l.verts.val[j.val]'hjl : U32).val := by simp [idsOf]
        have hft := find_take_succ (f := badV l ext a isE.val) hil hfind
        have htake : (idsOf l.verts).take (j.val + 1) = (idsOf l.verts).take j.val ++ [(idsOf l.verts)[j.val]'hil] :=
          List.take_succ_eq_append_getElem hil
        step as ⟨ci, hci⟩
        have hcil : ci.val < (allCells l ext).length := by rw [hci]; exact hV _ (List.getElem_mem _)
        change (do let ar ← around l ext a ci; _) ⦃ _ ⦄
        step with around_spec l ext hok a W h ci hcil as ⟨ar, har⟩
        change (do let b ← bad_vert l ext a (alloc.vec.Vec.deref isE) ci (alloc.vec.Vec.deref ar); _) ⦃ _ ⦄
        step with bad_vert_spec l ext hok a W h (alloc.vec.Vec.deref isE) (by simpa [alloc.vec.Vec.deref] using hN) ci
          hcil (alloc.vec.Vec.deref ar) (by simpa [alloc.vec.Vec.deref] using har) as ⟨b, hb⟩
        have hcv : ci.val = (idsOf l.verts)[j.val]'hil := by rw [hget, hci]
        have hael' : av0.val.length < Usize.max := by
          have := u32_le_usize; have := U32.max_eq; scalar_tac
        have hbadv : badV l ext a isE.val ((idsOf l.verts)[j.val]'hil) = b := by
          rw [← hcv]
          have hl : (alloc.vec.Vec.deref ar).val.length = (aroundL l ext a (covAt l ext ci.val)).length := by
            simp only [alloc.vec.Vec.deref, Slice.from_val]; rw [← har, List.length_map]
          have hm : (alloc.vec.Vec.deref ar).val.map (·.val) = aroundL l ext a (covAt l ext ci.val) := by
            simpa [alloc.vec.Vec.deref] using har
          have hiff := badV_iff l ext a isE.val ci.val
          rw [hl, hm] at hb
          simp only [alloc.vec.Vec.deref, Slice.from_val] at hb
          cases hbb : b
          · cases hv : badV l ext a isE.val ci.val
            · rfl
            · exact absurd (hb.mpr (hiff.mp hv)) (by simp [hbb])
          · exact hiff.mpr (hb.mp hbb)
        split
        · rename_i hbt
          rw [← hbadv] at hbt
          rw [if_pos hbt] at hft
          have hcn : ci ≠ NONE := by
            intro e; rw [e, none_val] at hcil; have := hok.small; omega
          step as ⟨ae1, hae1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by simp [hae1, havl, hj1], fun e => absurd e hcn, fun _ => ⟨fun e => absurd hb0 e, fun _ => ?_⟩,
            by scalar_tac⟩
          rw [hcv]; exact find_of_take hft
        · rename_i hbf
          have hbf' : badV l ext a isE.val ((idsOf l.verts)[j.val]'hil) = false := by rw [hbadv]; simpa using hbf
          rw [if_neg (by simp [hbf'])] at hft
          step as ⟨ae1, hae1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by simp [hae1, havl, hj1], fun _ => ⟨hb0, by rw [hj1]; exact hft, ?_⟩,
            fun e => absurd hnone e, by scalar_tac⟩
          rw [hj1, hae1, htake, List.map_append, List.map_append, hmap, ← hcv]; simp [har]
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = l.verts.val.length := by scalar_tac
        have ht : (idsOf l.verts).take j.val = idsOf l.verts := List.take_of_length_le (by simp [idsOf]; omega)
        rw [ht] at hfind hmap
        exact ⟨fun e => absurd hb0 e, fun _ => ⟨fun _ => ⟨hfind, hmap⟩, fun e => absurd hnone e⟩⟩
    · rename_i hsome
      simp only [WP.spec_ok]
      obtain ⟨h1, h2⟩ := hb2 hsome
      exact ⟨h1, fun e => ⟨fun e2 => absurd e2 hsome, fun _ => h2 e⟩⟩
  · refine ⟨by simp, by simp, fun e => ⟨e, by simp, by simp⟩, fun e => ⟨fun _ => rfl, fun e2 => absurd e2 e⟩⟩

end forge_bool_core
