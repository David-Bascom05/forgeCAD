import ForgeFont.ReadSimple

open Aeneas Aeneas.Std Result ControlFlow
open forge_font

namespace ForgeFont

/-! # The charstring machine (cff_core.rs): its pen -/

def cpt (p : cff_core.Pc) : ForgeText.Pt := ⟨p.x.val, p.y.val⟩

def cseg : cff_core.SegC → ForgeText.Seg
  | .Line a b => .line (cpt a) (cpt b)
  | .Cubic a c d b => .cubic (cpt a) (cpt c) (cpt d) (cpt b)

/-- A contour that chains from a start and ends back on it. -/
def Closed (c : List cff_core.SegC) : Prop :=
  ∃ s, ForgeText.Chain s (c.map cseg) ∧ ForgeText.endOf s (c.map cseg) = s

/-- The pen's invariant: every contour kept is closed; the open one (if any) chains from its
start to where the pen is, and with none open there is no piece pending. -/
def PenOk (p : cff_core.Pen) : Prop :=
  (∀ c ∈ p.out.val, Closed c.val) ∧
  match p.start with
  | none => p.contour.val = []
  | some s => ForgeText.Chain (cpt s) (p.contour.val.map cseg) ∧
      ForgeText.endOf (cpt s) (p.contour.val.map cseg) = cpt p.at

/-- The pen within `M` of the origin on each axis. -/
def Near (M : Int) (p : cff_core.Pen) : Prop :=
  -M ≤ p.at.x.val ∧ p.at.x.val ≤ M ∧ -M ≤ p.at.y.val ∧ p.at.y.val ≤ M

theorem cpt_eq {p q : cff_core.Pc} : cpt p = cpt q ↔ p = q := by
  obtain ⟨px, py⟩ := p
  obtain ⟨qx, qy⟩ := q
  simp [cpt, IScalar.eq_equiv]

@[step]
theorem pc_eq_spec (p q : cff_core.Pc) :
    cff_core.Pc.Insts.CoreCmpPartialEqPc.eq p q ⦃ b => b ↔ p = q ⦄ := by
  obtain ⟨px, py⟩ := p
  obtain ⟨qx, qy⟩ := q
  unfold cff_core.Pc.Insts.CoreCmpPartialEqPc.eq
  split <;> simp_all

@[step]
theorem pc_ne_spec (p q : cff_core.Pc) :
    core.cmp.PartialEq.ne.trait_default cff_core.Pc.Insts.CoreCmpPartialEqPc p q
      ⦃ b => b ↔ p ≠ q ⦄ :=
  core.cmp.PartialEq.ne.trait_default.spec _ p q (pc_eq_spec p q)

/-- Closing keeps the invariant and leaves nothing open. -/
@[step]
theorem close_spec (p : cff_core.Pen) (h : PenOk p) (hl : p.contour.val.length + 1 < Usize.max)
    (ho : p.out.val.length + 1 < Usize.max) :
    cff_core.close p ⦃ p' => PenOk p' ∧ p'.start = none ∧ p'.at = p.at ∧
      p'.out.val.length ≤ p.out.val.length + 1 ⦄ := by
  obtain ⟨hout, hopen⟩ := h
  obtain ⟨at_, start, contour, out⟩ := p
  unfold cff_core.close
  cases start with
  | none => simp_all [PenOk]
  | some s =>
    simp only at hopen hout hl ho ⊢
    obtain ⟨hc, he⟩ := hopen
    -- the contour with its closing line (if the pen is not on the start)
    have hcl : ∀ v : alloc.vec.Vec cff_core.SegC,
        (v.val = contour.val ++ [cff_core.SegC.Line at_ s] ∧ at_ ≠ s) ∨ (v.val = contour.val ∧ at_ = s) →
        Closed v.val := by
      rintro v (⟨hv, _⟩ | ⟨hv, rfl⟩)
      · refine ⟨cpt s, ?_, ?_⟩
        · rw [hv, List.map_append, List.map_cons, List.map_nil, ForgeText.chain_append]
          exact ⟨hc, by simp [cseg, ForgeText.Seg.start, he]⟩
        · rw [hv, List.map_append, List.map_cons, List.map_nil, ForgeText.endOf_append]; rfl
      · exact ⟨cpt at_, by rw [hv]; exact hc, by rw [hv]; exact he⟩
    step as ⟨b, hb⟩
    apply WP.spec_bind (Pₘ := fun v : alloc.vec.Vec cff_core.SegC =>
      (v.val = contour.val ++ [cff_core.SegC.Line at_ s] ∧ at_ ≠ s) ∨ (v.val = contour.val ∧ at_ = s))
    · split
      · rename_i hbt
        step*
      · rename_i hbt
        simp only [WP.spec_ok]
        exact Or.inr ⟨trivial, by by_contra h; exact hbt (hb.2 h)⟩
    intro v hv
    have hvc := hcl v hv
    step as ⟨b1, hb1⟩
    split
    · simp only [WP.spec_ok, PenOk]
      rename_i h1
      have : v.val = [] := by simp at hb1; simp_all
      exact ⟨⟨hout, this⟩, trivial, trivial, by omega⟩
    · simp only [core.mem.take, alloc.vec.Vec.Insts.CoreDefaultDefault.default, bind_ok,
        uncurry_apply_pair]
      step*
      refine ⟨⟨?_, by simp [alloc.vec.Vec.new]⟩, by simp [v2_post]⟩
      intro c hc'
      rw [v2_post] at hc'
      rcases List.mem_append.1 hc' with h1 | h1
      · exact hout c h1
      · simp at h1; subst h1; exact hvc

/-- The open contour, a piece from the pen to `b` added (or none, the pen not moving). -/
theorem open_add (s a b : cff_core.Pc) (c v : List cff_core.SegC) (g : cff_core.SegC)
    (hg : (cseg g).start = cpt a ∧ (cseg g).stop = cpt b)
    (hop : ForgeText.Chain (cpt s) (c.map cseg) ∧ ForgeText.endOf (cpt s) (c.map cseg) = cpt a)
    (hv : v = c ++ [g] ∨ (v = c ∧ a = b)) :
    ForgeText.Chain (cpt s) (v.map cseg) ∧ ForgeText.endOf (cpt s) (v.map cseg) = cpt b := by
  rcases hv with rfl | ⟨rfl, rfl⟩
  · rw [List.map_append, List.map_cons, List.map_nil, ForgeText.chain_append,
      ForgeText.endOf_append]
    exact ⟨⟨hop.1, by rw [hg.1, hop.2]⟩, hg.2⟩
  · exact hop

/-- A delta within `D` either way. -/
def Small (D : Int) (v : I64) : Prop := -D ≤ v.val ∧ v.val ≤ D

@[step]
theorem move_to_spec (p : cff_core.Pen) (dx dy : I64) (M D : Int) (h : PenOk p) (hn : Near M p)
    (hx : Small D dx) (hy : Small D dy) (hb : M + D ≤ 2^62) (hD : 0 ≤ D)
    (hl : p.contour.val.length + 1 < Usize.max) (ho : p.out.val.length + 1 < Usize.max) :
    cff_core.move_to p dx dy ⦃ p' => PenOk p' ∧ Near (M + D) p' ∧ p'.start.isSome ∧
      p'.contour.val = [] ∧ p'.out.val.length ≤ p.out.val.length + 1 ⦄ := by
  unfold cff_core.move_to
  step with close_spec p h hl ho as ⟨p1, h1, hs1, ha1, hl1⟩
  obtain ⟨hn0, hn1, hn2, hn3⟩ := hn
  obtain ⟨hx0, hx1⟩ := hx
  obtain ⟨hy0, hy1⟩ := hy
  rw [ha1]
  step*
  obtain ⟨hout, hop⟩ := h1
  rw [hs1] at hop
  simp only at hop
  refine ⟨⟨hout, ?_⟩, ?_, by simp, hop, hl1⟩
  · simp [hop, ForgeText.Chain, ForgeText.endOf]
  · simp only [Near]; omega

@[step]
theorem line_spec (p : cff_core.Pen) (dx dy : I64) (M D : Int) (h : PenOk p) (hn : Near M p)
    (hx : Small D dx) (hy : Small D dy) (hb : M + D ≤ 2^62) (hD : 0 ≤ D)
    (hl : p.contour.val.length + 1 < Usize.max) :
    cff_core.line p dx dy ⦃ c p' => PenOk p' ∧ Near (M + D) p' ∧ p'.start = p.start ∧
      p'.out = p.out ∧ p'.contour.val.length ≤ p.contour.val.length + 1 ⦄ := by
  obtain ⟨at_, start, contour, out⟩ := p
  unfold cff_core.line
  obtain ⟨hn0, hn1, hn2, hn3⟩ := hn
  obtain ⟨hx0, hx1⟩ := hx
  obtain ⟨hy0, hy1⟩ := hy
  simp only at hn0 hn1 hn2 hn3
  obtain ⟨hout, hop⟩ := h
  step*
  all_goals refine ⟨⟨hout, ?_⟩, by simp only [Near]; omega, by simp [*]⟩
  all_goals cases start
  all_goals simp_all [core.option.Option.is_none]
  all_goals (rw [ForgeText.chain_append, ForgeText.endOf_append]
             simp_all [cseg, ForgeText.Seg.start, ForgeText.Seg.stop])

@[step]
theorem curve_spec (p : cff_core.Pen) (d0 d1 d2 d3 d4 d5 : I64) (M D : Int) (h : PenOk p)
    (hn : Near M p) (h0 : Small D d0) (h1 : Small D d1) (h2 : Small D d2) (h3 : Small D d3)
    (h4 : Small D d4) (h5 : Small D d5) (hb : M + 3 * D ≤ 2^62) (hD : 0 ≤ D)
    (hl : p.contour.val.length + 1 < Usize.max) :
    cff_core.curve p d0 d1 d2 d3 d4 d5 ⦃ c p' => PenOk p' ∧ Near (M + 3 * D) p' ∧
      p'.start = p.start ∧ p'.out = p.out ∧ p'.contour.val.length ≤ p.contour.val.length + 1 ⦄ := by
  obtain ⟨at_, start, contour, out⟩ := p
  unfold cff_core.curve
  obtain ⟨hn0, hn1, hn2, hn3⟩ := hn
  obtain ⟨_, _⟩ := h0
  obtain ⟨_, _⟩ := h1
  obtain ⟨_, _⟩ := h2
  obtain ⟨_, _⟩ := h3
  obtain ⟨_, _⟩ := h4
  obtain ⟨_, _⟩ := h5
  simp only at hn0 hn1 hn2 hn3
  obtain ⟨hout, hop⟩ := h
  step*
  all_goals refine ⟨⟨hout, ?_⟩, by simp only [Near]; omega, ?_⟩
  all_goals try (first | exact Nat.le_succ _ | (rw [v_post]; simp))
  all_goals cases start
  all_goals try (simpa using hop)
  all_goals try (simp only [Option.isNone_some, Option.isNone_none, not_true_eq_false,
    Bool.false_eq_true] at *; done)
  all_goals simp only []
  all_goals first
    | exact open_add _ at_ _ contour.val v.val _ ⟨rfl, rfl⟩ hop (Or.inl v_post)
    | (rw [ForgeText.chain_append, ForgeText.endOf_append]
       exact ⟨⟨hop.1, by simp [cseg, ForgeText.Seg.start, hop.2]⟩, rfl⟩)
    | (have e1 := b1_post.1 (by assumption)
       have e2 := b2_post.1 (by assumption)
       have e3 := b3_post.1 (by assumption)
       exact open_add _ at_ _ contour.val contour.val (cff_core.SegC.Line at_ ⟨i4, i5⟩) ⟨rfl, rfl⟩
         hop (Or.inr ⟨rfl, e1.trans (e2.trans e3)⟩))

/-! ## The operators -/

/-- The operand stack: the width offset within it, at most 48 operands, each within `V`. -/
def StackOk (V : Int) (v : alloc.vec.Vec I64) (lo : Usize) : Prop :=
  lo.val ≤ v.val.length ∧ v.val.length ≤ 48 ∧ ∀ x ∈ v.val, Small V x

@[step]
theorem arg_spec (V : Int) (v : alloc.vec.Vec I64) (lo k : Usize) (h : StackOk V v lo)
    (hk : lo.val + k.val < v.val.length) :
    cff_core.arg { v, lo } k ⦃ x => Small V x ⦄ := by
  unfold cff_core.arg
  step*
  exact h.2.2 _ (by simp [x_post])

theorem near_mono {A B : Int} {p : cff_core.Pen} (h : Near A p) (hab : A ≤ B) : Near B p := by
  obtain ⟨a, b, c, d⟩ := h; exact ⟨by omega, by omega, by omega, by omega⟩

theorem kmul {k m : Nat} {V : Int} (h : k ≤ m) (hV : 0 ≤ V) : (k : Int) * V ≤ m * V :=
  Int.mul_le_mul_of_nonneg_right (by omega) hV

/-- What an operator's loop keeps: the pen's invariant, within `M + k·V` after `k` operands,
the kept contours untouched, the open one grown by at most `k`. -/
def Kept (M V : Int) (p0 p : cff_core.Pen) (k : Nat) : Prop :=
  PenOk p ∧ Near (M + k * V) p ∧ p.out = p0.out ∧ p.contour.val.length ≤ p0.contour.val.length + k

theorem lines_loop0_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i n : Usize) (e : U8)
    (k : Usize) (M V : Int) (hs : StackOk V v i) (hn : n.val = v.val.length - i.val)
    (hk : k.val ≤ n.val + 1) (hK : Kept M V p0 p k.val) (hV : 0 ≤ V)
    (hb : M + 50 * V ≤ 2^62) (hl : p0.contour.val.length + 50 < Usize.max) :
    cff_core.lines_loop0 p v i n e k ⦃ e' p' => Kept M V p0 p' (n.val + 1) ⦄ := by
  unfold cff_core.lines_loop0
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × U8 × Usize) => n.val + 2 - s.2.2.val)
    (fun s => s.2.2.val ≤ n.val + 1 ∧ Kept M V p0 s.1 s.2.2.val)
  · rintro ⟨p1, e1, k1⟩ ⟨hk1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.lines_loop0.body
    simp only at *
    have hk49 : k1.val ≤ 49 := by have := hs.2.1; omega
    have hm := kmul (V := V) hk49 hV
    split
    · step as ⟨k2, hk2⟩
      split
      · rename_i _ hlt
        have hlt' : k2.val < n.val := by scalar_tac
        step with arg_spec V v i k1 hs (by omega) as ⟨x, hx⟩
        step with arg_spec V v i k2 hs (by omega) as ⟨y, hy⟩
        step with line_spec p1 x y (M + k1.val * V) V hp hnr hx hy (by omega) hV (by omega)
          as ⟨e2, p2, hp2, hn2, hs2, ho2, hc2⟩
        step as ⟨k3, hk3⟩
        refine ⟨by omega, ⟨hp2, near_mono hn2 ?_, ho2.trans ho, by omega⟩, by omega⟩
        rw [hk3]; push_cast; have := kmul (V := V) (le_refl k1.val) hV; nlinarith
      · simp only [WP.spec_ok]
        exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
    · simp only [WP.spec_ok]
      exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
  · exact ⟨hk, hK⟩

theorem small_zero {V : Int} (hV : 0 ≤ V) : Small V 0#i64 := by
  simp only [Small]; constructor <;> simp <;> omega

theorem lines_loop1_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i n : Usize) (e : U8)
    (h : Bool) (k : Usize) (M V : Int) (hs : StackOk V v i) (hn : n.val = v.val.length - i.val)
    (hk : k.val ≤ n.val + 1) (hK : Kept M V p0 p k.val) (hV : 0 ≤ V)
    (hb : M + 50 * V ≤ 2^62) (hl : p0.contour.val.length + 50 < Usize.max) :
    cff_core.lines_loop1 p v i n e h k ⦃ e' p' => Kept M V p0 p' (n.val + 1) ⦄ := by
  unfold cff_core.lines_loop1
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × U8 × Bool × Usize) => n.val + 2 - s.2.2.2.val)
    (fun s => s.2.2.2.val ≤ n.val + 1 ∧ Kept M V p0 s.1 s.2.2.2.val)
  · rintro ⟨p1, e1, h1, k1⟩ ⟨hk1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.lines_loop1.body
    simp only at *
    have hk49 : k1.val ≤ 49 := by have := hs.2.1; omega
    have hm := kmul (V := V) hk49 hV
    have hm1 := kmul (V := V) (le_refl k1.val) hV
    split
    · split
      · rename_i _ hlt
        have hlt' : k1.val < n.val := by scalar_tac
        step with arg_spec V v i k1 hs (by omega) as ⟨x, hx⟩
        apply WP.spec_bind (Pₘ := fun (r : cff_core.Pen × U8) => Kept M V p0 r.1 (k1.val + 1))
        · split
          · step with line_spec p1 x 0#i64 (M + k1.val * V) V hp hnr hx (small_zero hV) (by omega)
              hV (by omega) as ⟨e2, p2, hp2, hn2, hs2, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
          · step with line_spec p1 0#i64 x (M + k1.val * V) V hp hnr (small_zero hV) hx (by omega)
              hV (by omega) as ⟨e2, p2, hp2, hn2, hs2, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
        rintro ⟨p2, e2⟩ hK2
        step as ⟨k3, hk3⟩
        refine ⟨by omega, ?_, by omega⟩
        rw [hk3]; exact hK2
      · simp only [WP.spec_ok]
        exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
    · simp only [WP.spec_ok]
      exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
  · exact ⟨hk, hK⟩

theorem kept_zero (M V : Int) (p : cff_core.Pen) (h : PenOk p) (hn : Near M p) : Kept M V p p 0 :=
  ⟨h, by simpa using hn, rfl, by omega⟩

@[step]
theorem lines_spec (p : cff_core.Pen) (s : cff_core.Stack) (op : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 50 * V ≤ 2^62) (hl : p.contour.val.length + 50 < Usize.max) :
    cff_core.lines p s op ⦃ e' p' => Kept M V p p' 49 ⦄ := by
  obtain ⟨v, lo⟩ := s
  unfold cff_core.lines
  have hn48 : v.val.length - lo.val ≤ 48 := by have := hs.2.1; simp only at this; omega
  step as ⟨n, hn'⟩
  apply WP.spec_mono (P₀ := fun r : U8 × cff_core.Pen => Kept M V p r.2 (n.val + 1))
  · split
    · apply WP.spec_bind (Pₘ := fun _ : U8 => True)
      · split
        · simp
        · step*
      intro e _
      exact lines_loop0_spec p p v lo n e 0#usize M V hs (by scalar_tac) (by simp)
        (by simpa using kept_zero M V p h hn) hV hb hl
    · apply WP.spec_bind (Pₘ := fun _ : U8 => True)
      · split <;> simp
      intro e _
      exact lines_loop1_spec p p v lo n e _ 0#usize M V hs (by scalar_tac) (by simp)
        (by simpa using kept_zero M V p h hn) hV hb hl
  · rintro ⟨e', p'⟩ ⟨a, b, c, d⟩
    exact ⟨a, near_mono b (by have := kmul (V := V) (show n.val + 1 ≤ 49 by scalar_tac) hV; push_cast at this ⊢; omega), c, by scalar_tac⟩

theorem kept_mono {M V : Int} {p0 p : cff_core.Pen} {k m : Nat} (h : Kept M V p0 p k) (hkm : k ≤ m)
    (hV : 0 ≤ V) : Kept M V p0 p m :=
  ⟨h.1, near_mono h.2.1 (by have := kmul (V := V) hkm hV; omega), h.2.2.1, by have := h.2.2.2; omega⟩

theorem sixes_loop_spec (p0 p : cff_core.Pen) (st : cff_core.Stack) (k0 n6 : Usize) (e : U8)
    (j : Usize) (M V : Int) (hs : StackOk V st.v st.lo)
    (hk0 : st.lo.val + k0.val + 6 * n6.val ≤ st.v.val.length)
    (hj : j.val ≤ n6.val) (hK : Kept M V p0 p (3 * j.val)) (hV : 0 ≤ V)
    (hb : M + 30 * V ≤ 2^62) (hl : p0.contour.val.length + 30 < Usize.max) :
    cff_core.sixes_loop p st k0 n6 e j ⦃ e' p' => Kept M V p0 p' (3 * n6.val) ⦄ := by
  unfold cff_core.sixes_loop
  have h8 : n6.val ≤ 8 := by have := hs.2.1; omega
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × U8 × Usize) => n6.val + 1 - s.2.2.val)
    (fun s => s.2.2.val ≤ n6.val ∧ Kept M V p0 s.1 (3 * s.2.2.val))
  · rintro ⟨p1, e1, j1⟩ ⟨hj1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.sixes_loop.body
    simp only at *
    have hm := kmul (V := V) (show 3 * j1.val ≤ 24 by omega) hV
    split
    · split
      · rename_i _ hlt
        have hlt' : j1.val < n6.val := by scalar_tac
        obtain ⟨v, lo⟩ := st
        simp only at hs hk0
        step as ⟨i, hi⟩
        step as ⟨k, hk⟩
        step with arg_spec V v lo k hs (by omega) as ⟨x0, h0⟩
        step as ⟨k1, hk1⟩
        step with arg_spec V v lo k1 hs (by omega) as ⟨x1, h1⟩
        step as ⟨k2, hk2⟩
        step with arg_spec V v lo k2 hs (by omega) as ⟨x2, h2⟩
        step as ⟨k3, hk3⟩
        step with arg_spec V v lo k3 hs (by omega) as ⟨x3, h3⟩
        step as ⟨k4, hk4⟩
        step with arg_spec V v lo k4 hs (by omega) as ⟨x4, h4⟩
        step as ⟨k5, hk5⟩
        step with arg_spec V v lo k5 hs (by omega) as ⟨x5, h5⟩
        step with curve_spec p1 x0 x1 x2 x3 x4 x5 (M + (3 * j1.val : Nat) * V) V hp hnr h0 h1 h2 h3 h4 h5
          (by push_cast at hm ⊢; omega) hV (by omega) as ⟨e2, p2, hp2, hn2, hs2, ho2, hc2⟩
        step as ⟨j2, hj2⟩
        refine ⟨by omega, ⟨hp2, near_mono hn2 ?_, ho2.trans ho, by omega⟩, by omega⟩
        rw [hj2]; push_cast; ring_nf; omega
      · simp only [WP.spec_ok]
        exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
    · simp only [WP.spec_ok]
      exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
  · exact ⟨hj, hK⟩

theorem kept_trans {M V : Int} {p0 p1 p2 : cff_core.Pen} {a b : Nat} (h1 : Kept M V p0 p1 a)
    (h2 : Kept (M + a * V) V p1 p2 b) : Kept M V p0 p2 (a + b) :=
  ⟨h2.1, near_mono h2.2.1 (by push_cast; ring_nf; omega), h2.2.2.1.trans h1.2.2.1,
    by have := h1.2.2.2; have := h2.2.2.2; omega⟩

theorem curves_loop_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i n : Usize) (e : U8)
    (k : Usize) (M V : Int) (hs : StackOk V v i) (hn : n.val = v.val.length - i.val)
    (hk : k.val ≤ n.val + 1) (hK : Kept M V p0 p k.val) (hV : 0 ≤ V)
    (hb : M + 50 * V ≤ 2^62) (hl : p0.contour.val.length + 50 < Usize.max) :
    cff_core.curves_loop p v i n e k ⦃ p' e' => Kept M V p0 p' (n.val + 1) ⦄ := by
  unfold cff_core.curves_loop
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × U8 × Usize) => n.val + 2 - s.2.2.val)
    (fun s => s.2.2.val ≤ n.val + 1 ∧ Kept M V p0 s.1 s.2.2.val)
  · rintro ⟨p1, e1, k1⟩ ⟨hk1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.curves_loop.body
    simp only at *
    have hk49 : k1.val ≤ 49 := by have := hs.2.1; omega
    have hm := kmul (V := V) hk49 hV
    split
    · step as ⟨k2, hk2⟩
      split
      · rename_i _ hlt
        have hlt' : k2.val < n.val := by scalar_tac
        step with arg_spec V v i k1 hs (by omega) as ⟨x, hx⟩
        step as ⟨k2, hk2⟩
        step with arg_spec V v i k2 hs (by omega) as ⟨y, hy⟩
        step with line_spec p1 x y (M + k1.val * V) V hp hnr hx hy (by omega) hV (by omega)
          as ⟨e2, p2, hp2, hn2, hs2, ho2, hc2⟩
        step as ⟨k3, hk3⟩
        refine ⟨by omega, ⟨hp2, near_mono hn2 ?_, ho2.trans ho, by omega⟩, by omega⟩
        rw [hk3]; push_cast; have := kmul (V := V) (le_refl k1.val) hV; nlinarith
      · simp only [WP.spec_ok]
        exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
    · simp only [WP.spec_ok]
      exact ⟨hp, near_mono hnr (by have := kmul (V := V) hk1 hV; omega), ho, by omega⟩
  · exact ⟨hk, hK⟩


@[step]
theorem curves_spec (p : cff_core.Pen) (s : cff_core.Stack) (op : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max) :
    cff_core.curves p s op ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  have k0 := kept_zero M V p h hn
  obtain ⟨v, lo⟩ := s
  unfold cff_core.curves
  have hn48 : v.val.length - lo.val ≤ 48 := by have := hs.2.1; simp only at this; omega
  have hlo : lo.val ≤ v.val.length := hs.1
  step as ⟨n, hn'⟩
  split
  · split
    · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
    · step as ⟨r, hr⟩
      split
      · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
      · step as ⟨n6, hn6⟩
        apply WP.spec_mono (sixes_loop_spec p p _ 0#usize n6 _ 0#usize M V hs
          (by simp only; scalar_tac) (by simp) (by simpa using k0) hV (by omega) (by omega))
        rintro ⟨e', p'⟩ hk; exact kept_mono hk (by scalar_tac) hV
  · split
    · split
      · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
      · step as ⟨n2, hn2⟩
        step as ⟨r, hr⟩
        split
        · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
        · step as ⟨n6, hn6⟩
          step with sixes_loop_spec p p _ 0#usize n6 _ 0#usize M V hs
            (by simp only; scalar_tac) (by simp) (by simpa using k0) hV (by omega) (by omega) as ⟨e1, p1, hk1⟩
          split
          · simp only [WP.spec_ok]; exact kept_mono hk1 (by scalar_tac) hV
          · obtain ⟨hp1, hnr1, ho1, hc1⟩ := hk1
            step with arg_spec V v lo n2 hs (by scalar_tac) as ⟨x, hx⟩
            step as ⟨n1, hn1⟩
            step with arg_spec V v lo n1 hs (by scalar_tac) as ⟨y, hy⟩
            have h8 := kmul (V := V) (show 3 * n6.val ≤ 24 by scalar_tac) hV
            apply WP.spec_mono (line_spec p1 x y (M + (3 * n6.val : Nat) * V) V hp1 hnr1 hx hy
              (by push_cast at h8 ⊢; omega) hV (by have := hc1; scalar_tac))
            rintro ⟨e2, p2⟩ ⟨hp2, hn2', _, ho2, hc2⟩
            refine ⟨hp2, near_mono hn2' ?_, ho2.trans ho1, by have := hc1; have := hc2; scalar_tac⟩
            have := kmul (V := V) (show 3 * n6.val + 1 ≤ 60 by scalar_tac) hV
            push_cast at this h8 ⊢; omega
    · split
      · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
      · step as ⟨n6', hn6'⟩
        step as ⟨r, hr⟩
        split
        · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
        · step with curves_loop_spec p p v lo n _ 0#usize M V hs (by scalar_tac) (by simp)
            (by simpa using k0) hV (by omega) (by omega) as ⟨p1, e1, hk1⟩
          split
          · simp only [WP.spec_ok]; exact kept_mono hk1 (by scalar_tac) hV
          · step as ⟨k6, hk6⟩
            have hk1' : Kept M V p p1 49 := kept_mono hk1 (by scalar_tac) hV
            have hz : Kept (M + (49 : Nat) * V) V p1 p1 0 :=
              kept_zero _ V p1 hk1'.1 hk1'.2.1
            apply WP.spec_mono (sixes_loop_spec p1 p1 _ k6 1#usize _ 0#usize _ V hs
              (by simp only; scalar_tac) (by simp) (by simpa using hz) hV
              (by push_cast; omega) (by have := hk1'.2.2.2; omega))
            rintro ⟨e2, p2⟩ hk2
            exact kept_mono (kept_trans hk1' hk2) (by simp) hV

theorem fours_loop_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i : Usize) (op : U8)
    (n : Usize) (d1 : I64) (e : U8) (k : Usize) (M V : Int) (hs : StackOk V v i)
    (hn : n.val = v.val.length - i.val) (hk : k.val ≤ n.val + 3) (hd : Small V d1)
    (hK : Kept M V p0 p k.val) (hV : 0 ≤ V)
    (hb : M + 60 * V ≤ 2^62) (hl : p0.contour.val.length + 60 < Usize.max) :
    cff_core.fours_loop p v i op n d1 e k ⦃ e' p' => Kept M V p0 p' (n.val + 3) ⦄ := by
  unfold cff_core.fours_loop
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × I64 × U8 × Usize) => n.val + 4 - s.2.2.2.val)
    (fun s => s.2.2.2.val ≤ n.val + 3 ∧ Small V s.2.1 ∧ Kept M V p0 s.1 s.2.2.2.val)
  · rintro ⟨p1, d, e1, k1⟩ ⟨hk1, hd1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.fours_loop.body
    simp only at *
    have hk51 : k1.val ≤ 51 := by have := hs.2.1; omega
    have hm := kmul (V := V) hk51 hV
    split
    · step as ⟨k3, hk3⟩
      split
      · rename_i _ hlt
        have hlt' : k3.val < n.val := by scalar_tac
        step with arg_spec V v i k1 hs (by omega) as ⟨a, ha⟩
        step as ⟨k2, hk2⟩
        step with arg_spec V v i k2 hs (by omega) as ⟨b, hb'⟩
        step as ⟨k2', hk2'⟩
        step with arg_spec V v i k2' hs (by omega) as ⟨c, hc'⟩
        step with arg_spec V v i k3 hs (by omega) as ⟨d', hd'⟩
        apply WP.spec_bind (Pₘ := fun (r : cff_core.Pen × U8) => Kept M V p0 r.1 (k1.val + 4))
        · split
          · step with curve_spec p1 d a b c 0#i64 d' (M + k1.val * V) V hp hnr hd1 ha hb' hc'
              (small_zero hV) hd' (by omega) hV (by omega) as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
          · step with curve_spec p1 a d b c d' 0#i64 (M + k1.val * V) V hp hnr ha hd1 hb' hc'
              hd' (small_zero hV) (by omega) hV (by omega) as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
        rintro ⟨p2, e2⟩ hK2
        step as ⟨k4, hk4⟩
        refine ⟨by omega, small_zero hV, ?_, by omega⟩
        rw [hk4]; exact hK2
      · simp only [WP.spec_ok]
        exact kept_mono ⟨hp, hnr, ho, hc⟩ hk1 hV
    · simp only [WP.spec_ok]
      exact kept_mono ⟨hp, hnr, ho, hc⟩ hk1 hV
  · exact ⟨hk, hd, hK⟩

@[step]
theorem fours_spec (p : cff_core.Pen) (s : cff_core.Stack) (op : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max) :
    cff_core.fours p s op ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  have k0 := kept_zero M V p h hn
  obtain ⟨v, lo⟩ := s
  unfold cff_core.fours
  have hn48 : v.val.length - lo.val ≤ 48 := by have := hs.2.1; simp only at this; omega
  have hlo : lo.val ≤ v.val.length := hs.1
  step as ⟨n, hn'⟩
  step as ⟨r, hr⟩
  apply WP.spec_bind (Pₘ := fun (q : I64 × Usize) => Small V q.1 ∧ q.2.val ≤ 1 ∧ q.2.val ≤ n.val)
  · split
    · rename_i h1
      step with arg_spec V v lo 0#usize hs (by scalar_tac) as ⟨x, hx⟩
      exact ⟨hx, by simp, by simp; scalar_tac⟩
    · simp only [WP.spec_ok]; exact ⟨small_zero hV, by simp, by simp⟩
  rintro ⟨d1, k0'⟩ ⟨hd1, hk0', hk0n⟩
  step as ⟨m, hm⟩
  split
  · split <;> (simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV)
  · step as ⟨r2, hr2⟩
    split
    · split <;> (simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV)
    · apply WP.spec_mono (fours_loop_spec p p v lo op n d1 _ k0' M V hs (by scalar_tac)
        (by omega) hd1 (kept_mono k0 (by omega) hV) hV (by omega) (by omega))
      rintro ⟨e', p'⟩ hk; exact kept_mono hk (by scalar_tac) hV

theorem alternating_loop0_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i n : Usize) (h : Bool)
    (m : Usize) (e : U8) (k : Usize) (M V : Int) (hs : StackOk V v i)
    (hn : n.val = v.val.length - i.val) (hm4 : 4 * m.val ≤ n.val) (hk : k.val ≤ m.val)
    (hK : Kept M V p0 p (4 * k.val)) (hV : 0 ≤ V)
    (hb : M + 60 * V ≤ 2^62) (hl : p0.contour.val.length + 60 < Usize.max) :
    cff_core.alternating_loop0 p v i n h m e k ⦃ e' p' => Kept M V p0 p' (4 * m.val) ⦄ := by
  unfold cff_core.alternating_loop0
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × Bool × U8 × Usize) => m.val + 1 - s.2.2.2.val)
    (fun s => s.2.2.2.val ≤ m.val ∧ Kept M V p0 s.1 (4 * s.2.2.2.val))
  · rintro ⟨p1, h1, e1, k1⟩ ⟨hk1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.alternating_loop0.body
    simp only at *
    have hk12 : 4 * k1.val ≤ 48 := by have := hs.2.1; omega
    have hmm := kmul (V := V) hk12 hV
    split
    · split
      · rename_i _ hlt
        have hlt' : k1.val < m.val := by scalar_tac
        step as ⟨j0, hj0⟩
        step with arg_spec V v i j0 hs (by omega) as ⟨a, ha⟩
        step as ⟨j1, hj1⟩
        step with arg_spec V v i j1 hs (by omega) as ⟨b, hb'⟩
        step as ⟨j2, hj2⟩
        step with arg_spec V v i j2 hs (by omega) as ⟨c, hc'⟩
        step as ⟨j3, hj3⟩
        step with arg_spec V v i j3 hs (by omega) as ⟨d, hd⟩
        step as ⟨k2, hk2⟩
        apply WP.spec_bind (Pₘ := fun (x : I64) => Small V x)
        · split
          · step as ⟨r, hr⟩
            split
            · step as ⟨n1, hn1⟩
              exact arg_spec V v i n1 hs (by scalar_tac)
            · simp only [WP.spec_ok]; exact small_zero hV
          · simp only [WP.spec_ok]; exact small_zero hV
        intro last hlast
        apply WP.spec_bind (Pₘ := fun (r : cff_core.Pen × U8) => Kept M V p0 r.1 (4 * k1.val + 4))
        · split
          · step with curve_spec p1 a 0#i64 b c last d (M + (4 * k1.val : Nat) * V) V hp hnr ha
              (small_zero hV) hb' hc' hlast hd (by push_cast at hmm ⊢; omega) hV (by omega)
              as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
          · step with curve_spec p1 0#i64 a b c d last (M + (4 * k1.val : Nat) * V) V hp hnr
              (small_zero hV) ha hb' hc' hd hlast (by push_cast at hmm ⊢; omega) hV (by omega)
              as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
        rintro ⟨p2, e2⟩ hK2
        simp only [uncurry_apply_pair, WP.spec_ok]
        refine ⟨by omega, ?_, by omega⟩
        rw [hk2]; exact hK2
      · simp only [WP.spec_ok]
        exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
    · simp only [WP.spec_ok]
      exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
  · exact ⟨hk, hK⟩

theorem alternating_loop1_spec (p0 p : cff_core.Pen) (v : alloc.vec.Vec I64) (i n : Usize) (h : Bool)
    (m : Usize) (e : U8) (k : Usize) (M V : Int) (hs : StackOk V v i)
    (hn : n.val = v.val.length - i.val) (hm4 : 4 * m.val ≤ n.val) (hk : k.val ≤ m.val)
    (hK : Kept M V p0 p (4 * k.val)) (hV : 0 ≤ V)
    (hb : M + 60 * V ≤ 2^62) (hl : p0.contour.val.length + 60 < Usize.max) :
    cff_core.alternating_loop1 p v i n h m e k ⦃ e' p' => Kept M V p0 p' (4 * m.val) ⦄ := by
  unfold cff_core.alternating_loop1
  apply loop.spec_decr_nat (fun (s : cff_core.Pen × Bool × U8 × Usize) => m.val + 1 - s.2.2.2.val)
    (fun s => s.2.2.2.val ≤ m.val ∧ Kept M V p0 s.1 (4 * s.2.2.2.val))
  · rintro ⟨p1, h1, e1, k1⟩ ⟨hk1, hK1⟩
    obtain ⟨hp, hnr, ho, hc⟩ := hK1
    unfold cff_core.alternating_loop1.body
    simp only at *
    have hk12 : 4 * k1.val ≤ 48 := by have := hs.2.1; omega
    have hmm := kmul (V := V) hk12 hV
    split
    · split
      · rename_i _ hlt
        have hlt' : k1.val < m.val := by scalar_tac
        step as ⟨j0, hj0⟩
        step with arg_spec V v i j0 hs (by omega) as ⟨a, ha⟩
        step as ⟨j1, hj1⟩
        step with arg_spec V v i j1 hs (by omega) as ⟨b, hb'⟩
        step as ⟨j2, hj2⟩
        step with arg_spec V v i j2 hs (by omega) as ⟨c, hc'⟩
        step as ⟨j3, hj3⟩
        step with arg_spec V v i j3 hs (by omega) as ⟨d, hd⟩
        step as ⟨k2, hk2⟩
        apply WP.spec_bind (Pₘ := fun (x : I64) => Small V x)
        · split
          · step as ⟨r, hr⟩
            split
            · step as ⟨n1, hn1⟩
              exact arg_spec V v i n1 hs (by scalar_tac)
            · simp only [WP.spec_ok]; exact small_zero hV
          · simp only [WP.spec_ok]; exact small_zero hV
        intro last hlast
        apply WP.spec_bind (Pₘ := fun (r : cff_core.Pen × U8) => Kept M V p0 r.1 (4 * k1.val + 4))
        · split
          · step with curve_spec p1 a 0#i64 b c last d (M + (4 * k1.val : Nat) * V) V hp hnr ha
              (small_zero hV) hb' hc' hlast hd (by push_cast at hmm ⊢; omega) hV (by omega)
              as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
          · step with curve_spec p1 0#i64 a b c d last (M + (4 * k1.val : Nat) * V) V hp hnr
              (small_zero hV) ha hb' hc' hd hlast (by push_cast at hmm ⊢; omega) hV (by omega)
              as ⟨e2, p2, hp2, hn2, _, ho2, hc2⟩
            exact ⟨hp2, near_mono hn2 (by push_cast; nlinarith), ho2.trans ho, by omega⟩
        rintro ⟨p2, e2⟩ hK2
        simp only [uncurry_apply_pair, WP.spec_ok]
        refine ⟨by omega, ?_, by omega⟩
        rw [hk2]; exact hK2
      · simp only [WP.spec_ok]
        exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
    · simp only [WP.spec_ok]
      exact kept_mono ⟨hp, hnr, ho, hc⟩ (by omega) hV
  · exact ⟨hk, hK⟩

@[step]
theorem alternating_spec (p : cff_core.Pen) (s : cff_core.Stack) (op : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max) :
    cff_core.alternating p s op ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  have k0 := kept_zero M V p h hn
  obtain ⟨v, lo⟩ := s
  unfold cff_core.alternating
  have hn48 : v.val.length - lo.val ≤ 48 := by have := hs.2.1; simp only at this; omega
  have hlo : lo.val ≤ v.val.length := hs.1
  step as ⟨n, hn'⟩
  split
  · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
  · step as ⟨r, hr⟩
    split
    · step as ⟨m, hm⟩
      apply WP.spec_mono (alternating_loop0_spec p p v lo n _ m _ 0#usize M V hs (by scalar_tac)
        (by scalar_tac) (by simp) (by simpa using k0) hV (by omega) (by omega))
      rintro ⟨e', p'⟩ hk; exact kept_mono hk (by scalar_tac) hV
    · split
      · step as ⟨m, hm⟩
        apply WP.spec_mono (alternating_loop1_spec p p v lo n _ m _ 0#usize M V hs (by scalar_tac)
          (by scalar_tac) (by simp) (by simpa using k0) hV (by omega) (by omega))
        rintro ⟨e', p'⟩ hk; exact kept_mono hk (by scalar_tac) hV
      · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV

theorem small_mono {D E : Int} {v : I64} (h : Small D v) (hde : D ≤ E) : Small E v :=
  ⟨by have := h.1; omega, by have := h.2; omega⟩

/-- A stack of exactly `n` operands after the width. -/
def Args (V : Int) (s : cff_core.Stack) (n : Nat) : Prop :=
  StackOk V s.v s.lo ∧ s.v.val.length - s.lo.val = n

@[step]
theorem abs_spec (v : I64) (h : -2^62 ≤ v.val ∧ v.val ≤ 2^62) :
    cff_core.abs v ⦃ a => a.val = |v.val| ⦄ := by
  unfold cff_core.abs
  split
  · step*
  · simp only [WP.spec_ok]; rename_i hv
    have : 0 ≤ v.val := by scalar_tac
    rw [abs_of_nonneg this]

theorem hflex_spec (p : cff_core.Pen) (st : cff_core.Stack) (M V : Int) (hs : Args V st 7)
    (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V) (hb : M + 150 * V ≤ 2^62)
    (hl : p.contour.val.length + 150 < Usize.max) (hV2 : V ≤ 2^40) :
    cff_core.hflex p st ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  obtain ⟨v, lo⟩ := st
  obtain ⟨hs, hn7⟩ := hs
  simp only at hn7
  unfold cff_core.hflex
  step with arg_spec V v lo 0#usize hs (by simp; omega) as ⟨a0, h0⟩
  step with arg_spec V v lo 1#usize hs (by simp; omega) as ⟨a1, h1⟩
  step with arg_spec V v lo 2#usize hs (by simp; omega) as ⟨a2, h2⟩
  step with arg_spec V v lo 3#usize hs (by simp; omega) as ⟨a3, h3⟩
  step with arg_spec V v lo 4#usize hs (by simp; omega) as ⟨a4, h4⟩
  step with arg_spec V v lo 5#usize hs (by simp; omega) as ⟨a5, h5⟩
  step with arg_spec V v lo 6#usize hs (by simp; omega) as ⟨a6, h6⟩
  step with curve_spec p a0 0#i64 a1 a2 a3 0#i64 M V h hn h0 (small_zero hV) h1 h2 h3
    (small_zero hV) (by omega) hV (by omega) as ⟨e1, p1, hp1, hn1, _, ho1, hc1⟩
  split
  · simp only [WP.spec_ok]
    exact ⟨hp1, near_mono hn1 (by omega), ho1, by omega⟩
  · obtain ⟨_, _⟩ := h2
    step as ⟨m2, hm2⟩
    have hm2' : Small V m2 := ⟨by omega, by omega⟩
    apply WP.spec_mono (curve_spec p1 a4 0#i64 a5 m2 a6 0#i64 (M + 3 * V) V hp1 hn1 h4
      (small_zero hV) h5 hm2' h6 (small_zero hV) (by omega) hV (by omega))
    rintro ⟨e2, p2⟩ ⟨hp2, hn2, _, ho2, hc2⟩
    exact ⟨hp2, near_mono hn2 (by omega), ho2.trans ho1, by omega⟩

theorem hflex1_spec (p : cff_core.Pen) (st : cff_core.Stack) (M V : Int) (hs : Args V st 9)
    (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V) (hb : M + 150 * V ≤ 2^62)
    (hl : p.contour.val.length + 150 < Usize.max) (hV2 : V ≤ 2^40) :
    cff_core.hflex1 p st ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  obtain ⟨v, lo⟩ := st
  obtain ⟨hs, hn9⟩ := hs
  simp only at hn9
  unfold cff_core.hflex1
  step with arg_spec V v lo 0#usize hs (by simp; omega) as ⟨a0, h0⟩
  step with arg_spec V v lo 1#usize hs (by simp; omega) as ⟨a1, h1⟩
  step with arg_spec V v lo 2#usize hs (by simp; omega) as ⟨a2, h2⟩
  step with arg_spec V v lo 3#usize hs (by simp; omega) as ⟨a3, h3⟩
  step with arg_spec V v lo 4#usize hs (by simp; omega) as ⟨a4, h4⟩
  step with arg_spec V v lo 5#usize hs (by simp; omega) as ⟨a5, h5⟩
  step with arg_spec V v lo 6#usize hs (by simp; omega) as ⟨a6, h6⟩
  step with arg_spec V v lo 7#usize hs (by simp; omega) as ⟨a7, h7⟩
  step with arg_spec V v lo 8#usize hs (by simp; omega) as ⟨a8, h8⟩
  step with curve_spec p a0 a1 a2 a3 a4 0#i64 M V h hn h0 h1 h2 h3 h4
    (small_zero hV) (by omega) hV (by omega) as ⟨e1, p1, hp1, hn1, _, ho1, hc1⟩
  split
  · simp only [WP.spec_ok]
    exact ⟨hp1, near_mono hn1 (by omega), ho1, by omega⟩
  · obtain ⟨_, _⟩ := h1
    obtain ⟨_, _⟩ := h3
    obtain ⟨_, _⟩ := h7
    step as ⟨s1, hs1⟩
    step as ⟨s2, hs2⟩
    step as ⟨m2, hm2⟩
    have hm2' : Small (3 * V) m2 := ⟨by omega, by omega⟩
    apply WP.spec_mono (curve_spec p1 a5 0#i64 a6 a7 a8 m2 (M + 3 * V) (3 * V) hp1 hn1
      (small_mono h5 (by omega)) (small_zero (by omega)) (small_mono h6 (by omega))
      (small_mono ⟨by omega, by omega⟩ (le_refl _)) (small_mono h8 (by omega)) hm2' (by omega)
      (by omega) (by omega))
    rintro ⟨e2, p2⟩ ⟨hp2, hn2, _, ho2, hc2⟩
    exact ⟨hp2, near_mono hn2 (by omega), ho2.trans ho1, by omega⟩

theorem flex1_spec (p : cff_core.Pen) (st : cff_core.Stack) (M V : Int) (hs : Args V st 11)
    (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V) (hb : M + 150 * V ≤ 2^62)
    (hl : p.contour.val.length + 150 < Usize.max) (hV2 : V ≤ 2^40) :
    cff_core.flex1 p st ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  obtain ⟨v, lo⟩ := st
  obtain ⟨hs, hn11⟩ := hs
  simp only at hn11
  unfold cff_core.flex1
  step with arg_spec V v lo 0#usize hs (by simp; omega) as ⟨a0, h0⟩
  step with arg_spec V v lo 2#usize hs (by simp; omega) as ⟨a2, h2⟩
  obtain ⟨_, _⟩ := h0
  obtain ⟨_, _⟩ := h2
  step as ⟨x1, hx1⟩
  step with arg_spec V v lo 4#usize hs (by simp; omega) as ⟨a4, h4⟩
  obtain ⟨_, _⟩ := h4
  step as ⟨x2, hx2⟩
  step with arg_spec V v lo 6#usize hs (by simp; omega) as ⟨a6, h6⟩
  obtain ⟨_, _⟩ := h6
  step as ⟨x3, hx3⟩
  step with arg_spec V v lo 8#usize hs (by simp; omega) as ⟨a8, h8⟩
  obtain ⟨_, _⟩ := h8
  step as ⟨dx, hdx⟩
  step with arg_spec V v lo 1#usize hs (by simp; omega) as ⟨a1, h1⟩
  step with arg_spec V v lo 3#usize hs (by simp; omega) as ⟨a3, h3⟩
  obtain ⟨_, _⟩ := h1
  obtain ⟨_, _⟩ := h3
  step as ⟨y1, hy1⟩
  step with arg_spec V v lo 5#usize hs (by simp; omega) as ⟨a5, h5⟩
  obtain ⟨_, _⟩ := h5
  step as ⟨y2, hy2⟩
  step with arg_spec V v lo 7#usize hs (by simp; omega) as ⟨a7, h7⟩
  obtain ⟨_, _⟩ := h7
  step as ⟨y3, hy3⟩
  step with arg_spec V v lo 9#usize hs (by simp; omega) as ⟨a9, h9⟩
  obtain ⟨_, _⟩ := h9
  step as ⟨dy, hdy⟩
  step with abs_spec dx (by omega) as ⟨ax, hax⟩
  step with abs_spec dy (by omega) as ⟨ay, hay⟩
  apply WP.spec_bind (Pₘ := fun (q : I64 × I64) => Small (5 * V) q.1 ∧ Small (5 * V) q.2)
  · split
    · step with arg_spec V v lo 10#usize hs (by simp; omega) as ⟨a10, h10⟩
      step as ⟨ny, hny⟩
      skip
      exact ⟨small_mono h10 (by omega), ⟨by omega, by omega⟩⟩
    · step as ⟨nx, hnx⟩
      step with arg_spec V v lo 10#usize hs (by simp; omega) as ⟨a10, h10⟩
      skip
      exact ⟨⟨by omega, by omega⟩, small_mono h10 (by omega)⟩
  rintro ⟨ex, ey⟩ ⟨hex, hey⟩
  have sm : ∀ x : I64, -V ≤ x.val → x.val ≤ V → Small (5 * V) x := fun x a b => ⟨by omega, by omega⟩
  step with curve_spec p a0 a1 a2 a3 a4 a5 M (5 * V) h hn (sm _ (by omega) (by omega))
    (sm _ (by omega) (by omega)) (sm _ (by omega) (by omega)) (sm _ (by omega) (by omega))
    (sm _ (by omega) (by omega)) (sm _ (by omega) (by omega)) (by omega) (by omega) (by omega)
    as ⟨e1, p1, hp1, hn1, _, ho1, hc1⟩
  split
  · simp only [WP.spec_ok]
    exact ⟨hp1, near_mono hn1 (by omega), ho1, by omega⟩
  · apply WP.spec_mono (curve_spec p1 a6 a7 a8 a9 ex ey (M + 3 * (5 * V)) (5 * V) hp1 hn1
      (sm _ (by omega) (by omega)) (sm _ (by omega) (by omega)) (sm _ (by omega) (by omega))
      (sm _ (by omega) (by omega)) hex hey (by omega) (by omega) (by omega))
    rintro ⟨e2, p2⟩ ⟨hp2, hn2, _, ho2, hc2⟩
    exact ⟨hp2, near_mono hn2 (by omega), ho2.trans ho1, by omega⟩

@[step]
theorem flex_spec (p : cff_core.Pen) (st : cff_core.Stack) (b1 : U8) (M V : Int)
    (hs : StackOk V st.v st.lo) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max) (hV2 : V ≤ 2^40) :
    cff_core.flex p st b1 ⦃ e' p' => Kept M V p p' 60 ⦄ := by
  have k0 := kept_zero M V p h hn
  unfold cff_core.flex
  step as ⟨n, hn'⟩
  have hA : ∀ m : Nat, n.val = m → Args V st m := fun m hm => ⟨hs, by scalar_tac⟩
  split
  · split
    · rename_i h13
      apply WP.spec_mono (sixes_loop_spec p p st 0#usize 2#usize _ 0#usize M V hs
        (by have := hA 13 (by scalar_tac); simp only [Args] at this; simp; omega)
        (by simp) (by simpa using k0) hV (by omega) (by omega))
      rintro ⟨e', p'⟩ hk; exact kept_mono hk (by simp) hV
    · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
  · split
    · split
      · exact hflex_spec p st M V (hA 7 (by scalar_tac)) h hn hV hb hl hV2
      · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
    · split
      · split
        · exact hflex1_spec p st M V (hA 9 (by scalar_tac)) h hn hV hb hl hV2
        · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
      · split
        · split
          · exact flex1_spec p st M V (hA 11 (by scalar_tac)) h hn hV hb hl hV2
          · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV
        · simp only [WP.spec_ok]; exact kept_mono k0 (by omega) hV

/-! ## Operands, subroutines, the width -/

@[step]
theorem hcast_u32_i32 (x : U32) :
    lift (UScalar.hcast IScalarTy.I32 x) ⦃ y => -2^31 ≤ y.val ∧ y.val < 2^31 ⦄ := by
  simp only [lift, WP.spec_ok]
  have := (UScalar.hcast IScalarTy.I32 x).hBounds
  simpa [IScalarTy.numBits] using this

@[step]
theorem cast_i32_i64 (x : I32) : lift (IScalar.cast IScalarTy.I64 x) ⦃ y => y.val = x.val ⦄ :=
  IScalar.cast_inBounds_spec _ x (by have := x.hBounds; simp [IScalarTy.numBits] at this; scalar_tac)

@[step]
theorem byte_or0_c (d : Slice U8) (o : Usize) : cff_core.byte_or0 d o ⦃ _ => True ⦄ := by
  unfold cff_core.byte_or0; step*

theorem one_val : cff_core.ONE.val = 65536 := by simp [cff_core.ONE]

/-- An operand: within 2³¹ (2⁻¹⁶ units) either way, read from at most the four bytes at `o`
before `e`. -/
@[step]
theorem operand_spec (d : Slice U8) (o e : Usize) (b0 : U8) (hoe : o.val ≤ e.val)
    (he : e.val + 4 ≤ Usize.max) :
    cff_core.operand d o e b0 ⦃ r => ∀ v o', r = some (v, o') →
      Small (2^31) v ∧ o.val ≤ o'.val ∧ o'.val ≤ e.val ⦄ := by
  unfold cff_core.operand
  walk
  all_goals (try simp only [one_val] at *)
  all_goals (first
    | (simp; done)
    | scalar_tac
    | (intro v o' h; simp at h; obtain ⟨rfl, rfl⟩ := h
       refine ⟨⟨?_, ?_⟩, ?_, ?_⟩ <;> scalar_tac))

@[step]
theorem bias_spec (n : Usize) : cff_core.bias n ⦃ b => 0 ≤ b.val ∧ b.val ≤ 32768 ⦄ := by
  unfold cff_core.bias; split <;> (try split) <;> simp

/-- A subroutine found is a range within the data. -/
@[step]
theorem subr_spec (d : Slice U8) (offs : Slice Usize) (k : I64) (hk : -2^40 ≤ k.val ∧ k.val ≤ 2^40) :
    cff_core.subr d offs k ⦃ r => ∀ a b, r = some (a, b) → a.val ≤ b.val ∧ b.val ≤ d.val.length ⦄ := by
  unfold cff_core.subr
  apply WP.spec_bind (Pₘ := fun c : Usize => c.val + 1 = offs.val.length ∨ (c.val = 0 ∧ offs.val.length = 0))
  · split
    · simp only [WP.spec_ok]; rename_i h; right; exact ⟨rfl, by scalar_tac⟩
    · step*
  intro count hc
  step as ⟨b, hb⟩
  step as ⟨i1, hi1⟩
  split
  · simp
  · rename_i hneg
    have h0 : 0 ≤ i1.val := by scalar_tac
    step with IScalar.hcast_inBounds_spec UScalarTy.U64 i1 ⟨h0, by simp [UScalar.max]; scalar_tac⟩ as ⟨i2, hi2⟩
    step with UScalar.cast_inBounds_spec UScalarTy.U64 count (by
      have := count.hBounds
      rcases System.Platform.numBits_eq with h | h <;> simp_all [UScalarTy.numBits, U64.max_eq] <;> omega) as ⟨i3, hi3⟩
    split
    · simp
    · rename_i hlt
      have hlt' : i1.val < count.val := by scalar_tac
      step with IScalar.hcast_inBounds_spec UScalarTy.Usize i1 ⟨h0, by simp [UScalar.max]; scalar_tac⟩ as ⟨i4, hi4⟩
      have hi4l : i4.val + 1 < offs.val.length := by rcases hc with hc | hc <;> scalar_tac
      step as ⟨a', ha'⟩
      step as ⟨i5, hi5⟩
      step as ⟨b', hb'⟩
      split
      · split
        · simp only [WP.spec_ok, Option.some.injEq, Prod.mk.injEq]
          rintro a2 b2 ⟨rfl, rfl⟩
          constructor <;> scalar_tac
        · simp
      · simp

/-- The width read past keeps the stack's invariant (`lo` becomes 1 only over an operand). -/
@[step]
theorem width_spec (V : Int) (st : cff_core.Stack) (read odd : Bool) (want : Usize)
    (hs : StackOk V st.v st.lo) (h0 : st.lo.val = 0) :
    cff_core.width st read odd want ⦃ s' r' => s'.v = st.v ∧ StackOk V s'.v s'.lo ∧ s'.lo.val ≤ 1 ⦄ := by
  obtain ⟨v, lo⟩ := st
  obtain ⟨h1, h2, h3⟩ := hs
  simp only at h0 h1 h2 h3
  unfold cff_core.width
  walk
  all_goals (simp only [WP.spec_ok, WP.uncurry'_pair]
             refine ⟨trivial, ⟨?_, h2, h3⟩, ?_⟩ <;> (try simp) <;> scalar_tac)

/-! ## One operator -/

/-- What an operator keeps: the pen's invariant, within `60·V` more, one more contour kept at
most, the open one 60 pieces longer at most, the charstring and its end the same and the next
byte between where it was and the end, 24 stems more at most. -/
def OpOk (M V : Int) (p p' : cff_core.Pen) (f f' : cff_core.Frame) (st st' : Usize) : Prop :=
  PenOk p' ∧ Near (M + 60 * V) p' ∧ p'.out.val.length ≤ p.out.val.length + 1 ∧
  p'.contour.val.length ≤ p.contour.val.length + 60 ∧ f'.s = f.s ∧ f'.e = f.e ∧
  f.o.val ≤ f'.o.val ∧ f'.o.val ≤ f'.e.val ∧ st'.val ≤ st.val + 24

theorem kept_op {M V : Int} {p p' : cff_core.Pen} {f : cff_core.Frame} {st : Usize}
    (h : Kept M V p p' 60) (hf : f.o.val ≤ f.e.val) : OpOk M V p p' f f st st :=
  ⟨h.1, h.2.1, by rw [h.2.2.1]; omega, h.2.2.2, rfl, rfl, le_refl _, hf, by omega⟩

/-- A stem operator (and, `mask`, a hint mask's bytes after it). -/
theorem op_stems (d : Slice U8) (f : cff_core.Frame) (p : cff_core.Pen) (s : cff_core.Stack)
    (stems : Usize) (wr dn : Bool) (M V : Int) (mask : Bool)
    (hs : StackOk V s.v s.lo) (h0 : s.lo.val = 0) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hf : f.o.val ≤ f.e.val) (hst : stems.val + 48 ≤ Usize.max) :
    (do
      let (s1, width_read1) ← cff_core.width s wr true 0#usize
      let i := alloc.vec.Vec.len s1.v
      let i1 ← i - s1.lo
      let i2 ← i1 / 2#usize
      let stems1 ← stems + i2
      if mask then
        let skip ← core.num.Usize.div_ceil stems1 8#usize
        let i3 ← f.e - f.o
        if skip > i3 then ok (cff_core.TRUNCATED, { f with o := f.e }, p, s1, stems1, width_read1, dn)
        else
          let i4 ← f.o + skip
          ok (cff_core.OK, { f with o := i4 }, p, s1, stems1, width_read1, dn)
      else ok (cff_core.OK, f, p, s1, stems1, width_read1, dn)) ⦃ c f' p' s' st' wr' dn' =>
        OpOk M V p p' f f' stems st' ⦄ := by
  step with width_spec V s wr true 0#usize hs h0 as ⟨s1, wr1, hv1, hs1, hlo1⟩
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  have h24 : i2.val ≤ 24 := by have := hs1.2.1; scalar_tac
  step as ⟨st1, hst1⟩
  have base : ∀ f' : cff_core.Frame, f'.s = f.s → f'.e = f.e → f.o.val ≤ f'.o.val →
      f'.o.val ≤ f'.e.val → OpOk M V p p f f' stems st1 := fun f' a b c e =>
    ⟨h, near_mono hn (by omega), by omega, by omega, a, b, c, e, by omega⟩
  cases mask
  · simp only [Bool.false_eq_true, ↓reduceIte, WP.spec_ok]
    exact base f rfl rfl (le_refl _) hf
  · simp only [↓reduceIte]
    step as ⟨sk, hsk⟩
    step as ⟨r, hr⟩
    split
    · simp only [WP.spec_ok]; exact base _ rfl rfl hf (le_refl _)
    · step as ⟨o4, ho4⟩
      exact base _ rfl rfl (by simp; omega) (by simp; scalar_tac)

/-- A moveto (21 rmoveto, 22 hmoveto, 4 vmoveto). -/
theorem op_move (f : cff_core.Frame) (p : cff_core.Pen) (s : cff_core.Stack)
    (stems : Usize) (wr dn : Bool) (b0 : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h0 : s.lo.val = 0) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max)
    (ho : p.out.val.length + 2 < Usize.max) (hf : f.o.val ≤ f.e.val) :
    (do
      let want ← if b0 = 21#u8 then ok 2#usize else ok 1#usize
      let (s1, width_read1) ← cff_core.width s wr false want
      let i := alloc.vec.Vec.len s1.v
      let i1 ← i - s1.lo
      if i1 != want then ok (cff_core.MOVETO_OPERANDS, f, p, s1, stems, width_read1, dn)
      else
        if b0 = 21#u8 then
          let i2 ← cff_core.arg s1 0#usize
          let i3 ← cff_core.arg s1 1#usize
          let p1 ← cff_core.move_to p i2 i3
          ok (cff_core.OK, f, p1, s1, stems, width_read1, dn)
        else
          if b0 = 22#u8 then
            let i2 ← cff_core.arg s1 0#usize
            let p1 ← cff_core.move_to p i2 0#i64
            ok (cff_core.OK, f, p1, s1, stems, width_read1, dn)
          else
            let i2 ← cff_core.arg s1 0#usize
            let p1 ← cff_core.move_to p 0#i64 i2
            ok (cff_core.OK, f, p1, s1, stems, width_read1, dn)) ⦃ c f' p' s' st' wr' dn' =>
        OpOk M V p p' f f' stems st' ⦄ := by
  have base : OpOk M V p p f f stems stems :=
    ⟨h, near_mono hn (by omega), by omega, by omega, rfl, rfl, le_refl _, hf, by omega⟩
  apply WP.spec_bind (Pₘ := fun w : Usize => 1 ≤ w.val ∧ w.val ≤ 2 ∧ (b0 = 21#u8 → w.val = 2))
  · split <;> simp_all
  intro want hw
  obtain ⟨hw1, hw2, hw3⟩ := hw
  step with width_spec V s wr false want hs h0 as ⟨s1, wr1, hv1, hs1, hlo1⟩
  obtain ⟨v1, lo1⟩ := s1
  simp only at hv1 hs1 hlo1
  step as ⟨i1, hi1⟩
  have mv : ∀ dx dy : I64, Small V dx → Small V dy →
      cff_core.move_to p dx dy ⦃ p1 => OpOk M V p p1 f f stems stems ⦄ := by
    intro dx dy hx hy
    apply WP.spec_mono (move_to_spec p dx dy M V h hn hx hy (by omega) hV (by omega) (by omega))
    rintro p1 ⟨hp1, hn1, _, hc1, ho1⟩
    exact ⟨hp1, near_mono hn1 (by omega), ho1, by rw [hc1]; simp, rfl, rfl, le_refl _, hf, by omega⟩
  split
  · simp only [WP.spec_ok]; exact base
  · rename_i hne
    have hi1w : i1.val = want.val := by simpa using hne
    split
    · rename_i h21
      have := hw3 h21
      step with arg_spec V v1 lo1 0#usize hs1 (by simp; scalar_tac) as ⟨x, hx⟩
      step with arg_spec V v1 lo1 1#usize hs1 (by simp; scalar_tac) as ⟨y, hy⟩
      step with mv x y hx hy as ⟨p1, hp1⟩
      exact hp1
    · split
      · step with arg_spec V v1 lo1 0#usize hs1 (by simp; scalar_tac) as ⟨x, hx⟩
        step with mv x 0#i64 hx (small_zero hV) as ⟨p1, hp1⟩
        exact hp1
      · step with arg_spec V v1 lo1 0#usize hs1 (by simp; scalar_tac) as ⟨x, hx⟩
        step with mv 0#i64 x (small_zero hV) hx as ⟨p1, hp1⟩
        exact hp1

theorem op_kept (f : cff_core.Frame) (p : cff_core.Pen) (s : cff_core.Stack) (stems : Usize)
    (wr dn : Bool) (M V : Int) (X : Result (U8 × cff_core.Pen)) (hf : f.o.val ≤ f.e.val)
    (hX : X ⦃ e' p' => Kept M V p p' 60 ⦄) :
    (do let (i, p1) ← X; ok (i, f, p1, s, stems, wr, dn)) ⦃ c f' p' s' st' wr' dn' =>
      OpOk M V p p' f f' stems st' ⦄ := by
  apply WP.spec_bind hX
  rintro ⟨i, p1⟩ hk
  simp only [uncurry_apply_pair, WP.spec_ok]
  exact kept_op hk hf

theorem op_spec (d : Slice U8) (f : cff_core.Frame) (p : cff_core.Pen) (s : cff_core.Stack)
    (stems : Usize) (wr dn : Bool) (b0 : U8) (M V : Int)
    (hs : StackOk V s.v s.lo) (h0 : s.lo.val = 0) (h : PenOk p) (hn : Near M p) (hV : 0 ≤ V)
    (hV2 : V ≤ 2^40) (hb : M + 150 * V ≤ 2^62) (hl : p.contour.val.length + 150 < Usize.max)
    (ho : p.out.val.length + 2 < Usize.max) (hf : f.o.val ≤ f.e.val)
    (he : f.e.val + 2 ≤ Usize.max) (hst : stems.val + 48 ≤ Usize.max) :
    cff_core.op d f p s stems wr dn b0 ⦃ c f' p' s' st' wr' dn' => OpOk M V p p' f f' stems st' ⦄ := by
  have L := lines_spec p s b0 M V hs h hn hV (by omega) (by omega)
  have Lk : cff_core.lines p s b0 ⦃ e' p' => Kept M V p p' 60 ⦄ :=
    WP.spec_mono L (fun r hr => kept_mono hr (by omega) hV)
  have C := curves_spec p s b0 M V hs h hn hV hb hl
  have F4 := fours_spec p s b0 M V hs h hn hV hb hl
  have A := alternating_spec p s b0 M V hs h hn hV hb hl
  have stems_ := fun mask => op_stems d f p s stems wr dn M V mask hs h0 h hn hV hf hst
  have mv := op_move f p s stems wr dn b0 M V hs h0 h hn hV hb hl ho hf
  unfold cff_core.op
  by_cases c1 : b0 = 1#u8
  · rw [if_pos c1]; exact stems_ false
  rw [if_neg c1]
  by_cases c3 : b0 = 3#u8
  · rw [if_pos c3]; exact stems_ false
  rw [if_neg c3]
  by_cases c18 : b0 = 18#u8
  · rw [if_pos c18]; exact stems_ false
  rw [if_neg c18]
  by_cases c23 : b0 = 23#u8
  · rw [if_pos c23]; exact stems_ false
  rw [if_neg c23]
  by_cases c19 : b0 = 19#u8
  · rw [if_pos c19]; exact stems_ true
  rw [if_neg c19]
  by_cases c20 : b0 = 20#u8
  · rw [if_pos c20]; exact stems_ true
  rw [if_neg c20]
  by_cases c21 : b0 = 21#u8
  · rw [if_pos c21]; exact mv
  rw [if_neg c21]
  by_cases c22 : b0 = 22#u8
  · rw [if_pos c22]; exact mv
  rw [if_neg c22]
  by_cases c4 : b0 = 4#u8
  · rw [if_pos c4]; exact mv
  rw [if_neg c4]
  by_cases c5 : b0 = 5#u8
  · rw [if_pos c5]; exact op_kept f p s stems wr dn M V _ hf Lk
  rw [if_neg c5]
  by_cases c6 : b0 = 6#u8
  · rw [if_pos c6]; exact op_kept f p s stems wr dn M V _ hf Lk
  rw [if_neg c6]
  by_cases c7 : b0 = 7#u8
  · rw [if_pos c7]; exact op_kept f p s stems wr dn M V _ hf Lk
  rw [if_neg c7]
  by_cases c8 : b0 = 8#u8
  · rw [if_pos c8]; exact op_kept f p s stems wr dn M V _ hf C
  rw [if_neg c8]
  by_cases c24 : b0 = 24#u8
  · rw [if_pos c24]; exact op_kept f p s stems wr dn M V _ hf C
  rw [if_neg c24]
  by_cases c25 : b0 = 25#u8
  · rw [if_pos c25]; exact op_kept f p s stems wr dn M V _ hf C
  rw [if_neg c25]
  by_cases c26 : b0 = 26#u8
  · rw [if_pos c26]; exact op_kept f p s stems wr dn M V _ hf F4
  rw [if_neg c26]
  by_cases c27 : b0 = 27#u8
  · rw [if_pos c27]; exact op_kept f p s stems wr dn M V _ hf F4
  rw [if_neg c27]
  by_cases c30 : b0 = 30#u8
  · rw [if_pos c30]; exact op_kept f p s stems wr dn M V _ hf A
  rw [if_neg c30]
  by_cases c31 : b0 = 31#u8
  · rw [if_pos c31]; exact op_kept f p s stems wr dn M V _ hf A
  rw [if_neg c31]
  have base : OpOk M V p p f f stems stems :=
    ⟨h, near_mono hn (by omega), by omega, by omega, rfl, rfl, le_refl _, hf, by omega⟩
  by_cases c14 : b0 = 14#u8
  · rw [if_pos c14]
    -- endchar
    step with width_spec V s wr true 0#usize hs h0 as ⟨s1, wr1, hv1, hs1, hlo1⟩
    step as ⟨i1, hi1⟩
    split
    · simp only [WP.spec_ok]; exact base
    · step with close_spec p h (by omega) (by omega) as ⟨p1, hp1, hs1', ha1, hl1⟩
      obtain ⟨hout, hop⟩ := hp1
      rw [hs1'] at hop
      simp only at hop
      refine ⟨⟨hout, ?_⟩, ?_, hl1, ?_, rfl, rfl, le_refl _, hf, by omega⟩
      · rw [hs1']; exact hop
      · obtain ⟨a, b, c, d'⟩ := hn; simp only [Near, ha1]; omega
      · rw [hop]; simp
  rw [if_neg c14]
  by_cases c12 : b0 = 12#u8
  · rw [if_pos c12]
    -- the flexes
    split
    · step as ⟨b1, hb1⟩
      step as ⟨o1, ho1⟩
      have hF := flex_spec p s b1 M V hs h hn hV hb hl hV2
      apply WP.spec_bind hF
      rintro ⟨i, p1⟩ hk
      simp only [uncurry_apply_pair, WP.spec_ok]
      obtain ⟨a, b, c, d'⟩ := hk
      exact ⟨a, b, by rw [c]; omega, d', rfl, rfl, by simp; omega, by simp; scalar_tac, by omega⟩
    · simp only [WP.spec_ok]; exact base
  · rw [if_neg c12]; simp only [WP.spec_ok]; exact base

/-! ## The interpreter's loop -/

/-- Steps used so far. -/
def used (m : cff_core.Machine) : Nat := 65536 - m.steps.val

/-- The loop's invariant: the pen's, its bounds by the steps used (each step moves the pen less
than 2³⁷ and adds at most 60 pieces, one contour, 24 stems), an operand stack of at most 48
values within 2³¹ with no width offset, and every charstring range within the data. -/
def RunInv (d : Slice U8) (p : cff_core.Pen) (s : cff_core.Stack) (f : cff_core.Frame)
    (frames : alloc.vec.Vec cff_core.Frame) (m : cff_core.Machine) : Prop :=
  PenOk p ∧ Near (2^37 * (used m : Int)) p ∧ m.steps.val ≤ 65536 ∧
  p.contour.val.length ≤ 60 * used m ∧ p.out.val.length ≤ used m ∧ m.stems.val ≤ 24 * used m ∧
  StackOk (2^31) s.v s.lo ∧ s.lo.val = 0 ∧
  f.e.val ≤ d.val.length ∧ frames.val.length ≤ 10 ∧
  (∀ g ∈ frames.val, g.o.val ≤ g.e.val ∧ g.e.val ≤ d.val.length)

theorem stack_val : cff_core.STACK.val = 48 := by simp [cff_core.STACK]
theorem nesting_val : cff_core.NESTING.val = 10 := by simp [cff_core.NESTING]

set_option hygiene false in
/-- `one`'s operand branch (the same for `b0 ≥ 32` and `b0 = 28`). -/
macro "operand_tac" : tactic => `(tactic| (
  step with operand_spec d i1 f.e b0 hfo1 (by omega) as ⟨r, hr⟩
  cases r with
  | none =>
    try simp only [WP.spec_ok, WP.uncurry'_pair]
    exact ⟨tick p s _ frames _ rfl hp hn1 hc1 ho1' hst1 hs hlo hfe hfr hfrs,
      Or.inl ⟨hsi, by omega⟩⟩
  | some q =>
    obtain ⟨x, o1⟩ := q
    obtain ⟨hx, ho1, ho1e⟩ := hr x o1 rfl
    by_cases hov : s.v.len ≥ cff_core.STACK
    · simp only [hov, ↓reduceIte]
      apply (WP.spec_ok _).2
      simp only [WP.uncurry'_pair]
      exact ⟨tick p s _ frames _ rfl hp hn1 hc1 ho1' hst1 hs hlo hfe hfr hfrs,
        Or.inl ⟨hsi, by omega⟩⟩
    · simp only [hov, ↓reduceIte]
      have hlt' : s.v.val.length < 48 := by
        have := stack_val; scalar_tac
      apply WP.spec_bind (alloc.vec.Vec.push_spec s.v x (by scalar_tac))
      intro v1 hv1
      apply (WP.spec_ok _).2
      simp only [WP.uncurry'_pair]
      refine ⟨tick p _ _ frames _ rfl hp hn1 hc1 ho1' hst1 ?_ hlo hfe hfr hfrs,
        Or.inl ⟨hsi, by omega⟩⟩
      refine ⟨by simp [hv1]; omega, by simp [hv1]; omega, ?_⟩
      intro y hy
      simp only [hv1, List.mem_append, List.mem_singleton] at hy
      rcases hy with hy | rfl
      · exact hs.2.2 y hy
      · exact hx))

theorem one_spec (d : Slice U8) (gsubrs local_ : Slice Usize) (f : cff_core.Frame)
    (frames : alloc.vec.Vec cff_core.Frame) (p : cff_core.Pen) (s : cff_core.Stack)
    (m : cff_core.Machine) (hd : d.val.length + 2^20 ≤ Usize.max)
    (hI : RunInv d p s f frames m) (hfo : f.o.val < f.e.val) :
    cff_core.one d gsubrs local_ f frames p s m ⦃ e' f' frames' p' s' m' =>
      RunInv d p' s' f' frames' m' ∧
      ((m'.steps.val + 1 = m.steps.val ∧ frames'.val.length ≤ frames.val.length + 1) ∨
        e' ≠ cff_core.OK) ⦄ := by
  obtain ⟨hp, hn, hsteps, hc, ho, hst, hs, hlo, hfe, hfr, hfrs⟩ := hI
  unfold cff_core.one
  split
  · simp only [WP.spec_ok]
    exact ⟨⟨hp, hn, hsteps, hc, ho, hst, hs, hlo, hfe, hfr, hfrs⟩, Or.inr (by simp [cff_core.TOO_LONG, cff_core.OK])⟩
  rename_i hs0
  have hs0' : m.steps.val ≠ 0 := by intro h; apply hs0; scalar_tac
  step as ⟨i, hi⟩
  step as ⟨b0⟩
  step as ⟨i1, hi1⟩
  -- one step more used: every bound grows
  have hu : used { m with steps := i } = used m + 1 := by simp only [used]; omega
  have tick : ∀ (p' : cff_core.Pen) (s' : cff_core.Stack) (f' : cff_core.Frame)
      (frames' : alloc.vec.Vec cff_core.Frame) (m' : cff_core.Machine), m'.steps = i →
      PenOk p' → Near (2^37 * (used m + 1 : Nat)) p' →
      p'.contour.val.length ≤ 60 * (used m + 1) → p'.out.val.length ≤ used m + 1 →
      m'.stems.val ≤ 24 * (used m + 1) → StackOk (2^31) s'.v s'.lo → s'.lo.val = 0 →
      f'.e.val ≤ d.val.length → frames'.val.length ≤ 10 →
      (∀ g ∈ frames'.val, g.o.val ≤ g.e.val ∧ g.e.val ≤ d.val.length) →
      RunInv d p' s' f' frames' m' := by
    intro p' s' f' frames' m' hm a b c e g k l q r t
    have : used m' = used m + 1 := by simp only [used, hm]; omega
    exact ⟨a, by rw [this]; exact b, by rw [hm]; omega, by rw [this]; exact c, by rw [this]; exact e,
      by rw [this]; exact g, k, l, q, r, t⟩
  have hn1 : Near (2^37 * (used m + 1 : Nat)) p := near_mono hn (by push_cast; omega)
  have hfo1 : i1.val ≤ f.e.val := by omega
  have hc1 : p.contour.val.length ≤ 60 * (used m + 1) := by omega
  have ho1' : p.out.val.length ≤ used m + 1 := by omega
  have hst1 : m.stems.val ≤ 24 * (used m + 1) := by omega
  have hsi : i.val + 1 = m.steps.val := by omega
  by_cases c32 : b0 ≥ 32#u8
  · rw [if_pos c32]; operand_tac
  rw [if_neg c32]
  by_cases c28 : b0 = 28#u8
  · rw [if_pos c28]; operand_tac
  rw [if_neg c28]
  by_cases c10 : b0 = 10#u8
  · rw [if_pos c10]
    have hpop : alloc.vec.Vec.pop alloc.alloc.Global s.v ⦃ r =>
        (∀ k, r.1 = some k → Small (2^31) k) ∧ StackOk (2^31) r.2 s.lo ⦄ := by
      simp only [alloc.vec.Vec.pop, WP.spec_ok]
      refine ⟨fun k hk => hs.2.2 k (List.mem_of_getLast? hk), ?_, ?_, ?_⟩
      · simp; omega
      · simp; have := hs.2.1; omega
      · intro y hy; exact hs.2.2 y (List.mem_of_mem_dropLast (by simpa using hy))
    apply WP.spec_bind hpop
    rintro ⟨o, v⟩ ⟨hk, hsv⟩
    simp only [uncurry_apply_pair]
    have base : ∀ e' : U8, RunInv d p { s with v := v } { f with o := i1 } frames { m with steps := i } ∧
        ((i.val + 1 = m.steps.val ∧ frames.val.length ≤ frames.val.length + 1) ∨ e' ≠ cff_core.OK) :=
      fun _ => ⟨tick p _ _ frames _ rfl hp hn1 hc1 ho1' hst1 hsv hlo hfe hfr hfrs, Or.inl ⟨hsi, by omega⟩⟩
    cases o with
    | none => simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
    | some k =>
      have hk' := hk k rfl
      obtain ⟨hk0, hk1⟩ := hk'
      simp only []
      have hone := one_val
      step as ⟨r2, hr2⟩
      by_cases hz : (r2 != 0#i64) = true
      · rw [if_pos hz]; simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
      · rw [if_neg hz]
        apply WP.spec_bind (m := if b0 = 10#u8 then ok local_ else ok gsubrs) (Pₘ := fun _ : Slice Usize => True)
        · by_cases hb : b0 = 10#u8 <;> simp [hb]
        intro idx _
        step as ⟨k3, hk3⟩
        have hk3b : -2^40 ≤ k3.val ∧ k3.val ≤ 2^40 := by
          have hr0 : r2.val = 0 := by simpa using hz
          have hdm := Int.tmod_add_mul_tdiv k.val 65536
          rw [one_val] at hr2 hk3
          rw [← hr2, hr0, ← hk3] at hdm
          constructor <;> omega
        step with subr_spec d idx k3 hk3b as ⟨o1, ho1s⟩
        cases o1 with
        | none => simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
        | some ab =>
          obtain ⟨a, b⟩ := ab
          obtain ⟨hab, hbd⟩ := ho1s a b rfl
          apply WP.spec_bind (Pₘ := fun i5 : Usize => i5.val = frames.val.length + 1)
          · step*
          intro i5 hi5
          by_cases hnest : i5 > cff_core.NESTING
          · rw [if_pos hnest]; simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
          · rw [if_neg hnest]
            have h10 : frames.val.length + 1 ≤ 10 := by have := nesting_val; scalar_tac
            apply WP.spec_bind (alloc.vec.Vec.push_spec frames { f with o := i1 } (by scalar_tac))
            intro frames1 hfr1
            simp only [WP.spec_ok, WP.uncurry'_pair]
            refine ⟨tick p _ _ _ _ rfl hp hn1 hc1 ho1' hst1 hsv hlo (by simpa using hbd)
              (by rw [hfr1]; simp; omega) ?_, Or.inl ⟨hsi, by rw [hfr1]; simp⟩⟩
            intro g hg
            rw [hfr1] at hg
            simp only [List.mem_append, List.mem_singleton] at hg
            rcases hg with hg | rfl
            · exact hfrs g hg
            · exact ⟨by simpa using hfo1, by simpa using hfe⟩
  rw [if_neg c10]
  by_cases c29 : b0 = 29#u8
  · rw [if_pos c29]
    have hpop : alloc.vec.Vec.pop alloc.alloc.Global s.v ⦃ r =>
        (∀ k, r.1 = some k → Small (2^31) k) ∧ StackOk (2^31) r.2 s.lo ⦄ := by
      simp only [alloc.vec.Vec.pop, WP.spec_ok]
      refine ⟨fun k hk => hs.2.2 k (List.mem_of_getLast? hk), ?_, ?_, ?_⟩
      · simp; omega
      · simp; have := hs.2.1; omega
      · intro y hy; exact hs.2.2 y (List.mem_of_mem_dropLast (by simpa using hy))
    apply WP.spec_bind hpop
    rintro ⟨o, v⟩ ⟨hk, hsv⟩
    simp only [uncurry_apply_pair]
    have base : ∀ e' : U8, RunInv d p { s with v := v } { f with o := i1 } frames { m with steps := i } ∧
        ((i.val + 1 = m.steps.val ∧ frames.val.length ≤ frames.val.length + 1) ∨ e' ≠ cff_core.OK) :=
      fun _ => ⟨tick p _ _ frames _ rfl hp hn1 hc1 ho1' hst1 hsv hlo hfe hfr hfrs, Or.inl ⟨hsi, by omega⟩⟩
    cases o with
    | none => simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
    | some k =>
      have hk' := hk k rfl
      obtain ⟨hk0, hk1⟩ := hk'
      simp only []
      have hone := one_val
      step as ⟨r2, hr2⟩
      by_cases hz : (r2 != 0#i64) = true
      · rw [if_pos hz]; simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
      · rw [if_neg hz]
        apply WP.spec_bind (m := if b0 = 10#u8 then ok local_ else ok gsubrs) (Pₘ := fun _ : Slice Usize => True)
        · by_cases hb : b0 = 10#u8 <;> simp [hb]
        intro idx _
        step as ⟨k3, hk3⟩
        have hk3b : -2^40 ≤ k3.val ∧ k3.val ≤ 2^40 := by
          have hr0 : r2.val = 0 := by simpa using hz
          have hdm := Int.tmod_add_mul_tdiv k.val 65536
          rw [one_val] at hr2 hk3
          rw [← hr2, hr0, ← hk3] at hdm
          constructor <;> omega
        step with subr_spec d idx k3 hk3b as ⟨o1, ho1s⟩
        cases o1 with
        | none => simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
        | some ab =>
          obtain ⟨a, b⟩ := ab
          obtain ⟨hab, hbd⟩ := ho1s a b rfl
          apply WP.spec_bind (Pₘ := fun i5 : Usize => i5.val = frames.val.length + 1)
          · step*
          intro i5 hi5
          by_cases hnest : i5 > cff_core.NESTING
          · rw [if_pos hnest]; simp only [WP.spec_ok, WP.uncurry'_pair]; exact base _
          · rw [if_neg hnest]
            have h10 : frames.val.length + 1 ≤ 10 := by have := nesting_val; scalar_tac
            apply WP.spec_bind (alloc.vec.Vec.push_spec frames { f with o := i1 } (by scalar_tac))
            intro frames1 hfr1
            simp only [WP.spec_ok, WP.uncurry'_pair]
            refine ⟨tick p _ _ _ _ rfl hp hn1 hc1 ho1' hst1 hsv hlo (by simpa using hbd)
              (by rw [hfr1]; simp; omega) ?_, Or.inl ⟨hsi, by rw [hfr1]; simp⟩⟩
            intro g hg
            rw [hfr1] at hg
            simp only [List.mem_append, List.mem_singleton] at hg
            rcases hg with hg | rfl
            · exact hfrs g hg
            · exact ⟨by simpa using hfo1, by simpa using hfe⟩
  rw [if_neg c29]
  have hfrp : alloc.vec.Vec.pop alloc.alloc.Global frames ⦃ r =>
      (∀ g, r.1 = some g → g.o.val ≤ g.e.val ∧ g.e.val ≤ d.val.length) ∧
      r.2.val.length ≤ frames.val.length ∧
      (∀ g ∈ r.2.val, g.o.val ≤ g.e.val ∧ g.e.val ≤ d.val.length) ⦄ := by
    simp only [alloc.vec.Vec.pop, WP.spec_ok]
    refine ⟨fun g hg => hfrs g (List.mem_of_getLast? hg), by simp, ?_⟩
    intro g hg; exact hfrs g (List.mem_of_mem_dropLast (by simpa using hg))
  by_cases c11 : b0 = 11#u8
  · rw [if_pos c11]
    apply WP.spec_bind hfrp
    rintro ⟨o, frames1⟩ ⟨hg, hl1, hfr1⟩
    simp only [uncurry_apply_pair]
    cases o with
    | none =>
      simp only [WP.spec_ok, WP.uncurry'_pair]
      exact ⟨tick p s _ _ _ rfl hp hn1 hc1 ho1' hst1 hs hlo hfe (by simp at hl1 ⊢; omega) hfr1,
        Or.inl ⟨hsi, by simp at hl1 ⊢; omega⟩⟩
    | some g =>
      obtain ⟨hgo, hge⟩ := hg g rfl
      simp only [WP.spec_ok, WP.uncurry'_pair]
      exact ⟨tick p s _ _ _ rfl hp hn1 hc1 ho1' hst1 hs hlo hge (by simp at hl1 ⊢; omega) hfr1,
        Or.inl ⟨hsi, by simp at hl1 ⊢; omega⟩⟩
  rw [if_neg c11]
  -- an operator
  have hU : used m ≤ 65535 := by simp only [used]; omega
  have hM : (2:Int)^37 * (used m : Int) + 150 * 2^31 ≤ 2^62 := by
    have : (used m : Int) ≤ 65535 := by exact_mod_cast hU
    nlinarith
  step with op_spec d { f with o := i1 } p s m.stems m.width_read m.done b0 (2^37 * (used m : Int))
    (2^31) hs hlo hp hn (by norm_num) (by norm_num) hM (by scalar_tac) (by scalar_tac) hfo1
    (by simp; scalar_tac) (by scalar_tac) as ⟨e1, f1, p1, s1, st1, wr1, dn1, hop⟩
  obtain ⟨hp1, hn1', ho1, hc1', hfs, hfe1, hfo1', hfo1e, hst1'⟩ := hop
  simp only [alloc.vec.Vec.clear, bind_ok, WP.spec_ok, WP.uncurry'_pair]
  refine ⟨tick p1 _ f1 frames _ rfl hp1 (near_mono hn1' ?_) (by omega) (by omega) (by simp; omega)
    ⟨by simp, by simp [alloc.vec.Vec.new], by simp [alloc.vec.Vec.new]⟩ (by simp)
    (by rw [hfe1]; exact hfe) hfr hfrs, Or.inl ⟨hsi, by omega⟩⟩
  push_cast; nlinarith

theorem run_loop_spec (d : Slice U8) (gsubrs local_ : Slice Usize) (p : cff_core.Pen)
    (s : cff_core.Stack) (f : cff_core.Frame) (frames : alloc.vec.Vec cff_core.Frame)
    (m : cff_core.Machine) (err : U8) (hd : d.val.length + 2^20 ≤ Usize.max)
    (hI : RunInv d p s f frames m) :
    cff_core.run_loop d gsubrs local_ p s f frames m err ⦃ p' m' e' =>
      PenOk p' ∧ p'.contour.val.length ≤ 60 * 65536 ∧ p'.out.val.length ≤ 65536 ⦄ := by
  unfold cff_core.run_loop
  apply loop.spec_decr_nat
    (fun (st : cff_core.Pen × cff_core.Stack × cff_core.Frame × alloc.vec.Vec cff_core.Frame ×
        cff_core.Machine × U8) =>
      if st.2.2.2.2.2 = cff_core.OK then 12 * st.2.2.2.2.1.steps.val + st.2.2.2.1.val.length + 1 else 0)
    (fun st => RunInv d st.1 st.2.1 st.2.2.1 st.2.2.2.1 st.2.2.2.2.1)
  · rintro ⟨p1, s1, f1, frames1, m1, e1⟩ hI1
    simp only at hI1
    have fin : RunInv d p1 s1 f1 frames1 m1 →
        PenOk p1 ∧ p1.contour.val.length ≤ 60 * 65536 ∧ p1.out.val.length ≤ 65536 := by
      rintro ⟨a, _, hs, c, o, _⟩
      refine ⟨a, ?_, ?_⟩ <;> simp only [used] at c o <;> omega
    unfold cff_core.run_loop.body
    simp only
    -- one step of `one`, from a frame with a byte left
    have stepOne : f1.o.val < f1.e.val → e1 = cff_core.OK →
        (do
          let (err1, f2, frames2, p2, s2, m2) ← cff_core.one d gsubrs local_ f1 frames1 p1 s1 m1
          ok (cont (p2, s2, f2, frames2, m2, err1))) ⦃ r =>
          match r with
          | done y => WP.uncurry' (fun p' => WP.uncurry' fun (m' : cff_core.Machine) (e' : U8) =>
              PenOk p' ∧ p'.contour.val.length ≤ 60 * 65536 ∧ p'.out.val.length ≤ 65536) y
          | cont x' => RunInv d x'.1 x'.2.1 x'.2.2.1 x'.2.2.2.1 x'.2.2.2.2.1 ∧
              (if x'.2.2.2.2.2 = cff_core.OK then 12 * x'.2.2.2.2.1.steps.val + x'.2.2.2.1.val.length + 1
               else 0) <
              12 * m1.steps.val + frames1.val.length + 1 ⦄ := by
      intro hlt he
      apply WP.spec_bind (one_spec d gsubrs local_ f1 frames1 p1 s1 m1 hd hI1 hlt)
      rintro ⟨err1, f2, frames2, p2, s2, m2⟩ ⟨hI2, hdec⟩
      simp only [uncurry_apply_pair, WP.spec_ok]
      refine ⟨hI2, ?_⟩
      split
      · rcases hdec with ⟨h1, h2⟩ | h
        · omega
        · contradiction
      · omega
    -- a subroutine run off its end returns
    have popOne : frames1.val ≠ [] → e1 = cff_core.OK →
        (do
          let (o, frames2) ← alloc.vec.Vec.pop alloc.alloc.Global frames1
          let f2 ← lift (core.option.Option.unwrap_or o f1)
          ok (cont (p1, s1, f2, frames2, m1, e1))) ⦃ r =>
          match r with
          | done y => WP.uncurry' (fun p' => WP.uncurry' fun (m' : cff_core.Machine) (e' : U8) =>
              PenOk p' ∧ p'.contour.val.length ≤ 60 * 65536 ∧ p'.out.val.length ≤ 65536) y
          | cont x' => RunInv d x'.1 x'.2.1 x'.2.2.1 x'.2.2.2.1 x'.2.2.2.2.1 ∧
              (if x'.2.2.2.2.2 = cff_core.OK then 12 * x'.2.2.2.2.1.steps.val + x'.2.2.2.1.val.length + 1
               else 0) <
              12 * m1.steps.val + frames1.val.length + 1 ⦄ := by
      intro hne he
      obtain ⟨a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11⟩ := hI1
      simp only [alloc.vec.Vec.pop, bind_ok, uncurry_apply_pair, lift, core.option.Option.unwrap_or,
        WP.spec_ok]
      have hlen : frames1.val.length ≠ 0 := by simpa using hne
      refine ⟨⟨a1, a2, a3, a4, a5, a6, a7, a8, ?_, by simp; omega, ?_⟩, ?_⟩
      · cases hg : frames1.val.getLast? with
        | none => simp at hg; exact absurd hg hne
        | some g => simp only [Option.getD_some]; exact (a11 g (List.mem_of_getLast? hg)).2
      · intro g hg; exact a11 g (List.mem_of_mem_dropLast (by simpa using hg))
      · simp only [he, ↓reduceIte]; simp; omega
    split
    · rename_i he
      split
      · simp only [WP.spec_ok]; exact fin hI1
      · split
        · simp only [WP.spec_ok]; exact fin hI1
        · split
          · rename_i hlt
            have hlt' : f1.o.val < f1.e.val := hlt
            rw [if_neg (by scalar_tac)]
            exact WP.spec_mono (stepOne hlt' he) (fun r hr => by
              cases r <;> (simp only at hr ⊢; first | exact hr | (split at hr <;> split <;> simp_all)))
          · step as ⟨b, hb⟩
            split
            · simp only [WP.spec_ok]; exact fin hI1
            · rename_i hb'
              have hne : frames1.val ≠ [] := by intro h; apply hb'; simp_all [alloc.vec.Vec.len]
              split
              · exact WP.spec_mono (popOne hne he) (fun r hr => by
                  cases r <;> (simp only at hr ⊢; first | exact hr | (split at hr <;> split <;> simp_all)))
              · rename_i hge
                exact WP.spec_mono (stepOne (by scalar_tac) he) (fun r hr => by
                  cases r <;> (simp only at hr ⊢; first | exact hr | (split at hr <;> split <;> simp_all)))
    · simp only [WP.spec_ok]; exact fin hI1
  · exact hI

theorem fail_ok (c : U8) : ∃ e, cff_core.fail c = ok e := by
  unfold cff_core.fail; split <;> exact ⟨_, rfl⟩

/-- **The charstring machine never fails, and every contour it returns is closed**: for any bytes,
any charstring range and any subroutine INDEXes, `cff_core::run` returns (no overflow, no index out
of range, no runaway: at most 2¹⁶ steps), and when it returns contours, each chains from a start
and ends back on it. -/
theorem run_spec (d : Slice U8) (cs : Usize × Usize) (gsubrs local_ : Slice Usize)
    (hd : d.val.length + 2^20 ≤ Usize.max) :
    cff_core.run d cs gsubrs local_ ⦃ r => ∀ out, r = .Ok out → ∀ c ∈ out.val, Closed c.val ⦄ := by
  obtain ⟨c0, c1⟩ := cs
  unfold cff_core.run
  simp only []
  apply WP.spec_bind (Pₘ := fun e : Usize => e.val ≤ d.val.length)
  · split
    · simp only [WP.spec_ok]; scalar_tac
    · simp
  intro e he
  have hS : cff_core.STEPS ⦃ x => x.val = 65536 ⦄ := by
    unfold cff_core.STEPS
    step*
    · rcases System.Platform.numBits_eq with h | h <;> simp [h]
    · rcases System.Platform.numBits_eq with h | h <;> simp_all [Usize.size, Usize.numBits]
  apply WP.spec_bind hS
  intro st hst
  have hI : RunInv d (⟨⟨0#i64, 0#i64⟩, none, alloc.vec.Vec.new _, alloc.vec.Vec.new _⟩ : cff_core.Pen)
      (⟨alloc.vec.Vec.with_capacity I64 cff_core.STACK, 0#usize⟩ : cff_core.Stack)
      (⟨c0, e, c0⟩ : cff_core.Frame) (alloc.vec.Vec.with_capacity cff_core.Frame cff_core.NESTING)
      (⟨0#usize, false, st, false, false⟩ : cff_core.Machine) := by
    refine ⟨⟨by simp [alloc.vec.Vec.new], by simp [alloc.vec.Vec.new]⟩, ?_, by simp; omega,
      by simp [alloc.vec.Vec.new], by simp [alloc.vec.Vec.new], by simp, ?_, by simp, he,
      by simp [alloc.vec.Vec.with_capacity], by simp [alloc.vec.Vec.with_capacity]⟩
    · simp only [Near, used, hst]; simp
    · refine ⟨by simp, by simp [alloc.vec.Vec.with_capacity], by simp [alloc.vec.Vec.with_capacity]⟩
  apply WP.spec_bind (run_loop_spec d gsubrs local_ _ _ _ _ _ cff_core.OK hd hI)
  rintro ⟨p, m, err⟩ ⟨hp, hc, ho⟩
  simp only [uncurry_apply_pair]
  split
  · obtain ⟨fe, hfe⟩ := fail_ok err
    simp [hfe]
  · split
    · step with close_spec p hp (by scalar_tac) (by scalar_tac) as ⟨p1, hp1, _, _, _⟩
      simp only [WP.spec_ok, core.result.Result.Ok.injEq]
      rintro out rfl c hc'
      exact hp1.1 c hc'
    · obtain ⟨fe, hfe⟩ := fail_ok cff_core.NO_END
      simp [hfe]

end ForgeFont
