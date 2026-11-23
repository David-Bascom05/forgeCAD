import ForgeExact.Alg.Real

/-!
# A polynomial of length `n` with `n` distinct roots is zero

Over ℚ: if a coefficient list of length `n` vanishes at `n` distinct rationals, it vanishes
everywhere. This is the factor theorem, `p = (x − a) q + p(a)` (`synDiv`, with its evaluation
identity), applied `n` times. It is what makes the wall of an arrangement of polynomials of any
degree nowhere dense (ForgeExact/Poly3/Basic.lean).
-/

namespace ForgeExact.Poly3

open ForgeExact.Alg

/-- Synthetic division by `x − a`: `p = (x − a) q + r`. -/
def synDiv (a : Rat) : Poly → Poly × Rat
  | [] => ([], 0)
  | [c] => ([], c)
  | c :: d :: p =>
    let (q, r) := synDiv a (d :: p)
    -- c + x ((x − a) q + r) = (x − a) (r :: q) + (c + a r)
    (r :: q, c + a * r)

theorem synDiv_eval (a : Rat) : ∀ (p : Poly) (x : Rat),
    Poly.ev p x = (x - a) * Poly.ev (synDiv a p).1 x + (synDiv a p).2
  | [], x => by simp [synDiv, Poly.ev]; grind
  | [c], x => by simp [synDiv, Poly.ev]; grind
  | c :: d :: p, x => by
    have ih := synDiv_eval a (d :: p) x
    simp only [synDiv]
    generalize (synDiv a (d :: p)).1 = q at ih ⊢
    generalize (synDiv a (d :: p)).2 = r at ih ⊢
    simp only [Poly.ev, List.foldr_cons] at ih ⊢
    rw [ih]; grind

theorem synDiv_length (a : Rat) : ∀ p : Poly, p ≠ [] → (synDiv a p).1.length + 1 = p.length
  | [], h => absurd rfl h
  | [c], _ => by simp [synDiv]
  | c :: d :: p, _ => by
    have ih := synDiv_length a (d :: p) (by simp)
    simp only [synDiv] at ih ⊢
    simp only [List.length_cons] at ih ⊢
    omega

/-- **`n` distinct roots of a polynomial of length `n`**: it vanishes everywhere. -/
theorem zero_of_roots : ∀ (p : Poly) (rs : List Rat), rs.Nodup → p.length ≤ rs.length →
    (∀ r ∈ rs, Poly.ev p r = 0) → ∀ x, Poly.ev p x = 0
  | [], _, _, _, _, x => by simp [Poly.ev]
  | c :: p, [], _, hl, _, _ => by simp at hl
  | c :: p, a :: rs, hnd, hl, hr, x => by
    have hp := synDiv_eval a (c :: p)
    have hlen := synDiv_length a (c :: p) (by simp)
    generalize hq : (synDiv a (c :: p)).1 = q at hp hlen
    generalize hrr : (synDiv a (c :: p)).2 = r at hp hlen
    have ha := hr a (List.mem_cons_self ..)
    rw [hp a] at ha
    have hr0 : r = 0 := by grind
    subst hr0
    -- q vanishes at the other roots
    have hq0 : ∀ b ∈ rs, Poly.ev q b = 0 := by
      intro b hb
      have hb' := hr b (List.mem_cons_of_mem _ hb)
      rw [hp b] at hb'
      have hne : b - a ≠ 0 := by
        have : a ∉ rs := (List.nodup_cons.mp hnd).1
        intro h; exact this (by rw [show a = b by grind]; exact hb)
      have : (b - a) * Poly.ev q b = 0 := by grind
      rcases Rat.mul_eq_zero.mp this with h | h
      · exact absurd h hne
      · exact h
    have hz := zero_of_roots q rs (List.nodup_cons.mp hnd).2
      (by simp only [List.length_cons] at hl hlen; omega) hq0 x
    rw [hp x, hz]; grind
termination_by p => p.length
decreasing_by simp only [List.length_cons] at hlen ⊢; omega

end ForgeExact.Poly3
