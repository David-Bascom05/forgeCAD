import ForgeExact.DecideOnce.CorrectK
import ForgeExact.DecideOnce.Local
import ForgeExact.DecideOnce.LocalK

/-!
# Sign formulas: `compat` with no Jordan (design §2.5a), and separator invariance (§2.3)

A body carries its **sign formula** (design decision D2): a boolean formula over literals
"carrier `j` has side `s`", the curved form of Nef.lean's half-space formulas and of exact-curved's
`QForm`. Its set is read off the carriers' signs at a point (`SForm.den`), so:

* `den_cell`: on a cell, the formula's membership is its value on the cell's covector;
* **`qform_compat`**: so membership is constant on every cell of the carriers, full or not, for
  any carriers: `ExactK.compat` needs no Jordan–Brouwer and no connectedness;
* **`qform_localCompat`**: and flat's `LocalCompat` (Local.lean) holds for formula bodies, for
  every region, at once. The coordinator's `localCompat_of` (LocalFinal.lean, from
  `contains_const_local`, JordanLocal.lean) proves it for closed flat B-reps classified by ray
  parity: the general case. This is the shorter route on the narrower domain of bodies that carry
  formulas; it does not replace that proof for B-rep inputs, whose formula would have to be
  computed by the same classification;
* `den_append`, `den_sep`: a formula over `cs` means the same over `cs ++ seps` (separators are
  appended after the input carriers), so the sidecar survives any choice of separators;
* `sep_invariant_reg`: two exact ledgers of one family of solids, over any two carrier lists,
  regularise their kept cells to one set: the regularised boolean. The outcome-kind half of
  `sep_invariant` (the same refusal over both) waits for Q1's `xNonManifold_iff_top`.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.DecideOnce Topology

/-- A sign formula: literals "carrier `j` has side `s`", and the connectives. -/
inductive SForm
  | lit (j : Nat) (s : Sgn)
  | tt
  | ff
  | and (a b : SForm)
  | or (a b : SForm)
  | not (a : SForm)

namespace SForm

/-- The formula's value on a covector (what the ledger evaluates: the witness). -/
def onCov : SForm → Cov → Bool
  | lit j s, X => X[j]? == some s
  | tt, _ => true
  | ff, _ => false
  | and a b, X => a.onCov X && b.onCov X
  | or a b, X => a.onCov X || b.onCov X
  | not a, X => !a.onCov X

/-- Every literal names a carrier below `n`. -/
def Below (n : Nat) : SForm → Prop
  | lit j _ => j < n
  | tt => True
  | ff => True
  | and a b => a.Below n ∧ b.Below n
  | or a b => a.Below n ∧ b.Below n
  | not a => a.Below n

variable {P K : Type} [Zero K] [LT K] [DecidableRel (α := K) (· < ·)]

/-- The formula's set, from the carriers' values at a point (the spec: no covector in sight). -/
def den (cs : List (Carrier P K)) : SForm → Pred P
  | lit j s => fun x => ∃ c, cs[j]? = some c ∧ sgnK (c x) = s
  | tt => fun _ => True
  | ff => fun _ => False
  | and a b => fun x => a.den cs x ∧ b.den cs x
  | or a b => fun x => a.den cs x ∨ b.den cs x
  | not a => fun x => ¬a.den cs x

theorem den_iff (cs : List (Carrier P K)) (x : P) :
    ∀ F : SForm, F.den cs x ↔ F.onCov (covOfK cs x) = true
  | lit j s => by
    simp only [den, onCov, covOfK, List.getElem?_map, beq_iff_eq]
    cases cs[j]? <;> simp
  | tt => by simp [den, onCov]
  | ff => by simp [den, onCov]
  | and a b => by simp [den, onCov, den_iff cs x a, den_iff cs x b]
  | or a b => by simp [den, onCov, den_iff cs x a, den_iff cs x b]
  | not a => by simp [den, onCov, den_iff cs x a]

/-- **On a cell, membership is the formula on the cell's covector.** -/
theorem den_cell {cs : List (Carrier P K)} {T : Cov} {x : P} (hx : covCellK cs T x) (F : SForm) :
    F.den cs x ↔ F.onCov T = true := by
  rw [den_iff]; unfold covCellK at hx; rw [hx]

/-- A formula does not read the coordinates past its bound. -/
theorem onCov_set_ge {n : Nat} : ∀ (F : SForm), F.Below n → ∀ (X : Cov) (k : Nat) (s : Sgn),
    n ≤ k → F.onCov (X.set k s) = F.onCov X
  | lit j s', h, X, k, s, hk => by
    simp only [Below] at h
    simp only [onCov]
    rw [List.getElem?_set_ne (by omega)]
  | tt, _, _, _, _, _ => rfl
  | ff, _, _, _, _, _ => rfl
  | and a b, h, X, k, s, hk => by
    simp only [Below] at h
    simp only [onCov, onCov_set_ge a h.1 X k s hk, onCov_set_ge b h.2 X k s hk]
  | or a b, h, X, k, s, hk => by
    simp only [Below] at h
    simp only [onCov, onCov_set_ge a h.1 X k s hk, onCov_set_ge b h.2 X k s hk]
  | not a, h, X, k, s, hk => by
    simp only [Below] at h
    simp only [onCov, onCov_set_ge a h X k s hk]

theorem den_append (cs seps : List (Carrier P K)) (x : P) :
    ∀ F : SForm, F.Below cs.length → (F.den (cs ++ seps) x ↔ F.den cs x)
  | lit j s, h => by
    simp only [Below] at h
    simp only [den, List.getElem?_append_left h]
  | tt, _ => Iff.rfl
  | ff, _ => Iff.rfl
  | and a b, ⟨ha, hb⟩ => by simp only [den, den_append cs seps x a ha, den_append cs seps x b hb]
  | or a b, ⟨ha, hb⟩ => by simp only [den, den_append cs seps x a ha, den_append cs seps x b hb]
  | not a, ha => by simp only [den, den_append cs seps x a ha]

end SForm

variable {P K : Type} [Zero K] [LT K] [DecidableRel (α := K) (· < ·)]

/-- **`compat` for formula bodies**, on every cell of any carriers. -/
theorem qform_compat (cs : List (Carrier P K)) (F : Nat → SForm) (T : Cov) (i : Nat) :
    (covCellK cs T).Sub ((F i).den cs) ∨ ∀ x, covCellK cs T x → ¬(F i).den cs x := by
  by_cases hv : (F i).onCov T = true
  · exact Or.inl fun x hx => ((F i).den_cell hx).mpr hv
  · exact Or.inr fun x hx hd => hv (((F i).den_cell hx).mp hd)

/-- A separator appended to the carriers leaves every formula body's set unchanged. -/
theorem den_sep (cs seps : List (Carrier P K)) (F : SForm) (hF : F.Below cs.length) :
    F.den (cs ++ seps) = F.den cs :=
  funext fun x => propext (SForm.den_append cs seps x F hF)

/-- **Flat's `LocalCompat`, for formula bodies over the carrier planes**, whatever the region. -/
theorem qform_localCompat (ps : List Ref.Plane) (F : Nat → SForm) (In : Cov → Prop) :
    LocalCompat ps (fun i => (F i).den (ps.map fun p => p.eval)) In := by
  intro T _ _ i
  rcases qform_compat (ps.map fun p => p.eval) F T i with hs | hd
  · exact Or.inl fun x hx => hs x ((covCellK_flat ps T x).mpr hx)
  · exact Or.inr fun x hx => hd x ((covCellK_flat ps T x).mpr hx)

/-- **`LocalCompatK` for formula bodies**: (c′) over any carriers needs no localized Jordan. -/
theorem qform_localCompatK [Topology P] (cs : List (Carrier P K)) (F : Nat → SForm) (In : Cov → Prop) :
    LocalCompatK cs (fun i => (F i).den cs) In :=
  fun T _ _ i => qform_compat cs F T i

/-- **Separator invariance, the set.** Two exact ledgers of one family of solids, over any two
carrier lists (say `cs` and `cs ++ seps`), regularise their kept full cells to the same set. -/
theorem sep_invariant_reg [Topology P] {L L' : Ledger} {cs cs' : List (Carrier P K)}
    {A : Nat → Pred P} (h : ExactK L cs A) (h' : ExactK L' cs' A) (hnb : L.nb = L'.nb)
    (rule : Rule) :
    reg ((decompK h).kept fun T => xkeepK L cs A rule T = true) =
      reg ((decompK h').kept fun T => xkeepK L' cs' A rule T = true) := by
  rw [(correct_k h rule).2, (correct_k h' rule).2]
  unfold opndsK; rw [hnb]

end ForgeExact.DecideOnce.Curved
