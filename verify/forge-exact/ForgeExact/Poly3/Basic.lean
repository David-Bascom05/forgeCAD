import ForgeExact.Poly3.Roots
import ForgeExact.Quad.Basic

/-!
# Arrangements of polynomials of any degree: openness and escape

Step 4 of the exact reference (tori, degree 4): the two analytic facts the arrangement theorem
needs, for polynomials in `x, y, z` of any degree with rational coefficients.

* `box_bound`: over the box of half-width `h` about `c`, `|p(x) − p(c)| ≤ bound p c h`, where
  each monomial contributes `|a| (Π(|c_v| + h) − Π|c_v|)`. This is exact and computable: the
  reference certifies boxes with it.
* `sign_nbhd`: a polynomial's sign is locally constant off its zeros.
* `escape`: in every basic open set there is a point where a nonzero polynomial (nonzero
  somewhere) is nonzero. Restricted to the line toward a nonzero point, it is a univariate
  polynomial (`line`, `ev_line`). If it vanished at as many points of the open set as it has
  coefficients, it would vanish everywhere (`Poly3.zero_of_roots`), at that point too.
-/

namespace ForgeExact.Poly3

open ForgeExact Topology Ref Flat Alg

/-- A coordinate. -/
def coord (x : V3) : Fin 3 → Rat
  | 0 => x.x
  | 1 => x.y
  | 2 => x.z

/-- A monomial: its coefficient and its variables, with multiplicity. -/
structure Mono where
  a : Rat
  vs : List (Fin 3)
  deriving Inhabited, Repr, BEq, DecidableEq

/-- A polynomial: a sum of monomials. -/
abbrev P3 := List Mono

def prodAt (vs : List (Fin 3)) (x : V3) : Rat := vs.foldr (fun v acc => coord x v * acc) 1

def Mono.eval (m : Mono) (x : V3) : Rat := m.a * prodAt m.vs x

def P3.eval (p : P3) (x : V3) : Rat := p.foldr (fun m acc => m.eval x + acc) 0

/-- Nonzero somewhere. -/
def P3.NZ (p : P3) : Prop := ∃ y, p.eval y ≠ 0

/-! ## Restriction to a line -/

/-- The monomial's variables along `x0 + t d`, as a polynomial in `t`. -/
def lineProd (x0 d : V3) : List (Fin 3) → Poly
  | [] => [1]
  | v :: vs => pmul [coord x0 v, coord d v] (lineProd x0 d vs)

/-- The polynomial along `x0 + t d`. -/
def line (p : P3) (x0 d : V3) : Poly :=
  p.foldr (fun m acc => padd (pscale m.a (lineProd x0 d m.vs)) acc) []

theorem coord_step (x0 d : V3) (t : Rat) (v : Fin 3) :
    coord (x0 + t * d) v = coord x0 v + t * coord d v := by
  match v with
  | 0 => rfl
  | 1 => rfl
  | 2 => rfl

theorem ev_lineProd (x0 d : V3) (t : Rat) : ∀ vs : List (Fin 3),
    Poly.ev (lineProd x0 d vs) t = prodAt vs (x0 + t * d)
  | [] => by simp [lineProd, prodAt, Poly.ev]; grind
  | v :: vs => by
    have ih := ev_lineProd x0 d t vs
    rw [Poly.ev_eq] at ih
    simp only [lineProd, prodAt, List.foldr_cons] at ih ⊢
    rw [Poly.ev_eq, eval_pmul, ih, coord_step]
    simp only [evalWith_cons, evalWith_nil, idHom]
    show (coord x0 v + t * (coord d v + t * 0)) * _ = _
    grind

theorem ev_line (x0 d : V3) (t : Rat) : ∀ p : P3, Poly.ev (line p x0 d) t = p.eval (x0 + t * d)
  | [] => by simp [line, Poly.ev, P3.eval]
  | m :: p => by
    have ih := ev_line x0 d t p
    simp only [line, P3.eval, List.foldr_cons] at ih ⊢
    rw [Poly.ev_eq, eval_padd, eval_pscale, ← Poly.ev_eq, ← Poly.ev_eq, ih, ev_lineProd]
    simp only [Mono.eval, idHom]; rfl

/-! ## Escape -/

theorem nodup_map_inj {f : Nat → Rat} (hf : ∀ a b, f a = f b → a = b) :
    ∀ l : List Nat, l.Nodup → (l.map f).Nodup
  | [], _ => List.nodup_nil
  | a :: l, h => by
    rw [List.map_cons, List.nodup_cons]
    obtain ⟨ha, hl⟩ := List.nodup_cons.mp h
    refine ⟨fun hm => ?_, nodup_map_inj hf l hl⟩
    obtain ⟨b, hb, e⟩ := List.mem_map.mp hm
    exact ha (hf b a e ▸ hb)

theorem nodup_steps (s : Rat) (hs : 0 < s) (n : Nat) :
    ((List.range n).map fun (i : Nat) => s / ((i : Rat) + 1)).Nodup := by
  apply nodup_map_inj _ _ List.nodup_range
  intro i j h
  try simp only at h
  have hi : (0 : Rat) < (i : Rat) + 1 := by have := Rat.natCast_nonneg (a := i); grind
  have hj : (0 : Rat) < (j : Rat) + 1 := by have := Rat.natCast_nonneg (a := j); grind
  have : s * ((j : Rat) + 1) = s * ((i : Rat) + 1) := by
    have e1 := Rat.div_mul_cancel (a := s) (Rat.ne_of_gt hi)
    have e2 := Rat.div_mul_cancel (a := s) (Rat.ne_of_gt hj)
    rw [h] at e1
    grind
  have : (j : Rat) = (i : Rat) := by
    have := Rat.mul_eq_zero.mp (show s * ((j : Rat) - (i : Rat)) = 0 by grind)
    rcases this with h | h
    · grind
    · grind
  exact (Rat.natCast_inj.mp this).symm

/-- **Escape**: in any basic open set, a nonzero polynomial is nonzero somewhere. -/
theorem escape_one (hs : List Plane) (p : P3) (hp : p.NZ) {x : V3} (hx : allPos hs x) :
    ∃ y, allPos hs y ∧ p.eval y ≠ 0 := by
  by_cases h0 : p.eval x = 0
  · obtain ⟨y, hy⟩ := hp
    let d := y - x
    let f := line p x d
    let s := qmin 1 (stepFor x d hs)
    have hs0 : 0 < s := by
      have := stepFor_pos x d hs hx
      simp only [s]; unfold qmin; split <;> grind
    have hsle : s ≤ stepFor x d hs := qmin_le_right _ _
    let rs := (List.range f.length).map fun (i : Nat) => s / ((i : Rat) + 1)
    have hin : ∀ r ∈ rs, 0 < r ∧ r ≤ s := by
      intro r hr
      obtain ⟨i, _, rfl⟩ := List.mem_map.mp hr
      have hi : (1 : Rat) ≤ (i : Rat) + 1 := by have := Rat.natCast_nonneg (a := i); grind
      have hq : 0 < s / ((i : Rat) + 1) := div_pos' hs0 (by grind)
      constructor
      · exact hq
      · have k := Rat.mul_le_mul_of_nonneg_left hi (Rat.le_of_lt hq)
        rw [Rat.div_mul_cancel (by grind)] at k
        grind
    by_cases hall : ∀ r ∈ rs, Poly.ev f r = 0
    · exfalso
      have hz := zero_of_roots f rs (nodup_steps s hs0 _) (by simp [rs]) hall 1
      rw [ev_line] at hz
      have e : x + (1 : Rat) * d = y := by
        obtain ⟨a, b, c⟩ := x; obtain ⟨a', b', c'⟩ := y
        show (⟨a + 1 * (a' - a), b + 1 * (b' - b), c + 1 * (c' - c)⟩ : V3) = ⟨a', b', c'⟩
        congr 1 <;> grind
      rw [e] at hz; exact hy hz
    · obtain ⟨r, hr, hfr⟩ : ∃ r ∈ rs, Poly.ev f r ≠ 0 := by
        apply Classical.byContradiction; intro hn
        exact hall fun r hr => Classical.byContradiction fun h => hn ⟨r, hr, h⟩
      obtain ⟨hr0, hrs⟩ := hin r hr
      refine ⟨x + r * d, fun P hP => ?_, by rw [← ev_line]; exact hfr⟩
      rw [Flat.eval_step]
      exact pos_of_small (hx P hP) hr0
        (Rat.le_trans (Rat.le_trans hrs hsle) (stepFor_le x d hs P hP))
  · exact ⟨x, hx, h0⟩

/-! ## The box bound -/

theorem qabs_mul (a b : Rat) : qabs (a * b) = qabs a * qabs b := by
  unfold qabs
  rcases Lean.Grind.LinearOrder.trichotomy a 0 with ha | ha | ha <;>
    rcases Lean.Grind.LinearOrder.trichotomy b 0 with hb | hb | hb
  all_goals first
    | (subst ha; simp)
    | (subst hb; simp)
    | (have := Lean.Grind.OrderedRing.mul_pos_of_neg_of_neg ha hb
       rw [if_neg (by grind), if_pos ha, if_pos hb]; grind)
    | (have := Lean.Grind.OrderedRing.mul_neg_of_neg_of_pos ha hb
       rw [if_pos this, if_pos ha, if_neg (by grind)]; grind)
    | (have := Lean.Grind.OrderedRing.mul_neg_of_pos_of_neg ha hb
       rw [if_pos this, if_neg (by grind), if_pos hb]; grind)
    | (have := Lean.Grind.OrderedRing.mul_pos ha hb
       rw [if_neg (by grind), if_neg (by grind), if_neg (by grind)])

theorem qabs_add_le (a b : Rat) : qabs (a + b) ≤ qabs a + qabs b := by
  unfold qabs; split <;> split <;> split <;> grind

theorem qabs_sub_le (a b : Rat) (h : Rat) (hab : qabs (a - b) ≤ h) : qabs a ≤ qabs b + h := by
  have := qabs_add_le (a - b) b; rw [show a - b + b = a by grind] at this; grind

/-- `Π (|y_v| + h)`. -/
def prodAbs (vs : List (Fin 3)) (y : V3) (h : Rat) : Rat :=
  vs.foldr (fun v acc => (qabs (coord y v) + h) * acc) 1

theorem prodAbs_nonneg (vs : List (Fin 3)) (y : V3) {h : Rat} (hh : 0 ≤ h) : 0 ≤ prodAbs vs y h := by
  induction vs with
  | nil => simp [prodAbs]; decide
  | cons v vs ih =>
    simp only [prodAbs, List.foldr_cons] at ih ⊢
    exact Rat.mul_nonneg (by have := qabs_nonneg (coord y v); grind) ih

/-- Within half-width `h` of `y` on every coordinate. -/
def Near (y p : V3) (h : Rat) : Prop := ∀ v : Fin 3, qabs (coord p v - coord y v) ≤ h

theorem prod_bound (y p : V3) {h : Rat} (hh : 0 ≤ h) (hn : Near y p h) : ∀ vs : List (Fin 3),
    qabs (prodAt vs p) ≤ prodAbs vs y h ∧
      qabs (prodAt vs p - prodAt vs y) ≤ prodAbs vs y h - prodAbs vs y 0
  | [] => by
    simp only [prodAt, prodAbs, List.foldr_nil]; unfold qabs; constructor <;> split <;> grind
  | v :: vs => by
    obtain ⟨ih1, ih2⟩ := prod_bound y p hh hn vs
    have ihy := (prod_bound y y (by decide : (0 : Rat) ≤ 0) (fun w => by unfold qabs; split <;> grind) vs).1
    simp only [prodAt, prodAbs, List.foldr_cons] at ih1 ih2 ihy ⊢
    generalize List.foldr (fun v acc => coord p v * acc) 1 vs = Rp at *
    generalize List.foldr (fun v acc => coord y v * acc) 1 vs = Ry at *
    generalize List.foldr (fun v acc => (qabs (coord y v) + h) * acc) 1 vs = Ah at *
    generalize List.foldr (fun v acc => (qabs (coord y v) + 0) * acc) 1 vs = A0 at *
    have hv := hn v
    generalize coord p v = up at *
    generalize coord y v = uy at *
    have hup : qabs up ≤ qabs uy + h := qabs_sub_le up uy h hv
    have huy := qabs_nonneg uy
    have hA0 : 0 ≤ A0 := Rat.le_trans (qabs_nonneg _) ihy
    have hAh : 0 ≤ Ah := Rat.le_trans (qabs_nonneg _) ih1
    constructor
    · rw [qabs_mul]
      have := Rat.mul_le_mul_of_nonneg_left ih1 (qabs_nonneg up)
      have := Rat.mul_le_mul_of_nonneg_right hup hAh
      grind
    · -- up Rp − uy Ry = up (Rp − Ry) + (up − uy) Ry
      have e : up * Rp - uy * Ry = up * (Rp - Ry) + (up - uy) * Ry := by grind
      rw [e]
      have t := qabs_add_le (up * (Rp - Ry)) ((up - uy) * Ry)
      rw [qabs_mul, qabs_mul] at t
      have k1 := Rat.mul_le_mul_of_nonneg_left ih2 (qabs_nonneg up)
      have k2 := Rat.mul_le_mul_of_nonneg_right hup (show 0 ≤ Ah - A0 by have := qabs_nonneg (Rp - Ry); grind)
      have k3 := Rat.mul_le_mul_of_nonneg_left ihy (qabs_nonneg (up - uy))
      have k4 := Rat.mul_le_mul_of_nonneg_right hv hA0
      have : qabs (Rp - Ry) ≤ Ah - A0 := ih2
      grind

/-- The bound on `|p(x) − p(y)|` over `Near y · h`. -/
def bound (p : P3) (y : V3) (h : Rat) : Rat :=
  p.foldr (fun m acc => qabs m.a * (prodAbs m.vs y h - prodAbs m.vs y 0) + acc) 0

/-- **The box bound.** -/
theorem box_bound (p : P3) (y x : V3) {h : Rat} (hh : 0 ≤ h) (hn : Near y x h) :
    qabs (p.eval x - p.eval y) ≤ bound p y h := by
  induction p with
  | nil => simp only [P3.eval, bound, List.foldr_nil]; unfold qabs; split <;> grind
  | cons m p ih =>
    simp only [P3.eval, bound, List.foldr_cons] at ih ⊢
    generalize List.foldr (fun m acc => m.eval x + acc) 0 p = Ex at *
    generalize List.foldr (fun m acc => m.eval y + acc) 0 p = Ey at *
    have e : m.eval x + Ex - (m.eval y + Ey) = m.a * (prodAt m.vs x - prodAt m.vs y) + (Ex - Ey) := by
      simp only [Mono.eval]; grind
    rw [e]
    have t := qabs_add_le (m.a * (prodAt m.vs x - prodAt m.vs y)) (Ex - Ey)
    rw [qabs_mul] at t
    have k := Rat.mul_le_mul_of_nonneg_left (prod_bound y x hh hn m.vs).2 (qabs_nonneg m.a)
    grind

/-! ## Openness -/

theorem prodAbs_mono (vs : List (Fin 3)) (y : V3) {g h : Rat} (hg : 0 ≤ g) (hgh : g ≤ h) :
    prodAbs vs y g ≤ prodAbs vs y h := by
  induction vs with
  | nil => simp [prodAbs]
  | cons v vs ih =>
    simp only [prodAbs, List.foldr_cons] at ih ⊢
    have a := prodAbs_nonneg vs y hg
    simp only [prodAbs] at a
    have u := qabs_nonneg (coord y v)
    have k1 := Rat.mul_le_mul_of_nonneg_left ih (show 0 ≤ qabs (coord y v) + h by grind)
    have k2 := Rat.mul_le_mul_of_nonneg_right (show qabs (coord y v) + g ≤ qabs (coord y v) + h by grind) a
    grind

theorem prodAbs_lip (vs : List (Fin 3)) (y : V3) {h : Rat} (h0 : 0 ≤ h) (h1 : h ≤ 1) :
    prodAbs vs y h - prodAbs vs y 0 ≤ h * (vs.length * prodAbs vs y 1) := by
  induction vs with
  | nil => simp [prodAbs]; grind
  | cons v vs ih =>
    simp only [prodAbs, List.foldr_cons, List.length_cons] at ih ⊢
    have m := prodAbs_mono vs y h0 h1
    have n0 := prodAbs_nonneg vs y ((by decide : (0 : Rat) ≤ 0))
    have nh := prodAbs_nonneg vs y h0
    have n1 := prodAbs_nonneg vs y (show (0 : Rat) ≤ 1 by decide)
    simp only [prodAbs] at m n0 nh n1
    have hu0 := qabs_nonneg (coord y v)
    generalize List.foldr (fun v acc => (qabs (coord y v) + h) * acc) 1 vs = Ah at *
    generalize List.foldr (fun v acc => (qabs (coord y v) + 0) * acc) 1 vs = A0 at *
    generalize List.foldr (fun v acc => (qabs (coord y v) + 1) * acc) 1 vs = A1 at *
    generalize qabs (coord y v) = u at *
    have hu : 0 ≤ u := by grind
    -- (u + h) Ah − u A0 = u (Ah − A0) + h Ah ≤ (u + 1) h n A1 + h (u + 1) A1
    have k1 := Rat.mul_le_mul_of_nonneg_left ih (show 0 ≤ u by grind)
    have k2 : h * Ah ≤ h * ((u + 1) * A1) := by
      apply Rat.mul_le_mul_of_nonneg_left _ h0
      have := Rat.mul_le_mul_of_nonneg_right (show (1 : Rat) ≤ u + 1 by grind) n1
      grind
    have k3 : u * (h * (vs.length * A1)) ≤ (u + 1) * (h * (vs.length * A1)) := by
      apply Rat.mul_le_mul_of_nonneg_right (by grind)
      exact Rat.mul_nonneg h0 (Rat.mul_nonneg (Rat.natCast_nonneg) n1)
    have e : ((vs.length + 1 : Nat) : Rat) = (vs.length : Rat) + 1 := by push_cast; rfl
    rw [e]
    grind

/-- The bound is at most `h K` for `h ≤ 1`. -/
def lipK (p : P3) (y : V3) : Rat := p.foldr (fun m acc => qabs m.a * (m.vs.length * prodAbs m.vs y 1) + acc) 0

theorem lipK_nonneg (p : P3) (y : V3) : 0 ≤ lipK p y := by
  induction p with
  | nil => simp [lipK]
  | cons m p ih =>
    simp only [lipK, List.foldr_cons] at ih ⊢
    have := Rat.mul_nonneg (qabs_nonneg m.a)
      (Rat.mul_nonneg (Rat.natCast_nonneg (a := m.vs.length)) (prodAbs_nonneg m.vs y (show (0 : Rat) ≤ 1 by decide)))
    grind

theorem bound_le (p : P3) (y : V3) {h : Rat} (h0 : 0 ≤ h) (h1 : h ≤ 1) :
    bound p y h ≤ h * lipK p y := by
  induction p with
  | nil => simp [bound, lipK]
  | cons m p ih =>
    simp only [bound, lipK, List.foldr_cons] at ih ⊢
    have := Rat.mul_le_mul_of_nonneg_left (prodAbs_lip m.vs y h0 h1) (qabs_nonneg m.a)
    grind

/-- The open box of half-width `h` about `y` is `Near`. -/
theorem near_of_box {y p : V3} {h : Rat} (hp : allPos (Quad.boxPlanes y h) p) : Near y p h := by
  have b := Quad.allPos_boxPlanes hp
  obtain ⟨x1, x2, y1, y2, z1, z2⟩ := b
  intro v
  match v with
  | 0 => show qabs (p.x - y.x) ≤ h; unfold qabs; split <;> grind
  | 1 => show qabs (p.y - y.y) ≤ h; unfold qabs; split <;> grind
  | 2 => show qabs (p.z - y.z) ≤ h; unfold qabs; split <;> grind

/-- A positive polynomial stays positive on an open box about the point. -/
theorem pos_nbhd (p : P3) {y : V3} (hy : 0 < p.eval y) :
    ∃ hs, allPos hs y ∧ ∀ x, allPos hs x → 0 < p.eval x := by
  let K := lipK p y
  have hK := lipK_nonneg p y
  let h := qmin 1 (p.eval y / (2 * (K + 1)))
  have hd : 0 < 2 * (K + 1) := by grind
  have h0 : 0 < h := by
    have := div_pos' hy hd
    simp only [h]; unfold qmin; split <;> grind
  have h1 : h ≤ 1 := qmin_le_left _ _
  have h2 : h * (2 * (K + 1)) ≤ p.eval y := by
    have := Rat.mul_le_mul_of_nonneg_right (qmin_le_right 1 (p.eval y / (2 * (K + 1)))) (Rat.le_of_lt hd)
    rw [Rat.div_mul_cancel (by grind)] at this; exact this
  refine ⟨Quad.boxPlanes y h, Quad.center_boxPlanes y h0, fun x hx => ?_⟩
  have b := box_bound p y x (Rat.le_of_lt h0) (near_of_box hx)
  have bl := bound_le p y (Rat.le_of_lt h0) h1
  have hk : h * K ≤ h * (K + 1) := Rat.mul_le_mul_of_nonneg_left (by grind) (Rat.le_of_lt h0)
  have := neg_le_qabs (p.eval x - p.eval y)
  grind

def P3.neg (p : P3) : P3 := p.map fun m => ⟨-m.a, m.vs⟩

theorem eval_neg (p : P3) (x : V3) : p.neg.eval x = -p.eval x := by
  induction p with
  | nil => simp [P3.neg, P3.eval]
  | cons m p ih =>
    simp only [P3.neg, P3.eval, List.map_cons, List.foldr_cons, Mono.eval] at ih ⊢
    rw [ih]; grind

/-- Signs are open. -/
theorem sign_nbhd (p : P3) {y : V3} (hy : p.eval y ≠ 0) :
    ∃ hs, allPos hs y ∧ ∀ x, allPos hs x → (0 < p.eval y ↔ 0 < p.eval x) ∧
      (p.eval y < 0 ↔ p.eval x < 0) := by
  by_cases h : 0 < p.eval y
  · obtain ⟨hs, h1, h2⟩ := pos_nbhd p h
    exact ⟨hs, h1, fun x hx => by have := h2 x hx; constructor <;> constructor <;> intro <;> grind⟩
  · obtain ⟨hs, h1, h2⟩ := pos_nbhd p.neg (y := y) (by rw [eval_neg]; grind)
    exact ⟨hs, h1, fun x hx => by
      have := h2 x hx; rw [eval_neg] at this
      constructor <;> constructor <;> intro <;> grind⟩

end ForgeExact.Poly3
