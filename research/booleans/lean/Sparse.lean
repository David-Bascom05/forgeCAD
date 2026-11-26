/-!
# Two small facts behind the ranked speed levers (research/booleans/prior-art.md)

Core Lean only, like verify/forge-exact. Points are homogeneous integer 4-vectors `(X, Y, Z, W)`,
the form `meet3` produces after the dyadic exponents are cleared; a plane is `n · X − c · W`.

* `skip_pos` (lever A2, sparse arrangement): a plane positive at every vertex of a convex cell is
  positive at every point of it (every nonnegative, nonzero combination of the vertices). So a
  class plane that has one strict sign on all corners of the working polygon cuts no cell there,
  and every cell inherits that sign in its covector without evaluating it.
* `mul_fits` (lever A3, fixed-width numbers): the bit budget composes, so a static width per
  expression (Nehring-Wirxel et al. 2021, §3.2) bounds every intermediate of `meet3` and `side`.
-/

namespace BoolResearch

/-- A homogeneous point `(X, Y, Z) / W`. -/
structure HP where
  x : Int
  y : Int
  z : Int
  w : Int

/-- A plane `a x + b y + c z = d`, evaluated homogeneously. -/
structure Pl where
  a : Int
  b : Int
  c : Int
  d : Int

def Pl.ev (p : Pl) (q : HP) : Int := p.a * q.x + p.b * q.y + p.c * q.z - p.d * q.w

def HP.add (p q : HP) : HP := ⟨p.x + q.x, p.y + q.y, p.z + q.z, p.w + q.w⟩
def HP.smul (k : Int) (p : HP) : HP := ⟨k * p.x, k * p.y, k * p.z, k * p.w⟩
def HP.zero : HP := ⟨0, 0, 0, 0⟩

/-- The combination `Σ kᵢ · vᵢ`. -/
def comb : List (Int × HP) → HP
  | [] => HP.zero
  | (k, v) :: r => (HP.smul k v).add (comb r)

theorem ev_add (p : Pl) (u v : HP) : p.ev (u.add v) = p.ev u + p.ev v := by
  simp only [Pl.ev, HP.add, Int.mul_add]; omega

theorem ev_smul (p : Pl) (k : Int) (v : HP) : p.ev (HP.smul k v) = k * p.ev v := by
  simp only [Pl.ev, HP.smul, Int.mul_sub, Int.mul_add, Int.mul_left_comm k]

theorem ev_zero (p : Pl) : p.ev HP.zero = 0 := by simp [Pl.ev, HP.zero]

/-- The plane at a combination is the combination of its values. -/
theorem ev_comb (p : Pl) : ∀ ws : List (Int × HP),
    p.ev (comb ws) = (ws.map fun kv => kv.1 * p.ev kv.2).foldr (· + ·) 0
  | [] => by simp [comb, ev_zero]
  | (k, v) :: r => by
    simp only [comb, ev_add, ev_smul, ev_comb p r, List.map_cons, List.foldr_cons]

/-- Nonnegative weights times positive values: the sum is nonnegative, and positive as soon as
one weight is. -/
theorem foldr_pos (p : Pl) : ∀ ws : List (Int × HP),
    (∀ kv ∈ ws, 0 ≤ kv.1 ∧ 0 < p.ev kv.2) →
    0 ≤ (ws.map fun kv => kv.1 * p.ev kv.2).foldr (· + ·) 0 ∧
    ((∃ kv ∈ ws, 0 < kv.1) → 0 < (ws.map fun kv => kv.1 * p.ev kv.2).foldr (· + ·) 0)
  | [], _ => by simp
  | (k, v) :: r, h => by
    have hk := h (k, v) (List.mem_cons_self ..)
    have ih := foldr_pos p r fun kv m => h kv (List.mem_cons_of_mem _ m)
    have h0 : 0 ≤ k * p.ev v := Int.mul_nonneg hk.1 (Int.le_of_lt hk.2)
    simp only [List.map_cons, List.foldr_cons]
    refine ⟨Int.add_nonneg h0 ih.1, fun ⟨kv, m, hp⟩ => ?_⟩
    rcases List.mem_cons.mp m with e | m'
    · subst e
      have : 0 < k * p.ev v := Int.mul_pos hp hk.2
      omega
    · have := ih.2 ⟨kv, m', hp⟩
      omega

/-- **Skip a plane that misses the cell.** If plane `p` is positive at every vertex, it is
positive at every point of the cell: every combination with nonnegative weights, one positive. -/
theorem skip_pos (p : Pl) (ws : List (Int × HP))
    (hv : ∀ kv ∈ ws, 0 ≤ kv.1 ∧ 0 < p.ev kv.2) (hne : ∃ kv ∈ ws, 0 < kv.1) :
    0 < p.ev (comb ws) := by
  rw [ev_comb]; exact (foldr_pos p ws hv).2 hne

/-- The same for negative: apply `skip_pos` to the flipped plane. -/
def Pl.flip (p : Pl) : Pl := ⟨-p.a, -p.b, -p.c, -p.d⟩

theorem ev_flip (p : Pl) (q : HP) : p.flip.ev q = - p.ev q := by
  simp only [Pl.ev, Pl.flip, Int.neg_mul]; omega

theorem skip_neg (p : Pl) (ws : List (Int × HP))
    (hv : ∀ kv ∈ ws, 0 ≤ kv.1 ∧ p.ev kv.2 < 0) (hne : ∃ kv ∈ ws, 0 < kv.1) :
    p.ev (comb ws) < 0 := by
  have := skip_pos p.flip ws (fun kv m => ⟨(hv kv m).1, by rw [ev_flip]; have := (hv kv m).2; omega⟩) hne
  rw [ev_flip] at this; omega

/-! ## Bit budgets -/

/-- **Widths add under products.** -/
theorem mul_fits {a b : Nat} {k l : Nat} (ha : a < 2 ^ k) (hb : b < 2 ^ l) :
    a * b < 2 ^ (k + l) := by
  rw [Nat.pow_add]
  exact Nat.mul_lt_mul'' ha hb

/-- **A sum costs one bit.** -/
theorem add_fits {a b : Nat} {k : Nat} (ha : a < 2 ^ k) (hb : b < 2 ^ k) :
    a + b < 2 ^ (k + 1) := by
  rw [Nat.pow_succ]; omega

/-- On integers: `|a b| < 2^(k+l)`. -/
theorem int_mul_fits {a b : Int} {k l : Nat} (ha : a.natAbs < 2 ^ k) (hb : b.natAbs < 2 ^ l) :
    (a * b).natAbs < 2 ^ (k + l) := by
  rw [Int.natAbs_mul]; exact mul_fits ha hb

end BoolResearch
