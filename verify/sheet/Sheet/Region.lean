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
   XOR, for each arc, its lens `{|p − c|² < r²} ∩ {p strictly on the arc's side of its chord}`
   (a counter-clockwise arc lies right of its chord, a clockwise one left, for every sweep).
5. Polygon membership: the horizontal ray to `+x`, half-open: edge `a → b` counts iff
   `(a.y > p.y) ≠ (b.y > p.y)` and `(b.y > a.y) = (orient(a, b, p) > 0)`. No turned ray: the
   half-open rule decides a vertex level with `p` exactly. Theorems: invariant under rotating the
   loop's start (`inPoly_rotate`), under reversing it for points off its edges
   (`crosses_reverse`), and under translation (`inPoly_translate`).
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

/-- The half-open crossing of the horizontal ray from `p` with the edge `a → b`. -/
def crosses (p a b : P) : Bool :=
  (decide (a.2 > p.2) != decide (b.2 > p.2)) && (decide (b.2 > a.2) == decide (orient a b p > 0))

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

/-- **Nor its direction**, for points off the edge: an edge counts the same either way it runs. -/
theorem crosses_reverse (p a b : P) (h : orient a b p ≠ 0) : crosses p b a = crosses p a b := by
  unfold crosses; rw [orient_swap]
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

theorem crosses_move (t p a b : P) : crosses (move t p) (move t a) (move t b) = crosses p a b := by
  unfold crosses; rw [orient_move]; simp only [move, add_lt_add_iff_right]

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

/-- An arc's lens: inside its circle and strictly on its side of the chord `a → b`
(a counter-clockwise arc, `ccw = true`, lies right of its chord). -/
def inLens (p : P) (a b c : P) (r : ℚ) (ccw : Bool) : Bool :=
  decide (norm2 (sub p c) < r * r) &&
    (if ccw then decide (orient a b p < 0) else decide (orient a b p > 0))

/-- A loop: its points, and for each edge from point `i` an optional arc `(c, r, ccw)`. -/
structure Loop where
  pts : List P
  arcs : List (Option (P × ℚ × Bool))

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

/-- `O` is the owner of a hole whose representative vertex is `h` among `os`: it holds `h`, and
every outer loop holding `h` holds `O`'s representative (it is innermost). -/
def isOwner (os : List Outer) (h : P) (O : Outer) : Bool :=
  inLoop h O.loop && os.all fun O' => !inLoop h O'.loop || inLoop O.rep O'.loop

def owner (os : List Outer) (h : P) : Option Outer := os.find? (isOwner os h)

/-- **The owner is unique** when containment between distinct outer loops is antisymmetric
(disjoint loops: two outer loops cannot each hold the other's vertex). -/
theorem owner_unique (os : List Outer) (h : P)
    (hanti : ∀ O ∈ os, ∀ O' ∈ os, inLoop O.rep O'.loop = true → inLoop O'.rep O.loop = true →
      O = O') :
    ∀ O ∈ os, ∀ O' ∈ os, isOwner os h O = true → isOwner os h O' = true → O = O' := by
  intro O hO O' hO' h1 h2
  simp only [isOwner, Bool.and_eq_true, List.all_eq_true, Bool.or_eq_true, Bool.not_eq_true'] at h1 h2
  have a := (h1.2 O' hO').resolve_left (by simp [h2.1])
  have b := (h2.2 O hO).resolve_left (by simp [h1.1])
  exact hanti O hO O' hO' a b

theorem owner_spec (os : List Outer) (h : P) (O : Outer) (hw : owner os h = some O) :
    O ∈ os ∧ isOwner os h O = true :=
  ⟨List.mem_of_find?_eq_some hw, List.find?_some hw⟩

end Sheet.Region
