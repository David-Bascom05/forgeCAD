import ForgeExact.Poly3.Basic

/-!
# A polynomial about a box's centre

The box bound (`box_bound`) is tight only about the origin of the monomials. The octree uses
`p` expanded about the box's centre `c`, `shift p c` (`eval_shift`: its value at `u` is `p`'s at
`c + u`), with like monomials collected (`collect`, `eval_collect`). Monomials are collected
only when their variable lists are equal, which is always sound. When `p`'s variable lists are
sorted, so are the shifted ones (each is a sublist), and then this collects every like term.
-/

namespace ForgeExact.Poly3

open ForgeExact Ref

theorem coord_add (c u : V3) (v : Fin 3) : coord (c + u) v = coord c v + coord u v := by
  match v with
  | 0 => rfl
  | 1 => rfl
  | 2 => rfl

theorem eval_nil (x : V3) : P3.eval [] x = 0 := rfl

theorem eval_cons (m : Mono) (p : P3) (x : V3) : P3.eval (m :: p) x = m.eval x + p.eval x := rfl

theorem eval_append (p q : P3) (x : V3) : P3.eval (p ++ q) x = p.eval x + q.eval x := by
  induction p with
  | nil => simp only [List.nil_append, eval_nil]; grind
  | cons m p ih => simp only [List.cons_append, eval_cons, ih]; grind

/-- Each monomial times `k`. -/
def scaleBy (k : Rat) (p : P3) : P3 := p.map fun m => ⟨k * m.a, m.vs⟩

theorem eval_scaleBy (k : Rat) (p : P3) (x : V3) : (scaleBy k p).eval x = k * p.eval x := by
  induction p with
  | nil => simp only [scaleBy, List.map_nil, eval_nil]; grind
  | cons m p ih =>
    simp only [scaleBy, List.map_cons, eval_cons] at ih ⊢
    rw [ih]; simp only [Mono.eval]; grind

/-- Each monomial times the variable `v`. -/
def timesVar (v : Fin 3) (p : P3) : P3 := p.map fun m => ⟨m.a, v :: m.vs⟩

theorem eval_timesVar (v : Fin 3) (p : P3) (x : V3) :
    (timesVar v p).eval x = coord x v * p.eval x := by
  induction p with
  | nil => simp only [timesVar, List.map_nil, eval_nil]; grind
  | cons m p ih =>
    simp only [timesVar, List.map_cons, eval_cons] at ih ⊢
    rw [ih]; simp only [Mono.eval, prodAt, List.foldr_cons]; grind

/-- `Π_{v ∈ vs} (u_v + c_v)`, expanded. -/
def shiftVs (c : V3) : List (Fin 3) → P3
  | [] => [⟨1, []⟩]
  | v :: vs => timesVar v (shiftVs c vs) ++ scaleBy (coord c v) (shiftVs c vs)

theorem eval_shiftVs (c u : V3) : ∀ vs, (shiftVs c vs).eval u = prodAt vs (c + u)
  | [] => by simp only [shiftVs, eval_cons, eval_nil, Mono.eval, prodAt, List.foldr_nil]; grind
  | v :: vs => by
    simp only [shiftVs, eval_append, eval_timesVar, eval_scaleBy, eval_shiftVs c u vs]
    simp only [prodAt, List.foldr_cons, coord_add]; grind

/-- `p` about `c`. -/
def shift (p : P3) (c : V3) : P3 := p.flatMap fun m => scaleBy m.a (shiftVs c m.vs)

theorem eval_shift (p : P3) (c u : V3) : (shift p c).eval u = p.eval (c + u) := by
  induction p with
  | nil => rfl
  | cons m p ih =>
    simp only [shift, List.flatMap_cons, eval_append, eval_scaleBy, eval_shiftVs, eval_cons] at ih ⊢
    rw [ih]; rfl

/-- Add a monomial into a like one, or append it. -/
def ins (m : Mono) : P3 → P3
  | [] => [m]
  | n :: p => if n.vs = m.vs then ⟨n.a + m.a, n.vs⟩ :: p else n :: ins m p

theorem eval_ins (m : Mono) (x : V3) : ∀ p, (ins m p).eval x = m.eval x + p.eval x
  | [] => by simp only [ins, eval_cons, eval_nil]
  | n :: p => by
    simp only [ins]
    split
    · rename_i h
      simp only [eval_cons, Mono.eval, h]; grind
    · simp only [eval_cons, eval_ins m x p]; grind

/-- Like monomials collected. -/
def collect (p : P3) : P3 := p.foldr ins []

theorem eval_collect (p : P3) (x : V3) : (collect p).eval x = p.eval x := by
  induction p with
  | nil => rfl
  | cons m p ih => simp only [collect, List.foldr_cons] at ih ⊢; rw [eval_ins, ih]; rfl

/-- `p` about `c`, collected. -/
def about (p : P3) (c : V3) : P3 := collect (shift p c)

theorem eval_about (p : P3) (c u : V3) : (about p c).eval u = p.eval (c + u) := by
  rw [about, eval_collect, eval_shift]

def zero3 : V3 := ⟨0, 0, 0⟩

theorem add_sub_cancel' (c x : V3) : c + (x - c) = x := by
  cases c; cases x
  show V3.add _ (V3.sub _ _) = _
  simp only [V3.add, V3.sub]; congr 1 <;> grind

theorem add_zero3 (c : V3) : c + zero3 = c := by
  cases c
  show V3.add _ _ = _
  simp only [V3.add, zero3]; congr 1 <;> grind

/-- Points near `c` give points near the origin, relative to `c`. -/
theorem near_rel {c x : V3} {h : Rat} (hn : Near c x h) : Near zero3 (x - c) h := by
  intro v
  have e : coord (x - c) v = coord x v - coord c v := by
    match v with
    | 0 => rfl
    | 1 => rfl
    | 2 => rfl
  have e0 : coord zero3 v = 0 := by
    match v with
    | 0 => rfl
    | 1 => rfl
    | 2 => rfl
  rw [e, e0]
  have := hn v
  have : coord x v - coord c v - 0 = coord x v - coord c v := by grind
  rwa [this]

/-- **The bound about the centre**: `|p(x) − p(c)| ≤ bound (about p c) 0 h` near `c`. -/
theorem about_bound (p : P3) (c x : V3) {h : Rat} (hh : 0 ≤ h) (hn : Near c x h) :
    qabs (p.eval x - p.eval c) ≤ bound (about p c) zero3 h := by
  have b := box_bound (about p c) zero3 (x - c) hh (near_rel hn)
  rwa [eval_about, eval_about, add_sub_cancel', add_zero3] at b

end ForgeExact.Poly3
