import ForgeWeldRs.Funs

open Aeneas Aeneas.Std Result forge_weld

namespace ForgeWeldRs

abbrev Seg := forge_ops.profile.Segment
abbrev P2 := forge_math.vec.Point2
abbrev V2 := forge_math.vec.Vec2

/-- `p · w` as forge_math's `Vec2::dot` computes it: the two products, then their sum. -/
def dot (p : P2) (w : V2) : F64 := F64.Ops.add (F64.Ops.mul p.x w.x) (F64.Ops.mul p.y w.y)

/-- `top`: `v` when it is greater, else `best`. -/
def top (best v : F64) : F64 := if F64.Ops.gt v best then v else best

/-- A run of lines' reach along `w`, as the loop takes it: each line's start, then its end. -/
def reach (w : V2) : F64 → List Seg → F64
  | b, .Line a c :: r => reach w (top (top b (dot a w)) (dot c w)) r
  | b, _ => b

def IsLine : Seg → Prop
  | .Line _ _ => True
  | _ => False

theorem top_eq (b v : F64) : section.top b v = ok (top b v) := by
  unfold section.top top; split <;> rfl

theorem loop_lines (l : Slice Seg) (w : V2) (wl : F64) :
    ∀ (n : Nat) (best wa : F64) (ang : Bool) (i : Usize), l.length - i.val = n → i.val ≤ l.length →
      (∀ s ∈ l.val.drop i.val, IsLine s) →
      section.support_loop_loop l w best wl wa ang i = ok (reach w best (l.val.drop i.val)) := by
  intro n
  induction n with
  | zero =>
    intro best wa ang i hn hi _
    have he : i.val = l.length := by omega
    unfold section.support_loop_loop
    rw [loop]
    simp [section.support_loop_loop.body, he, reach]
  | succ n ih =>
    intro best wa ang i hn hi hl
    have hlt : i.val < l.length := by omega
    have hd : l.val.drop i.val = l.val[i.val] :: l.val.drop (i.val + 1) := List.drop_eq_getElem_cons hlt
    have hs : IsLine l.val[i.val] := hl _ (by rw [hd]; exact List.mem_cons_self ..)
    rw [hd] at hl ⊢
    obtain ⟨a, c, hac⟩ : ∃ a c, l.val[i.val] = .Line a c := by
      revert hs; cases l.val[i.val] <;> simp [IsLine]
    have hmax := l.length_ineq
    have hsz : Usize.max < Usize.size := by
      simp only [Usize.max_def, Usize.size_def]
      have : 0 < 2 ^ Usize.numBits := Nat.two_pow_pos _
      omega
    have h2 : i.val + 1 ≤ l.val.length := hlt
    have h1 : (Std.Usize.wrapping_add i 1#usize).val = i.val + 1 := by
      simp only [Std.Usize.wrapping_add_val_eq, UScalar.size_UScalarTyUsize]
      apply Nat.mod_eq_of_lt
      simp; omega
    have hidx : Slice.index_usize l i = ok (.Line a c) := by
      simp [Slice.index_usize, List.getElem?_eq_getElem hlt, hac]
    have step : section.support_loop_loop.body l wl w best wa ang i =
        ok (.cont (w, top (top best (dot a w)) (dot c w), wa, ang, Std.Usize.wrapping_add i 1#usize)) := by
      simp only [section.support_loop_loop.body]
      simp [hlt, hidx, forge_ops.profile.Segment.start, forge_ops.profile.Segment.end,
        section.arc_of, top_eq, lift, forge_math.vec.Point2.to_vec, forge_math.vec.Vec2.new,
        forge_math.vec.Vec2.dot, dot]
    have ih' := ih (top (top best (dot a w)) (dot c w)) wa ang (Std.Usize.wrapping_add i 1#usize)
      (by omega) (by omega) (by rw [h1]; intro s hs'; exact hl s (List.mem_cons_of_mem _ hs'))
    rw [h1] at ih'
    unfold section.support_loop_loop at ih' ⊢
    rw [loop]
    simp only [step, bind_ok]
    rw [ih', hac]
    rfl

/-- **The translated `support_loop` on a loop of lines** is the fold of `top` over the float dot
products of the lines' ends, from `-∞`: what the Rust computes, step for step, with nothing
opaque reached (no arc, so no angle). -/
theorem support_loop_lines (l : Slice Seg) (w : V2) (hl : ∀ s ∈ l.val, IsLine s) :
    section.support_loop l w = ok (reach w (.inf true) l.val) := by
  unfold section.support_loop
  have h := loop_lines l w (IEEE754.FP.sqrt (dot ⟨w.x, w.y⟩ w)) l.length (.inf true) (F64.Ops.ofBits 0)
    false 0#usize (by simp) (by simp) (by simpa using hl)
  have hinf : core.f64.F64.NEG_INFINITY = ok (.inf true) := by
    simp only [core.f64.F64.NEG_INFINITY]; rfl
  simp only [hinf, bind_ok, forge_math.vec.Vec2.length, forge_math.vec.Vec2.length_sq,
    forge_math.vec.Vec2.dot, std.f64.F64.sqrt, lift]
  simpa [dot] using h

/-! ## What the fold is: the largest of the float dot products -/

/-- The ends of a run of lines, in the loop's order. -/
def ends : List Seg → List P2
  | .Line a c :: r => a :: c :: ends r
  | _ => []

theorem reach_eq (w : V2) : ∀ (b : F64) (L : List Seg),
    reach w b L = ((ends L).map (dot · w)).foldl top b
  | b, .Line a c :: r => by simp only [reach, ends, List.map_cons, List.foldl_cons]; exact reach_eq w _ r
  | _, [] => rfl
  | _, .Arc .. :: _ => rfl
  | _, .Circle .. :: _ => rfl
  | _, .Spline .. :: _ => rfl

/-- `-∞` or finite. -/
def Low (b : F64) : Prop := b = .inf true ∨ b.isFinite

theorem top_step {b v : F64} (hb : Low b) (hv : v.isFinite) :
    (top b v).isFinite ∧ v.val ≤ (top b v).val ∧ (b.isFinite → b.val ≤ (top b v).val) ∧
      (top b v = b ∨ top b v = v) := by
  unfold top
  rcases hb with rfl | hb
  · have : (IEEE754.FP.inf true).lt v = true := by
      simp only [IEEE754.FP.lt, IEEE754.FP.cmp]
      cases v <;> simp_all [IEEE754.FP.isFinite]
    simp only [F64.Ops.gt, this, ite_true]
    exact ⟨hv, le_refl _, fun h => by simp [IEEE754.FP.isFinite] at h, by simp⟩
  · simp only [F64.Ops.gt]
    by_cases h : (IEEE754.FP.lt b v) = true
    · have := (IEEE754.FP.lt_iff hb hv).mp h
      simp only [h, ite_true]; exact ⟨hv, le_refl _, fun _ => this.le, by simp⟩
    · have := (IEEE754.FP.lt_iff hb hv).not.mp h
      simp only [Bool.not_eq_true] at h
      simp only [h]; exact ⟨hb, by simpa using this, fun _ => le_refl _, by simp⟩

theorem fold_max : ∀ (vs : List F64) {b : F64}, Low b → (∀ v ∈ vs, v.isFinite) →
    Low (vs.foldl top b) ∧ (∀ v ∈ vs, v.val ≤ (vs.foldl top b).val ∧ (vs.foldl top b).isFinite) ∧
      (b.isFinite → b.val ≤ (vs.foldl top b).val) ∧ (vs.foldl top b = b ∨ vs.foldl top b ∈ vs)
  | [], b, hb, _ => ⟨hb, by simp, fun _ => le_refl _, Or.inl rfl⟩
  | v :: vs, b, hb, hv => by
    obtain ⟨f1, f2, f3, f4⟩ := top_step hb (hv v (List.mem_cons_self ..))
    obtain ⟨g1, g2, g3, g4⟩ := fold_max vs (Or.inr f1) (fun u hu => hv u (List.mem_cons_of_mem _ hu))
    simp only [List.foldl_cons]
    refine ⟨g1, ?_, fun h => le_trans (f3 h) (g3 f1), ?_⟩
    · intro u hu
      rcases List.mem_cons.mp hu with rfl | hu
      · exact ⟨le_trans f2 (g3 f1), by
          rcases g4 with e | e
          · rw [e]; exact f1
          · exact hv _ (List.mem_cons_of_mem _ e)⟩
      · exact g2 u hu
    · rcases g4 with e | e
      · rw [e]; rcases f4 with e' | e'
        · exact Or.inl e'
        · rw [e']; exact Or.inr List.mem_cons_self
      · exact Or.inr (List.mem_cons_of_mem _ e)

/-- **`support_loop` on a loop of lines returns the largest float dot product of its ends**: it
is one of them, finite, and no less than any (where each is finite: no overflow, no NaN). -/
theorem support_loop_max (l : Slice Seg) (w : V2) (hl : ∀ s ∈ l.val, IsLine s)
    (hne : ends l.val ≠ []) (hfin : ∀ p ∈ ends l.val, (dot p w).isFinite) :
    ∃ r, section.support_loop l w = ok r ∧ r.isFinite ∧ (∃ p ∈ ends l.val, r = dot p w) ∧
      ∀ p ∈ ends l.val, (dot p w).val ≤ r.val := by
  rw [support_loop_lines l w hl, reach_eq]
  obtain ⟨_, h2, _, h4⟩ := fold_max ((ends l.val).map (dot · w)) (b := .inf true) (Or.inl rfl)
    (by simpa using hfin)
  obtain ⟨p, hp⟩ := List.exists_mem_of_ne_nil _ hne
  refine ⟨_, rfl, (h2 _ (List.mem_map_of_mem hp)).2, ?_, fun q hq => (h2 _ (List.mem_map_of_mem hq)).1⟩
  rcases h4 with e | e
  · exfalso; have := (h2 _ (List.mem_map_of_mem hp)).2; rw [e] at this; simp [IEEE754.FP.isFinite] at this
  · obtain ⟨q, hq, e⟩ := List.mem_map.mp e; exact ⟨q, hq, e.symm⟩

/-! ## Against the exact dot product -/

/-- `p · w` exactly, over ℚ, from the floats' values. -/
def dotQ (p : P2) (w : V2) : ℚ := p.x.val * w.x.val + p.y.val * w.y.val

/-- The bound on one end's rounding: the unit roundoff `2⁻⁵³` times the sizes of the computed sum
and products, plus two half-subnormals (`2⁻¹⁰⁷⁵` each) for the products. A posteriori: from
the floats the Rust computed. -/
def dotErr (p : P2) (w : V2) : ℚ :=
  2 ^ (-(53 : ℤ)) * (|(dot p w).val| + |(F64.Ops.mul p.x w.x).val| + |(F64.Ops.mul p.y w.y).val|) +
    2 * 2 ^ (-(1075 : ℤ))

theorem dot_err (p : P2) (w : V2) (h : (dot p w).isFinite) :
    |(dot p w).val - dotQ p w| ≤ dotErr p w := by
  unfold dot at h
  obtain ⟨h1, h2⟩ := IEEE754.isFinite_of_add h
  obtain ⟨hpx, hwx⟩ := IEEE754.isFinite_of_mul h1
  obtain ⟨hpy, hwy⟩ := IEEE754.isFinite_of_mul h2
  have ea := IEEE754.add_err_res h1 h2 h
  have e1 := IEEE754.mul_err_abs hpx hwx h1
  have e2 := IEEE754.mul_err_abs hpy hwy h2
  simp only [IEEE754.binary64_prec, IEEE754.binary64_qmin] at ea e1 e2
  unfold dotErr dotQ dot
  simp only [F64.Ops.add, F64.Ops.mul] at ea e1 e2 ⊢
  have : (-1074 : ℤ) - 1 = -1075 := by norm_num
  rw [this] at e1 e2
  have tri : |(IEEE754.FP.add (IEEE754.FP.mul p.x w.x) (IEEE754.FP.mul p.y w.y)).val -
      (p.x.val * w.x.val + p.y.val * w.y.val)| ≤
      |(IEEE754.FP.add (IEEE754.FP.mul p.x w.x) (IEEE754.FP.mul p.y w.y)).val -
        ((IEEE754.FP.mul p.x w.x).val + (IEEE754.FP.mul p.y w.y).val)| +
      |(IEEE754.FP.mul p.x w.x).val - p.x.val * w.x.val| +
      |(IEEE754.FP.mul p.y w.y).val - p.y.val * w.y.val| := by
    have := abs_add_three
      ((IEEE754.FP.add (IEEE754.FP.mul p.x w.x) (IEEE754.FP.mul p.y w.y)).val -
        ((IEEE754.FP.mul p.x w.x).val + (IEEE754.FP.mul p.y w.y).val))
      ((IEEE754.FP.mul p.x w.x).val - p.x.val * w.x.val)
      ((IEEE754.FP.mul p.y w.y).val - p.y.val * w.y.val)
    convert this using 2; ring
  simp only [Nat.cast_ofNat] at ea e1 e2
  linarith

/-- **The Rust's support of a loop of lines is the exact support of its corners, to within the
rounding of one dot product**: with `r` what `support_loop` returns, every corner `p` has
`p · w ≤ r + dotErr p`, and some corner `q` has `r - dotErr q ≤ q · w`. With
research/weld-lean's `support_of_corners` (the support of a polygonal section is the largest
`p · w` over its corners), the float support is the section's exact support to within the
largest `dotErr`. -/
theorem support_loop_near (l : Slice Seg) (w : V2) (hl : ∀ s ∈ l.val, IsLine s)
    (hne : ends l.val ≠ []) (hfin : ∀ p ∈ ends l.val, (dot p w).isFinite) :
    ∃ r, section.support_loop l w = ok r ∧ r.isFinite ∧
      (∀ p ∈ ends l.val, dotQ p w ≤ r.val + dotErr p w) ∧
      ∃ q ∈ ends l.val, r.val - dotErr q w ≤ dotQ q w := by
  obtain ⟨r, hr, rf, ⟨q, hq, rq⟩, hmax⟩ := support_loop_max l w hl hne hfin
  refine ⟨r, hr, rf, fun p hp => ?_, q, hq, ?_⟩
  · have := abs_le.mp (dot_err p w (hfin p hp)); linarith [hmax p hp]
  · have := abs_le.mp (dot_err q w (hfin q hq)); rw [rq]; linarith

end ForgeWeldRs
