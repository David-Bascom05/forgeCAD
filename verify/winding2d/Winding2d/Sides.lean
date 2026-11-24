import Winding2d.Polygon

/-!
# The two sides of an edge of a simple polygon

For a side `a → b`, `hdir a b` is an axis direction pointing to its left (`omega_pos`). A probe is
the point `lerp a b t + κ·ς·hdir` beside the side, left for `ς = 1`, right for `ς = -1`. For
small `κ` the winding number there does not depend on `t ∈ (0,1)` nor on `κ`
(`side_value`): it is the side's value `val vs i ς`.
-/

namespace Winding2d

def hdir (a b : Pt) : Pt :=
  if a.y < b.y then ⟨-1, 0⟩ else if b.y < a.y then ⟨1, 0⟩ else if a.x < b.x then ⟨0, 1⟩ else ⟨0, -1⟩

def smul (ς : Rat) (u : Pt) : Pt := ⟨ς * u.x, ς * u.y⟩

/-- `d × u` for the side's direction `d = b - a`. -/
def crs (a b u : Pt) : Rat := (b.x - a.x) * u.y - (b.y - a.y) * u.x

theorem omega_pos {a b : Pt} (hab : a ≠ b) : 0 < crs a b (hdir a b) := by
  unfold crs hdir
  by_cases h1 : a.y < b.y
  · simp [h1]; grind
  · by_cases h2 : b.y < a.y
    · simp [h1, h2]; grind
    · by_cases h3 : a.x < b.x
      · simp [h1, h2, h3]; grind
      · simp [h1, h2, h3]
        have : a.x ≠ b.x := fun h => hab (by ext <;> grind)
        grind

def probe (a b : Pt) (t ς κ : Rat) : Pt := addv (lerp a b t) κ (smul ς (hdir a b))

theorem orient_addv (a b z u : Pt) (κ : Rat) : orient a b (addv z κ u) = orient a b z + κ * crs a b u := by
  simp only [orient, addv, crs]; grind

theorem crs_smul (a b u : Pt) (ς : Rat) : crs a b (smul ς u) = ς * crs a b u := by
  simp only [crs, smul]; grind

theorem orient_probe (a b : Pt) (t ς κ : Rat) :
    orient a b (probe a b t ς κ) = κ * (ς * crs a b (hdir a b)) := by
  rw [probe, orient_addv, orient_lerp_self, crs_smul]; grind

/-- Two points strictly on one side (`ς`) of the line `PQ`, two on the other or on it: the
segments between them do not meet. -/
theorem sep {P Q p p' a b : Pt} {ς : Rat} (h1 : 0 < ς * orient P Q p) (h2 : 0 < ς * orient P Q p')
    (h3 : ς * orient P Q a ≤ 0) (h4 : ς * orient P Q b ≤ 0) : ¬Meets p p' a b := by
  rintro ⟨z, ⟨t, t0, t1, rfl⟩, ⟨s, s0, s1, hz⟩⟩
  have e1 := orient_lerp P Q p p' t
  have e2 := orient_lerp P Q a b s
  rw [hz] at e1
  have k1 : ς * orient P Q (lerp a b s) = (1 - t) * (ς * orient P Q p) + t * (ς * orient P Q p') := by
    rw [e1]; grind
  have k2 : ς * orient P Q (lerp a b s) = (1 - s) * (ς * orient P Q a) + s * (ς * orient P Q b) := by
    rw [e2]; grind
  have := conv_pos h1 h2 t0 t1
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - s) (by grind : (0:Rat) ≤ -(ς * orient P Q a))
  have := Rat.mul_nonneg s0 (by grind : (0:Rat) ≤ -(ς * orient P Q b))
  grind

def Sgn (ς : Rat) : Prop := ς = 1 ∨ ς = -1

theorem sgn_sq {ς : Rat} (h : Sgn ς) : ς * ς = 1 := by rcases h with rfl | rfl <;> grind

theorem probe_side {a b : Pt} {t ς κ : Rat} (hab : a ≠ b) (hς : Sgn ς) (hκ : 0 < κ) :
    0 < ς * orient a b (probe a b t ς κ) := by
  rw [orient_probe]
  have := Rat.mul_pos hκ (omega_pos hab)
  have e : ς * (κ * (ς * crs a b (hdir a b))) = (ς * ς) * (κ * crs a b (hdir a b)) := by grind
  rw [e, sgn_sq hς]; grind

theorem probe_eq (a b : Pt) (t ς κ : Rat) :
    probe a b t ς κ = addv (lerp a b t) κ (smul ς (hdir a b)) := rfl

theorem addv_zero (p : Pt) (κ : Rat) : addv p κ ⟨0, 0⟩ = p := by ext <;> simp only [addv] <;> grind

theorem ev_forall_lt (n : Nat) (P : Nat → Rat → Prop) (h : ∀ j < n, Ev (P j)) :
    Ev fun κ => ∀ j < n, P j κ :=
  (Ev.all (fun j κ => j < n → P j κ) (List.range n) (fun j hj =>
    (h j (by simpa using hj)).mono fun _ _ h _ => h)).mono fun _ _ h j hj => h j (by simpa using hj) hj

section simple
variable {vs : List Pt} (hs : Simple vs)
include hs

/-- Two probes on one side of side `i` (`i < n`): for small `κ` the segment between them misses
every side. -/
theorem ev_probe_seg {i : Nat} (hi : i < vs.length) {t0 t1 ς : Rat} (h0 : 0 < t0) (h1 : t0 < 1)
    (h2 : 0 < t1) (h3 : t1 < 1) (hς : Sgn ς) :
    Ev fun κ => ∀ j < vs.length, ¬Meets (probe (vtx vs i) (vtx vs (i + 1)) t0 ς κ)
      (probe (vtx vs i) (vtx vs (i + 1)) t1 ς κ) (vtx vs j) (vtx vs (j + 1)) := by
  have others := ev_forall_lt vs.length (fun j κ => j ≠ i → ¬Meets
      (probe (vtx vs i) (vtx vs (i + 1)) t0 ς κ) (probe (vtx vs i) (vtx vs (i + 1)) t1 ς κ)
      (vtx vs j) (vtx vs (j + 1))) (fun j hj => by
    by_cases e : j = i
    · exact Ev.true.mono fun _ _ _ h => absurd e h
    · exact (ev_not_meets (hs.nondeg j) (sub_off hs hi hj (Ne.symm e) h0 h1 h2 h3) _ _).mono
        fun _ _ h _ => h)
  refine others.mono fun κ hκ h j hj => ?_
  by_cases e : j = i
  · subst e
    exact wit_not_meets (by
      have k0 := probe_side (t := t0) (κ := κ) (hs.nondeg j) hς hκ
      have k1 := probe_side (t := t1) (κ := κ) (hs.nondeg j) hς hκ
      unfold Wit; rcases hς with rfl | rfl
      · left; grind
      · right; left; grind)
  · exact h j hj e

/-- **Sliding along a side.** -/
theorem slide {i : Nat} (hi : i < vs.length) {t0 t1 ς : Rat} (h0 : 0 < t0) (h1 : t0 < 1)
    (h2 : 0 < t1) (h3 : t1 < 1) (hς : Sgn ς) :
    Ev fun κ => wind vs (probe (vtx vs i) (vtx vs (i + 1)) t0 ς κ) =
      wind vs (probe (vtx vs i) (vtx vs (i + 1)) t1 ς κ) :=
  (ev_probe_seg hs hi h0 h1 h2 h3 hς).mono fun _ _ h =>
    wind_eq_of_seg vs _ _ ((allMiss_iff vs _ _).mpr h)

theorem const {i : Nat} (hi : i < vs.length) {t ς : Rat} (h0 : 0 < t) (h1 : t < 1) (hς : Sgn ς) :
    ∃ L, Ev fun κ => wind vs (probe (vtx vs i) (vtx vs (i + 1)) t ς κ) = L := by
  have others := ev_forall_lt vs.length (fun j κ => j ≠ i → ¬Meets
      (addv (lerp (vtx vs i) (vtx vs (i + 1)) t) κ ⟨0, 0⟩)
      (addv (lerp (vtx vs i) (vtx vs (i + 1)) t) κ (smul ς (hdir (vtx vs i) (vtx vs (i + 1)))))
      (vtx vs j) (vtx vs (j + 1))) (fun j hj => by
    by_cases e : j = i
    · exact Ev.true.mono fun _ _ _ h => absurd e h
    · exact (ev_not_meets_pt (hs.nondeg j) (interior_off hs hi hj (Ne.symm e) h0 h1) _ _).mono
        fun _ _ h _ => h)
  obtain ⟨k0, hk0, hall⟩ := others
  refine ⟨wind vs (probe (vtx vs i) (vtx vs (i + 1)) t ς k0), k0, hk0, fun κ hκ hκ0 => ?_⟩
  apply wind_eq_of_seg
  rw [allMiss_iff]
  intro j hj
  by_cases e : j = i
  · subst e
    exact wit_not_meets (by
      have k0' := probe_side (t := t) (κ := κ) (hs.nondeg j) hς hκ
      have k1 := probe_side (t := t) (κ := k0) (hs.nondeg j) hς hk0
      unfold Wit; rcases hς with rfl | rfl
      · left; grind
      · right; left; grind)
  · have hm := hall k0 hk0 (by grind) j hj e
    rw [addv_zero] at hm
    intro ⟨z, hz1, hz2⟩
    apply hm
    refine ⟨z, ?_, hz2⟩
    have hd := Rat.div_mul_cancel (a := κ) (Rat.ne_of_gt hk0)
    have hs0 : 0 ≤ κ / k0 := Rat.le_of_lt ((pos_mul_iff hk0).mp (by rw [hd]; exact hκ))
    have hs1 : κ / k0 ≤ 1 := by
      have := (nonneg_mul_iff hk0 (x := 1 - κ / k0)).mp (by grind); grind
    have e : probe (vtx vs i) (vtx vs (i + 1)) t ς κ =
        lerp (lerp (vtx vs i) (vtx vs (i + 1)) t) (probe (vtx vs i) (vtx vs (i + 1)) t ς k0) (κ / k0) := by
      simp only [probe, addv, lerp, smul]; ext <;> simp only <;> grind
    rw [e] at hz1
    exact onSeg_sub_right hs0 hs1 hz1

/-- **The value of a side.** -/
theorem side_value {i : Nat} (hi : i < vs.length) {ς : Rat} (hς : Sgn ς) :
    ∃ L, ∀ t, 0 < t → t < 1 → Ev fun κ => wind vs (probe (vtx vs i) (vtx vs (i + 1)) t ς κ) = L := by
  obtain ⟨L, hL⟩ := const hs hi (t := 1 / 2) (by grind) (by grind) hς
  exact ⟨L, fun t h0 h1 => ((slide hs hi h0 h1 (by grind : (0:Rat) < 1 / 2) (by grind) hς).and hL).mono
    fun _ _ ⟨a, b⟩ => by rw [a, b]⟩

end simple

open Classical in
/-- The value of side `i` on side `ς` (left `1`, right `-1`). -/
noncomputable def val (vs : List Pt) (i : Nat) (ς : Rat) : Int :=
  if h : Simple vs ∧ i < vs.length ∧ Sgn ς then Classical.choose (side_value h.1 h.2.1 h.2.2) else 0

theorem val_spec {vs : List Pt} (hs : Simple vs) {i : Nat} (hi : i < vs.length) {ς : Rat} (hς : Sgn ς)
    {t : Rat} (h0 : 0 < t) (h1 : t < 1) :
    Ev fun κ => wind vs (probe (vtx vs i) (vtx vs (i + 1)) t ς κ) = val vs i ς := by
  have h : Simple vs ∧ i < vs.length ∧ Sgn ς := ⟨hs, hi, hς⟩
  unfold val; simp only [h, dite_true]
  exact Classical.choose_spec (side_value h.1 h.2.1 h.2.2) t h0 h1

end Winding2d
