import ForgeExact.Flat

/-!
# Weldment members and why they never overlap (research/PROOF-STANDARD.md, L1)

Over ℚ³, on the boolean's own ground (`ForgeExact`: `Ref.V3`, `Pred`, planes, `wall`).

* A **straight member** is its section `S` (a region in the member's frame `(u, v)`) carried
  along the line through `a` with direction `d`, frame axes `ex`, `ey`, intersected with the
  closed half-spaces its end cuts keep (`Member`). This is forge-weld's member: the kernel's
  extrusion, run on far enough, less each cut's half-space (forge-model `weld::body`).
* A **separating plane** for two members: each lies in its own closed side (`Apart`). Then they
  meet at most on that plane (`apart_inter_wall`), a nowhere-dense set: their interiors are
  disjoint (`apart_nowhereDense`), so the regularised union is exactly the two side by side.

The two ways the node rule (forge_weld::frame) produces such a plane:

* **miter** (`miter_apart`): both members are cut on one plane `m · x = m · p`, each `gap / 2`
  back on its own side.
* **butt** (`butt_apart`): member `B` is cut on the plane touching `A`'s section in the direction
  `w` square to `A`'s axis (`A`'s support `h`), `gap` past it; a *straight* `A` lies on the near
  side of that plane by the support property alone (`straight_in_support`).

The second is the "certified apart without a boolean" shortcut. It holds for a straight `A`, or
when the two share one cut plane from opposite sides (the miter case); for an arc member it is
false in general (its section curves off its tangent prism), so forge-lang keeps the boolean
check for pairs with an arc member there. `node_apart` (Node.lean) states that the rank rule gives
every pair of members at a node one of these planes.
-/

namespace ForgeWeld

open ForgeExact ForgeExact.Ref ForgeExact.Topology

/-! ## Vectors -/

@[simp] theorem dot_add (a b c : V3) : a.dot (b + c) = a.dot b + a.dot c := by
  show a.dot (V3.add b c) = _
  simp only [V3.dot, V3.add]; grind

@[simp] theorem dot_sub (a b c : V3) : a.dot (b - c) = a.dot b - a.dot c := by
  show a.dot (V3.sub b c) = _
  simp only [V3.dot, V3.sub]; grind

@[simp] theorem dot_smul (a b : V3) (k : Rat) : a.dot (k * b) = k * a.dot b := by
  show a.dot (V3.smul k b) = _
  simp only [V3.dot, V3.smul]; grind

/-- The opposite direction. -/
def neg (a : V3) : V3 := (-1 : Rat) * a

@[simp] theorem neg_dot (a b : V3) : (neg a).dot b = -(a.dot b) := by
  show (V3.smul (-1) a).dot b = _
  simp only [V3.dot, V3.smul]; grind

/-! ## Members -/

/-- A closed half-space kept by a cut: `n · x ≤ c`. -/
def Half (n : V3) (c : Rat) : Pred V3 := fun x => n.dot x ≤ c

/-- A straight member's frame: a point of its path, its direction, its section's axes. -/
structure Frame where
  a : V3
  d : V3
  ex : V3
  ey : V3

/-- The member's section carried along its line: `a + s d + u ex + v ey` for `(u, v)` in `S`. -/
def Prism (F : Frame) (S : Rat → Rat → Prop) : Pred V3 :=
  fun x => ∃ s u v : Rat, S u v ∧ x = F.a + s * F.d + u * F.ex + v * F.ey

/-- A straight member: the prism less what each cut takes (each cut `(n, c)` keeps `n · x ≤ c`). -/
def Member (F : Frame) (S : Rat → Rat → Prop) (cuts : List (V3 × Rat)) : Pred V3 :=
  fun x => Prism F S x ∧ ∀ k ∈ cuts, Half k.1 k.2 x

theorem member_half {F : Frame} {S : Rat → Rat → Prop} {cuts : List (V3 × Rat)} {n : V3} {c : Rat}
    (hk : (n, c) ∈ cuts) {x : V3} (hx : Member F S cuts x) : n.dot x ≤ c :=
  hx.2 _ hk

/-- `h` bounds the section in the direction `(wu, wv)`: the support function's defining property. -/
def Upper (S : Rat → Rat → Prop) (wu wv h : Rat) : Prop := ∀ u v, S u v → wu * u + wv * v ≤ h

/-- A straight member lies on the near side of the plane touching its section in any direction
square to its axis: `w · (x - a) ≤ h` where `h` bounds the section in `(w · ex, w · ey)`. -/
theorem straight_in_support {F : Frame} {S : Rat → Rat → Prop} {w : V3} {h : Rat}
    (hw : w.dot F.d = 0) (hS : Upper S (w.dot F.ex) (w.dot F.ey) h) {x : V3} (hx : Prism F S x) :
    w.dot x ≤ w.dot F.a + h := by
  obtain ⟨s, u, v, hSuv, rfl⟩ := hx
  have := hS u v hSuv
  simp only [dot_add, dot_smul, hw]
  grind

/-! ## Apart by a plane -/

/-- `A` and `B` lie on the two closed sides of the plane `w · x = c`. -/
def Apart (A B : Pred V3) (w : V3) (c : Rat) : Prop :=
  (∀ x, A x → w.dot x ≤ c) ∧ (∀ x, B x → c ≤ w.dot x)

/-- Two members apart by a plane meet at most on it. -/
theorem apart_inter_wall {A B : Pred V3} {w : V3} {c : Rat} (h : Apart A B w c) :
    (A.inter B).Sub (Flat.wall [⟨w, c⟩]) := by
  intro x ⟨ha, hb⟩
  refine ⟨⟨w, c⟩, List.mem_singleton.mpr rfl, ?_⟩
  have h1 := h.1 x ha
  have h2 := h.2 x hb
  show w.dot x - c = 0
  grind

/-- A subset of a nowhere-dense set is nowhere dense. -/
theorem nowhereDense_mono {Z W : Pred V3} (hW : NowhereDense W) (h : Z.Sub W) : NowhereDense Z :=
  fun x hx => hW x (interior_mono (closure_mono h) x hx)

/-- So their common part is nowhere dense: the members' interiors are disjoint, and the
regularised union of the two is each of them with nothing shared. -/
theorem apart_nowhereDense {A B : Pred V3} {w : V3} {c : Rat} (hw : w ≠ ⟨0, 0, 0⟩)
    (h : Apart A B w c) : NowhereDense (A.inter B) :=
  nowhereDense_mono
    (Flat.wall_nowhereDense [⟨w, c⟩] fun p hp => by
      rw [List.mem_singleton.mp hp]; exact hw)
    (apart_inter_wall h)

/-- With a gap past the plane, they do not meet at all. -/
theorem apart_gap_empty {A B : Pred V3} {w : V3} {c g : Rat} (hg : 0 < g)
    (hA : ∀ x, A x → w.dot x ≤ c) (hB : ∀ x, B x → c + g ≤ w.dot x) : ∀ x, ¬(A x ∧ B x) := by
  intro x ⟨ha, hb⟩
  have := hA x ha
  have := hB x hb
  grind

/-! ## The two joints -/

/-- **Miter.** Member 1 keeps `m · x ≥ m · p + g/2` (its cut `(neg m, -(m·p + g/2))`), member 2 keeps
`m · x ≤ m · p - g/2` (its cut `(m, m·p - g/2)`). With `g ≥ 0` they are apart by the bisecting
plane `m · x = m · p`. -/
theorem miter_apart {F₁ F₂ : Frame} {S₁ S₂ : Rat → Rat → Prop} {c₁ c₂ : List (V3 × Rat)}
    {m p : V3} {g : Rat} (hg : 0 ≤ g)
    (h₁ : (neg m, -(m.dot p + g / 2)) ∈ c₁) (h₂ : (m, m.dot p - g / 2) ∈ c₂) :
    Apart (Member F₂ S₂ c₂) (Member F₁ S₁ c₁) m (m.dot p) := by
  refine ⟨fun x hx => ?_, fun x hx => ?_⟩
  · have := member_half h₂ hx; grind
  · have := member_half h₁ hx
    rw [neg_dot] at this; grind

/-- **Butt.** `B` is cut on the plane touching straight `A`'s section in the direction `w` square
to `A`'s axis, `g ≥ 0` past it (its cut `(neg w, -(w·a + h + g))`): they are apart by the plane
`w · x = w · a + h`, with nothing about `B`'s shape needed. -/
theorem butt_apart {FA FB : Frame} {SA SB : Rat → Rat → Prop} {cA cB : List (V3 × Rat)}
    {w : V3} {h g : Rat} (hg : 0 ≤ g) (hw : w.dot FA.d = 0)
    (hS : Upper SA (w.dot FA.ex) (w.dot FA.ey) h)
    (hB : (neg w, -(w.dot FA.a + h + g)) ∈ cB) :
    Apart (Member FA SA cA) (Member FB SB cB) w (w.dot FA.a + h) := by
  refine ⟨fun x hx => straight_in_support hw hS hx.1, fun x hx => ?_⟩
  have := member_half hB hx
  rw [neg_dot] at this; grind

/-- **Butt, the run-on member.** `A` runs on to the plane touching `B`'s section on its far side
(its cut `(w', h_B + w'·b)`): `A`'s end lies on that plane, flush with `B`'s far face. -/
theorem runon_flush {FA FB : Frame} {SA SB : Rat → Rat → Prop} {cA cB : List (V3 × Rat)}
    {w' : V3} {hB : Rat} (hw : w'.dot FB.d = 0) (hS : Upper SB (w'.dot FB.ex) (w'.dot FB.ey) hB)
    (hA : (w', w'.dot FB.a + hB) ∈ cA) :
    (∀ x, Member FA SA cA x → w'.dot x ≤ w'.dot FB.a + hB) ∧
      (∀ x, Member FB SB cB x → w'.dot x ≤ w'.dot FB.a + hB) :=
  ⟨fun _ hx => member_half hA hx, fun _ hx => straight_in_support hw hS hx.1⟩

end ForgeWeld
