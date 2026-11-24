import ForgeCertify.Walls
import ForgeExact.Regular

/-!
# The theorem

When the checker accepts, the result's point set, regularised, is the boolean of the operands'
point sets, regularised: `reg (den res) = reg {x | op (memberships of x)}`. The proof: off the
finitely many planes `walls` (nowhere dense), nothing is tied (`gen_of_walls`), and there the
result's membership is the boolean of the operands' (`at_generic`); `reg` does not see a nowhere
dense set (`reg_eq_of_sdiff`).

There is no gap for faces along `x` (`n.x = 0`): those are handled by `pt_vert`.
-/

namespace ForgeCertify

open ForgeExact ForgeExact.Ref Topology

/-- The operands' boolean, as a point set. -/
def opSet (op : List Bool → Bool) (ins : List Opd) : Pred V3 :=
  fun x => op (ins.map fun o => (Solid.mk "" o.faces.toArray).contains x) = true

/-- **Soundness.** The checker accepts ⇒ the result is the exact regularised boolean. -/
theorem check_sound (op : List Bool → Bool) (ins : List Opd) (res : List Face)
    (hc : check op ins res = true) : reg (den res) = reg (opSet op ins) := by
  apply reg_eq_of_sdiff (Flat.wall_nowhereDense (walls ins res) (walls_nonzero ins res))
  intro x hx
  refine Classical.byContradiction fun hw => ?_
  have off : ∀ Q ∈ walls ins res, Q.eval x ≠ 0 := fun Q hQ e => hw ⟨Q, hQ, e⟩
  have e := at_generic op ins res hc x (gen_of_walls op ins res hc x off)
  unfold Pred.sdiff den opSet at hx
  rw [e] at hx
  rcases hx with ⟨a, b⟩ | ⟨a, b⟩ <;> exact b a

/-! ## Two operands -/

/-- **Soundness, two operands**, in `forge-exact`'s own terms: the checker accepts ⇒ the result
is `regBool o A B`, the regularised boolean of the operands' point sets. -/
theorem check_sound_bool (o : Op) (A B : Opd) (res : List Face) (hc : check (binOp o) [A, B] res = true) :
    reg (den res) = regBool o (den A.faces) (den B.faces) := by
  rw [check_sound _ _ _ hc]
  unfold regBool
  congr 1
  funext x
  unfold opSet combine Op.rule pair den
  cases o <;> simp [binOp, Op.eval]

end ForgeCertify
