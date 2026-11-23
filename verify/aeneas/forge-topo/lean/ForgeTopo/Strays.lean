import ForgeTopo.Funs
import TopoCheck.Model
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo::strays` refines the model's `checkStrays`

`ForgeTopo/Funs.lean` is Aeneas's translation of kernel/forge-topo/src/topo.rs, the code `check`
runs (`check_strays` calls it). Here:
* `strays_spec`: for any `Topo` instance whose methods give `nv` vertices, `ne` edges and ends
  `S e`, `E e`, `strays` returns ok (no index out of bounds, no overflow, no divergence) exactly
  `strayRef`'s list;
* `checkStrays_eq`: verify/forge-topo-check's model `checkStrays` is that list, as problems.
So `check_sound`'s `noStrays` is about the shipped code, given that the instance reads the
body's vertex count, edge count and edge ends (`impl Topo for Body`, check.rs).
-/

namespace forge_topo

/-- The stray vertices of `nv` vertices and `ne` edges with ends `S`, `E`. -/
def strayRef (nv ne : Nat) (S E : Nat → Nat) : List Nat :=
  (List.range nv).filter fun v => !((List.range ne).any fun k => S k == v || E k == v)

theorem idxu {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_usize v i = ok v.val[i.val]! := by
  simp only [alloc.vec.Vec.index_usize]
  have : v[i.val]? = some v.val[i.val]! := by
    simp [getElem!_pos, h]
  rw [this]

theorem idx {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok v.val[i.val]! := by
  rw [alloc.vec.Vec.index_slice_index, idxu v i h]

theorem idxm {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice α) v i =
      ok (v.val[i.val]!, v.set i) := by
  rw [alloc.vec.Vec.index_mut_slice_index, alloc.vec.Vec.index_mut_usize, idxu v i h, bind_tc_ok]

theorem le_max (x : Usize) : x.val ≤ Usize.max := by scalar_tac

theorem succ_val (e ne : Usize) (h : e.val < ne.val) :
    (Std.Usize.wrapping_add e 1#usize).val = e.val + 1 := by
  rw [Usize.wrapping_add_val_eq]
  apply Nat.mod_eq_of_lt
  have h1 := ne.hBounds
  have h2 : (1#usize).val = 1 := rfl
  simp only [UScalar.size] at *
  omega

theorem getElem!_set {α} [Inhabited α] (l : List α) (j i : Nat) (x : α) (hi : i < l.length) :
    (l.set j x)[i]! = if i = j then x else l[i]! := by
  rw [getElem!_pos (l.set j x) i (by simpa using hi), getElem!_pos l i hi, List.getElem_set]
  split <;> simp_all [eq_comm]

/-- The marks after the first `k` edges. -/
def marked (S E : Nat → Nat) (k i : Nat) : Bool := (List.range k).any fun j => S j == i || E j == i

theorem marked_succ (S E : Nat → Nat) (k i : Nat) :
    marked S E (k + 1) i = (marked S E k i || (S k == i || E k == i)) := by
  simp [marked, List.range_succ, List.any_append]

section
variable {T : Type} (TopoInst : topo.Topo T) (t : T)

theorem loop0_spec (nv : Usize) (ne : Usize) (S E : Nat → Nat)
    (hse : ∀ e : Usize, e.val < ne.val →
      TopoInst.edge_ends t e ⦃ p => p.1.val = S e.val ∧ p.2.val = E e.val ⦄)
    (used : alloc.vec.Vec Bool) (hl : used.val.length = nv.val)
    (h0 : ∀ i < nv.val, used.val[i]! = false) :
    topo.strays_loop0 TopoInst t used ne 0#usize ⦃ r =>
      r.val.length = nv.val ∧ ∀ i < nv.val, r.val[i]! = marked S E ne.val i ⦄ := by
  unfold topo.strays_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Bool × Usize) => ne.val - y.2.val)
    (fun y => y.2.val ≤ ne.val ∧ y.1.val.length = nv.val ∧
      ∀ i < nv.val, y.1.val[i]! = marked S E y.2.val i)
  · rintro ⟨u, e⟩ ⟨hle, hlen, hu⟩
    simp only at hle hlen hu ⊢
    unfold topo.strays_loop0.body
    by_cases hlt : e.val < ne.val
    · have hlt' : e < ne := by scalar_tac
      simp only [hlt', ite_true]
      apply WP.spec_bind (hse e hlt)
      rintro ⟨a, b⟩ ⟨ha, hb⟩
      show (do
        let used1 ← if a < alloc.vec.Vec.len u then do
            let (_, index_mut_back) ← alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice Bool) u a
            ok (index_mut_back true)
          else ok u
        let used2 ← if b < alloc.vec.Vec.len used1 then do
            let (_, index_mut_back) ← alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice Bool) used1 b
            ok (index_mut_back true)
          else ok used1
        let e1 ← lift (e.wrapping_add 1#usize)
        ok (ControlFlow.cont (used2, e1))) ⦃ _ => _ ⦄
      -- the first end
      have hvl : (alloc.vec.Vec.len u).val = nv.val := by rw [alloc.vec.Vec.len_val]; exact hlen
      have step1 : (if a < alloc.vec.Vec.len u then do
            let (_, index_mut_back) ← alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice Bool) u a
            ok (index_mut_back true)
          else ok u) = ok (if a.val < nv.val then u.set a true else u) := by
        by_cases han : a.val < nv.val
        · have han' : a < alloc.vec.Vec.len u := by scalar_tac
          simp only [han', ite_true, han]
          rw [idxm u a (by omega)]; simp [Std.bind_ok]
        · have han' : ¬ a < alloc.vec.Vec.len u := by scalar_tac
          simp only [han', ite_false, han]
      rw [step1]; simp only [Std.bind_ok]
      set u1 := (if a.val < nv.val then u.set a true else u)
      have hl1 : u1.val.length = nv.val := by
        simp only [u1]; split <;> simp [alloc.vec.Vec.set_val_eq, hlen]
      have hvl1 : (alloc.vec.Vec.len u1).val = nv.val := by rw [alloc.vec.Vec.len_val]; exact hl1
      have step2 : (if b < alloc.vec.Vec.len u1 then do
            let (_, index_mut_back) ← alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice Bool) u1 b
            ok (index_mut_back true)
          else ok u1) = ok (if b.val < nv.val then u1.set b true else u1) := by
        by_cases hbn : b.val < nv.val
        · have hbn' : b < alloc.vec.Vec.len u1 := by scalar_tac
          simp only [hbn', ite_true, hbn]
          rw [idxm u1 b (by omega)]; simp [Std.bind_ok]
        · have hbn' : ¬ b < alloc.vec.Vec.len u1 := by scalar_tac
          simp only [hbn', ite_false, hbn]
      rw [step2]; simp only [Std.bind_ok]
      have hw := succ_val e ne hlt
      simp only [lift, Std.bind_ok, WP.spec_ok]
      set u2 := (if b.val < nv.val then u1.set b true else u1)
      refine ⟨⟨by rw [hw]; omega, ?_, ?_⟩, by rw [hw]; omega⟩
      · simp only [u2]; split <;> simp [alloc.vec.Vec.set_val_eq, hl1]
      · intro i hi
        rw [hw, marked_succ, ← hu i hi]
        simp only [u2, u1]
        have hl0 : i < u.val.length := by omega
        have hl1' : i < u1.val.length := by omega
        by_cases hbn : b.val < nv.val <;> by_cases han : a.val < nv.val <;>
          simp only [hbn, han, ite_true, ite_false, alloc.vec.Vec.set_val_eq, u1] at hl1' ⊢ <;>
          (try rw [getElem!_set _ _ _ _ (by simpa [alloc.vec.Vec.set_val_eq] using hl1')]) <;>
          (try rw [getElem!_set _ _ _ _ hl0]) <;>
          by_cases hib : i = b.val <;> by_cases hia : i = a.val <;>
          simp_all <;> omega
    · have hlt' : ¬ (e < ne) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      have : e.val = ne.val := by omega
      exact ⟨hlen, fun i hi => by rw [hu i hi, this]⟩
  · refine ⟨by simp, hl, fun i hi => by rw [h0 i hi]; simp [marked]⟩

theorem loop1_spec (nv : Usize) (used : alloc.vec.Vec Bool) (hl : used.val.length = nv.val) :
    topo.strays_loop1 used (alloc.vec.Vec.new Std.Usize) 0#usize ⦃ r =>
      r.val.map (·.val) = (List.range nv.val).filter fun i => !used.val[i]! ⦄ := by
  unfold topo.strays_loop1
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Std.Usize × Usize) => nv.val - y.2.val)
    (fun y => y.2.val ≤ nv.val ∧
      y.1.val.map (·.val) = (List.range y.2.val).filter fun i => !used.val[i]!)
  · rintro ⟨o, v⟩ ⟨hle, ho⟩
    simp only at hle ho ⊢
    unfold topo.strays_loop1.body
    have hvl : (alloc.vec.Vec.len used).val = nv.val := by rw [alloc.vec.Vec.len_val]; exact hl
    by_cases hlt : v.val < nv.val
    · have hlt' : v < alloc.vec.Vec.len used := by scalar_tac
      simp only [hlt', ite_true]
      rw [idx used v (by omega)]
      simp only [Std.bind_ok]
      have hw := succ_val v nv hlt
      have hlen : o.val.length ≤ v.val := by
        have := congrArg List.length ho
        simp only [List.length_map] at this
        rw [this]; exact (List.length_filter_le _ _).trans (by simp)
      by_cases hb : used.val[v.val]! = true
      · simp only [hb, ite_true, lift, Std.bind_ok, WP.spec_ok]
        refine ⟨⟨by rw [hw]; omega, ?_⟩, by rw [hw]; omega⟩
        rw [hw, List.range_succ, List.filter_append, ho]; simp_all
      · simp only [hb, Bool.false_eq_true, ite_false]
        have hmax : o.val.length < Usize.max := by
          have : nv.val ≤ Usize.max := le_max nv
          omega
        apply WP.spec_bind (alloc.vec.Vec.push_spec o v hmax)
        intro o1 ho1
        simp only [lift, Std.bind_ok, WP.spec_ok]
        refine ⟨⟨by rw [hw]; omega, ?_⟩, by rw [hw]; omega⟩
        rw [hw, List.range_succ, List.filter_append, ho1, List.map_append, ho]
        simp_all
    · have hlt' : ¬ (v < alloc.vec.Vec.len used) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      have : v.val = nv.val := by omega
      rw [ho, this]
  · exact ⟨by simp, by simp⟩

/-- The shipped `strays` returns `strayRef`'s list, for every instance with these values. -/
theorem strays_spec (NV NE : Usize) (S E : Nat → Nat)
    (hnv : TopoInst.n_vertices t = ok NV) (hne : TopoInst.n_edges t = ok NE)
    (hse : ∀ e : Usize, e.val < NE.val →
      TopoInst.edge_ends t e ⦃ p => p.1.val = S e.val ∧ p.2.val = E e.val ⦄) :
    topo.strays TopoInst t ⦃ r => r.val.map (·.val) = strayRef NV.val NE.val S E ⦄ := by
  unfold topo.strays
  rw [hnv]; simp only [Std.bind_ok]
  apply WP.spec_bind (alloc.vec.from_elem_spec core.clone.CloneBool false NV rfl)
  rintro used ⟨hused, _⟩
  rw [hne]; simp only [Std.bind_ok]
  have hl : used.val.length = NV.val := by rw [hused]; simp
  have h0 : ∀ i < NV.val, used.val[i]! = false := by
    intro i hi; rw [hused]; simp [hi]
  apply WP.spec_bind (loop0_spec TopoInst t NV NE S E hse used hl h0)
  rintro u1 ⟨hl1, hm⟩
  apply WP.spec_mono (loop1_spec NV u1 hl1)
  intro r hr
  rw [hr, strayRef]
  apply List.filter_congr
  intro i hi
  rw [hm i (List.mem_range.1 hi)]
  rfl

end

theorem filterMap_ite_eq {α β} (l : List α) (p : α → Bool) (g : α → β) :
    l.filterMap (fun v => if p v = true then none else some (g v)) =
      (l.filter fun v => !p v).map g := by
  induction l with
  | nil => rfl
  | cons a t ih => cases h : p a <;> simp [h, ih]

/-- The model's `checkStrays` is `strayRef` of the model body's counts and ends. -/
theorem checkStrays_eq (b : TopoCheck.Body) :
    b.checkStrays = (strayRef b.nv b.edges.size (fun k => (b.ed k).start) (fun k => (b.ed k).stop)).map
      TopoCheck.Problem.strayVertex := by
  unfold TopoCheck.Body.checkStrays strayRef
  rw [filterMap_ite_eq]
  congr 1
  apply List.filter_congr
  intro v _
  have : (b.edges.any fun e => e.start == v || e.stop == v) =
      ((List.range b.edges.size).any fun k => (b.ed k).start == v || (b.ed k).stop == v) := by
    rw [Bool.eq_iff_iff, Array.any_eq_true, List.any_eq_true]
    constructor
    · rintro ⟨i, hi, hp⟩
      refine ⟨i, List.mem_range.2 hi, ?_⟩
      simpa [TopoCheck.Body.ed, Array.getD_eq_getD_getElem?, hi] using hp
    · rintro ⟨i, hi, hp⟩
      have hi' := List.mem_range.1 hi
      refine ⟨i, hi', ?_⟩
      simpa [TopoCheck.Body.ed, Array.getD_eq_getD_getElem?, hi'] using hp
  rw [this]

/-- **The shipped `check_strays` refines the model.** For any `Topo` view whose counts and
edge ends are the model body's (as `Body`'s instance is, by `verify/forge-topo-check`'s encoding),
the translated Rust `strays` terminates without panic or overflow, and its result, as problems,
is exactly `Model.checkStrays`. -/
theorem strays_refines {T : Type} (TopoInst : topo.Topo T) (t : T) (b : TopoCheck.Body) (NV NE : Usize)
    (hnv : TopoInst.n_vertices t = ok NV) (hne : TopoInst.n_edges t = ok NE)
    (hnv' : NV.val = b.nv) (hne' : NE.val = b.edges.size)
    (hse : ∀ e : Usize, e.val < NE.val → TopoInst.edge_ends t e ⦃ p =>
      p.1.val = (b.ed e.val).start ∧ p.2.val = (b.ed e.val).stop ⦄) :
    topo.strays TopoInst t ⦃ r => (r.val.map (·.val)).map TopoCheck.Problem.strayVertex =
      b.checkStrays ⦄ := by
  have h := strays_spec TopoInst t NV NE (fun k => (b.ed k).start) (fun k => (b.ed k).stop)
    hnv hne hse
  apply WP.spec_mono h
  intro r hr
  rw [hr, checkStrays_eq, hnv', hne']

end forge_topo
