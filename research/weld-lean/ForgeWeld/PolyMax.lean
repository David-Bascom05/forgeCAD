import ForgeWeld.Cell

/-!
# The long point over a polygonal section (L3, step 3)

A polygonal section in the boolean's own terms (forge-exact Flat.lean: a solid is a boolean of
half-spaces, its membership the same on each sign pattern of its planes): `S` and the lines `Ls`
its edges lie on (with four lines of a box round it), membership depending only on each point's
signs against the lines (`Polygonal`). The point `x`'s **closed face** is where every line keeps
the side `x` is on (and the lines through `x` hold): a bounded convex cell.

* `section_max` (completeness): every point of the section is matched in the reach by a candidate
  of its closed face: two lines meeting, a ridge on a line, or three cuts meeting (`LineCand`).
* `face_sound` (soundness): every point `q` of a section point's closed face is a limit of section
  points: the open segment from `q` to `x` lies in the section.

So the long point is the largest reach over the finite set of candidates lying in the closure of the
section: every pairwise meeting of the section's edge lines and the cuts' ridge lines, and every
apex. That set is forge_weld::frame `extreme`'s candidates (L4 is its being computed so).
-/

namespace ForgeWeld.Extreme

def Aff2.neg (f : Aff2) : Aff2 := ⟨-f.c, -f.a, -f.b⟩

/-- The sign of a rational. -/
def sgn (r : Rat) : Int := if 0 < r then 1 else if r < 0 then -1 else 0

/-- Membership depends only on the signs against the lines. -/
def Polygonal (S : Rat × Rat → Prop) (Ls : List Aff2) : Prop :=
  ∀ x y, (∀ L ∈ Ls, sgn (L.at x) = sgn (L.at y)) → (S x ↔ S y)

/-- The closed face of `x`: each line on `x`'s side, or through it where `x` is on it. -/
def face (Ls : List Aff2) (x : Rat × Rat) : List Aff2 :=
  Ls.flatMap fun L => if 0 < L.at x then [L] else if L.at x < 0 then [L.neg] else [L, L.neg]

theorem neg_at (f : Aff2) (q : Rat × Rat) : f.neg.at q = -(f.at q) := by
  simp only [Aff2.neg, Aff2.at]; grind

theorem neg_slope (f : Aff2) (δ : Rat × Rat) : f.neg.slope δ = -(f.slope δ) := by
  simp only [Aff2.neg, Aff2.slope]; grind

theorem mem_face {Ls : List Aff2} {x : Rat × Rat} {g : Aff2} (h : g ∈ face Ls x) :
    ∃ L ∈ Ls, g = L ∨ g = L.neg := by
  simp only [face, List.mem_flatMap] at h
  obtain ⟨L, hL, hg⟩ := h
  refine ⟨L, hL, ?_⟩
  split at hg
  · simp at hg; exact Or.inl hg
  · split at hg
    · simp at hg; exact Or.inr hg
    · simp at hg; exact hg

/-- `x` is in its own closed face. -/
theorem face_self (Ls : List Aff2) (x : Rat × Rat) : ∀ g ∈ face Ls x, 0 ≤ g.at x := by
  intro g hg
  simp only [face, List.mem_flatMap] at hg
  obtain ⟨L, _, hg⟩ := hg
  split at hg
  · simp at hg; subst hg; grind
  · split at hg
    · simp at hg; subst hg; rw [neg_at]; grind
    · simp at hg; rcases hg with rfl | rfl
      · grind
      · rw [neg_at]; grind

/-- Candidates over the lines: two lines meeting, a ridge on a line, three cuts meeting. -/
def LineCand (F Ls : List Aff2) (q : Rat × Rat) : Prop :=
  (∃ L₁ ∈ Ls, ∃ L₂ ∈ Ls, L₁.at q = 0 ∧ L₂.at q = 0 ∧ det L₁ L₂ ≠ 0) ∨
  (∃ f₁ ∈ F, ∃ f₂ ∈ F, ∃ L ∈ Ls, f₁.at q = f₂.at q ∧ L.at q = 0 ∧ det (f₁.sub f₂) L ≠ 0) ∨
  (∃ f₁ ∈ F, ∃ f₂ ∈ F, ∃ f₃ ∈ F, f₁.at q = f₂.at q ∧ f₁.at q = f₃.at q ∧
    det (f₁.sub f₂) (f₁.sub f₃) ≠ 0)

theorem det_neg_left (f g : Aff2) : det f.neg g = -(det f g) := by
  simp only [det, Aff2.neg]; grind

theorem det_neg_right (f g : Aff2) : det f g.neg = -(det f g) := by
  simp only [det, Aff2.neg]; grind

/-- A face's candidate is a candidate over the lines. -/
theorem lineCand_of {F Ls : List Aff2} {x q : Rat × Rat} (h : Cand2 F (face Ls x) q) :
    LineCand F Ls q := by
  rcases h with ⟨g₁, h₁, g₂, h₂, e₁, e₂, d⟩ | ⟨f₁, hf₁, f₂, hf₂, g, hg, e, eg, d⟩ | h
  · obtain ⟨L₁, hL₁, r₁⟩ := mem_face h₁
    obtain ⟨L₂, hL₂, r₂⟩ := mem_face h₂
    refine Or.inl ⟨L₁, hL₁, L₂, hL₂, ?_, ?_, ?_⟩
    · rcases r₁ with rfl | rfl
      · exact e₁
      · rw [neg_at] at e₁; grind
    · rcases r₂ with rfl | rfl
      · exact e₂
      · rw [neg_at] at e₂; grind
    · rcases r₁ with rfl | rfl <;> rcases r₂ with rfl | rfl <;>
        (try simp only [det_neg_left, det_neg_right] at d) <;> grind
  · obtain ⟨L, hL, r⟩ := mem_face hg
    refine Or.inr (Or.inl ⟨f₁, hf₁, f₂, hf₂, L, hL, e, ?_, ?_⟩)
    · rcases r with rfl | rfl
      · exact eg
      · rw [neg_at] at eg; grind
    · rcases r with rfl | rfl
      · exact d
      · rw [det_neg_right] at d; grind
  · exact Or.inr (Or.inr h)

/-- **Completeness.** Every point `x` of a polygonal section whose closed faces are bounded, with
`k` the least cut there, is matched in the reach by a candidate over the lines lying in `x`'s
closed face. -/
theorem section_max (F Ls : List Aff2) {x : Rat × Rat}
    (bounded : ∀ δ : Rat × Rat, δ ≠ (0, 0) → ∃ g ∈ face Ls x, g.slope δ < 0)
    {k : Aff2} (hk : k ∈ F) (hmin : ∀ f ∈ F, k.at x ≤ f.at x) :
    ∃ q, (∀ g ∈ face Ls x, 0 ≤ g.at q) ∧ (∀ f ∈ F, k.at x ≤ f.at q) ∧ LineCand F Ls q := by
  obtain ⟨q, hq, hv, hc⟩ := cell_max F (face Ls x) bounded (face_self Ls x) hk hmin
  exact ⟨q, hq, hv, lineCand_of hc⟩

/-- **Soundness.** A point `q` of the closed face of a section point `x` is a limit of section
points: every point strictly between `q` and `x` has `x`'s signs, so is in the section. -/
theorem face_sound {S : Rat × Rat → Prop} {Ls : List Aff2} (hS : Polygonal S Ls) {x q : Rat × Rat}
    (hx : S x) (hq : ∀ g ∈ face Ls x, 0 ≤ g.at q) {t : Rat} (ht0 : 0 < t) (ht1 : t ≤ 1) :
    S (along q (x.1 - q.1, x.2 - q.2) t) := by
  refine (hS x _ fun L hL => ?_).mp hx
  -- the point is (1 - t) q + t x: L there is (1 - t) L(q) + t L(x)
  have e : L.at (along q (x.1 - q.1, x.2 - q.2) t) = (1 - t) * L.at q + t * L.at x := by
    simp only [Aff2.at, along]; grind
  rw [e]
  have mem : ∀ g, g ∈ face Ls x → 0 ≤ g.at q := hq
  by_cases hp : 0 < L.at x
  · have hL' : L ∈ face Ls x := by
      simp only [face, List.mem_flatMap]; exact ⟨L, hL, by simp [hp]⟩
    have := mem L hL'
    have a : 0 ≤ (1 - t) * L.at q := Rat.mul_nonneg (by grind) this
    have b : 0 < t * L.at x := Rat.mul_pos ht0 hp
    simp only [sgn]; grind
  · by_cases hn : L.at x < 0
    · have hL' : L.neg ∈ face Ls x := by
        simp only [face, List.mem_flatMap]; exact ⟨L, hL, by simp [hp, hn]⟩
      have := mem L.neg hL'; rw [neg_at] at this
      have a : 0 ≤ (1 - t) * -(L.at q) := Rat.mul_nonneg (by grind) this
      have b : 0 < t * -(L.at x) := Rat.mul_pos ht0 (by grind)
      simp only [sgn]; grind
    · have z : L.at x = 0 := by grind
      have h1 : L ∈ face Ls x := by
        simp only [face, List.mem_flatMap]; exact ⟨L, hL, by simp [hp, hn]⟩
      have h2 : L.neg ∈ face Ls x := by
        simp only [face, List.mem_flatMap]; exact ⟨L, hL, by simp [hp, hn]⟩
      have a := mem L h1
      have b := mem L.neg h2; rw [neg_at] at b
      have : L.at q = 0 := by grind
      rw [this, z]; simp only [sgn]; grind

/-- Four lines of a box round the section bound every closed face of a point inside the box. -/
theorem box_bounded (Ls : List Aff2) (lo₁ hi₁ lo₂ hi₂ : Rat) {x : Rat × Rat}
    (h₁ : (⟨-lo₁, 1, 0⟩ : Aff2) ∈ Ls) (h₂ : (⟨hi₁, -1, 0⟩ : Aff2) ∈ Ls)
    (h₃ : (⟨-lo₂, 0, 1⟩ : Aff2) ∈ Ls) (h₄ : (⟨hi₂, 0, -1⟩ : Aff2) ∈ Ls)
    (hx : lo₁ < x.1 ∧ x.1 < hi₁ ∧ lo₂ < x.2 ∧ x.2 < hi₂) :
    ∀ δ : Rat × Rat, δ ≠ (0, 0) → ∃ g ∈ face Ls x, g.slope δ < 0 := by
  intro δ hδ
  have inface : ∀ L ∈ Ls, 0 < L.at x → L ∈ face Ls x := fun L hL hp => by
    simp only [face, List.mem_flatMap]; exact ⟨L, hL, by simp [hp]⟩
  have pos : ∀ c a b : Rat, (⟨c, a, b⟩ : Aff2).at x = c + a * x.1 + b * x.2 := fun _ _ _ => rfl
  by_cases d1 : δ.1 < 0
  · exact ⟨_, inface _ h₁ (by rw [pos]; grind), by simp only [Aff2.slope]; grind⟩
  · by_cases d1' : 0 < δ.1
    · exact ⟨_, inface _ h₂ (by rw [pos]; grind), by simp only [Aff2.slope]; grind⟩
    · by_cases d2 : δ.2 < 0
      · exact ⟨_, inface _ h₃ (by rw [pos]; grind), by simp only [Aff2.slope]; grind⟩
      · have : 0 < δ.2 := by
          apply Classical.byContradiction; intro hn; apply hδ
          ext <;> simp <;> grind
        exact ⟨_, inface _ h₄ (by rw [pos]; grind), by simp only [Aff2.slope]; grind⟩

end ForgeWeld.Extreme
