import ForgeExact.Ref.Geom
import ForgeCertify.Sign

/-!
# Mod-2 1-chains that vanish

A list of segments is *zero* (`zero1`) when, keying every non-degenerate segment by its line
(`key`: the direction scaled to lead with 1, the base point where that leading coordinate is 0),
every (line, endpoint) pair occurs an even number of times. Subdividing a segment keeps a chain's
zero-ness, so an edge split where another solid crosses it needs no bookkeeping.

The executable test sorts the pairs and checks they come in equal twos (`pairsUp`); only that the
sort is a permutation is used, so the test is sound whatever order it sorts by.

`chainZ` : over a zero chain, any crossing predicate of the form "the point `S` picks on the
segment's line lies inside the segment" is true an even number of times. Every closedness or
tiling argument of the certificate is this.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

abbrev Seg := V3 × V3

theorem v3_ext {a b : V3} (hx : a.x = b.x) (hy : a.y = b.y) (hz : a.z = b.z) : a = b := by
  cases a; cases b; simp_all

@[simp] theorem v3_add_x (a b : V3) : (a + b).x = a.x + b.x := rfl
@[simp] theorem v3_add_y (a b : V3) : (a + b).y = a.y + b.y := rfl
@[simp] theorem v3_add_z (a b : V3) : (a + b).z = a.z + b.z := rfl
@[simp] theorem v3_sub_x (a b : V3) : (a - b).x = a.x - b.x := rfl
@[simp] theorem v3_sub_y (a b : V3) : (a - b).y = a.y - b.y := rfl
@[simp] theorem v3_sub_z (a b : V3) : (a - b).z = a.z - b.z := rfl
@[simp] theorem v3_smul_x (k : Rat) (a : V3) : (k * a).x = k * a.x := rfl
@[simp] theorem v3_smul_y (k : Rat) (a : V3) : (k * a).y = k * a.y := rfl
@[simp] theorem v3_smul_z (k : Rat) (a : V3) : (k * a).z = k * a.z := rfl

/-- Coordinate `k` (0 x, 1 y, else z). -/
def coord (k : Nat) (p : V3) : Rat := if k = 0 then p.x else if k = 1 then p.y else p.z

/-- The first coordinate of `d` that is not zero. -/
def lead (d : V3) : Nat := if d.x ≠ 0 then 0 else if d.y ≠ 0 then 1 else 2

/-- A line: its direction (leading coordinate 1), its point with leading coordinate 0. -/
structure Key where
  d : V3
  o : V3
  k : Nat
  deriving DecidableEq, Inhabited

def key (a b : V3) : Key :=
  let k := lead (b - a)
  let d := (1 / coord k (b - a)) * (b - a)
  ⟨d, a - coord k a * d, k⟩

/-- `q` is on the line `K`. -/
def onKey (K : Key) (q : V3) : Prop := q = K.o + coord K.k q * K.d

/-- `q` lies strictly inside the segment `a b`. -/
def inOpen (a b q : V3) : Prop := ∃ μ : Rat, 0 < μ ∧ μ < 1 ∧ q = a + μ * (b - a)

theorem coord_add (k : Nat) (a b : V3) : coord k (a + b) = coord k a + coord k b := by
  unfold coord; split
  · simp
  · split <;> simp
theorem coord_sub (k : Nat) (a b : V3) : coord k (a - b) = coord k a - coord k b := by
  unfold coord; split
  · simp
  · split <;> simp
theorem coord_smul (k : Nat) (c : Rat) (a : V3) : coord k (c * a) = c * coord k a := by
  unfold coord; split
  · simp
  · split <;> simp

theorem lead_ne (d : V3) (h : d ≠ ⟨0, 0, 0⟩) : coord (lead d) d ≠ 0 := by
  unfold lead coord
  by_cases hx : d.x = 0 <;> by_cases hy : d.y = 0 <;> simp [hx, hy]
  intro hz; apply h; exact v3_ext hx hy hz

theorem lead_lt (d : V3) : lead d = 0 ∨ lead d = 1 ∨ lead d = 2 := by
  unfold lead; split
  · simp
  · split <;> simp

/-- Equal in all three coordinates as `coord`. -/
theorem v3_ext_coord {a b : V3} (h : ∀ k, coord k a = coord k b) : a = b := by
  have h0 := h 0; have h1 := h 1; have h2 := h 2
  simp [coord] at h0 h1 h2; exact v3_ext h0 h1 h2

theorem sub_ne_zero_of_ne {a b : V3} (h : a ≠ b) : b - a ≠ ⟨0, 0, 0⟩ := by
  intro e; apply h
  have ex := congrArg V3.x e; have ey := congrArg V3.y e; have ez := congrArg V3.z e
  simp at ex ey ez; exact v3_ext (by grind) (by grind) (by grind)

theorem coord_cases (k : Nat) (p : V3) : coord k p = p.x ∨ coord k p = p.y ∨ coord k p = p.z := by
  unfold coord; split
  · simp
  · split <;> simp

/-- The two ends lie on their segment's line. -/
theorem onKey_ends {a b : V3} (h : a ≠ b) : onKey (key a b) a ∧ onKey (key a b) b := by
  have hne := lead_ne (b - a) (sub_ne_zero_of_ne h)
  obtain ⟨k, hk⟩ : ∃ k, k = lead (b - a) := ⟨_, rfl⟩
  rw [← hk] at hne
  rw [coord_sub] at hne
  have hinv : (coord k b - coord k a) * (1 / (coord k b - coord k a)) = 1 := by
    have := Rat.div_mul_cancel (a := 1) hne; grind
  unfold onKey key
  simp only [← hk, coord_sub]
  constructor
  · apply v3_ext <;> simp <;> grind
  · have e : ∀ t : Rat, (coord k b - coord k a) * (1 / (coord k b - coord k a) * t) = t := fun t => by
      rw [← Rat.mul_assoc, hinv, Rat.one_mul]
    apply v3_ext <;> simp
    · have := e (b.x - a.x); grind
    · have := e (b.y - a.y); grind
    · have := e (b.z - a.z); grind

/-- On one line, a point is the one with its leading coordinate. -/
theorem onKey_eq {K : Key} {p q : V3} (hp : onKey K p) (hq : onKey K q)
    (h : coord K.k p = coord K.k q) : p = q := by
  rw [hp, hq, h]

/-- On the segment's line, inside the segment is strictly between the ends' leading
coordinates. -/
theorem inOpen_iff {a b q : V3} (h : a ≠ b) (hq : onKey (key a b) q) :
    inOpen a b q ↔ (coord (key a b).k a < coord (key a b).k q ∧ coord (key a b).k q < coord (key a b).k b) ∨
      (coord (key a b).k b < coord (key a b).k q ∧ coord (key a b).k q < coord (key a b).k a) := by
  obtain ⟨ha, hb⟩ := onKey_ends h
  have hne : coord (key a b).k b - coord (key a b).k a ≠ 0 := by
    have := lead_ne (b - a) (sub_ne_zero_of_ne h); rw [coord_sub] at this; exact this
  constructor
  · rintro ⟨μ, h0, h1, rfl⟩
    rw [coord_add, coord_smul, coord_sub]
    rcases lt_trichotomy' (coord (key a b).k a) (coord (key a b).k b) with l | l | l
    · left
      have := Rat.mul_lt_mul_of_pos_left (show 0 < coord (key a b).k b - coord (key a b).k a by grind) h0
      have := Rat.mul_lt_mul_of_pos_left (show 0 < coord (key a b).k b - coord (key a b).k a by grind)
        (show 0 < 1 - μ by grind)
      grind
    · exact absurd (by grind) hne
    · right
      have := Rat.mul_lt_mul_of_pos_left (show 0 < coord (key a b).k a - coord (key a b).k b by grind) h0
      have := Rat.mul_lt_mul_of_pos_left (show 0 < coord (key a b).k a - coord (key a b).k b by grind)
        (show 0 < 1 - μ by grind)
      grind
  · intro hbtw
    obtain ⟨μ, hμ⟩ : ∃ μ, μ = (coord (key a b).k q - coord (key a b).k a) /
        (coord (key a b).k b - coord (key a b).k a) := ⟨_, rfl⟩
    have hm : μ * (coord (key a b).k b - coord (key a b).k a) = coord (key a b).k q - coord (key a b).k a := by
      rw [hμ]; exact Rat.div_mul_cancel hne
    have h01 : 0 < μ ∧ μ < 1 := by
      rcases hbtw with ⟨l1, l2⟩ | ⟨l1, l2⟩
      · have p1 := (mul_pos_iff' μ _).mp (show 0 < μ * (coord (key a b).k b - coord (key a b).k a) by rw [hm]; grind)
        have p2 := (mul_pos_iff' (1 - μ) _).mp
          (show 0 < (1 - μ) * (coord (key a b).k b - coord (key a b).k a) by grind)
        grind
      · have p1 := (mul_neg_iff' μ _).mp (show μ * (coord (key a b).k b - coord (key a b).k a) < 0 by rw [hm]; grind)
        have p2 := (mul_neg_iff' (1 - μ) _).mp
          (show (1 - μ) * (coord (key a b).k b - coord (key a b).k a) < 0 by grind)
        grind
    refine ⟨μ, h01.1, h01.2, ?_⟩
    have hon : onKey (key a b) (a + μ * (b - a)) := by
      unfold onKey
      have ea : a = (key a b).o + coord (key a b).k a * (key a b).d := ha
      have eb : b = (key a b).o + coord (key a b).k b * (key a b).d := hb
      apply v3_ext_coord; intro k
      have e1 := congrArg (coord k) ea; have e2 := congrArg (coord k) eb
      simp only [coord_add, coord_smul, coord_sub] at e1 e2 ⊢
      have e3 : coord (key a b).k a + μ * (coord (key a b).k b - coord (key a b).k a) =
        coord (key a b).k a + μ * (coord (key a b).k b - coord (key a b).k a) := rfl
      grind
    apply onKey_eq hq hon
    rw [coord_add, coord_smul, coord_sub]; grind

/-! ## The test -/

/-- Every non-degenerate segment's two (line, end) pairs. -/
def ends (C : List Seg) : List (Key × V3) :=
  C.flatMap fun s => if s.1 = s.2 then [] else [(key s.1 s.2, s.1), (key s.1 s.2, s.2)]

def v3le (a b : V3) : Bool :=
  a.x < b.x || (a.x == b.x && (a.y < b.y || (a.y == b.y && a.z ≤ b.z)))

def keLe (p q : Key × V3) : Bool :=
  if p.1.d != q.1.d then v3le p.1.d q.1.d
  else if p.1.o != q.1.o then v3le p.1.o q.1.o
  else v3le p.2 q.2

/-- Equal in consecutive twos. -/
def pairsUp {α : Type} [DecidableEq α] : List α → Bool
  | [] => true
  | [_] => false
  | x :: y :: r => decide (x = y) && pairsUp r

/-- The chain is zero mod 2, up to subdivision. -/
def zero1 (C : List Seg) : Bool := pairsUp ((ends C).mergeSort keLe)

theorem par_pairsUp {α : Type} [DecidableEq α] (g : α → Bool) :
    ∀ l : List α, pairsUp l = true → par (l.map g) = false
  | [], _ => rfl
  | [_], h => by simp [pairsUp] at h
  | x :: y :: r, h => by
    simp only [pairsUp, Bool.and_eq_true, decide_eq_true_eq] at h
    obtain ⟨rfl, h⟩ := h
    simp only [List.map_cons, par_cons, par_pairsUp g r h]
    cases g x <;> rfl

theorem par_perm {α : Type} (g : α → Bool) {l l' : List α} (h : l.Perm l') :
    par (l.map g) = par (l'.map g) := by
  induction h with
  | nil => rfl
  | cons x _ ih => simp [ih]
  | swap x y l => simp only [List.map_cons, par_cons]; cases g x <;> cases g y <;> cases par (l.map g) <;> rfl
  | trans _ _ ih1 ih2 => rw [ih1, ih2]

theorem par_ends_zero (C : List Seg) (h : zero1 C = true) (g : Key × V3 → Bool) :
    par ((ends C).map g) = false := by
  rw [← par_perm g (List.mergeSort_perm _ keLe)]
  exact par_pairsUp g _ h

/-- **Zero chains.** If `S` picks at most one point of each segment's line, on that line, never an
end, then the segments containing the picked point are even in number. -/
theorem chainZ (C : List Seg) (hz : zero1 C = true) (S : Key → V3 → Prop)
    (hsub : ∀ s ∈ C, s.1 ≠ s.2 → ∀ q q', S (key s.1 s.2) q → S (key s.1 s.2) q' → q = q')
    (hon : ∀ s ∈ C, s.1 ≠ s.2 → ∀ q, S (key s.1 s.2) q → onKey (key s.1 s.2) q)
    (hend : ∀ s ∈ C, s.1 ≠ s.2 → ∀ q, S (key s.1 s.2) q → q ≠ s.1 ∧ q ≠ s.2) :
    par (C.map fun s => decide (s.1 ≠ s.2 ∧ ∃ q, S (key s.1 s.2) q ∧ inOpen s.1 s.2 q)) = false := by
  let g : Key × V3 → Bool := fun e => decide (∃ q, S e.1 q ∧ coord e.1.k e.2 < coord e.1.k q)
  have hend' := par_ends_zero C hz g
  unfold ends at hend'
  rw [par_flatMap] at hend'
  rw [← hend']
  apply par_map_congr
  intro s hs
  by_cases hab : s.1 = s.2
  · simp [hab]
  · simp only [hab, ite_false, List.map_cons, List.map_nil, par_cons, par_nil, Bool.xor_false]
    obtain ⟨ha, hb⟩ := onKey_ends hab
    by_cases hS : ∃ q, S (key s.1 s.2) q
    · obtain ⟨q, hq⟩ := hS
      have hq' := hon s hs hab q hq
      obtain ⟨hqa, hqb⟩ := hend s hs hab q hq
      have hka : coord (key s.1 s.2).k q ≠ coord (key s.1 s.2).k s.1 := fun e => hqa (onKey_eq hq' ha e)
      have hkb : coord (key s.1 s.2).k q ≠ coord (key s.1 s.2).k s.2 := fun e => hqb (onKey_eq hq' hb e)
      have l : (∃ q', S (key s.1 s.2) q' ∧ inOpen s.1 s.2 q') ↔ inOpen s.1 s.2 q :=
        ⟨fun ⟨q', h1, h2⟩ => hsub s hs hab q q' hq h1 ▸ h2, fun h => ⟨q, hq, h⟩⟩
      have ga : (∃ q', S (key s.1 s.2) q' ∧ coord (key s.1 s.2).k s.1 < coord (key s.1 s.2).k q') ↔
          coord (key s.1 s.2).k s.1 < coord (key s.1 s.2).k q :=
        ⟨fun ⟨q', h1, h2⟩ => hsub s hs hab q q' hq h1 ▸ h2, fun h => ⟨q, hq, h⟩⟩
      have gb : (∃ q', S (key s.1 s.2) q' ∧ coord (key s.1 s.2).k s.2 < coord (key s.1 s.2).k q') ↔
          coord (key s.1 s.2).k s.2 < coord (key s.1 s.2).k q :=
        ⟨fun ⟨q', h1, h2⟩ => hsub s hs hab q q' hq h1 ▸ h2, fun h => ⟨q, hq, h⟩⟩
      have hio := inOpen_iff hab hq'
      apply Bool.eq_iff_iff.mpr
      simp only [g, decide_eq_true_eq]
      rw [ga, gb]
      constructor
      · rintro ⟨-, h⟩; rw [l, hio] at h; grind
      · intro h; refine ⟨hab, ?_⟩; rw [l, hio]; grind
    · have : ∀ q, ¬ S (key s.1 s.2) q := fun q hq => hS ⟨q, hq⟩
      simp [g, this]

end ForgeCertify
