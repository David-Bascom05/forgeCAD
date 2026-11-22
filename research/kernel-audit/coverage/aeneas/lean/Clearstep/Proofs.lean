import Clearstep.Funs
open Aeneas Aeneas.Std Result

/-!
# The extracted `interior_point` refines a pure reference, and never fails

`Clearstep/Funs.lean` is Aeneas's translation of `aeneas/clearstep/src/lib.rs` (charon 9e193f0,
aeneas 2e3d4488), the float-free `interior_point`/`clear_step` (boolean.rs:6150-6207). Each loop
is proven with `loop.spec_decr_nat` to return what a pure reference returns; `interior_point_spec`:
on sized tables it returns ok, equal to `ipRef`.
-/

namespace clearstep

theorem idxu {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_usize v i = ok v.val[i.val]! := by
  simp only [alloc.vec.Vec.index_usize]
  have : v[i.val]? = some v.val[i.val]! := by
    simp [getElem!_pos, h]
  rw [this]

theorem idx {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok v.val[i.val]! := by
  rw [alloc.vec.Vec.index_slice_index, idxu v i h]

abbrev Loops := alloc.vec.Vec (alloc.vec.Vec (Usize × Bool))
abbrev T3 := alloc.vec.Vec (alloc.vec.Vec (alloc.vec.Vec Bool))
abbrev T4 := alloc.vec.Vec (alloc.vec.Vec (alloc.vec.Vec (alloc.vec.Vec Bool)))

/-- A loop's half-edges, as naturals. -/
def lv (l : alloc.vec.Vec (Usize × Bool)) : List (Nat × Bool) := l.val.map fun o => (o.1.val, o.2)
/-- The region's loops, as naturals. -/
def LV (L : Loops) : List (List (Nat × Bool)) := L.val.map lv

theorem lv_len (l : alloc.vec.Vec (Usize × Bool)) : (lv l).length = l.val.length := by simp [lv]
theorem LV_len (L : Loops) : (LV L).length = L.val.length := by simp [LV]

theorem lv_get (l : alloc.vec.Vec (Usize × Bool)) (j : Nat) (h : j < l.val.length) :
    (lv l)[j]! = ((l.val[j]!).1.val, (l.val[j]!).2) := by
  simp [lv, getElem!_pos, h]

theorem LV_get (L : Loops) (i : Nat) (h : i < L.val.length) : (LV L)[i]! = lv (L.val[i]!) := by
  simp [LV, getElem!_pos, h]

/-- The tables, as functions on naturals. -/
def row1 (r : alloc.vec.Vec Bool) (o : Nat) : Bool := r.val[o]!
def sm (s : alloc.vec.Vec Bool) (p : Nat) : Bool := s.val[p]!
def ns (n : alloc.vec.Vec Usize) (p : Nat) : Nat := (n.val[p]!).val
def pr (t : T3) (p f k : Nat) : Bool := t.val[p]!.val[f]!.val[k]!
def clr (t : T4) (p f k : Nat) : alloc.vec.Vec Bool := t.val[p]!.val[f]!.val[k]!
def cl (t : T4) (p f k o : Nat) : Bool := row1 (clr t p f k) o
def fb (t : alloc.vec.Vec (alloc.vec.Vec Bool)) (p f : Nat) : Bool := t.val[p]!.val[f]!
def fiN (f : Bool) : Nat := if f then 1 else 0

theorem fi_spec (f : Bool) : fi f ⦃ r => r.val = fiN f ⦄ := by
  unfold fi fiN; cases f <;> simp

/-! ## `clear_all` -/

def clrIn (row : Nat → Bool) (h0 : Nat) (l : List (Nat × Bool)) (ok : Bool) (j : Nat) : Bool :=
  if j < l.length then
    clrIn row h0 l (if l[j]!.1 ≠ h0 then (if row l[j]!.1 then ok else false) else ok) (j + 1)
  else ok
termination_by l.length - j

def clrOut (row : Nat → Bool) (h0 : Nat) (L : List (List (Nat × Bool))) (ok : Bool) (i : Nat) : Bool :=
  if i < L.length then clrOut row h0 L (clrIn row h0 L[i]! ok 0) (i + 1) else ok
termination_by L.length - i

def clearAllRef (row : Nat → Bool) (h0 : Nat) (L : List (List (Nat × Bool))) : Bool := clrOut row h0 L true 0

/-- Every piece index in the loops is below `P`, and the row has `P` entries. -/
def PiecesBelow (P : Nat) (L : Loops) : Prop := ∀ l ∈ LV L, ∀ o ∈ l, o.1 < P

theorem mem_lv (l : alloc.vec.Vec (Usize × Bool)) (j : Nat) (h : j < l.val.length) :
    ((l.val[j]!).1.val, (l.val[j]!).2) ∈ lv l := by
  rw [← lv_get l j h, getElem!_pos (lv l) j (by rw [lv_len]; exact h)]; apply List.getElem_mem

theorem mem_LV (L : Loops) (i : Nat) (h : i < L.val.length) : lv (L.val[i]!) ∈ LV L := by
  rw [← LV_get L i h, getElem!_pos (LV L) i (by rw [LV_len]; exact h)]; apply List.getElem_mem

theorem clear_all_in_spec (row : alloc.vec.Vec Bool) (h0 : Usize) (l : alloc.vec.Vec (Usize × Bool))
    (P : Nat) (hP : P ≤ row.val.length) (hl : ∀ o ∈ lv l, o.1 < P) (ok1 : Bool) (j : Usize)
    (hj : j.val ≤ l.val.length) :
    clear_all_loop0_loop0 row h0 ok1 l j ⦃ r => r = clrIn (row1 row) h0 (lv l) ok1 j ⦄ := by
  unfold clear_all_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => l.val.length - x.2.val)
    (fun x => x.2.val ≤ l.val.length ∧ clrIn (row1 row) h0 (lv l) x.1 x.2 = clrIn (row1 row) h0 (lv l) ok1 j)
  · rintro ⟨c, j1⟩ ⟨hj1, heq⟩
    unfold clear_all_loop0_loop0.body
    simp only at hj1 heq ⊢
    rw [← heq]
    by_cases hlt : j1.val < l.val.length
    · have hlt' : j1 < l.len := by scalar_tac
      simp only [hlt', ite_true]
      rw [idx l j1 hlt]; simp only [Std.bind_ok]
      have hlt2 : j1.val < (lv l).length := by rw [lv_len]; exact hlt
      conv => enter [2, r]; rw [clrIn.eq_1 (j := j1.val), if_pos hlt2, lv_get l j1.val hlt]
      have ho : (l.val[j1.val]!).1.val < P := hl _ (mem_lv l j1.val hlt)
      generalize hx : l.val[j1.val]! = x at ho ⊢
      obtain ⟨o, ob⟩ := x
      simp only [Std.uncurry] at ho ⊢
      by_cases hne : o = h0
      · have hne' : ¬ (o.val ≠ h0.val) := by simp [hne]
        simp only [hne, bne_self_eq_false, Bool.false_eq_true, not_true_eq_false, ite_false, ne_eq, not_false_eq_true]
        step as ⟨j2, hj2⟩
        refine ⟨by omega, ?_, by omega⟩
        rw [hj2]
      · have hne' : o.val ≠ h0.val := fun h => hne (by scalar_tac)
        have hb : (o != h0) = true := by simp [hne]
        simp only [hb, hne', not_false_eq_true, ite_true, ne_eq]
        rw [idx row _ (by omega)]; simp only [Std.bind_ok]
        split
        all_goals
          simp only [Std.bind_ok]
          step as ⟨j2, hj2⟩
          refine ⟨by omega, ?_, by omega⟩
          rw [hj2]; simp_all [row1]
    · have hlt' : ¬ (j1 < l.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [clrIn, if_neg (by rw [lv_len]; exact hlt)]
  · exact ⟨hj, rfl⟩

theorem clear_all_spec (loops : Loops) (row : alloc.vec.Vec Bool) (h0 : Usize) (P : Nat)
    (hP : P ≤ row.val.length) (hL : PiecesBelow P loops) :
    clear_all_loop0 loops row h0 true 0#usize ⦃ r => r = clearAllRef (row1 row) h0 (LV loops) ⦄ := by
  unfold clear_all_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => loops.val.length - x.2.val)
    (fun x => x.2.val ≤ loops.val.length ∧ clrOut (row1 row) h0 (LV loops) x.1 x.2 = clearAllRef (row1 row) h0 (LV loops))
  · rintro ⟨c, i1⟩ ⟨hi1, heq⟩
    unfold clear_all_loop0.body
    simp only at hi1 heq ⊢
    rw [← heq]
    by_cases hlt : i1.val < loops.val.length
    · have hlt' : i1 < loops.len := by scalar_tac
      simp only [hlt', ite_true]
      rw [idx loops i1 hlt]; simp only [Std.bind_ok]
      apply WP.spec_bind (clear_all_in_spec row h0 _ P hP (hL _ (mem_LV loops i1.val hlt)) c 0#usize (by simp))
      intro c2 hc2
      step as ⟨i2, hi2⟩
      refine ⟨by omega, ?_, by omega⟩
      conv => rhs; rw [clrOut.eq_1 (i := i1.val), if_pos (by rw [LV_len]; exact hlt)]
      rw [hi2, hc2, LV_get loops i1.val hlt]; rfl
    · have hlt' : ¬ (i1 < loops.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [clrOut, if_neg (by rw [LV_len]; exact hlt)]
  · exact ⟨by simp, rfl⟩

/-! ## `clear_step` -/

inductive StepR | fail | found (k : Nat) | exh
  deriving DecidableEq, Repr

def Step.toR : Step → StepR
  | .Fail => .fail
  | .Found k => .found k.val
  | .Exhausted => .exh

def csIn (L : List (List (Nat × Bool))) (P : Nat → Bool) (row : Nat → Nat → Bool) (p n : Nat)
    (res : StepR) (stop : Bool) (k : Nat) : StepR :=
  if k < n then
    if stop then res
    else if P k then
      (if clearAllRef (row k) p L then csIn L P row p n (.found k) true (k + 1)
       else csIn L P row p n res stop (k + 1))
    else csIn L P row p n .fail true (k + 1)
  else res
termination_by n - k

/-- Every table entry the code reads is there. -/
structure Sized (P : Nat) (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool)) : Prop where
  pieces : PiecesBelow P loops
  hs : P ≤ small.val.length
  hn : P ≤ nsteps.val.length
  hp : P ≤ probed.val.length
  hc : P ≤ clear.val.length
  hf : P ≤ fallback.val.length
  hp2 : ∀ p < P, 2 ≤ (probed.val[p]!).val.length ∧
    ∀ f < 2, ns nsteps p ≤ (probed.val[p]!.val[f]!).val.length
  hc2 : ∀ p < P, 2 ≤ (clear.val[p]!).val.length ∧
    ∀ f < 2, ns nsteps p ≤ (clear.val[p]!.val[f]!).val.length ∧
      ∀ k < ns nsteps p, P ≤ (clear.val[p]!.val[f]!.val[k]!).val.length
  hf2 : ∀ p < P, 2 ≤ (fallback.val[p]!).val.length

/-- `clear_step` off half-edge (p, f), as the reference computes it. -/
def csRef (L : List (List (Nat × Bool))) (nsteps : alloc.vec.Vec Usize) (probed : T3) (clear : T4)
    (p : Nat) (f : Bool) : StepR :=
  csIn L (pr probed p (fiN f)) (fun k => row1 (clr clear p (fiN f) k)) p (ns nsteps p) .exh false 0

theorem fiN_lt (f : Bool) : fiN f < 2 := by cases f <;> simp [fiN]

theorem clear_step_spec (P : Nat) (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool))
    (hS : Sized P loops small nsteps probed clear fallback) (p : Usize) (hp : p.val < P) (f : Bool) :
    clear_step loops nsteps probed clear p f ⦃ r => r.toR = csRef (LV loops) nsteps probed clear p f ⦄ := by
  unfold clear_step
  rw [idx nsteps p (by have := hS.hn; omega)]; simp only [Std.bind_ok]
  apply WP.spec_bind (fi_spec f)
  intro fx hfx
  have hfl : fx.val < 2 := by rw [hfx]; exact fiN_lt f
  obtain ⟨hp1, hp2⟩ := hS.hp2 p hp
  obtain ⟨hc1, hc2⟩ := hS.hc2 p hp
  have hpf := hp2 fx.val hfl
  obtain ⟨hcf, hck⟩ := hc2 fx.val hfl
  have hn : (nsteps.val[p.val]!).val = ns nsteps p := rfl
  have hPp := hS.hp
  have hPc := hS.hc
  unfold clear_step_loop
  apply loop.spec_decr_nat (fun (x : Step × Bool × Usize) => ns nsteps p - x.2.2.val)
    (fun x => x.2.2.val ≤ ns nsteps p ∧
      csIn (LV loops) (pr probed p fx) (fun k => row1 (clr clear p fx k)) p (ns nsteps p) x.1.toR x.2.1 x.2.2 =
      csRef (LV loops) nsteps probed clear p f)
  · rintro ⟨res, stop, k⟩ ⟨hk, heq⟩
    unfold clear_step_loop.body
    simp only at hk heq ⊢
    rw [← heq]
    by_cases hlt : k.val < ns nsteps p
    · have hlt' : k < nsteps.val[p.val]! := by scalar_tac
      simp only [hlt', ite_true]
      conv => enter [2, r]; rw [csIn.eq_1 (k := k.val), if_pos hlt]
      cases stop
      · simp only [Bool.false_eq_true, ite_false]
        rw [idx probed p (by omega)]; simp only [Std.bind_ok]
        rw [idx _ fx (by omega)]; simp only [Std.bind_ok]
        rw [idx _ k (by omega)]; simp only [Std.bind_ok]
        have hpk : (probed.val[p.val]!.val[fx.val]!.val[k.val]!) = pr probed p fx k := rfl
        rw [hpk]
        cases hb : pr probed p fx k
        · simp only [Bool.false_eq_true, ite_false, Std.bind_ok]
          step as ⟨k2, hk2⟩
          refine ⟨by omega, ?_, by omega⟩
          rw [hk2]; rfl
        · simp only [ite_true]
          rw [idx clear p (by omega)]; simp only [Std.bind_ok]
          rw [idx _ fx (by omega)]; simp only [Std.bind_ok]
          rw [idx _ k (by omega)]; simp only [Std.bind_ok]
          simp only [clear_all, Std.bind_assoc]
          apply WP.spec_bind (clear_all_spec loops _ p P (hck k.val hlt) hS.pieces)
          intro b1 hb1
          have hrow : row1 (clear.val[p.val]!.val[fx.val]!.val[k.val]!) = row1 (clr clear p fx k) := rfl
          rw [hrow] at hb1
          subst hb1
          split
          all_goals
            rename_i hb2
            simp only [Std.bind_ok]
            step as ⟨k2, hk2⟩
            refine ⟨by omega, ?_, by omega⟩
            rw [hk2]; try simp only [hb2]
            all_goals rfl
      · simp only [ite_true, WP.spec_ok]
    · have hlt' : ¬ (k < nsteps.val[p.val]!) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [csIn, if_neg hlt]
  · refine ⟨by simp, ?_⟩
    simp only [csRef, hfx]; rfl

/-! ## `pick` -/

def pickIn (first : List (Nat × Bool)) (sm : Nat → Bool) (h : Nat × Bool) (found : Bool) (j : Nat) : Nat × Bool :=
  if j < first.length then
    if found then h
    else if sm first[j]!.1 then pickIn first sm h found (j + 1)
    else pickIn first sm first[j]! true (j + 1)
  else h
termination_by first.length - j

def pickRef (first : List (Nat × Bool)) (sm : Nat → Bool) : Nat × Bool :=
  if sm first[0]!.1 then pickIn first sm first[0]! false 0 else first[0]!

theorem gmem {α} [Inhabited α] (l : List α) (j : Nat) (h : j < l.length) : l[j]! ∈ l := by
  rw [getElem!_pos l j h]; apply List.getElem_mem

theorem pickIn_mem (first : List (Nat × Bool)) (sm : Nat → Bool) (h : Nat × Bool) (found : Bool) (j : Nat)
    (hh : h ∈ first) : pickIn first sm h found j ∈ first := by
  fun_induction pickIn first sm h found j
  all_goals
    first
    | assumption
    | (rename_i ih; exact ih hh)
    | (rename_i hj _ _ ih; exact ih (gmem first _ hj))

theorem pickRef_mem (first : List (Nat × Bool)) (sm : Nat → Bool) (h : 0 < first.length) :
    pickRef first sm ∈ first := by
  have h0 : first[0]! ∈ first := by rw [getElem!_pos first 0 h]; apply List.getElem_mem
  unfold pickRef; split
  · exact pickIn_mem _ _ _ _ _ h0
  · exact h0

theorem pick_spec (first : alloc.vec.Vec (Usize × Bool)) (small : alloc.vec.Vec Bool) (P : Nat)
    (hP : P ≤ small.val.length) (hl : ∀ o ∈ lv first, o.1 < P) (h0 : 0 < first.val.length) :
    pick first small ⦃ r => (r.1.val, r.2) = pickRef (lv first) (sm small) ⦄ := by
  unfold pick
  rw [idx first 0#usize (by simpa using h0)]; simp only [Std.bind_ok]
  have hf0 : (lv first)[0]! = ((first.val[0]!).1.val, (first.val[0]!).2) := lv_get first 0 h0
  have hz : (0#usize : Usize).val = 0 := by simp
  rw [hz]
  generalize hx : first.val[0]! = x at hf0
  obtain ⟨i0, b0⟩ := x
  simp only [Std.uncurry] at hf0 ⊢
  have hi0 : i0.val < P := by
    have := hl _ (gmem (lv first) 0 (by rw [lv_len]; exact h0))
    rw [hf0] at this; simpa using this
  rw [idx small i0 (by omega)]; simp only [Std.bind_ok]
  have hsm : small.val[i0.val]! = sm small i0 := rfl
  rw [hsm]
  unfold pickRef
  rw [hf0]
  cases hs : sm small i0
  · simp
  · simp only [ite_true]
    unfold pick_loop
    apply loop.spec_decr_nat (fun (y : (Usize × Bool) × Bool × Usize) => first.val.length - y.2.2.val)
      (fun y => y.2.2.val ≤ first.val.length ∧
        pickIn (lv first) (sm small) (y.1.1.val, y.1.2) y.2.1 y.2.2 = pickIn (lv first) (sm small) (i0.val, b0) false 0)
    · rintro ⟨⟨hi, hb⟩, found, j⟩ ⟨hj, heq⟩
      unfold pick_loop.body
      simp only at hj heq ⊢
      rw [← heq]
      by_cases hlt : j.val < first.val.length
      · have hlt' : j < first.len := by scalar_tac
        have hlt2 : j.val < (lv first).length := by rw [lv_len]; exact hlt
        simp only [hlt', ite_true]
        conv => enter [2, r]; rw [pickIn.eq_1 (j := j.val), if_pos hlt2]
        cases found
        · simp only [Bool.false_eq_true, ite_false]
          rw [idx first j hlt]; simp only [Std.bind_ok]
          rw [lv_get first j.val hlt]
          generalize hy : first.val[j.val]! = y
          obtain ⟨i1, b1⟩ := y
          have hi1 : i1.val < P := by
            have := hl _ (mem_lv first j.val hlt); rw [hy] at this; simpa using this
          simp only [Std.uncurry]
          rw [idx small i1 (by omega)]; simp only [Std.bind_ok]
          have hsm1 : small.val[i1.val]! = sm small i1 := rfl
          rw [hsm1]
          cases hs1 : sm small i1
          all_goals
            simp only [Bool.false_eq_true, ite_false, ite_true, Std.bind_ok]
            step as ⟨j2, hj2⟩
            refine ⟨by omega, ?_, by omega⟩
            rw [hj2]
        · simp only [ite_true, WP.spec_ok]
      · have hlt' : ¬ (j < first.len) := by scalar_tac
        simp only [hlt', ite_false, WP.spec_ok]
        rw [pickIn, if_neg (by rw [lv_len]; exact hlt)]
    · exact ⟨by simp, by simp⟩

/-! ## The second pass, `scan_one` and `scan` -/

inductive R2 | go | fail | found (p : Nat) (f : Bool) (k : Nat)
  deriving DecidableEq, Repr

def Res2.toR : Res2 → R2
  | .Go => .go
  | .Fail => .fail
  | .Found p f k => .found p.val f k.val

theorem is_go_eq (r : Res2) : Res2.is_go r = ok (decide (r.toR = .go)) := by
  cases r <;> simp [Res2.is_go, Res2.toR]

/-- `clear_step`'s outcome for half-edge `o`, as the second pass reads it. -/
def r2of (o : Nat × Bool) : StepR → R2
  | .fail => .fail
  | .found k => .found o.1 o.2 k
  | .exh => .go

def soIn (cs : Nat × Bool → StepR) (l : List (Nat × Bool)) (h : Nat × Bool) (out : R2) (j : Nat) : R2 :=
  if j < l.length then
    if out = .go then
      soIn cs l h (if l[j]!.1 ≠ h.1 ∨ l[j]!.2 ≠ h.2 then r2of l[j]! (cs l[j]!) else out) (j + 1)
    else out
  else out
termination_by l.length - j

def scanIn (cs : Nat × Bool → StepR) (L : List (List (Nat × Bool))) (h : Nat × Bool) (out : R2) (i : Nat) : R2 :=
  if i < L.length then
    if out = .go then scanIn cs L h (soIn cs L[i]! h .go 0) (i + 1) else out
  else out
termination_by L.length - i

theorem r2_match (o : Usize × Bool) (s : Step) :
    (match s with
      | Step.Fail => ok Res2.Fail
      | Step.Found k => ok (Res2.Found o.1 o.2 k)
      | Step.Exhausted => ok Res2.Go : Result Res2) = ok (match s with
      | Step.Fail => Res2.Fail
      | Step.Found k => Res2.Found o.1 o.2 k
      | Step.Exhausted => Res2.Go) := by
  cases s <;> rfl

theorem r2_toR (o : Usize × Bool) (s : Step) :
    (match s with
      | Step.Fail => Res2.Fail
      | Step.Found k => Res2.Found o.1 o.2 k
      | Step.Exhausted => Res2.Go).toR = r2of (o.1.val, o.2) s.toR := by
  cases s <;> rfl

theorem scan_one_spec (P : Nat) (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool))
    (hS : Sized P loops small nsteps probed clear fallback) (l : alloc.vec.Vec (Usize × Bool))
    (hl : lv l ∈ LV loops) (h0 : Usize) (h1 : Bool) :
    scan_one_loop loops l nsteps probed clear h0 h1 Res2.Go 0#usize ⦃ r =>
      r.toR = soIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (lv l) (h0.val, h1) .go 0 ⦄ := by
  unfold scan_one_loop
  apply loop.spec_decr_nat (fun (x : Res2 × Usize) => l.val.length - x.2.val)
    (fun x => x.2.val ≤ l.val.length ∧
      soIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (lv l) (h0.val, h1) x.1.toR x.2 =
      soIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (lv l) (h0.val, h1) .go 0)
  · rintro ⟨out, j⟩ ⟨hj, heq⟩
    unfold scan_one_loop.body
    simp only at hj heq ⊢
    rw [← heq]
    by_cases hlt : j.val < l.val.length
    · have hlt' : j < l.len := by scalar_tac
      have hlt2 : j.val < (lv l).length := by rw [lv_len]; exact hlt
      simp only [hlt', ite_true]
      rw [is_go_eq]; simp only [Std.bind_ok]
      conv => enter [2, r]; rw [soIn.eq_1 (j := j.val), if_pos hlt2]
      by_cases hg : out.toR = .go
      · simp only [hg, decide_true, ite_true]
        rw [idx l j hlt]; simp only [Std.bind_ok]
        rw [lv_get l j.val hlt]
        have hmem := hS.pieces _ hl _ (mem_lv l j.val hlt)
        generalize hy : l.val[j.val]! = y at hmem
        obtain ⟨i1, b1⟩ := y
        simp only [Std.uncurry] at hmem ⊢
        by_cases hne : i1 = h0
        · have hb : (i1 != h0) = false := by simp [hne]
          subst hne
          simp only [hb, Bool.false_eq_true, ite_false]
          by_cases hnb : b1 = h1
          · subst hnb
            simp only [bne_self_eq_false, Bool.false_eq_true, ite_false, Std.bind_ok]
            step as ⟨j2, hj2⟩
            refine ⟨by omega, ?_, by omega⟩
            rw [hj2]; simp [hg]
          · have hb2 : (b1 != h1) = true := by simp [hnb]
            simp only [hb2, ite_true]
            apply WP.spec_bind (clear_step_spec P loops small nsteps probed clear fallback hS i1 hmem b1)
            intro s hs
            rw [r2_match]; simp only [Std.bind_ok]
            step as ⟨j2, hj2⟩
            refine ⟨by omega, ?_, by omega⟩
            rw [hj2, r2_toR, hs]; simp [hnb]
        · have hb : (i1 != h0) = true := by simp [hne]
          have hne' : i1.val ≠ h0.val := fun h => hne (by scalar_tac)
          simp only [hb, ite_true]
          apply WP.spec_bind (clear_step_spec P loops small nsteps probed clear fallback hS i1 hmem b1)
          intro s hs
          rw [r2_match]; simp only [Std.bind_ok]
          step as ⟨j2, hj2⟩
          refine ⟨by omega, ?_, by omega⟩
          rw [hj2, r2_toR, hs]; simp [hne']
      · simp only [hg, decide_false, Bool.false_eq_true, ite_false, WP.spec_ok]
    · have hlt' : ¬ (j < l.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [soIn, if_neg (by rw [lv_len]; exact hlt)]
  · exact ⟨by simp, by simp [Res2.toR]⟩

theorem scan_spec (P : Nat) (loops : Loops) (small : alloc.vec.Vec Bool) (nsteps : alloc.vec.Vec Usize)
    (probed : T3) (clear : T4) (fallback : alloc.vec.Vec (alloc.vec.Vec Bool))
    (hS : Sized P loops small nsteps probed clear fallback) (h0 : Usize) (h1 : Bool) :
    scan_loop loops nsteps probed clear h0 h1 Res2.Go 0#usize ⦃ r =>
      r.toR = scanIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (LV loops) (h0.val, h1) .go 0 ⦄ := by
  unfold scan_loop
  apply loop.spec_decr_nat (fun (x : Res2 × Usize) => loops.val.length - x.2.val)
    (fun x => x.2.val ≤ loops.val.length ∧
      scanIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (LV loops) (h0.val, h1) x.1.toR x.2 =
      scanIn (fun o => csRef (LV loops) nsteps probed clear o.1 o.2) (LV loops) (h0.val, h1) .go 0)
  · rintro ⟨out, i⟩ ⟨hi, heq⟩
    unfold scan_loop.body
    simp only at hi heq ⊢
    rw [← heq]
    by_cases hlt : i.val < loops.val.length
    · have hlt' : i < loops.len := by scalar_tac
      have hlt2 : i.val < (LV loops).length := by rw [LV_len]; exact hlt
      simp only [hlt', ite_true]
      rw [is_go_eq]; simp only [Std.bind_ok]
      conv => enter [2, r]; rw [scanIn.eq_1 (i := i.val), if_pos hlt2]
      by_cases hg : out.toR = .go
      · simp only [hg, decide_true, ite_true]
        rw [idx loops i hlt]; simp only [Std.bind_ok, scan_one]
        apply WP.spec_bind (scan_one_spec P loops small nsteps probed clear fallback hS _ (mem_LV loops i.val hlt) h0 h1)
        intro o ho
        step as ⟨i2, hi2⟩
        refine ⟨by omega, ?_, by omega⟩
        rw [hi2, ho, LV_get loops i.val hlt]
      · simp only [hg, decide_false, Bool.false_eq_true, ite_false, WP.spec_ok]
    · have hlt' : ¬ (i < loops.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [scanIn, if_neg (by rw [LV_len]; exact hlt)]
  · exact ⟨by simp, by simp [Res2.toR]⟩

end clearstep
