import Winding2d.Symmetry

/-!
# Theorem 4: degenerate points, and the simulation-of-simplicity `side` rule

The half-open rule of `cross` already decides every degenerate point: a vertex at `q`'s height is
classed as below it, and a side through `q` itself crosses nothing. No general-position
hypothesis appears in any theorem of this project.

Sheet metal's `inLoop` decides the side of a point on a side's line by simulation of simplicity:
`side a b q` is `orient a b q`'s sign where nonzero, else `sign (b.x - a.x)`, else
`-sign (b.y - a.y)`. That is the sign of `orient a b (shift q ε)` for every small `ε > 0`, where
`shift q ε = (q.x + ε², q.y + ε)` (`side_shift`). Hence:

* off the polygon the SoS count is the half-open one (`crossSoS_eq`, `windSoS_eq`);
* everywhere, on the polygon too, the SoS count is the half-open winding number at `shift q ε`,
  a point off the polygon, for all small `ε` (`windSoS_shift`): so every theorem about `wind`
  off the polygon applies to `windSoS` at every point;
* the SoS crossing is nonzero exactly when sheet metal's rule counts the side
  (`crossSoS_ne_zero_iff`: the ends straddle `q`'s height and `(b.y > a.y) = (side a b q > 0)`).

The perturbation bounds follow Jordan.lean's `small_pos`/`bndAll`, here as an "eventually"
predicate (`Ev`) closed under conjunction and finite families.
-/

namespace Winding2d

def sgn (x : Rat) : Int := if 0 < x then 1 else if x < 0 then -1 else 0

/-- **The SoS side rule.** -/
def side (a b q : Pt) : Int :=
  if orient a b q ≠ 0 then sgn (orient a b q)
  else if b.x ≠ a.x then sgn (b.x - a.x) else -sgn (b.y - a.y)

/-- The signed crossing with the SoS side rule in place of the strict signs. -/
def crossSoS (q a b : Pt) : Int :=
  if q.y < b.y ∧ ¬q.y < a.y then (if 0 < side a b q then 1 else 0)
  else if q.y < a.y ∧ ¬q.y < b.y then (if side a b q < 0 then -1 else 0)
  else 0

def windSoS (vs : List Pt) (q : Pt) : Int := ((sides vs).map fun e => crossSoS q e.1 e.2).sum

/-- The perturbed point. -/
def shift (q : Pt) (ε : Rat) : Pt := ⟨q.x + ε * ε, q.y + ε⟩

theorem orient_onSeg {z a b : Pt} (h : OnSeg z a b) : orient a b z = 0 := by
  obtain ⟨t, -, -, rfl⟩ := h
  simp only [orient, lerp]; grind

/-- **Off the polygon, the SoS crossing is the half-open one.** -/
theorem crossSoS_eq {q a b : Pt} (h : ¬OnSeg q a b) : crossSoS q a b = cross q a b := by
  by_cases ho : orient a b q = 0
  · have hn : ¬((q.y < b.y ∧ ¬q.y < a.y) ∨ (q.y < a.y ∧ ¬q.y < b.y)) := by
      intro hs; exact h (onSeg_of_orient ho (by grind) (by unfold Btw; grind))
    unfold crossSoS cross; grind
  · unfold crossSoS cross side sgn; grind

theorem windSoS_eq {vs : List Pt} {q : Pt} (h : OffPoly vs q) : windSoS vs q = wind vs q := by
  unfold windSoS wind
  rw [List.map_congr_left (fun e he => crossSoS_eq (h e he))]

/-- **Sheet metal's counted rule**: the SoS crossing is nonzero iff the ends straddle `q`'s
height and `(b.y > a.y) = (side a b q > 0)`. -/
theorem crossSoS_ne_zero_iff (q a b : Pt) :
    crossSoS q a b ≠ 0 ↔ (above q a ≠ above q b) ∧ (a.y < b.y ↔ 0 < side a b q) := by
  have hs : a.y ≠ b.y → side a b q ≠ 0 := by
    intro hab; unfold side sgn; grind
  unfold crossSoS above
  by_cases h1 : q.y < a.y <;> by_cases h2 : q.y < b.y <;> simp [h1, h2] <;>
    first | grind | (have := hs (by grind); grind)

/-! ## Eventually, for small ε -/

/-- `P ε` holds for every `ε` in some `(0, ε₀]`. -/
def Ev (P : Rat → Prop) : Prop := ∃ ε0, 0 < ε0 ∧ ∀ ε, 0 < ε → ε ≤ ε0 → P ε

theorem Ev.mono {P Q : Rat → Prop} (h : Ev P) (hpq : ∀ ε, 0 < ε → P ε → Q ε) : Ev Q := by
  obtain ⟨e, he, hp⟩ := h; exact ⟨e, he, fun ε h0 h1 => hpq ε h0 (hp ε h0 h1)⟩

theorem Ev.and {P Q : Rat → Prop} (hp : Ev P) (hq : Ev Q) : Ev fun ε => P ε ∧ Q ε := by
  obtain ⟨e1, h1, p1⟩ := hp
  obtain ⟨e2, h2, p2⟩ := hq
  refine ⟨if e1 ≤ e2 then e1 else e2, by split <;> grind, fun ε h0 h => ⟨p1 ε h0 ?_, p2 ε h0 ?_⟩⟩
  · split at h <;> grind
  · split at h <;> grind

theorem Ev.true : Ev fun _ => True := ⟨1, by grind, fun _ _ _ => trivial⟩

theorem Ev.all {α : Type} (P : α → Rat → Prop) : ∀ l : List α, (∀ x ∈ l, Ev (P x)) →
    Ev fun ε => ∀ x ∈ l, P x ε
  | [], _ => Ev.true.mono fun _ _ _ x hx => by simp at hx
  | y :: l, h => ((h y (by simp)).and (Ev.all P l fun x hx => h x (by simp [hx]))).mono
      fun ε _ ⟨hy, hl⟩ x hx => by
        rcases List.mem_cons.mp hx with rfl | hx
        · exact hy
        · exact hl x hx

theorem ev_lt {d : Rat} (h : 0 < d) : Ev fun ε => ε < d :=
  ⟨d / 2, by have := Rat.div_mul_cancel (a := d) (b := 2) (by decide); grind,
    fun ε _ h1 => by have := Rat.div_mul_cancel (a := d) (b := 2) (by decide); grind⟩

theorem ev_le {d : Rat} (h : 0 < d) : Ev fun ε => ε ≤ d := ⟨d, h, fun _ _ h1 => h1⟩

def qabs (x : Rat) : Rat := if x < 0 then -x else x

theorem qabs_ge (x : Rat) : -qabs x ≤ x ∧ x ≤ qabs x ∧ 0 ≤ qabs x := by unfold qabs; split <;> grind

/-- A constant beats a small linear-plus-quadratic term. -/
theorem ev_pos_of_pos {o c1 c2 : Rat} (ho : 0 < o) : Ev fun ε => 0 < o + c1 * ε + c2 * (ε * ε) := by
  have k := qabs_ge c1
  have k2 := qabs_ge c2
  have hK : 0 < 2 * (qabs c1 + qabs c2 + 1) := by grind
  have hd : 0 < o / (2 * (qabs c1 + qabs c2 + 1)) := by
    have := Rat.div_mul_cancel (a := o) (Rat.ne_of_gt hK)
    exact (pos_mul_iff hK).mp (by rw [this]; exact ho)
  refine ((ev_le hd).and (ev_le (d := 1) (by decide))).mono fun ε h0 ⟨h1, h2⟩ => ?_
  have e := Rat.div_mul_cancel (a := o) (Rat.ne_of_gt hK)
  have m1 : ε * (2 * (qabs c1 + qabs c2 + 1)) ≤ o := by
    have := Rat.mul_nonneg (by grind : (0:Rat) ≤ o / (2 * (qabs c1 + qabs c2 + 1)) - ε)
      (Rat.le_of_lt hK)
    grind
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ qabs c1 + c1) (Rat.le_of_lt h0)
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ qabs c2 + c2) (Rat.mul_nonneg (Rat.le_of_lt h0) (Rat.le_of_lt h0))
  have := Rat.mul_nonneg (Rat.mul_nonneg k2.2.2 (Rat.le_of_lt h0)) (by grind : (0:Rat) ≤ 1 - ε)
  grind

theorem ev_pos_lin {c1 c2 : Rat} (h1 : 0 < c1) : Ev fun ε => 0 < c1 * ε + c2 * (ε * ε) := by
  refine (ev_pos_of_pos (o := c1) (c1 := c2) (c2 := 0) h1).mono fun ε h0 h => ?_
  have := Rat.mul_pos (by grind : 0 < c1 + c2 * ε) h0
  grind

/-- The lexicographic sign of `(c₀, c₁, c₂)`. -/
def lexsgn (c0 c1 c2 : Rat) : Int :=
  if c0 ≠ 0 then sgn c0 else if c1 ≠ 0 then sgn c1 else sgn c2

theorem sgn_neg (x : Rat) : sgn (-x) = -sgn x := by unfold sgn; grind

theorem lexsgn_neg (c0 c1 c2 : Rat) : lexsgn (-c0) (-c1) (-c2) = -lexsgn c0 c1 c2 := by
  unfold lexsgn; simp only [sgn_neg]; grind

/-- **The sign of a small perturbation is the lexicographic sign.** -/
theorem ev_sgn (c0 c1 c2 : Rat) :
    Ev fun ε => sgn (c0 + c1 * ε + c2 * (ε * ε)) = lexsgn c0 c1 c2 := by
  have pos : ∀ c0 c1 c2 : Rat, 0 < lexsgn c0 c1 c2 →
      Ev fun ε => 0 < c0 + c1 * ε + c2 * (ε * ε) := by
    intro c0 c1 c2 h
    unfold lexsgn sgn at h
    by_cases h0 : c0 = 0
    · by_cases h1 : c1 = 0
      · refine Ev.true.mono fun ε he _ => ?_
        have := Rat.mul_pos (by grind : 0 < c2) (Rat.mul_pos he he)
        grind
      · exact (ev_pos_lin (c2 := c2) (by grind : 0 < c1)).mono fun ε _ h => by grind
    · exact ev_pos_of_pos (by grind)
  by_cases hz : lexsgn c0 c1 c2 = 0
  · have : c0 = 0 ∧ c1 = 0 ∧ c2 = 0 := by unfold lexsgn sgn at hz; grind
    refine Ev.true.mono fun ε _ _ => ?_
    rw [hz, this.1, this.2.1, this.2.2]; unfold sgn; grind
  · have hv : lexsgn c0 c1 c2 = 1 ∨ lexsgn c0 c1 c2 = -1 := by unfold lexsgn sgn at *; grind
    rcases hv with hv | hv
    · exact (pos c0 c1 c2 (by grind)).mono fun ε _ h => by rw [hv]; unfold sgn; grind
    · have := pos (-c0) (-c1) (-c2) (by rw [lexsgn_neg]; grind)
      exact this.mono fun ε _ h => by rw [hv]; unfold sgn; grind

theorem orient_shift (a b q : Pt) (ε : Rat) :
    orient a b (shift q ε) = orient a b q + (b.x - a.x) * ε + (a.y - b.y) * (ε * ε) := by
  simp only [orient, shift]; grind

theorem side_eq_lexsgn (a b q : Pt) : side a b q = lexsgn (orient a b q) (b.x - a.x) (a.y - b.y) := by
  unfold side lexsgn
  have : a.y - b.y = -(b.y - a.y) := by grind
  rw [this, sgn_neg]; grind

/-- **`side` is the sign of `orient` at the perturbed point**, for every small `ε`. -/
theorem side_shift (a b q : Pt) : Ev fun ε => sgn (orient a b (shift q ε)) = side a b q := by
  rw [side_eq_lexsgn]
  exact (ev_sgn _ _ _).mono fun ε _ h => by rw [orient_shift]; exact h

theorem ev_above (q v : Pt) : Ev fun ε => (q.y < v.y ↔ (shift q ε).y < v.y) := by
  by_cases h : q.y < v.y
  · exact (ev_lt (d := v.y - q.y) (by grind)).mono fun ε _ he => by simp only [shift]; grind
  · exact Ev.true.mono fun ε he _ => by simp only [shift]; grind

theorem pos_iff_sgn (x : Rat) : (0 < x ↔ sgn x = 1) ∧ (x < 0 ↔ sgn x = -1) := by
  unfold sgn; grind

theorem crossSoS_shift (q a b : Pt) : Ev fun ε => crossSoS q a b = cross (shift q ε) a b := by
  refine ((side_shift a b q).and ((ev_above q a).and (ev_above q b))).mono
    fun ε _ ⟨hs, ha, hb⟩ => ?_
  have k := pos_iff_sgn (orient a b (shift q ε))
  rw [hs] at k
  unfold crossSoS cross
  have : side a b q = 1 ∨ side a b q = -1 ∨ side a b q = 0 := by
    unfold side sgn; grind
  grind

theorem off_shift (q a b : Pt) : Ev fun ε => ¬OnSeg (shift q ε) a b := by
  by_cases hab : a = b
  · subst hab
    refine (ev_above q a).mono fun ε he hp h => ?_
    obtain ⟨t, -, -, hz⟩ := h
    have : (shift q ε).y = a.y := by rw [hz]; simp only [lerp]; grind
    simp only [shift] at this hp; grind
  · have hne : side a b q ≠ 0 := by
      intro h; unfold side sgn at h
      apply hab; ext <;> grind
    refine (side_shift a b q).mono fun ε _ hs h => ?_
    rw [orient_onSeg h] at hs; unfold sgn at hs; grind

/-- **The SoS count is a winding number.** For every point `q` (on the polygon or not), and every
small enough `ε > 0`, the point `shift q ε` lies on no side, and the SoS count at `q` is the
half-open winding number there. -/
theorem windSoS_shift (vs : List Pt) (q : Pt) :
    ∃ ε0, 0 < ε0 ∧ ∀ ε, 0 < ε → ε ≤ ε0 →
      OffPoly vs (shift q ε) ∧ windSoS vs q = wind vs (shift q ε) := by
  have h := Ev.all (fun (e : Pt × Pt) ε => ¬OnSeg (shift q ε) e.1 e.2 ∧
      crossSoS q e.1 e.2 = cross (shift q ε) e.1 e.2) (sides vs)
    (fun e _ => (off_shift q e.1 e.2).and (crossSoS_shift q e.1 e.2))
  obtain ⟨e0, he0, hall⟩ := h
  refine ⟨e0, he0, fun ε h0 h1 => ⟨fun e he => (hall ε h0 h1 e he).1, ?_⟩⟩
  unfold windSoS wind
  rw [List.map_congr_left (fun e he => (hall ε h0 h1 e he).2)]

end Winding2d
