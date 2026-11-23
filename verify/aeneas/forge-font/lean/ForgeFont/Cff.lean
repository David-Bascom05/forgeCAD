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

end ForgeFont
