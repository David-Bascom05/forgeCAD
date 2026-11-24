import Winding2d.Segment

/-!
# Theorem 1: the winding number does not depend on where the vertex list starts, and negates
when the list is reversed

`rot1`/`rot` as in verify/forge-text (ForgeText/Winding.lean, `winding_from_any_start`).
Reversal holds exactly, for every `q`, on or off the polygon (`cross_swap` is exact).
-/

namespace Winding2d

/-- One step of rotation: the first vertex moved to the end. -/
def rot1 {α : Type} : List α → List α
  | [] => []
  | a :: t => t ++ [a]

/-- `k` steps of rotation. -/
def rot {α : Type} : Nat → List α → List α
  | 0, l => l
  | k + 1, l => rot1 (rot k l)

theorem chain_snoc : ∀ (l : List Pt) (y x : Pt), chain (l ++ [y, x]) = chain (l ++ [y]) ++ [(y, x)]
  | [], _, _ => rfl
  | [_], _, _ => rfl
  | a :: b :: t, y, x => by
    have ih := chain_snoc (b :: t) y x
    simp only [List.cons_append] at ih ⊢
    simp only [chain]; rw [ih]; rfl

theorem chain_reverse : ∀ l : List Pt, chain l.reverse = ((chain l).map Prod.swap).reverse
  | [] => rfl
  | [_] => rfl
  | a :: b :: t => by
    have ih := chain_reverse (b :: t)
    have e : (a :: b :: t).reverse = t.reverse ++ [b, a] := by simp
    have e' : (b :: t).reverse = t.reverse ++ [b] := by simp
    rw [e, chain_snoc, ← e', ih]
    simp [chain]

theorem sum_sides_rot1 (f : Pt × Pt → Int) (vs : List Pt) :
    ((sides (rot1 vs)).map f).sum = ((sides vs).map f).sum := by
  match vs with
  | [] => rfl
  | [_] => rfl
  | v :: r :: rs =>
    simp only [rot1, sides, List.cons_append]
    have e : r :: (rs ++ [v] ++ [r]) = (r :: rs) ++ [v, r] := by simp
    rw [e, chain_snoc]
    simp only [List.cons_append, chain, List.map_append, List.sum_append, List.map_cons,
      List.sum_cons, List.map_nil, List.sum_nil]
    grind

theorem wind_rot1 (vs : List Pt) (q : Pt) : wind (rot1 vs) q = wind vs q :=
  sum_sides_rot1 _ vs

/-- **Rotation.** Where the vertex list starts does not change the winding number. -/
theorem wind_rot (vs : List Pt) (q : Pt) (k : Nat) : wind (rot k vs) q = wind vs q := by
  induction k with
  | zero => rfl
  | succ k ih => simp only [rot]; rw [wind_rot1, ih]

theorem sum_map_neg {α : Type} (l : List α) (f : α → Int) :
    (l.map fun x => -f x).sum = -(l.map f).sum := by
  induction l with
  | nil => rfl
  | cons x l ih => simp only [List.map_cons, List.sum_cons, ih]; grind

theorem sum_reverse_int (l : List Int) : l.reverse.sum = l.sum := by
  induction l with
  | nil => rfl
  | cons x l ih => simp only [List.reverse_cons, List.sum_append, ih, List.sum_cons]; grind

/-- **Reversal.** Running the polygon the other way negates its winding number, about every
point. -/
theorem wind_reverse (vs : List Pt) (q : Pt) : wind vs.reverse q = -wind vs q := by
  cases vs with
  | nil => rfl
  | cons v rest =>
    have e1 : (v :: rest).reverse = rot1 (v :: rest.reverse) := by simp [rot1]
    rw [e1, wind_rot1]
    have e2 : v :: rest.reverse ++ [v] = (v :: rest ++ [v]).reverse := by simp
    unfold wind
    simp only [sides]
    rw [e2, chain_reverse, List.map_reverse, sum_reverse_int, List.map_map]
    rw [← sum_map_neg]
    congr 1
    apply List.map_congr_left
    intro e _
    simp only [Function.comp, Prod.fst_swap, Prod.snd_swap]
    exact cross_swap q e.1 e.2

end Winding2d
