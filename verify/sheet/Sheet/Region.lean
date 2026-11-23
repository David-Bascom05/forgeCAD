import Sheet.Classes
import Mathlib.Algebra.Order.Field.Basic
import Mathlib.Data.Rat.Defs
import Mathlib.Tactic

/-!
# Flat-pattern regions: the one LINEAR rule in 2D, and exact point-in-region

The statement for kernel/forge-sheet/src/region.rs. Every input is a double, so a dyadic
rational: everything here is over ℚ and every decision is an exact sign. `L` is `tol::LINEAR`.

**Incidence, decided once per pair** (README §6.1, Snap.lean's rule in 2D):
1. Points: `near p q ↔ |p − q|² ≤ L²`.
2. Lines on one carrier: `within a b c ↔ orient(a, b, c)² ≤ L² |b − a|²` (the distance of `c` from
   the line `ab` is at most `L`, `within_iff_dist`). Two lines: `one` if all four ends are within
   `L` of the other line, `ambiguous` if all four are within `2L` but not `one`, else `two`
   (region.rs `carrier`, which divides `orient2d` by a float length: the same rule, inexact at its
   boundary). A zero-length line is `degenerate`.
3. Vertex classes: the transitive closure of `near` over the split points (`Classes.cls`), each
   class its least-named point (`Classes.rep`, names `(loop, piece, fraction)` lexicographic),
   independent of order (`vrep_order`). Spread cap: every two points of a class within `2L`
   (`spreadOk`); region.rs checks the class's box diagonal, which is stricter (`box_spread`).

**Point in region, exact** (replacing the 0.0137-rad turned ray and the 0.3/0.7 hole vote):
4. A loop is a cyclic list of class points (lines end exactly on them) and arcs `(c, r, sense)`
   between consecutive points. Its region, by the even–odd rule, is the polygon over the points
   XOR, for each arc, its lens: its disc and its side of the chord (a counter-clockwise arc lies
   right of its chord, a clockwise one left, for every sweep).
5. Every test is decided at `p` moved by `(δ³, δ²)`, `δ → 0` (simulation of simplicity), so no
   tie is left: `side a b p` is `orient(a, b, p)` where nonzero, else `b.x − a.x`, else
   `a.y − b.y` (`side_move`); in the disc on its circle means below the centre's height, or
   level and left of it (`inDisc`, `disc_move`); the height test `a.y > p.y` is already the
   move's. Polygon membership: the horizontal ray to `+x`, edge `a → b` counts iff
   `(a.y > p.y) ≠ (b.y > p.y)` and `(b.y > a.y) = (side(a, b, p) > 0)`. Theorems: invariant
   under rotating the loop's start (`inPoly_rotate`), under reversing it, for every point
   (`crosses_reverse`: `side` is antisymmetric and never `0`), and under translation
   (`inPoly_translate`).
6. Holes: after `chain` there are no pinches, so the loops are pairwise disjoint. A hole `H` goes
   to the outer loop `O` that holds `H`'s representative vertex and is innermost among those:
   every other outer loop holding it also holds `O`'s representative (`owner`, unique when the
   containment is antisymmetric, `owner_unique`). One exact query each; no vote, no areas.

Splines (not modelled here): subdivide exactly (rational de Casteljau on the dyadic control
points) until no piece's control hull holds `p`, then use their chords; refuse on fuel.
-/

namespace Sheet.Region

abbrev P := ℚ × ℚ

def orient (a b c : P) : ℚ := (b.1 - a.1) * (c.2 - a.2) - (b.2 - a.2) * (c.1 - a.1)

def norm2 (v : P) : ℚ := v.1 * v.1 + v.2 * v.2

def sub (a b : P) : P := (a.1 - b.1, a.2 - b.2)

/-! ## 1–2. Incidence -/

def near (L : ℚ) (p q : P) : Bool := norm2 (sub p q) ≤ L * L

theorem near_symm (L : ℚ) (p q : P) : near L p q = near L q p := by
  simp only [near, norm2, sub]; congr 1; ring_nf

def within (L : ℚ) (a b c : P) : Bool := orient a b c * orient a b c ≤ L * L * norm2 (sub b a)

/-- `within` is the distance from `c` to the line `ab` being at most `L`. -/
theorem within_iff_dist (L : ℚ) (hL : 0 ≤ L) (a b c : P) (h : 0 < norm2 (sub b a)) :
    within L a b c = true ↔ |orient a b c| / Real.sqrt (norm2 (sub b a)) ≤ L := by
  have hs : 0 < Real.sqrt (norm2 (sub b a) : ℝ) := Real.sqrt_pos.mpr (by exact_mod_cast h)
  rw [div_le_iff₀ hs]
  simp only [within, decide_eq_true_eq]
  have key : ((L : ℝ) * Real.sqrt (norm2 (sub b a))) ^ 2 = L * L * norm2 (sub b a) := by
    rw [mul_pow, Real.sq_sqrt (by exact_mod_cast h.le)]; ring
  constructor
  · intro hq
    have hq' : ((orient a b c : ℚ) : ℝ) ^ 2 ≤ ((L * L * norm2 (sub b a) : ℚ) : ℝ) := by
      rw [sq]; exact_mod_cast hq
    have : |((orient a b c : ℚ) : ℝ)| ≤ |(L : ℝ) * Real.sqrt (norm2 (sub b a))| :=
      sq_le_sq.mp (by rw [key]; push_cast at hq' ⊢; linarith)
    rwa [abs_of_nonneg (mul_nonneg (by exact_mod_cast hL) hs.le)] at this
  · intro hd
    have h0 : 0 ≤ |((orient a b c : ℚ) : ℝ)| := abs_nonneg _
    have := pow_le_pow_left₀ h0 hd 2
    rw [key, sq_abs] at this
    have : ((orient a b c * orient a b c : ℚ) : ℝ) ≤ ((L * L * norm2 (sub b a) : ℚ) : ℝ) := by
      push_cast; nlinarith
    exact_mod_cast this

/-- The verdict on two lines. -/
inductive Carrier
  | one
  | two
  | ambiguous
  | degenerate
  deriving DecidableEq, Repr

def carrier (L : ℚ) (a b c d : P) : Carrier :=
  if norm2 (sub b a) = 0 ∨ norm2 (sub d c) = 0 then .degenerate
  else if within L a b c && within L a b d && within L c d a && within L c d b then .one
  else if within (2 * L) a b c && within (2 * L) a b d && within (2 * L) c d a &&
      within (2 * L) c d b then .ambiguous
  else .two

theorem orient_swap (a b c : P) : orient b a c = -orient a b c := by simp only [orient]; ring

theorem norm2_sub_swap (a b : P) : norm2 (sub a b) = norm2 (sub b a) := by
  simp only [norm2, sub]; ring

theorem within_swap (L : ℚ) (a b c : P) : within L b a c = within L a b c := by
  unfold within; rw [orient_swap, norm2_sub_swap, neg_mul_neg]

/-- **Decided once per pair**: the verdict does not depend on which piece comes first. -/
theorem carrier_symm (L : ℚ) (a b c d : P) : carrier L a b c d = carrier L c d a b := by
  have e : ∀ x y z w : Bool, (x && y && z && w) = (z && w && x && y) := by decide
  unfold carrier
  rw [e (within L c d a), e (within (2 * L) c d a)]
  by_cases h1 : norm2 (sub b a) = 0 <;> by_cases h2 : norm2 (sub d c) = 0 <;> simp [h1, h2]

/-- **Nor on which way either line runs.** -/
theorem carrier_reverse (L : ℚ) (a b c d : P) : carrier L b a c d = carrier L a b c d := by
  unfold carrier
  rw [within_swap L a b c, within_swap L a b d, within_swap (2 * L) a b c,
    within_swap (2 * L) a b d, norm2_sub_swap a b]
  simp only [Bool.and_assoc]
  rw [Bool.and_comm (within L c d b) (within L c d a),
    Bool.and_comm (within (2 * L) c d b) (within (2 * L) c d a)]

/-- The verdicts are exclusive and exhaustive by construction; `one` means every end is within
`L` of the other line (the distance, by `within_iff_dist`), `two` means some end is past `2L`. -/
theorem carrier_spec (L : ℚ) (a b c d : P) :
    (carrier L a b c d = .one →
      within L a b c ∧ within L a b d ∧ within L c d a ∧ within L c d b) ∧
    (carrier L a b c d = .two →
      ¬ (within (2 * L) a b c ∧ within (2 * L) a b d ∧ within (2 * L) c d a ∧
        within (2 * L) c d b)) := by
  unfold carrier
  split_ifs with h1 h2 h3 <;> simp_all

/-! ## 3. Vertex classes -/

/-- A split point: where it is, and its name (loop, piece, fraction along it). -/
structure Vtx where
  pos : P
  name : Lex (ℕ × Lex (ℕ × ℚ))
  deriving DecidableEq

def vnear (L : ℚ) (u v : Vtx) : Bool := near L u.pos v.pos

theorem vnear_symm (L : ℚ) (u v : Vtx) : vnear L u v = vnear L v u := near_symm L _ _

def vrep (L : ℚ) (vs : List Vtx) (v : Vtx) : Vtx := Classes.rep (vnear L) Vtx.name vs v

/-- **The vertex classes do not hang on the order the points come in.** -/
theorem vrep_order (L : ℚ) {vs vs' : List Vtx} (hp : vs.Perm vs') (hn : vs.Nodup)
    (hnames : (vs.map Vtx.name).Nodup) {v : Vtx} (hv : v ∈ vs) : vrep L vs' v = vrep L vs v :=
  Classes.rep_order Vtx.name hp hn hnames hv

/-- One representative per class. -/
theorem vrep_class (L : ℚ) {vs : List Vtx} (hn : vs.Nodup) (hnames : (vs.map Vtx.name).Nodup)
    {u v : Vtx} (hu : u ∈ vs) (h : Classes.Conn (vnear L) vs u v) : vrep L vs v = vrep L vs u :=
  Classes.rep_class Vtx.name (vnear_symm L) hn hnames hu h

/-- Translation `t` keeps every decision (`near` is translation invariant). -/
def Vtx.move (t : P) (v : Vtx) : Vtx := ⟨(v.pos.1 + t.1, v.pos.2 + t.2), v.name⟩

theorem vnear_move (L : ℚ) (t : P) (u v : Vtx) : vnear L (u.move t) (v.move t) = vnear L u v := by
  simp only [vnear, near, Vtx.move, norm2, sub]; congr 2 <;> ring

theorem conn_move (L : ℚ) (t : P) {vs : List Vtx} {u v : Vtx}
    (h : Classes.Conn (vnear L) vs u v) :
    Classes.Conn (vnear L) (vs.map (Vtx.move t)) (u.move t) (v.move t) :=
  Classes.conn_map (Vtx.move t) (vnear_move L t) h

/-- The spread cap: every two points of a class within `2L`. -/
def spreadOk (L : ℚ) (vs : List Vtx) : Bool :=
  vs.all fun u => (Classes.cls (vnear L) vs u).all fun v => near (2 * L) u.pos v.pos

theorem spreadOk_spec (L : ℚ) {vs : List Vtx} (hn : vs.Nodup) (h : spreadOk L vs = true)
    {u v : Vtx} (hu : u ∈ vs) (huv : Classes.Conn (vnear L) vs u v) :
    near (2 * L) u.pos v.pos = true := by
  simp only [spreadOk, List.all_eq_true] at h
  exact h u hu v ((Classes.cls_spec hn hu v).mpr huv)

/-- region.rs's cap, the class's box diagonal within `2L`, implies the pairwise one. -/
theorem box_spread (L : ℚ) (lo hi : P) (p q : P)
    (hp : lo.1 ≤ p.1 ∧ p.1 ≤ hi.1 ∧ lo.2 ≤ p.2 ∧ p.2 ≤ hi.2)
    (hq : lo.1 ≤ q.1 ∧ q.1 ≤ hi.1 ∧ lo.2 ≤ q.2 ∧ q.2 ≤ hi.2)
    (hbox : norm2 (sub hi lo) ≤ 2 * L * (2 * L)) : near (2 * L) p q = true := by
  simp only [near]; apply decide_eq_true
  simp only [norm2, sub] at hbox ⊢
  obtain ⟨a1, a2, a3, a4⟩ := hp; obtain ⟨b1, b2, b3, b4⟩ := hq
  have hx : (p.1 - q.1) * (p.1 - q.1) ≤ (hi.1 - lo.1) * (hi.1 - lo.1) := by nlinarith
  have hy : (p.2 - q.2) * (p.2 - q.2) ≤ (hi.2 - lo.2) * (hi.2 - lo.2) := by nlinarith
  linarith

/-! ## 4–5. Point in region -/

/-- **The side of `p` from `a → b`, simulation of simplicity**: `orient a b p` where it is not
`0`; on the line, the sign at `p` moved by `(δ³, δ²)`, `δ → 0` (`side_move`): `b.x − a.x`, else
`a.y − b.y`. Never `0` for `a ≠ b` (`side_ne`), and antisymmetric (`side_swap`), so an edge
counts alike either way it runs for every `p` (`crosses_reverse`). -/
def side (a b p : P) : ℚ :=
  if orient a b p ≠ 0 then orient a b p else if b.1 ≠ a.1 then b.1 - a.1 else a.2 - b.2

theorem side_swap (a b p : P) : side b a p = -side a b p := by
  unfold side; rw [orient_swap]
  by_cases h : orient a b p = 0
  · simp only [h, neg_zero, ne_eq, not_true_eq_false, if_false]
    by_cases hx : b.1 = a.1
    · simp [hx]
    · have : a.1 ≠ b.1 := fun e => hx e.symm
      simp [hx, this]
  · simp [h]

theorem side_ne {a b : P} (hab : a ≠ b) (p : P) : side a b p ≠ 0 := by
  unfold side
  by_cases h : orient a b p = 0
  · simp only [h, ne_eq, not_true_eq_false, if_false]
    by_cases hx : b.1 = a.1
    · simp only [hx, not_true_eq_false, if_false]
      intro hy; apply hab; exact Prod.ext hx.symm (by linarith)
    · simp only [hx, not_false_eq_true, if_true]; intro e; exact hx (by linarith)
  · simpa [h] using h

theorem side_of_ne {a b p : P} (h : orient a b p ≠ 0) : side a b p = orient a b p := by
  simp [side, h]

/-- A sign survives a perturbation that shrinks with `δ`. -/
theorem sign_ev {s K : ℚ} (hs : s ≠ 0) (hK : 0 ≤ K) :
    ∃ δ0 > 0, ∀ δ, 0 < δ → δ < δ0 → ∀ e, |e| ≤ K * δ →
      (0 < s + e ↔ 0 < s) ∧ (s + e < 0 ↔ s < 0) := by
  have hs' : 0 < |s| := abs_pos.mpr hs
  refine ⟨|s| / (K + 1), by positivity, fun δ hδ hδ0 e he => ?_⟩
  have : K * δ < |s| := by
    rw [lt_div_iff₀ (by linarith)] at hδ0
    nlinarith
  have := abs_lt.mp (lt_of_le_of_lt he this)
  rcases lt_or_gt_of_ne hs with h | h
  · rw [abs_of_neg h] at *
    constructor <;> constructor <;> intro <;> linarith
  · rw [abs_of_pos h] at *
    constructor <;> constructor <;> intro <;> linarith

def mv (p : P) (δ : ℚ) : P := (p.1 + δ ^ 3, p.2 + δ ^ 2)

theorem orient_mv (a b p : P) (δ : ℚ) :
    orient a b (mv p δ) = orient a b p + (b.1 - a.1) * δ ^ 2 + (a.2 - b.2) * δ ^ 3 := by
  simp only [orient, mv]; ring

/-- **`side` is the sign at the moved point**: for `a ≠ b`, for every small enough `δ > 0`,
`orient a b (p + (δ³, δ²))` has `side a b p`'s sign. -/
theorem side_move {a b : P} (hab : a ≠ b) (p : P) : ∃ δ0 > 0, ∀ δ, 0 < δ → δ < δ0 →
    (0 < orient a b (mv p δ) ↔ 0 < side a b p) ∧ (orient a b (mv p δ) < 0 ↔ side a b p < 0) := by
  by_cases ho : orient a b p = 0
  · by_cases hx : b.1 - a.1 = 0
    · have hy : a.2 - b.2 ≠ 0 := by
        intro hy; apply hab; exact Prod.ext (by linarith) (by linarith)
      have hs : side a b p = a.2 - b.2 := by
        simp [side, ho, show b.1 = a.1 by linarith]
      refine ⟨1, one_pos, fun δ hδ _ => ?_⟩
      rw [orient_mv, ho, hx, hs, zero_mul, zero_add, zero_add]
      have h3 : 0 < δ ^ 3 := by positivity
      constructor
      · exact ⟨fun h => pos_of_mul_pos_left h h3.le, fun h => mul_pos h h3⟩
      · exact ⟨fun h => by nlinarith, fun h => mul_neg_of_neg_of_pos h h3⟩
    · have hs : side a b p = b.1 - a.1 := by
        simp [side, ho, show b.1 ≠ a.1 from fun e => hx (by rw [e]; ring)]
      obtain ⟨δ0, h0, H⟩ := sign_ev hx (abs_nonneg (a.2 - b.2))
      refine ⟨min δ0 1, by positivity, fun δ hδ hδ0 => ?_⟩
      have hH := H δ hδ (lt_of_lt_of_le hδ0 (min_le_left _ _)) ((a.2 - b.2) * δ)
        (by rw [abs_mul, abs_of_pos hδ])
      have e : orient a b (mv p δ) = δ ^ 2 * ((b.1 - a.1) + (a.2 - b.2) * δ) := by
        rw [orient_mv, ho]; ring
      rw [e, hs]
      have h2 : 0 < δ ^ 2 := by positivity
      constructor
      · rw [← hH.1]; exact ⟨fun h => pos_of_mul_pos_right h h2.le, fun h => mul_pos h2 h⟩
      · rw [← hH.2]; exact ⟨fun h => by nlinarith, fun h => mul_neg_of_pos_of_neg h2 h⟩
  · rw [side_of_ne ho]
    obtain ⟨δ0, h0, H⟩ := sign_ev ho (by positivity : 0 ≤ |b.1 - a.1| + |a.2 - b.2|)
    refine ⟨min δ0 1, by positivity, fun δ hδ hδ0 => ?_⟩
    have hd1 : δ ≤ 1 := le_trans hδ0.le (min_le_right _ _)
    have hH := H δ hδ (lt_of_lt_of_le hδ0 (min_le_left _ _))
      ((b.1 - a.1) * δ ^ 2 + (a.2 - b.2) * δ ^ 3) (by
        have h2 : δ ^ 2 ≤ δ := by nlinarith
        have h3 : δ ^ 3 ≤ δ := by nlinarith
        calc _ ≤ |(b.1 - a.1) * δ ^ 2| + |(a.2 - b.2) * δ ^ 3| := abs_add_le _ _
          _ = |b.1 - a.1| * δ ^ 2 + |a.2 - b.2| * δ ^ 3 := by
              rw [abs_mul, abs_mul, abs_of_pos (by positivity : (0:ℚ) < δ ^ 2),
                abs_of_pos (by positivity : (0:ℚ) < δ ^ 3)]
          _ ≤ |b.1 - a.1| * δ + |a.2 - b.2| * δ := by
              gcongr
          _ = _ := by ring)
    rw [orient_mv, add_assoc]; exact hH

/-- The half-open crossing of the horizontal ray from `p` with the edge `a → b`. -/
def crosses (p a b : P) : Bool :=
  (decide (a.2 > p.2) != decide (b.2 > p.2)) && (decide (b.2 > a.2) == decide (side a b p > 0))

/-- The cyclic edges of a loop. -/
def edges : List P → List (P × P)
  | [] => []
  | a :: rest => (List.zip (a :: rest) (rest ++ [a]))

def parity (bs : List Bool) : Bool := bs.foldl (fun acc b => acc != b) false

theorem parity_cons (b : Bool) (bs : List Bool) : parity (b :: bs) = (b != parity bs) := by
  simp only [parity]
  suffices ∀ acc, List.foldl (fun acc b => acc != b) acc bs =
      (acc != List.foldl (fun acc b => acc != b) false bs) by
    simp only [List.foldl_cons, Bool.false_bne]; rw [this b]
  induction bs with
  | nil => intro acc; simp
  | cons c cs ih =>
      intro acc; simp only [List.foldl_cons]; rw [ih, ih (false != c)]; cases acc <;> cases c <;> simp

theorem parity_append (as bs : List Bool) : parity (as ++ bs) = (parity as != parity bs) := by
  induction as with
  | nil => simp [parity]
  | cons a as ih => rw [List.cons_append, parity_cons, parity_cons, ih]; cases a <;> simp

theorem parity_perm {as bs : List Bool} (h : as.Perm bs) : parity as = parity bs := by
  induction h with
  | nil => rfl
  | cons x _ ih => rw [parity_cons, parity_cons, ih]
  | swap x y l => rw [parity_cons, parity_cons, parity_cons, parity_cons]; cases x <;> cases y <;> simp
  | trans _ _ ih1 ih2 => rw [ih1, ih2]

/-- Polygon membership by the half-open crossing parity. -/
def inPoly (p : P) (vs : List P) : Bool := parity ((edges vs).map fun e => crosses p e.1 e.2)

theorem edges_rotate (a : P) (rest : List P) :
    (edges (rest ++ [a])).Perm (edges (a :: rest)) := by
  cases rest with
  | nil => simp [edges]
  | cons b rest =>
      simp only [edges, List.cons_append]
      -- (b :: rest ++ [a]) zipped with (rest ++ [a] ++ [b]) is a rotation of (a :: b :: rest)
      -- zipped with (b :: rest ++ [a])
      have h1 : List.zip (a :: b :: rest) (b :: (rest ++ [a])) =
          (a, b) :: List.zip (b :: rest) (rest ++ [a]) := rfl
      rw [h1]
      have h2 : List.zip (b :: (rest ++ [a])) (rest ++ [a] ++ [b]) =
          List.zip (b :: rest) (rest ++ [a]) ++ [(a, b)] := by
        rw [show b :: (rest ++ [a]) = (b :: rest) ++ [a] by rfl]
        rw [List.zip_append (by simp)]
        rfl
      rw [h2]
      exact List.perm_append_singleton _ _

/-- **The loop's starting point does not matter.** -/
theorem inPoly_rotate (p a : P) (rest : List P) : inPoly p (rest ++ [a]) = inPoly p (a :: rest) :=
  parity_perm ((edges_rotate a rest).map _)

/-- **Nor its direction**, for every point: an edge counts the same either way it runs. -/
theorem crosses_reverse (p a b : P) : crosses p b a = crosses p a b := by
  by_cases hab : a = b
  · subst hab; rfl
  have h := side_ne hab p
  unfold crosses; rw [side_swap]
  by_cases ha : a.2 > p.2 <;> by_cases hb : b.2 > p.2 <;> simp [ha, hb]
  · have hab : a.2 > b.2 := by linarith
    have : ¬ b.2 > a.2 := by linarith
    simp only [hab, this]
    rcases lt_or_gt_of_ne h with h' | h'
    · simp [h', not_lt.mpr h'.le]
    · simp [h', not_lt.mpr h'.le]
  · have hab : b.2 > a.2 := by linarith
    have : ¬ a.2 > b.2 := by linarith
    simp only [hab, this]
    rcases lt_or_gt_of_ne h with h' | h'
    · simp [h', not_lt.mpr h'.le]
    · simp [h', not_lt.mpr h'.le]

def move (t : P) (p : P) : P := (p.1 + t.1, p.2 + t.2)

theorem orient_move (t a b c : P) : orient (move t a) (move t b) (move t c) = orient a b c := by
  simp only [orient, move]; ring

theorem side_move' (t a b p : P) : side (move t a) (move t b) (move t p) = side a b p := by
  unfold side; rw [orient_move]; simp only [move, add_sub_add_right_eq_sub, ne_eq,
    add_left_inj]

theorem crosses_move (t p a b : P) : crosses (move t p) (move t a) (move t b) = crosses p a b := by
  unfold crosses; rw [side_move']; simp only [move, add_lt_add_iff_right]

theorem edges_map (f : P → P) (vs : List P) :
    edges (vs.map f) = (edges vs).map fun e => (f e.1, f e.2) := by
  cases vs with
  | nil => rfl
  | cons a rest =>
      have h1 : f a :: rest.map f = (a :: rest).map f := rfl
      have h2 : rest.map f ++ [f a] = (rest ++ [a]).map f := by simp
      simp only [edges, List.map_cons]
      rw [h1, h2, List.zip_map]
      rfl

/-- **Translation invariance.** -/
theorem inPoly_translate (t p : P) (vs : List P) :
    inPoly (move t p) (vs.map (move t)) = inPoly p vs := by
  simp only [inPoly, edges_map, List.map_map]
  congr 1; apply List.map_congr_left; intro e _; exact crosses_move t p e.1 e.2

/-- In the disc `|p − c| < r`, at `p` moved by `(δ³, δ²)` as `side` moves it (`disc_move`): on
the circle, below the centre's height, or level with it and left of it. -/
def inDisc (p c : P) (r : ℚ) : Bool :=
  decide (norm2 (sub p c) < r * r) ||
    (decide (norm2 (sub p c) = r * r) && (decide (p.2 < c.2) || (decide (p.2 = c.2) && decide (p.1 < c.1))))

theorem norm2_mv (p c : P) (δ : ℚ) : norm2 (sub (mv p δ) c) = norm2 (sub p c) +
    2 * (p.2 - c.2) * δ ^ 2 + 2 * (p.1 - c.1) * δ ^ 3 + δ ^ 4 + δ ^ 6 := by
  simp only [norm2, sub, mv]; ring

/-- **`inDisc` is the disc at the moved point**: for every small enough `δ > 0`,
`|p + (δ³, δ²) − c| < r` exactly when `inDisc p c r`. -/
theorem disc_move (p c : P) (r : ℚ) : ∃ δ0 > 0, ∀ δ, 0 < δ → δ < δ0 →
    (norm2 (sub (mv p δ) c) < r * r ↔ inDisc p c r = true) := by
  set A := norm2 (sub p c) - r * r with hA
  set dx := p.1 - c.1
  set dy := p.2 - c.2
  have key : ∀ δ, norm2 (sub (mv p δ) c) < r * r ↔
      A + 2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6 < 0 := fun δ => by
    rw [norm2_mv]; constructor <;> intro h <;> linarith
  simp only [inDisc, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  by_cases h0 : A = 0
  · have heq : norm2 (sub p c) = r * r := by linarith
    have hlt : ¬ norm2 (sub p c) < r * r := by linarith
    rw [or_iff_right hlt, and_iff_right heq]
    by_cases hy : dy = 0
    · have hy' : p.2 = c.2 := by simp only [dy] at hy; linarith
      have hy'' : ¬ p.2 < c.2 := by linarith
      rw [or_iff_right hy'', and_iff_right hy']
      by_cases hx : dx = 0
      · have hx' : ¬ p.1 < c.1 := by simp only [dx] at hx; linarith
        refine ⟨1, one_pos, fun δ hδ _ => ?_⟩
        rw [key, h0, hy, hx]
        constructor
        · intro h; nlinarith [pow_pos hδ 4, pow_pos hδ 6]
        · intro h; exact absurd h hx'
      · have h2 : (2 * dx : ℚ) ≠ 0 := by positivity
        obtain ⟨δ0, hδ0, H⟩ := sign_ev h2 (by norm_num : (0 : ℚ) ≤ 2)
        refine ⟨min δ0 1, by positivity, fun δ hδ hδ1 => ?_⟩
        have hd1 : δ ≤ 1 := le_trans hδ1.le (min_le_right _ _)
        have hH := (H δ hδ (lt_of_lt_of_le hδ1 (min_le_left _ _)) (δ + δ ^ 3) (by
          rw [abs_of_pos (by positivity)]
          linarith [pow_le_of_le_one hδ.le hd1 (by norm_num : (3 : ℕ) ≠ 0)])).2
        have e : A + 2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6 = δ ^ 3 * (2 * dx + (δ + δ ^ 3)) := by
          rw [h0, hy]; ring
        rw [key, e]
        have h3 : 0 < δ ^ 3 := by positivity
        have : p.1 < c.1 ↔ 2 * dx < 0 := by simp only [dx]; constructor <;> intro <;> linarith
        rw [this, ← hH]
        exact ⟨fun h => by nlinarith, fun h => mul_neg_of_pos_of_neg h3 h⟩
    · have h2 : (2 * dy : ℚ) ≠ 0 := by positivity
      obtain ⟨δ0, hδ0, H⟩ := sign_ev h2 (by positivity : (0 : ℚ) ≤ 2 * |dx| + 2)
      refine ⟨min δ0 1, by positivity, fun δ hδ hδ1 => ?_⟩
      have hd1 : δ ≤ 1 := le_trans hδ1.le (min_le_right _ _)
      have hH := (H δ hδ (lt_of_lt_of_le hδ1 (min_le_left _ _)) (2 * dx * δ + δ ^ 2 + δ ^ 4) (by
        have h2 : δ ^ 2 ≤ δ := by nlinarith
        have h4 : δ ^ 4 ≤ δ := by nlinarith
        calc _ ≤ |2 * dx * δ| + |δ ^ 2| + |δ ^ 4| := abs_add_three _ _ _
          _ = 2 * |dx| * δ + δ ^ 2 + δ ^ 4 := by
              rw [abs_mul, abs_mul, abs_of_pos hδ, abs_of_pos (by positivity : (0:ℚ) < δ ^ 2),
                abs_of_pos (by positivity : (0:ℚ) < δ ^ 4)]; norm_num
          _ ≤ _ := by nlinarith)).2
      have e : A + 2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6 =
          δ ^ 2 * (2 * dy + (2 * dx * δ + δ ^ 2 + δ ^ 4)) := by rw [h0]; ring
      rw [key, e]
      have h2' : 0 < δ ^ 2 := by positivity
      have hyne : p.2 ≠ c.2 := fun e => hy (by simp only [dy]; rw [e]; ring)
      have : (p.2 < c.2 ∨ p.2 = c.2 ∧ p.1 < c.1) ↔ 2 * dy < 0 := by
        simp only [hyne, false_and, or_false, dy]; constructor <;> intro <;> linarith
      rw [this, ← hH]
      exact ⟨fun h => by nlinarith, fun h => mul_neg_of_pos_of_neg h2' h⟩
  · obtain ⟨δ0, hδ0, H⟩ := sign_ev h0 (by positivity : (0 : ℚ) ≤ 2 * |dy| + 2 * |dx| + 2)
    refine ⟨min δ0 1, by positivity, fun δ hδ hδ1 => ?_⟩
    have hd1 : δ ≤ 1 := le_trans hδ1.le (min_le_right _ _)
    have hH := (H δ hδ (lt_of_lt_of_le hδ1 (min_le_left _ _))
      (2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6) (by
        have h2 : δ ^ 2 ≤ δ := by nlinarith
        have h3 : δ ^ 3 ≤ δ := by nlinarith
        have h4 : δ ^ 4 ≤ δ := by nlinarith
        have h6 : δ ^ 6 ≤ δ := by nlinarith
        have a1 : |2 * dy * δ ^ 2| ≤ 2 * |dy| * δ := by
          rw [abs_mul, abs_mul, abs_of_pos (by positivity : (0:ℚ) < δ ^ 2)]; norm_num
          exact mul_le_mul_of_nonneg_left h2 (by positivity)
        have a2 : |2 * dx * δ ^ 3| ≤ 2 * |dx| * δ := by
          rw [abs_mul, abs_mul, abs_of_pos (by positivity : (0:ℚ) < δ ^ 3)]; norm_num
          exact mul_le_mul_of_nonneg_left h3 (by positivity)
        have a3 : |(δ ^ 4 : ℚ)| = δ ^ 4 := abs_of_pos (by positivity)
        have a4 : |(δ ^ 6 : ℚ)| = δ ^ 6 := abs_of_pos (by positivity)
        calc _ ≤ |2 * dy * δ ^ 2 + 2 * dx * δ ^ 3| + |δ ^ 4 + δ ^ 6| := by
              rw [show 2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6 =
                (2 * dy * δ ^ 2 + 2 * dx * δ ^ 3) + (δ ^ 4 + δ ^ 6) by ring]; exact abs_add_le _ _
          _ ≤ (|2 * dy * δ ^ 2| + |2 * dx * δ ^ 3|) + (|δ ^ 4| + |δ ^ 6|) :=
              add_le_add (abs_add_le _ _) (abs_add_le _ _)
          _ ≤ _ := by rw [a3, a4]; linarith)).2
    rw [key, show A + 2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6 =
      A + (2 * dy * δ ^ 2 + 2 * dx * δ ^ 3 + δ ^ 4 + δ ^ 6) by ring, hH]
    have hne : norm2 (sub p c) ≠ r * r := fun e => h0 (by rw [hA, e]; ring)
    simp only [hne, false_and, or_false]
    constructor <;> intro <;> linarith

/-- An arc's lens: in its disc and on its side of the chord `a → b` (a counter-clockwise arc,
`ccw = true`, lies right of its chord), both at `p` moved by `(δ³, δ²)`. -/
def inLens (p : P) (a b c : P) (r : ℚ) (ccw : Bool) : Bool :=
  inDisc p c r && (if ccw then decide (side a b p < 0) else decide (side a b p > 0))

/-- A loop: its points, and for each edge from point `i` an optional arc `(c, r, ccw)`. -/
structure Loop where
  pts : List P
  arcs : List (Option (P × ℚ × Bool))
  deriving DecidableEq

/-- An edge's lens term: none for a line. -/
def lensTerm (p : P) (ea : (P × P) × Option (P × ℚ × Bool)) : Bool :=
  match ea.2 with
  | none => false
  | some (c, r, ccw) => inLens p ea.1.1 ea.1.2 c r ccw

/-- **The region of a loop**, even–odd: its polygon XOR its arcs' lenses. -/
def inLoop (p : P) (l : Loop) : Bool :=
  inPoly p l.pts != parity ((List.zip (edges l.pts) l.arcs).map (lensTerm p))

/-- With no arcs the region is the polygon's. -/
theorem inLoop_lines (p : P) (pts : List P) :
    inLoop p ⟨pts, List.replicate pts.length none⟩ = inPoly p pts := by
  simp only [inLoop]
  suffices h : ∀ (es : List (P × P)) (n : ℕ),
      parity ((List.zip es (List.replicate n none)).map (lensTerm p)) = false by
    rw [h (edges pts) pts.length]; simp
  intro es n
  induction es generalizing n with
  | nil => simp [parity]
  | cons e es ih =>
      cases n with
      | zero => simp [parity]
      | succ n =>
          simp only [List.replicate_succ, List.zip_cons_cons, List.map_cons, parity_cons]
          rw [ih]; rfl

/-! ## 6. Holes -/

/-- An outer loop with its representative vertex. -/
structure Outer where
  loop : Loop
  rep : P
  deriving DecidableEq

/-- `O` is the owner of a hole whose representative vertex is `h` among `os`: it holds `h`, and
every other outer loop holding `h` holds `O`'s representative (it is innermost; a loop is not
asked about its own vertex, which is on its boundary). -/
def isOwner (os : List Outer) (h : P) (O : Outer) : Bool :=
  inLoop h O.loop && os.all fun O' => O' == O || !inLoop h O'.loop || inLoop O.rep O'.loop

def owner (os : List Outer) (h : P) : Option Outer := os.find? (isOwner os h)

/-- **The owner is unique** when containment between distinct outer loops is antisymmetric
(disjoint loops: two outer loops cannot each hold the other's vertex). -/
theorem owner_unique (os : List Outer) (h : P)
    (hanti : ∀ O ∈ os, ∀ O' ∈ os, inLoop O.rep O'.loop = true → inLoop O'.rep O.loop = true →
      O = O') :
    ∀ O ∈ os, ∀ O' ∈ os, isOwner os h O = true → isOwner os h O' = true → O = O' := by
  intro O hO O' hO' h1 h2
  by_contra hne
  simp only [isOwner, Bool.and_eq_true, List.all_eq_true, Bool.or_eq_true, Bool.not_eq_true',
    beq_iff_eq] at h1 h2
  have a : inLoop O.rep O'.loop = true := by
    rcases h1.2 O' hO' with (e | e) | e
    · exact absurd e.symm hne
    · rw [h2.1] at e; cases e
    · exact e
  have b : inLoop O'.rep O.loop = true := by
    rcases h2.2 O hO with (e | e) | e
    · exact absurd e hne
    · rw [h1.1] at e; cases e
    · exact e
  exact hne (hanti O hO O' hO' a b)

theorem owner_spec (os : List Outer) (h : P) (O : Outer) (hw : owner os h = some O) :
    O ∈ os ∧ isOwner os h O = true :=
  ⟨List.mem_of_find?_eq_some hw, List.find?_some hw⟩

end Sheet.Region
