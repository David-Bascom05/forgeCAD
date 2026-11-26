/-
G5, the machine (research/cam/design.md §5.2), for 3-axis machines: the joints are the tip's
coordinates less the work offset, so the inverse kinematics is a translation, continuous, with
no singularity. Every joint value along a piece must be within the machine's travel.

A line move is within the travel iff both its ends are (the travel is a box, a convex set). An
arc in the xy plane is within it if the box of its circle is. The executable check over ℚ is
proven equal to the spec over ℝ, for line moves.
-/
import ForgeCam.Spec

open Set

namespace ForgeCam

/-- The machine's travel: joint i in [lo i, hi i]. -/
structure Travel where
  lo : Fin 3 → ℝ
  hi : Fin 3 → ℝ

/-- The joints at a tip position, for the work offset o. -/
def joints (o p : E3) (i : Fin 3) : ℝ := p i - o i

def within (L : Travel) (o p : E3) : Prop := ∀ i, L.lo i ≤ joints o p i ∧ joints o p i ≤ L.hi i

/-- **G5**: every joint value along the piece is within travel, and the joints move
continuously. -/
def G5 (L : Travel) (o : E3) (g : ℝ → E3) : Prop :=
  (∀ s ∈ I01, within L o (g s)) ∧ ∀ i, ContinuousOn (fun s => joints o (g s) i) I01

/-- A line move from a to b. -/
def line (a b : E3) (s : ℝ) : E3 := (1 - s) • a + s • b

theorem line_apply (a b : E3) (s : ℝ) (i : Fin 3) : line a b s i = (1 - s) * a i + s * b i := by
  simp [line]

theorem continuousOn_joints_line (o a b : E3) (i : Fin 3) :
    ContinuousOn (fun s => joints o (line a b s) i) I01 := by
  simp only [joints, line_apply]; fun_prop

/-- **A line move is within travel iff both its ends are.** -/
theorem G5_line_iff (L : Travel) (o a b : E3) :
    G5 L o (line a b) ↔ within L o a ∧ within L o b := by
  constructor
  · rintro ⟨h, _⟩
    have h0 := h 0 ⟨le_rfl, zero_le_one⟩
    have h1 := h 1 ⟨zero_le_one, le_rfl⟩
    simp only [within, joints, line_apply] at h0 h1
    simp only [within, joints]
    exact ⟨fun i => by have := h0 i; simpa using this, fun i => by have := h1 i; simpa using this⟩
  · rintro ⟨ha, hb⟩
    refine ⟨fun s ⟨hs0, hs1⟩ i => ?_, continuousOn_joints_line o a b⟩
    obtain ⟨ha1, ha2⟩ := ha i
    obtain ⟨hb1, hb2⟩ := hb i
    simp only [joints, line_apply] at ha1 ha2 hb1 hb2 ⊢
    have e : (1 - s) * a i + s * b i - o i = (1 - s) * (a i - o i) + s * (b i - o i) := by ring
    rw [e]
    have h1s : 0 ≤ 1 - s := by linarith
    constructor <;> nlinarith [mul_le_mul_of_nonneg_left ha1 h1s, mul_le_mul_of_nonneg_left ha2 h1s,
      mul_le_mul_of_nonneg_left hb1 hs0, mul_le_mul_of_nonneg_left hb2 hs0]

/-- **An arc is within travel if its circle's box is.** The piece stays on the circle of
centre (cx, cy) and radius r in xy, and its height runs linearly from za to zb (a helix when they
differ). It is within travel if the corners (cx ± r, cy ± r) are, and so are both heights. -/
theorem G5_arc {L : Travel} {o : E3} {g : ℝ → E3} {cx cy r za zb : ℝ}
    (hc : ∀ i, ContinuousOn (fun s => joints o (g s) i) I01)
    (hcirc : ∀ s ∈ I01, (g s 0 - cx) ^ 2 + (g s 1 - cy) ^ 2 = r ^ 2) (hr : 0 ≤ r)
    (hz : ∀ s ∈ I01, g s 2 = (1 - s) * za + s * zb)
    (hx : L.lo 0 ≤ cx - r - o 0 ∧ cx + r - o 0 ≤ L.hi 0)
    (hy : L.lo 1 ≤ cy - r - o 1 ∧ cy + r - o 1 ≤ L.hi 1)
    (hza : L.lo 2 ≤ za - o 2 ∧ za - o 2 ≤ L.hi 2) (hzb : L.lo 2 ≤ zb - o 2 ∧ zb - o 2 ≤ L.hi 2) :
    G5 L o g := by
  refine ⟨fun s hs i => ?_, hc⟩
  have hcs := hcirc s hs
  have bx : |g s 0 - cx| ≤ r := abs_le_of_sq_le_sq' (by nlinarith [sq_nonneg (g s 1 - cy)]) hr
    |> fun h => abs_le.mpr h
  have by' : |g s 1 - cy| ≤ r := abs_le_of_sq_le_sq' (by nlinarith [sq_nonneg (g s 0 - cx)]) hr
    |> fun h => abs_le.mpr h
  obtain ⟨hs0, hs1⟩ := hs
  simp only [joints]
  fin_cases i
  · obtain ⟨l, u⟩ := abs_le.mp bx; simp; constructor <;> linarith
  · obtain ⟨l, u⟩ := abs_le.mp by'; simp; constructor <;> linarith
  · simp only [Fin.reduceFinMk, Fin.isValue]
    rw [hz s ⟨hs0, hs1⟩]
    have e : (1 - s) * za + s * zb - o 2 = (1 - s) * (za - o 2) + s * (zb - o 2) := by ring
    rw [e]
    have h1s : 0 ≤ 1 - s := by linarith
    constructor <;> nlinarith [mul_le_mul_of_nonneg_left hza.1 h1s,
      mul_le_mul_of_nonneg_left hza.2 h1s, mul_le_mul_of_nonneg_left hzb.1 hs0,
      mul_le_mul_of_nonneg_left hzb.2 hs0]

/-! ### The executable check, over ℚ -/

/-- The travel and a point in exact rationals. -/
structure QTravel where
  lo : Fin 3 → ℚ
  hi : Fin 3 → ℚ

def QTravel.toReal (L : QTravel) : Travel := ⟨fun i => L.lo i, fun i => L.hi i⟩

/-- A point of ℚ³ in space. -/
noncomputable def ofQ (p : Fin 3 → ℚ) : E3 := WithLp.toLp 2 fun i => (p i : ℝ)

@[simp] theorem ofQ_apply (p : Fin 3 → ℚ) (i : Fin 3) : ofQ p i = p i := rfl

def withinQ (L : QTravel) (o p : Fin 3 → ℚ) : Bool :=
  decide (∀ i, L.lo i ≤ p i - o i ∧ p i - o i ≤ L.hi i)

/-- The check a 3-axis line move gets: both ends. -/
def lineOkQ (L : QTravel) (o a b : Fin 3 → ℚ) : Bool := withinQ L o a && withinQ L o b

theorem withinQ_iff (L : QTravel) (o p : Fin 3 → ℚ) :
    withinQ L o p = true ↔ within L.toReal (ofQ o) (ofQ p) := by
  simp only [withinQ, decide_eq_true_eq, within, joints, QTravel.toReal, ofQ_apply]
  refine forall_congr' fun i => ?_
  constructor
  · rintro ⟨h1, h2⟩
    exact ⟨by exact_mod_cast h1, by exact_mod_cast h2⟩
  · rintro ⟨h1, h2⟩
    exact ⟨by exact_mod_cast h1, by exact_mod_cast h2⟩

/-- **The executable check is exactly G5 for a line move** (L2 for G5). -/
theorem lineOkQ_iff (L : QTravel) (o a b : Fin 3 → ℚ) :
    lineOkQ L o a b = true ↔ G5 L.toReal (ofQ o) (line (ofQ a) (ofQ b)) := by
  rw [G5_line_iff, lineOkQ, Bool.and_eq_true, withinQ_iff, withinQ_iff]

-- The check runs (compiled, not a proof): a move inside the travel, and one past it.
#guard lineOkQ ⟨fun _ => -100, fun _ => 100⟩ (fun _ => 0) ![0, 0, 5] ![50, -20, 5]
#guard !lineOkQ ⟨fun _ => -100, fun _ => 100⟩ (fun _ => 0) ![0, 0, 5] ![50, -20, 101/1]


/-! ### G5, runtime: the posted feeds and spindle speeds

The control reads a feed word as an integer q at k decimals, the feed q / 10^k mm/min, and a
spindle word as a whole number of rpm. forge-cam-lang's `runtime_limits` passes a feed when
q ≤ lo, lo a lower end of an enclosure of fmax · 10^k (outward-rounded), and a speed n when
rmin ≤ n ≤ rmax. -/

/-- **G5, runtime** (the spec): every feed at most the machine's limit, every speed in range. -/
def G5rt (fmax rmin rmax : ℝ) (k : ℕ) (feeds speeds : List ℤ) : Prop :=
  (∀ q ∈ feeds, (q : ℝ) / 10 ^ k ≤ fmax) ∧ ∀ n ∈ speeds, rmin ≤ (n : ℝ) ∧ (n : ℝ) ≤ rmax

/-- **The runtime check is sound**: a lower end of fmax · 10^k bounding every posted feed word,
and every speed in range, give G5rt. -/
theorem G5rt_of_check {fmax rmin rmax lo : ℝ} {k : ℕ} {feeds speeds : List ℤ}
    (hlo : lo ≤ fmax * 10 ^ k) (hf : ∀ q ∈ feeds, (q : ℝ) ≤ lo)
    (hs : ∀ n ∈ speeds, rmin ≤ (n : ℝ) ∧ (n : ℝ) ≤ rmax) : G5rt fmax rmin rmax k feeds speeds := by
  refine ⟨fun q hq => ?_, hs⟩
  rw [div_le_iff₀ (by positivity)]
  exact (hf q hq).trans hlo

end ForgeCam
