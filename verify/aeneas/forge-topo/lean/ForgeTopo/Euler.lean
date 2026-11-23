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

theorem faceLoops_ge (b : Body) (f : Nat) (h : b.faces.size ≤ f) : b.faceLoops f = [] := by
  unfold Body.faceLoops; rw [Array.getD_eq_getD_getElem?, Array.getElem?_eq_none h]; rfl

theorem flatMap_take_succ {α β} [Inhabited α] (L : List α) (g : α → List β) (j : Nat) (hj : j < L.length) :
    (L.take (j + 1)).flatMap g = (L.take j).flatMap g ++ g L[j]! := by
  rw [List.take_succ_eq_append_getElem hj, getElem!_pos L j hj, List.flatMap_append]; simp

theorem flatMap_take_le {α β} (L : List α) (g : α → List β) (j : Nat) :
    ((L.take j).flatMap g).length ≤ (L.flatMap g).length := by
  have := congrArg (fun L => (L.flatMap g).length) (List.take_append_drop j L)
  simp only [List.flatMap_append, List.length_append] at this
  omega

section
variable {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body)

/-- The coedges of a face's loops, in order. -/
def faceCoedges (f : Nat) : List Nat := (b.faceLoops f).flatMap b.loopCoedges

theorem face_loops_spec (R : ShellReads SI t b) (H : Sound b) (W : b.WF) (σ : U32)
    (f nl : Usize) (hnl : nl.val = (b.faceLoops f.val).length)
    (ea va : alloc.vec.Vec U32) (E : List Nat) (l e v : I64)
    (hE : Stamped b.edges.size σ E ea.val) (hV : Stamped b.nv σ (vertsOf b E) va.val)
    (he : e.val = Body.distinct E) (hv : v.val = Body.distinct (vertsOf b E))
    (hB : 2 * ((E.length : Int) + (faceCoedges b f.val).length) + 2 ≤ I64.max)
    (hL : l.val + nl.val ≤ I64.max) :
    topo_euler.shell_euler_loop0_loop0_loop0 SI t ea va σ l e v f nl 0#usize ⦃ r =>
      Stamped b.edges.size σ (E ++ edgesOf b (faceCoedges b f.val)) r.1.val ∧
      Stamped b.nv σ (vertsOf b (E ++ edgesOf b (faceCoedges b f.val))) r.2.1.val ∧
      r.2.2.1.val = l.val + nl.val ∧
      r.2.2.2.1.val = Body.distinct (E ++ edgesOf b (faceCoedges b f.val)) ∧
      r.2.2.2.2.val = Body.distinct (vertsOf b (E ++ edgesOf b (faceCoedges b f.val))) ⦄ := by
  unfold topo_euler.shell_euler_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × I64 × I64 × I64 × Usize) => nl.val - y.2.2.2.2.2.val)
    (fun y => y.2.2.2.2.2.val ≤ nl.val ∧
      Stamped b.edges.size σ (E ++ edgesOf b (((b.faceLoops f.val).take y.2.2.2.2.2.val).flatMap b.loopCoedges)) y.1.val ∧
      Stamped b.nv σ (vertsOf b (E ++ edgesOf b (((b.faceLoops f.val).take y.2.2.2.2.2.val).flatMap b.loopCoedges))) y.2.1.val ∧
      y.2.2.1.val = l.val + y.2.2.2.2.2.val ∧
      y.2.2.2.1.val = Body.distinct (E ++ edgesOf b (((b.faceLoops f.val).take y.2.2.2.2.2.val).flatMap b.loopCoedges)) ∧
      y.2.2.2.2.1.val = Body.distinct (vertsOf b (E ++ edgesOf b (((b.faceLoops f.val).take y.2.2.2.2.2.val).flatMap b.loopCoedges))))
  · rintro ⟨ea0, va0, l0, e0, v0, j⟩ ⟨hle, hE0, hV0, hl0, he0, hv0⟩
    simp only at hle hE0 hV0 hl0 he0 hv0 ⊢
    unfold topo_euler.shell_euler_loop0_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : j.val < nl.val := by scalar_tac
      have hj : j.val < (b.faceLoops f.val).length := by omega
      have hf : f.val < b.faces.size := by
        by_contra h
        rw [faceLoops_ge b f.val (by omega)] at hj; simp at hj
      step with R.base.face_loop f j hf hj as ⟨lp, hlp⟩
      have hlm : lp.val ∈ b.faceLoops f.val := by
        rw [hlp, getElem!_pos (b.faceLoops f.val) j.val hj]; exact List.getElem_mem hj
      obtain ⟨hln, -, hl1⟩ := H.loops f.val hf lp.val hlm
      step as ⟨l1, hl1v⟩
      step with R.base.loop_first lp hln as ⟨first, hfirst⟩
      step with R.base.loop_len lp hln as ⟨len, hlen⟩
      have hpre := flatMap_take_succ (b.faceLoops f.val) b.loopCoedges j.val hj
      rw [← hlp] at hpre
      have hle2 := flatMap_take_le (b.faceLoops f.val) b.loopCoedges (j.val + 1)
      have hlenL : (b.loopCoedges lp.val).length = len.val := by simp [Body.loopCoedges, hlen]
      have hB' : 2 * (((E ++ edgesOf b (((b.faceLoops f.val).take j.val).flatMap b.loopCoedges)).length : Nat) + (len.val : Int)) + 2 ≤ I64.max := by
        rw [hpre] at hle2
        simp only [faceCoedges, edgesOf, List.length_append, List.length_map] at hB hle2 ⊢
        omega
      step with coedges_spec SI t b R H W σ lp first len hln hl1 hfirst hlen ea0 va0 _ e0 v0 hE0 hV0 he0 hv0 hB'
        as ⟨ea1, va1, e1, v1, hE1, hV1, he1, hv1⟩
      step with usize_add_wspec j 1#usize (succ_le_max j nl hlt') as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact succ_val j nl hlt'
      have hEj : E ++ edgesOf b (((b.faceLoops f.val).take j1.val).flatMap b.loopCoedges) =
          E ++ edgesOf b (((b.faceLoops f.val).take j.val).flatMap b.loopCoedges) ++ edgesOf b (b.loopCoedges lp.val) := by
        rw [hj1v, hpre]; simp [edgesOf]
      rw [hEj]
      refine ⟨by omega, hE1, hV1, by rw [hl1v, hl0, hj1v]; push_cast; ring, he1, hv1, by omega⟩
    · rename_i hlt
      have hj : j.val = nl.val := by scalar_tac
      simp only [WP.spec_ok]
      have htk : (b.faceLoops f.val).take j.val = b.faceLoops f.val := by
        rw [hj, hnl]; exact List.take_length
      rw [htk] at hE0 hV0 he0 hv0
      rw [hj] at hl0
      exact ⟨hE0, hV0, hl0, he0, hv0⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, List.flatMap_nil, edgesOf, List.map_nil, List.append_nil]
    exact ⟨by simp, hE, hV, by simp, he, hv⟩

end

section
variable {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body)

/-- A shell's listed faces, loops and coedges fit `i64` (as a body's arrays, far below `2^63`
entries, do): the hypothesis under which `f`, `l`, `e`, `v` and V − E + 2F − L never overflow. -/
def EulerSmall (s : Nat) : Prop :=
  2 * (((b.shellList s).length : Int) + ((b.shellList s).flatMap b.faceLoops).length +
    2 * ((b.shellList s).flatMap (faceCoedges b)).length) + 2 ≤ I64.max

theorem shell_faces_euler_spec (R : ShellReads SI t b) (H : Sound b) (W : b.WF) (σ : U32)
    (nfc s nf : Usize) (hnfc : nfc.val = b.faces.size) (hs : s.val < b.shells.size)
    (hnf : nf.val = (b.shellList s.val).length) (hsm : EulerSmall b s.val)
    (ea va : alloc.vec.Vec U32)
    (hE : Stamped b.edges.size σ [] ea.val) (hV : Stamped b.nv σ [] va.val) :
    topo_euler.shell_euler_loop0_loop0 SI t nfc ea va s σ 0#i64 0#i64 0#i64 0#i64 nf 0#usize ⦃ r =>
      Stamped b.edges.size σ (edgesOf b ((b.shellList s.val).flatMap (faceCoedges b))) r.1.val ∧
      Stamped b.nv σ (vertsOf b (edgesOf b ((b.shellList s.val).flatMap (faceCoedges b)))) r.2.1.val ∧
      r.2.2.1.val = (b.shellList s.val).length ∧
      r.2.2.2.1.val = ((b.shellList s.val).flatMap b.faceLoops).length ∧
      r.2.2.2.2.1.val = Body.distinct (edgesOf b ((b.shellList s.val).flatMap (faceCoedges b))) ∧
      r.2.2.2.2.2.val = Body.distinct (vertsOf b (edgesOf b ((b.shellList s.val).flatMap (faceCoedges b)))) ⦄ := by
  set fs := b.shellList s.val with hfs
  unfold topo_euler.shell_euler_loop0_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec U32 × alloc.vec.Vec U32 × I64 × I64 × I64 × I64 × Usize) => nf.val - y.2.2.2.2.2.2.val)
    (fun y => y.2.2.2.2.2.2.val ≤ nf.val ∧
      Stamped b.edges.size σ (edgesOf b ((fs.take y.2.2.2.2.2.2.val).flatMap (faceCoedges b))) y.1.val ∧
      Stamped b.nv σ (vertsOf b (edgesOf b ((fs.take y.2.2.2.2.2.2.val).flatMap (faceCoedges b)))) y.2.1.val ∧
      y.2.2.1.val = y.2.2.2.2.2.2.val ∧
      y.2.2.2.1.val = ((fs.take y.2.2.2.2.2.2.val).flatMap b.faceLoops).length ∧
      y.2.2.2.2.1.val = Body.distinct (edgesOf b ((fs.take y.2.2.2.2.2.2.val).flatMap (faceCoedges b))) ∧
      y.2.2.2.2.2.1.val = Body.distinct (vertsOf b (edgesOf b ((fs.take y.2.2.2.2.2.2.val).flatMap (faceCoedges b)))))
  · rintro ⟨ea0, va0, f0, l0, e0, v0, i⟩ ⟨hle, hE0, hV0, hf0, hl0, he0, hv0⟩
    simp only at hle hE0 hV0 hf0 hl0 he0 hv0 ⊢
    unfold topo_euler.shell_euler_loop0_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < nf.val := by scalar_tac
      have hi : i.val < fs.length := by omega
      step with R.shell_face s i hs hi as ⟨face, hface⟩
      have hpreC := flatMap_take_succ fs (faceCoedges b) i.val hi
      have hpreL := flatMap_take_succ fs b.faceLoops i.val hi
      rw [← hface] at hpreC hpreL
      have hleC := flatMap_take_le fs (faceCoedges b) (i.val + 1)
      have hleL := flatMap_take_le fs b.faceLoops (i.val + 1)
      rw [hpreC] at hleC; rw [hpreL] at hleL
      simp only [List.length_append] at hleC hleL
      unfold EulerSmall at hsm; rw [← hfs] at hsm
      step as ⟨f1, hf1⟩
      apply WP.spec_bind (Pₘ := fun nl : Usize => nl.val = (b.faceLoops face.val).length)
      · split
        · rename_i hfn
          exact R.base.face_n_loops face (by rw [← hnfc]; exact hfn)
        · rename_i hfn
          simp only [WP.spec_ok]
          rw [faceLoops_ge b face.val (by rw [← hnfc]; scalar_tac)]; rfl
      · intro nl hnl
        have hB : 2 * (((edgesOf b ((fs.take i.val).flatMap (faceCoedges b))).length : Nat) +
            ((faceCoedges b face.val).length : Int)) + 2 ≤ I64.max := by
          simp only [edgesOf, List.length_map]; omega
        have hL : l0.val + nl.val ≤ I64.max := by rw [hl0, hnl]; omega
        step with face_loops_spec SI t b R H W σ face nl hnl ea0 va0 _ l0 e0 v0 hE0 hV0 he0 hv0 hB hL
          as ⟨ea1, va1, l1, e1, v1, hE1, hV1, hl1, he1, hv1⟩
        step with usize_add_wspec i 1#usize (succ_le_max i nf hlt') as ⟨i1, hi1⟩
        have hi1v : i1.val = i.val + 1 := by rw [hi1]; exact succ_val i nf hlt'
        have hEi : edgesOf b ((fs.take i1.val).flatMap (faceCoedges b)) =
            edgesOf b ((fs.take i.val).flatMap (faceCoedges b)) ++ edgesOf b (faceCoedges b face.val) := by
          rw [hi1v, hpreC]; simp [edgesOf]
        have hLi : ((fs.take i1.val).flatMap b.faceLoops).length =
            ((fs.take i.val).flatMap b.faceLoops).length + nl.val := by
          rw [hi1v, hpreL, List.length_append, hnl]
        rw [hEi, hLi]
        refine ⟨by omega, hE1, hV1, by rw [hf1, hf0, hi1v]; push_cast; ring, by rw [hl1, hl0]; push_cast; ring,
          he1, hv1, by omega⟩
    · rename_i hlt
      have hi : i.val = nf.val := by scalar_tac
      simp only [WP.spec_ok]
      have htk : fs.take i.val = fs := by rw [hi, hnf]; exact List.take_length
      rw [htk] at hE0 hV0 hl0 he0 hv0
      rw [hi, hnf] at hf0
      exact ⟨hE0, hV0, hf0, hl0, he0, hv0⟩
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0, List.take_zero, List.flatMap_nil, edgesOf, List.map_nil]
    exact ⟨by simp, hE, by simpa [vertsOf] using hV, rfl, rfl, by simp [Body.distinct],
      by simp [Body.distinct, vertsOf]⟩

end

/-! ## The shells, and `shell_euler` -/

/-- Every mark is an earlier shell's (below `s`) or none. -/
def Fresh (n s : Nat) (a : List U32) : Prop :=
  a.length = n ∧ ∀ x < n, (a[x]!).val < s ∨ a[x]! = core.num.U32.MAX

theorem stamped_of_fresh {n s : Nat} {a : List U32} (h : Fresh n s a) (σ : U32) (hσ : σ.val = s)
    (hs : s < U32.max) : Stamped n σ [] a := by
  refine ⟨h.1, fun y hy => ?_, fun y hy => ?_, fun y hy => by simp at hy⟩
  · simp only [List.not_mem_nil, iff_false]
    intro he
    rcases h.2 y hy with h1 | h1
    · rw [he] at h1; omega
    · rw [he] at h1; have := max_val; rw [h1] at hσ; omega
  · rcases h.2 y hy with h1 | h1
    · left; omega
    · right; exact h1

theorem fresh_of_stamped {n : Nat} {σ : U32} {L : List Nat} {a : List U32} (h : Stamped n σ L a) :
    Fresh n (σ.val + 1) a := by
  refine ⟨h.len, fun y hy => ?_⟩
  rcases h.le y hy with h1 | h1
  · left; omega
  · right; exact h1

theorem fresh_replicate (n : Nat) : Fresh n 0 (List.replicate n core.num.U32.MAX) := by
  refine ⟨by simp, fun y hy => Or.inr ?_⟩
  rw [getElem!_pos _ y (by simpa using hy)]; simp

/-- The model's shell value, read through the lists the code walks. -/
theorem shellValue_eq (b : Body) (s : Nat) :
    b.shellValue (b.shells.getD s []) =
      (Body.distinct (vertsOf b (edgesOf b ((b.shellList s).flatMap (faceCoedges b)))) : Int) -
      Body.distinct (edgesOf b ((b.shellList s).flatMap (faceCoedges b))) +
      2 * (b.shellList s).length - ((b.shellList s).flatMap b.faceLoops).length := by
  have hfc : faceCoedges b = fun x => (b.faceLoops x).flatMap b.loopCoedges := rfl
  unfold Body.shellValue
  rw [hfc]
  simp only [Body.shellList, vertsOf, edgesOf, List.flatMap_assoc]

theorem tmod_two_ne (x : Int) : Int.tmod x 2 ≠ 0 ↔ x % 2 ≠ 0 := by
  rw [ne_eq, ne_eq, ← Int.dvd_iff_tmod_eq_zero, ← Int.dvd_iff_emod_eq_zero]

/-- A sink that appends the problem it is handed. -/
def AppendsEu {S : Type} (EI : topo_euler.EulerSink S) (P : S → List Problem) : Prop :=
  ∀ (o : S) (s : Usize) (x : I64), EI.shell_euler o s x ⦃ o' => P o' = P o ++ [Problem.shellEuler s.val x.val] ⦄

section
variable {T : Type} (SI : topo_shell.ShellTopo T) (t : T) (b : Body)

set_option hygiene false in
/-- The end of a shell: the next index, the marks now an earlier shell's, the problem list. -/
macro "finish_shell" o:term "," h:term : tactic => `(tactic| (
  step with usize_add_wspec s 1#usize (succ_le_max s ns hlt') as ⟨s1, hs1⟩
  have hE2 := fresh_of_stamped hE1
  have hV2 := fresh_of_stamped hV1
  rw [hσv] at hE2 hV2
  rw [hw s1 hs1]
  exact ⟨by omega, hE2, hV2, hacc $o $h, by omega⟩))

theorem shells_euler_spec {S : Type} (EI : topo_euler.EulerSink S) (P : S → List Problem)
    (hS : AppendsEu EI P) (R : ShellReads SI t b) (H : Sound b) (W : b.WF)
    (hsm : ∀ s < b.shells.size, EulerSmall b s)
    (ns nfc : Usize) (hns : ns.val = b.shells.size) (hnfc : nfc.val = b.faces.size)
    (ea va : alloc.vec.Vec U32) (hE : Fresh b.edges.size 0 ea.val) (hV : Fresh b.nv 0 va.val) (out : S) :
    topo_euler.shell_euler_loop0 SI EI t out ns nfc ea va 0#usize ⦃ r =>
      P r = P out ++ b.checkShellEuler ⦄ := by
  have hlt32 := R.ns_lt
  unfold topo_euler.shell_euler_loop0
  apply loop.spec_decr_nat
    (fun (y : S × alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => ns.val - y.2.2.2.val)
    (fun y => y.2.2.2.val ≤ ns.val ∧ Fresh b.edges.size y.2.2.2.val y.2.1.val ∧
      Fresh b.nv y.2.2.2.val y.2.2.1.val ∧
      P y.1 = P out ++ ((List.range y.2.2.2.val).filterMap fun s =>
        let value := b.shellValue (b.shells.getD s [])
        if value % 2 != 0 || value > 2 then some (Problem.shellEuler s value) else none))
  · rintro ⟨o, ea0, va0, s⟩ ⟨hle, hE0, hV0, ho⟩
    simp only at hle hE0 hV0 ho ⊢
    unfold topo_euler.shell_euler_loop0.body
    split
    · rename_i hlt
      have hlt' : s.val < ns.val := by scalar_tac
      have hs : s.val < b.shells.size := by omega
      have hσv : (UScalar.cast .U32 s).val = s.val := cast32_val s (by omega)
      have hEs := stamped_of_fresh hE0 (UScalar.cast .U32 s) hσv (by omega)
      have hVs := stamped_of_fresh hV0 (UScalar.cast .U32 s) hσv (by omega)
      simp only [lift, Std.bind_ok]
      step with R.shell_n_faces s hs as ⟨nf, hnf⟩
      step with shell_faces_euler_spec SI t b R H W (UScalar.cast .U32 s) nfc s nf hnfc hs hnf (hsm s.val hs)
        ea0 va0 hEs hVs as ⟨ea1, va1, f, l, e, v, hE1, hV1, hf, hl, he, hv⟩
      have hsm' := hsm s.val hs
      unfold EulerSmall at hsm'
      set ES := edgesOf b ((b.shellList s.val).flatMap (faceCoedges b)) with hES
      have hESl : ES.length = ((b.shellList s.val).flatMap (faceCoedges b)).length := by
        simp [hES, edgesOf]
      have hde := distinct_le ES
      have hdv := distinct_le (vertsOf b ES)
      have hvl := vertsOf_length b ES
      have hval := shellValue_eq b s.val
      rw [← hES] at hval
      step as ⟨i, hi⟩
      step as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      step as ⟨value, hvalue⟩
      have hvv : value.val = b.shellValue (b.shells.getD s.val []) := by
        (rw [hval, hvalue, hi2, hi1, hi, hv, he, hf, hl]) <;> push_cast <;> ring
      step with IScalar.rem_spec value (y := 2#i64) (by decide) as ⟨i3, hi3⟩
      have hacc : ∀ o', P o' = P o ++ (if b.shellValue (b.shells.getD s.val []) % 2 != 0 ||
            b.shellValue (b.shells.getD s.val []) > 2
          then some (Problem.shellEuler s.val (b.shellValue (b.shells.getD s.val []))) else none).toList →
          P o' = P out ++ ((List.range (s.val + 1)).filterMap fun s =>
            let value := b.shellValue (b.shells.getD s [])
            if value % 2 != 0 || value > 2 then some (Problem.shellEuler s value) else none) := by
        intro o' h; rw [h, filterMap_range_succ, ho, List.append_assoc]
      have hw : ∀ s1 : Usize, s1 = s.wrapping_add 1#usize → s1.val = s.val + 1 := by
        intro s1 h; rw [h]; exact succ_val s ns hlt'
      have hmod : (i3 != 0#i64) = (b.shellValue (b.shells.getD s.val []) % 2 != 0) := by
        have h0 : (0#i64).val = 0 := rfl
        by_cases hz : value.val % 2 = 0
        · have : Int.tmod value.val 2 = 0 := by
            by_contra hc; exact ((tmod_two_ne _).1 hc) hz
          have hi30 : i3 = 0#i64 := IScalar.eq_of_val_eq (by rw [hi3, this]; rfl)
          rw [hi30, ← hvv, hz]; simp
        · have : Int.tmod value.val 2 ≠ 0 := (tmod_two_ne _).2 hz
          have hi30 : i3 ≠ 0#i64 := by intro hc; rw [hc, h0] at hi3; exact this hi3.symm
          rw [← hvv, bne_iff_ne.mpr hi30, bne_iff_ne.mpr hz]
      split
      · rename_i hodd
        have hodd' : (b.shellValue (b.shells.getD s.val []) % 2 != 0) = true := by rw [← hmod]; exact hodd
        step with hS o s value as ⟨o1, ho1⟩
        finish_shell o1, (by rw [ho1, hvv, hodd']; simp)
      · rename_i hodd
        have hodd' : (b.shellValue (b.shells.getD s.val []) % 2 != 0) = false := by
          rw [← hmod]; revert hodd; cases (i3 != 0#i64) <;> simp
        split
        · rename_i hgt
          have hgt' : b.shellValue (b.shells.getD s.val []) > 2 := by
            rw [← hvv]; have h2 : (2#i64).val = 2 := rfl; scalar_tac
          step with hS o s value as ⟨o1, ho1⟩
          finish_shell o1, (by rw [ho1, hvv, hodd', Bool.false_or, decide_eq_true hgt']; simp)
        · rename_i hgt
          have hgt' : ¬ b.shellValue (b.shells.getD s.val []) > 2 := by
            rw [← hvv]; have h2 : (2#i64).val = 2 := rfl; scalar_tac
          simp only [Std.bind_ok]
          finish_shell o, (by rw [hodd', Bool.false_or, decide_eq_false hgt']; simp)
    · rename_i hlt
      have hsn : s.val = ns.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [ho, hsn, hns]; rfl
  · have h0 : (0#usize).val = 0 := rfl
    simp only [h0]
    exact ⟨by simp, hE, hV, by simp⟩

/-- **The shipped `check_shell_euler` refines the model.** For a body whose links are sound (as
`check_known` runs it: only once `check_links` found nothing), whose shells' listings fit `i64`
(`EulerSmall`), any `ShellTopo` view that reads it (`ShellReads`) and any sink that appends what it
is handed, the translated Rust `shell_euler` terminates without panic or overflow and appends
exactly `Model.checkShellEuler`, a shell's face ids past the faces included (counted as faces with
no loops, as the model reads them). -/
theorem shell_euler_refines {S : Type} (EI : topo_euler.EulerSink S) (P : S → List Problem)
    (hS : AppendsEu EI P) (R : ShellReads SI t b) (hL : b.checkLinks = [])
    (hsm : ∀ s < b.shells.size, EulerSmall b s) (out : S) :
    topo_euler.shell_euler SI EI t out ⦃ r => P r = P out ++ b.checkShellEuler ⦄ := by
  have H := sound_of b hL
  have W := b.links_ok hL
  unfold topo_euler.shell_euler
  have rns := R.ns
  have rnf := R.base.nf
  have rne := R.base.ne
  have rnv := R.base.nv
  step with rns as ⟨ns, hns⟩
  step with rnf as ⟨nfc, hnfc⟩
  step with rne as ⟨ne, hne⟩
  step as ⟨ea, hea, hea'⟩
  step with rnv as ⟨nv, hnv⟩
  step as ⟨va, hva, hva'⟩
  have hE : Fresh b.edges.size 0 ea.val := by rw [hea, hne]; exact fresh_replicate _
  have hV : Fresh b.nv 0 va.val := by rw [hva, hnv]; exact fresh_replicate _
  exact shells_euler_spec SI t b EI P hS R H W hsm ns nfc hns hnfc ea va hE hV out

end

end forge_topo
