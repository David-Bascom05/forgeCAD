import ForgeTopo.ShellFaces
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo_euler::shell_euler` refines the model's `checkShellEuler`

`shell_euler` (kernel/forge-topo/src/topo_euler.rs; check.rs's `check_shell_euler` calls it) as
Aeneas translates it, against verify/forge-topo-check's `Body.checkShellEuler`, for bodies whose
links are sound (`checkLinks = []`). The edges and vertices of a shell are counted by stamping
each with the shell (`Stamped`: the stamp marks exactly the ids met so far in this shell, every
other mark is an earlier shell's or none), which counts the distinct ids (`distinct_snoc`) as
the model's `eraseDups` does.
-/

set_option maxRecDepth 8192

namespace forge_topo

open TopoCheck

/-! ## Distinct counts -/

theorem distinct_snoc (xs : List Nat) (x : Nat) :
    Body.distinct (xs ++ [x]) = Body.distinct xs + (if x ∈ xs then 0 else 1) := by
  unfold Body.distinct
  rw [List.eraseDups_append, List.length_append]
  by_cases h : x ∈ xs
  · simp [h, List.removeAll]
  · simp [h, List.removeAll, List.eraseDups_cons]

theorem distinct_le (xs : List Nat) : Body.distinct xs ≤ xs.length := by
  induction xs using List.reverseRecOn with
  | nil => simp [Body.distinct]
  | append_singleton xs x ih =>
    rw [distinct_snoc, List.length_append]; split <;> simp <;> omega

/-! ## Stamps -/

/-- After the ids `L` of this shell: `σ` marks exactly them; every mark is at most `σ` or none. -/
structure Stamped (n : Nat) (σ : U32) (L : List Nat) (a : List U32) : Prop where
  len : a.length = n
  mem : ∀ x < n, (a[x]! = σ ↔ x ∈ L)
  le : ∀ x < n, (a[x]!).val ≤ σ.val ∨ a[x]! = core.num.U32.MAX
  lt : ∀ x ∈ L, x < n

theorem stamped_new {n : Nat} {σ : U32} {L : List Nat} {a : List U32} (h : Stamped n σ L a)
    {x : Nat} (hx : x < n) (hne : a[x]! ≠ σ) :
    Stamped n σ (L ++ [x]) (a.set x σ) ∧ x ∉ L := by
  have hxl : x ∉ L := fun hm => hne ((h.mem x hx).2 hm)
  refine ⟨⟨by simp [h.len], fun y hy => ?_, fun y hy => ?_, fun y hy => ?_⟩, hxl⟩
  · rw [getElem!_set _ _ _ _ (by rw [h.len]; exact hy)]
    by_cases hyx : y = x
    · subst hyx; simp
    · rw [if_neg hyx, h.mem y hy]; simp [hyx]
  · rw [getElem!_set _ _ _ _ (by rw [h.len]; exact hy)]
    split
    · left; exact le_refl _
    · exact h.le y hy
  · rcases List.mem_append.1 hy with hy | hy
    · exact h.lt y hy
    · simp at hy; omega

theorem stamped_old {n : Nat} {σ : U32} {L : List Nat} {a : List U32} (h : Stamped n σ L a)
    {x : Nat} (hx : x < n) (heq : a[x]! = σ) : Stamped n σ (L ++ [x]) a ∧ x ∈ L := by
  have hxl : x ∈ L := (h.mem x hx).1 heq
  refine ⟨⟨h.len, fun y hy => ?_, h.le, fun y hy => ?_⟩, hxl⟩
  · rw [h.mem y hy]; simp only [List.mem_append, List.mem_singleton]
    constructor
    · exact Or.inl
    · rintro (h' | rfl)
      · exact h'
      · exact hxl
  · rcases List.mem_append.1 hy with hy | hy
    · exact h.lt y hy
    · simp at hy; omega

/-- The next shell's stamp finds nothing marked. -/
theorem stamped_next {n : Nat} {σ σ' : U32} {L : List Nat} {a : List U32}
    (h : Stamped n σ L a) (hσ : σ'.val = σ.val + 1) (hmax : σ'.val < U32.max) : Stamped n σ' [] a := by
  refine ⟨h.len, fun y hy => ?_, fun y hy => ?_, fun y hy => by simp at hy⟩
  · simp only [List.not_mem_nil, iff_false]
    intro he
    rcases h.le y hy with h1 | h1
    · rw [he] at h1; omega
    · rw [he] at h1; have := max_val; rw [h1] at hσ hmax; omega
  · rcases h.le y hy with h1 | h1
    · left; omega
    · right; exact h1

/-- One id stamped: marked and counted once if new. -/
theorem mark_spec {n : Nat} {σ : U32} {L : List Nat} (a : alloc.vec.Vec U32) (x : Usize) (c : I64)
    (hS : Stamped n σ L a.val) (hx : x.val < n) (hc : c.val = Body.distinct L)
    (hb : (Body.distinct L : Int) + 1 ≤ I64.max) (i : U32) (hi : i = a.val[x.val]!) :
    (if i != σ then do
        let (_, back) ← a.index_mut_usize x
        let c2 ← c + 1#i64
        ok (back σ, c2)
      else ok (a, c)) ⦃ r => Stamped n σ (L ++ [x.val]) r.1.val ∧ r.2.val = Body.distinct (L ++ [x.val]) ⦄ := by
  have hxa : x.val < a.val.length := by rw [hS.len]; exact hx
  split
  · rename_i hne
    have hne' : a.val[x.val]! ≠ σ := by rw [← hi]; simpa using hne
    have ⟨hS', hxL⟩ := stamped_new hS hx hne'
    step as ⟨i0, back, hi0, hback⟩
    have h1 : (1#i64).val = 1 := rfl
    step as ⟨c2, hc2⟩
    rw [hback]
    simp only [alloc.vec.Vec.set_val_eq]
    refine ⟨hS', ?_⟩
    rw [hc2, hc, distinct_snoc, if_neg hxL]; push_cast; rfl
  · rename_i heq
    have heq' : a.val[x.val]! = σ := by rw [← hi]; exact UScalar.eq_of_val_eq (by simpa using heq)
    have ⟨hS', hxL⟩ := stamped_old hS hx heq'
    simp only [WP.spec_ok]
    refine ⟨hS', ?_⟩
    rw [hc, distinct_snoc, if_pos hxL]; simp

/-! ## The model's lists -/

/-- The edges of a run of coedges. -/
def edgesOf (b : Body) (cs : List Nat) : List Nat := cs.map fun c => (b.co c).edge

/-- The ends of a run of edges, start then stop. -/
def vertsOf (b : Body) (es : List Nat) : List Nat := es.flatMap fun e => [(b.ed e).start, (b.ed e).stop]

theorem vertsOf_snoc (b : Body) (es : List Nat) (x : Nat) :
    vertsOf b (es ++ [x]) = vertsOf b es ++ [(b.ed x).start] ++ [(b.ed x).stop] := by
  simp [vertsOf, List.flatMap_append]

theorem vertsOf_length (b : Body) (es : List Nat) : (vertsOf b es).length = 2 * es.length := by
  induction es with
  | nil => simp [vertsOf]
  | cons x xs ih => simp only [vertsOf, List.flatMap_cons, List.length_append] at ih ⊢; simp; omega

theorem take_loopCoedges_succ (b : Body) (l k : Nat) (hk : k < (b.lo l).len) :
    (b.loopCoedges l).take (k + 1) = (b.loopCoedges l).take k ++ [(b.lo l).first + k] := by
  have hlen : k < (b.loopCoedges l).length := by simp [Body.loopCoedges]; exact hk
  rw [List.take_add_one, List.getElem?_eq_getElem hlen]
  simp [Body.loopCoedges]

section
variable {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body)

theorem coedges_spec (R : ShellReads SI t b) (H : Sound b) (W : b.WF) (σ : U32)
    (lp first len : Usize) (hlp : lp.val < b.loops.size) (hl1 : b.listed lp.val = 1)
    (hfirst : first.val = (b.lo lp.val).first) (hlen : len.val = (b.lo lp.val).len)
    (ea va : alloc.vec.Vec U32) (E : List Nat) (e v : I64)
    (hE : Stamped b.edges.size σ E ea.val) (hV : Stamped b.nv σ (vertsOf b E) va.val)
    (he : e.val = Body.distinct E) (hv : v.val = Body.distinct (vertsOf b E))
    (hB : 2 * ((E.length : Int) + len.val) + 2 ≤ I64.max) :
    topo_euler.shell_euler_loop0_loop0_loop0_loop0 SI t ea va σ e v first len 0#usize ⦃ r =>
      Stamped b.edges.size σ (E ++ edgesOf b (b.loopCoedges lp.val)) r.1.val ∧
      Stamped b.nv σ (vertsOf b (E ++ edgesOf b (b.loopCoedges lp.val))) r.2.1.val ∧
      r.2.2.1.val = Body.distinct (E ++ edgesOf b (b.loopCoedges lp.val)) ∧
      r.2.2.2.val = Body.distinct (vertsOf b (E ++ edgesOf b (b.loopCoedges lp.val))) ⦄ := by
  unfold topo_euler.shell_euler_loop0_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × I64 × I64 × Usize) => len.val - y.2.2.2.2.val)
    (fun y => y.2.2.2.2.val ≤ len.val ∧
      Stamped b.edges.size σ (E ++ edgesOf b ((b.loopCoedges lp.val).take y.2.2.2.2.val)) y.1.val ∧
      Stamped b.nv σ (vertsOf b (E ++ edgesOf b ((b.loopCoedges lp.val).take y.2.2.2.2.val))) y.2.1.val ∧
      y.2.2.1.val = Body.distinct (E ++ edgesOf b ((b.loopCoedges lp.val).take y.2.2.2.2.val)) ∧
      y.2.2.2.1.val = Body.distinct (vertsOf b (E ++ edgesOf b ((b.loopCoedges lp.val).take y.2.2.2.2.val))))
  · rintro ⟨ea0, va0, e0, v0, k⟩ ⟨hle, hE0, hV0, he0, hv0⟩
    simp only at hle hE0 hV0 he0 hv0 ⊢
    unfold topo_euler.shell_euler_loop0_loop0_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : k.val < len.val := by scalar_tac
      have hk : k.val < (b.lo lp.val).len := by omega
      set Ek := E ++ edgesOf b ((b.loopCoedges lp.val).take k.val) with hEk
      have hEkl : Ek.length = E.length + k.val := by
        simp [hEk, edgesOf, Body.loopCoedges]; omega
      have hcm : (b.lo lp.val).first + k.val ∈ b.loopCoedges lp.val := by
        rw [mem_loopCoedges]; omega
      obtain ⟨hcn, hen, -⟩ := H.coedges lp.val hlp hl1 _ hcm
      obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
      step with usize_add_wspec first k (le_max_of_lt_val m _ (by have := m.hBounds; omega)) as ⟨c, hc⟩
      have hcv : c.val = (b.lo lp.val).first + k.val := by
        rw [hc, add_val first k (by have := m.hBounds; omega), hfirst]
      clear hc
      step with R.base.coedge_edge c (by rw [hcv]; exact hcn) as ⟨ed, hed⟩
      have hedn : ed.val < b.edges.size := by rw [hed, hcv]; exact hen
      have hb1 : ed.val < ea0.length := by
        have := hE0.len; simp only [alloc.vec.Vec.length] at *; omega
      step as ⟨i1, hi1⟩
      have hi1' : i1 = ea0.val[ed.val]! := by rw [hi1, getElem!_pos]
      have hbE : (Body.distinct Ek : Int) + 1 ≤ I64.max := by
        have := distinct_le Ek; omega
      step with mark_spec ea0 ed e0 hE0 hedn he0 hbE i1 hi1' as ⟨ea1, e1, hE1, he1⟩
      have hends := W.ends ed.val hedn
      step with R.base.edge_ends ed hedn as ⟨a, bb, ha, hbb⟩
      have hb2 : a.val < va0.length := by
        have := hV0.len; simp only [alloc.vec.Vec.length] at *; rw [ha]; omega
      step as ⟨i2, hi2⟩
      have hi2' : i2 = va0.val[a.val]! := by rw [hi2, getElem!_pos]
      have hVl : (vertsOf b Ek).length = 2 * Ek.length := vertsOf_length b Ek
      have hbV : (Body.distinct (vertsOf b Ek) : Int) + 1 ≤ I64.max := by
        have := distinct_le (vertsOf b Ek); omega
      step with mark_spec va0 a v0 hV0 (by rw [ha]; exact hends.1) hv0 hbV i2 hi2' as ⟨va1, v1, hV1, hv1⟩
      have hb3 : bb.val < va1.length := by
        have := hV1.len; simp only [alloc.vec.Vec.length] at *; rw [hbb]; omega
      step as ⟨i3, hi3⟩
      have hi3' : i3 = va1.val[bb.val]! := by rw [hi3, getElem!_pos]
      have hbV' : (Body.distinct (vertsOf b Ek ++ [a.val]) : Int) + 1 ≤ I64.max := by
        have := distinct_le (vertsOf b Ek ++ [a.val]); simp at this; omega
      step with mark_spec va1 bb v1 hV1 (by rw [hbb]; exact hends.2) hv1 hbV' i3 hi3' as ⟨va2, v2, hV2, hv2⟩
      step with usize_add_wspec k 1#usize (succ_le_max k len hlt') as ⟨k1, hk1⟩
      have hk1v : k1.val = k.val + 1 := by rw [hk1]; exact succ_val k len hlt'
      have hEk1 : E ++ edgesOf b ((b.loopCoedges lp.val).take k1.val) = Ek ++ [ed.val] := by
        rw [hk1v, take_loopCoedges_succ b lp.val k.val hk, hEk]
        simp [edgesOf, hed, hcv]
      have hVk1 : vertsOf b (Ek ++ [ed.val]) = vertsOf b Ek ++ [a.val] ++ [bb.val] := by
        rw [vertsOf_snoc, ha, hbb]
      rw [hEk1, hVk1]
      exact ⟨by omega, hE1, hV2, he1, hv2, by omega⟩
    · rename_i hlt
      have hk : k.val = len.val := by scalar_tac
      simp only [WP.spec_ok]
      have htk : (b.loopCoedges lp.val).take k.val = b.loopCoedges lp.val := by
        rw [hk, hlen]; apply List.take_of_length_le; simp [Body.loopCoedges]
      rw [htk] at hE0 hV0 he0 hv0
      exact ⟨hE0, hV0, he0, hv0⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, edgesOf, List.map_nil, List.append_nil]
    exact ⟨by simp, hE, hV, he, hv⟩

end

end forge_topo
