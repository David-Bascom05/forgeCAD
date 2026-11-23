import ForgeExact.Ops.Pattern
import ForgeExact.Fillet.Basic

/-!
# Holes: a cut by an exact solid of revolution

research/ops-spec/design.md §2. The kernel's hole (forge-model hole.rs:146–227) is one revolve of
a meridian polygon about the drill axis, subtracted. In the hole's own frame (`z` down the drill
from the mouth; the placement is a rigid motion, so `Pattern.lean` covers it and the places), with
`ρ² = x² + y²`, every piece of the polygon is a **frustum**: between depths `z₀ < z₁` the radius
runs linearly from `r₀` to `r₁`. A wall is `r₀ = r₁`, a countersink or a drill point a cone. With
no division and no square root, a frustum is

  `z₀ ≤ z ≤ z₁ ∧ (z₁ − z₀)² ρ² ≤ (r₀ (z₁ − z) + r₁ (z − z₀))²`.

A **thru** hole ends in a half-infinite wall (`Tail`): `thru` is defined by no box at all.

* H1 `thru_finite`: the kernel's finite thru depth `L` (hole.rs:156–167, `hi − lo + 1` from
  `reach`) cuts exactly what the half-infinite wall cuts iff it clears every point of the part in
  the wall: `∀ x, S x → tail x → z x < L`. So the one obligation on `reach` is that it encloses
  the part along the axis.
* H3 `profile_wf`: the kernel's size checks (hole.rs:149–197) make every frustum well formed.
* H4 `no_target_iff`: a cut changes a solid iff the interiors meet.

No `sorry`; `Ops/Axioms.lean` checks the axioms.
-/

namespace ForgeExact.Ops

open ForgeExact Topology Fillet Lean.Grind

set_option linter.unusedSectionVars false

/-! ## H4: a cut that misses (generic topology) -/

section Generic

universe u
variable {X : Type u} [Topology X]

/-- **H4.** For solids `S` and `H`, the regularised cut leaves `S` unchanged iff their interiors
are disjoint. The kernel's `NoTarget{place}` refusal is to be exactly the right-hand side. -/
theorem no_target_iff {S H : Pred X} (hS : Solid S) (hH : Solid H) :
    reg (S.diff H) = S ↔ ∀ x, ¬(interior S x ∧ interior H x) := by
  have hHc := closed_of_solid hH
  constructor
  · intro h x ⟨hxS, hxH⟩
    have hx : reg (S.diff H) x := by rw [h]; exact interior_sub S x hxS
    obtain ⟨U, hU, hUH, hxU⟩ := hxH
    obtain ⟨y, hyU, hy⟩ := hx U hU hxU
    exact (interior_sub _ y hy).2 (hUH y hyU)
  · intro hdis
    apply Pred.ext; intro x; constructor
    · intro hx
      have := reg_mono (fun q (hq : S.diff H q) => hq.1) x hx
      unfold Solid at hS; rw [hS] at this; exact this
    · intro hxS U hU hxU
      have hx' : reg S x := by unfold Solid at hS; rw [hS]; exact hxS
      obtain ⟨y, hyU, ⟨W, hW, hWS, hyW⟩⟩ := hx' U hU hxU
      -- the open set U ∩ W lies in S; it is not inside H, else it is in int H ∩ int S
      have : ∃ w, (U w ∧ W w) ∧ ¬H w := by
        apply Classical.byContradiction; intro hno
        apply hdis y
        refine ⟨⟨W, hW, hWS, hyW⟩, Pred.inter U W, isOpen_inter _ _ hU hW, ?_, hyU, hyW⟩
        intro q hq; exact Classical.byContradiction fun hc => hno ⟨q, hq, hc⟩
      obtain ⟨w, ⟨hwU, hwW⟩, hwH⟩ := this
      -- w has a neighbourhood V off the closed H
      have : ∃ V, IsOpen V ∧ V w ∧ ∀ q, V q → ¬H q := by
        apply Classical.byContradiction; intro hno
        apply hwH; apply hHc; intro V hV hwV
        apply Classical.byContradiction; intro hmiss
        exact hno ⟨V, hV, hwV, fun q hq hHq => hmiss ⟨q, hq, hHq⟩⟩
      obtain ⟨V, hV, hwV, hVH⟩ := this
      refine ⟨w, hwU, Pred.inter (Pred.inter U W) V,
        isOpen_inter _ _ (isOpen_inter _ _ hU hW) hV, ?_, ⟨hwU, hwW⟩, hwV⟩
      intro q ⟨⟨_, hqW⟩, hqV⟩
      exact ⟨hWS q hqW, hVH q hqV⟩

end Generic

/-! ## The solid of revolution -/

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- The squared distance from the drill axis, in the hole's frame. -/
def rho2 (x : V K) : K := x.x * x.x + x.y * x.y

/-- A piece of the meridian polygon: depths `z₀ < z₁`, radii `r₀` at `z₀` and `r₁` at `z₁`. -/
structure Frustum (K : Type) where
  r₀ : K
  r₁ : K
  z₀ : K
  z₁ : K

namespace Frustum

/-- Well formed: a positive depth range and radii that are not negative. -/
def WF (f : Frustum K) : Prop := f.z₀ < f.z₁ ∧ 0 ≤ f.r₀ ∧ 0 ≤ f.r₁

/-- The frustum's solid. -/
def mem (f : Frustum K) (x : V K) : Prop :=
  f.z₀ ≤ x.z ∧ x.z ≤ f.z₁ ∧
  (f.z₁ - f.z₀) * (f.z₁ - f.z₀) * rho2 x ≤
    (f.r₀ * (f.z₁ - x.z) + f.r₁ * (x.z - f.z₀)) * (f.r₀ * (f.z₁ - x.z) + f.r₁ * (x.z - f.z₀))

/-- A wall: radius `r` from `z₀` to `z₁`. -/
def wall (r z₀ z₁ : K) : Frustum K := ⟨r, r, z₀, z₁⟩

end Frustum

/-- The half-infinite wall a thru hole ends in. -/
structure Tail (K : Type) where
  r : K
  z₀ : K

def Tail.mem (t : Tail K) (x : V K) : Prop := t.z₀ ≤ x.z ∧ rho2 x ≤ t.r * t.r

/-- A hole's solid: its frusta, and for a thru hole the tail. -/
structure HoleSolid (K : Type) where
  pieces : List (Frustum K)
  tail : Option (Tail K)

def HoleSolid.mem (H : HoleSolid K) : Pred (V K) := fun x =>
  (∃ f ∈ H.pieces, f.mem x) ∨ (∃ t, H.tail = some t ∧ t.mem x)

/-- The kernel's thru hole: the tail cut off at depth `L`. -/
def HoleSolid.truncate (H : HoleSolid K) (L : K) : HoleSolid K :=
  match H.tail with
  | none => H
  | some t => ⟨H.pieces ++ [Frustum.wall t.r t.z₀ L], none⟩

/-- A wall frustum is the cylinder between its depths (when the range is positive). -/
theorem wall_mem_iff {r z₀ z₁ : K} (h : z₀ < z₁) (x : V K) :
    (Frustum.wall r z₀ z₁).mem x ↔ z₀ ≤ x.z ∧ x.z ≤ z₁ ∧ rho2 x ≤ r * r := by
  simp only [Frustum.mem, Frustum.wall]
  have e : r * (z₁ - x.z) + r * (x.z - z₀) = (z₁ - z₀) * r := by grind
  rw [e]
  have hp : 0 < (z₁ - z₀) * (z₁ - z₀) := mul_pos' (by grind) (by grind)
  constructor
  · rintro ⟨h1, h2, h3⟩
    refine ⟨h1, h2, le_of_mul_le_mul_left' (c := (z₁ - z₀) * (z₁ - z₀)) ?_ hp⟩
    have : (z₁ - z₀) * r * ((z₁ - z₀) * r) = (z₁ - z₀) * (z₁ - z₀) * (r * r) := by grind
    grind
  · rintro ⟨h1, h2, h3⟩
    refine ⟨h1, h2, ?_⟩
    have := mul_le_mul_of_nonneg_left' h3 (le_of_lt hp)
    have e2 : (z₁ - z₀) * r * ((z₁ - z₀) * r) = (z₁ - z₀) * (z₁ - z₀) * (r * r) := by grind
    grind
where le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind

/-- **H1.** A finite thru depth `L` past the tail's start cuts exactly what the half-infinite wall
cuts, as soon as every point of the part inside the wall lies above `L`. -/
theorem thru_finite (S : Pred (V K)) (H : HoleSolid K) (t : Tail K) (ht : H.tail = some t)
    {L : K} (hL : t.z₀ < L) (hclear : ∀ x, S x → t.mem x → x.z < L) :
    S.diff H.mem = S.diff (H.truncate L).mem := by
  apply Pred.ext; intro x
  simp only [Pred.diff, HoleSolid.mem, HoleSolid.truncate, ht]
  constructor
  · rintro ⟨hS, hno⟩
    refine ⟨hS, ?_⟩
    rintro (⟨f, hf, hfx⟩ | ⟨t', ht', _⟩)
    · rcases List.mem_append.mp hf with hf | hf
      · exact hno (Or.inl ⟨f, hf, hfx⟩)
      · rw [List.mem_singleton.mp hf] at hfx
        have := (wall_mem_iff hL x).mp hfx
        exact hno (Or.inr ⟨t, rfl, this.1, this.2.2⟩)
    · cases ht'
  · rintro ⟨hS, hno⟩
    refine ⟨hS, ?_⟩
    rintro (⟨f, hf, hfx⟩ | ⟨t', ht', ht'x⟩)
    · exact hno (Or.inl ⟨f, List.mem_append_left _ hf, hfx⟩)
    · cases ht'
      have hz := hclear x hS ht'x
      exact hno (Or.inl ⟨_, List.mem_append_right _ (List.mem_singleton_self _),
        (wall_mem_iff hL x).mpr ⟨ht'x.1, by grind, ht'x.2⟩⟩)

/-- H1 in the form the kernel uses: a depth past an enclosure of the whole part. -/
theorem thru_enclosed (S : Pred (V K)) (H : HoleSolid K) (t : Tail K) (ht : H.tail = some t)
    {L : K} (hL : t.z₀ < L) (henc : ∀ x, S x → x.z < L) :
    S.diff H.mem = S.diff (H.truncate L).mem :=
  thru_finite S H t ht hL fun x hx _ => henc x hx

/-! ## H3: the kernel's profiles -/

/-- The hole's head, as the kernel builds it (hole.rs:170–190): a counterbore `(Rc, dc)` or a
countersink `(Rc, dk)`, `dk` its depth (from the angle, one rounding). -/
inductive Head (K : Type)
  | plain
  | cbore (Rc dc : K)
  | csink (Rc dk : K)

/-- The end: blind at wall depth `D`, with a drill point of length `p` or a flat bottom (`none`),
or thru. -/
inductive HoleEnd (K : Type)
  | blind (D : K) (p : Option K)
  | thru

/-- The wall's start depth below the head. -/
def Head.seat : Head K → K
  | .plain => 0
  | .cbore _ dc => dc
  | .csink _ dk => dk

def Head.pieces (r : K) : Head K → List (Frustum K)
  | .plain => []
  | .cbore Rc dc => [Frustum.wall Rc 0 dc]
  | .csink Rc dk => [⟨Rc, r, 0, dk⟩]

/-- The meridian polygon, as a solid. -/
def profile (r : K) (hd : Head K) : HoleEnd K → HoleSolid K
  | .blind D p =>
    ⟨hd.pieces r ++ [Frustum.wall r hd.seat D] ++ (match p with | some p => [⟨r, 0, D, D + p⟩] | none => []), none⟩
  | .thru => ⟨hd.pieces r, some ⟨r, hd.seat⟩⟩

/-- The kernel's size checks (hole.rs:149–197, `HoleError::BadSize` otherwise). -/
def Head.ok (r : K) : Head K → Prop
  | .plain => True
  | .cbore Rc dc => 0 < dc ∧ r < Rc
  | .csink Rc dk => 0 < dk ∧ r < Rc

def HoleEnd.ok (seat : K) : HoleEnd K → Prop
  | .blind D p => seat < D ∧ ∀ q, p = some q → 0 < q
  | .thru => True

/-- **H3.** Under the kernel's checks every frustum of the profile is well formed. -/
theorem profile_wf {r : K} (hr : 0 < r) (hd : Head K) (e : HoleEnd K) (hhd : hd.ok r)
    (he : e.ok hd.seat) : ∀ f ∈ (profile r hd e).pieces, f.WF := by
  have hseat : 0 ≤ hd.seat := by
    cases hd <;> simp only [Head.seat, Head.ok] at * <;> grind
  have hhead : ∀ f ∈ hd.pieces r, f.WF := by
    cases hd <;> simp only [Head.pieces, Head.ok, List.mem_cons, List.not_mem_nil, or_false] at *
    · intro f hf; cases hf
    · intro f hf; subst hf; simp only [Frustum.WF, Frustum.wall]; grind
    · intro f hf; subst hf; simp only [Frustum.WF]; grind
  cases e with
  | thru => exact hhead
  | blind D p =>
    simp only [HoleEnd.ok] at he
    intro f hf
    simp only [profile, List.mem_append, List.mem_singleton] at hf
    rcases hf with (hf | hf) | hf
    · exact hhead f hf
    · subst hf; simp only [Frustum.WF, Frustum.wall]; grind
    · cases p with
      | none => cases hf
      | some q =>
        simp only [List.mem_singleton] at hf; subst hf
        have := he.2 q rfl
        simp only [Frustum.WF]; grind

end ForgeExact.Ops
