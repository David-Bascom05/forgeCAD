/-!
# `check_against` answers as `check` does

The model of `kernel/forge-topo/src/check.rs`. A body is what `check` reads of it: its edges' and faces'
footprints (every input to their geometric tests) and whatever the whole-body rules read. Nothing is
assumed about what the tests compute: only that each is a function of its footprint.
See `README.md` for what that assumption means in the Rust and how it is tested.
-/

namespace CheckLocal

/-- A body as `check` sees it. `E` and `F` are the footprints of an edge and a face (`same_edge` and
`same_face` compare them), `W` all the rules run over the whole body read. -/
structure Body (E F W : Type) where
  edges : List E
  faces : List F
  whole : W

variable {E F W : Type}

/-- The rules. A test is a function of its footprint alone; that is the one assumption about the code. -/
structure Rules (E F W : Type) where
  edgeOk : E → Bool
  faceOk : F → Bool
  wholeOk : W → Bool

/-- `check` found nothing: `check_known` with nothing known. -/
def full (r : Rules E F W) (b : Body E F W) : Prop :=
  r.wholeOk b.whole = true ∧ (∀ e ∈ b.edges, r.edgeOk e = true) ∧ ∀ f ∈ b.faces, r.faceOk f = true

/-- `check_against` found nothing: the whole-body rules, and the tests of what `same` does not find in
`old`. `same` stands for the comparison the code does, entity by entity. -/
def localOk (r : Rules E F W) (sameE : E → Bool) (sameF : F → Bool) (b : Body E F W) : Prop :=
  r.wholeOk b.whole = true ∧ (∀ e ∈ b.edges, sameE e = false → r.edgeOk e = true) ∧
    ∀ f ∈ b.faces, sameF f = false → r.faceOk f = true

/-- The comparison is sound: it says "same" only for a footprint that `old` has. -/
def Sound {α : Type} (same : α → Bool) (old : List α) : Prop :=
  ∀ x, same x = true → x ∈ old

/-- Old passed and the rest passes: new passes. -/
theorem full_of_local (r : Rules E F W) {old new : Body E F W} {sameE : E → Bool} {sameF : F → Bool}
    (hE : Sound sameE old.edges) (hF : Sound sameF old.faces)
    (ho : full r old) (hl : localOk r sameE sameF new) : full r new := by
  refine ⟨hl.1, ?_, ?_⟩
  · intro e he
    cases h : sameE e with
    | true => exact ho.2.1 e (hE e h)
    | false => exact hl.2.1 e he h
  · intro f hf
    cases h : sameF f with
    | true => exact ho.2.2 f (hF f h)
    | false => exact hl.2.2 f hf h

/-- No false alarms, and this needs neither `old` nor a sound comparison: `check_against` only skips tests. -/
theorem local_of_full (r : Rules E F W) {new : Body E F W} (sameE : E → Bool) (sameF : F → Bool)
    (h : full r new) : localOk r sameE sameF new :=
  ⟨h.1, fun e he _ => h.2.1 e he, fun f hf _ => h.2.2 f hf⟩

/-- The two answers are one, given that old passed. -/
theorem local_iff_full (r : Rules E F W) {old new : Body E F W} {sameE : E → Bool} {sameF : F → Bool}
    (hE : Sound sameE old.edges) (hF : Sound sameF old.faces) (ho : full r old) :
    localOk r sameE sameF new ↔ full r new :=
  ⟨full_of_local r hE hF ho, local_of_full r sameE sameF⟩

/-- Old is needed: with old failing, a footprint that fails and is shared is not found. This is the
hypothesis regeneration does not establish by itself (see README). -/
theorem old_is_needed :
    ∃ (r : Rules Nat Nat Nat) (old new : Body Nat Nat Nat) (sameE sameF : Nat → Bool),
      Sound sameE old.edges ∧ Sound sameF old.faces ∧ localOk r sameE sameF new ∧ ¬ full r new := by
  refine ⟨⟨fun _ => false, fun _ => true, fun _ => true⟩, ⟨[0], [], 0⟩, ⟨[0], [], 0⟩,
    fun e => e == 0, fun _ => false, ?_, ?_, ?_, ?_⟩
  · intro x h
    simp at h
    simp [h]
  · intro _ h
    simp at h
  · refine ⟨rfl, ?_, ?_⟩
    · intro e he hs
      simp at he
      simp [he] at hs
    · intro _ hf
      simp at hf
  · intro h
    have := h.2.1 0 (by simp)
    simp at this

/-! ## A test of a pair of faces: `selfx::crossing` -/

/-- No two faces of a list fail a test of the pair. -/
def apart {α : Type} (ok : α → α → Bool) (l : List α) : Prop :=
  ∀ x ∈ l, ∀ y ∈ l, ok x y = true

/-- `crossing` with `flagged`: only pairs with a flagged face. -/
def apartFlagged {α : Type} (ok : α → α → Bool) (flagged : α → Bool) (l : List α) : Prop :=
  ∀ x ∈ l, ∀ y ∈ l, (flagged x = true ∨ flagged y = true) → ok x y = true

/-- Only a pair with a face not found in old can newly cross; a flag that holds every changed face is enough. -/
theorem apart_of_flagged {α : Type} (ok : α → α → Bool) {old new : List α} {flagged : α → Bool}
    (hf : ∀ x, flagged x = false → x ∈ old) (ho : apart ok old) (hn : apartFlagged ok flagged new) :
    apart ok new := by
  intro x hx y hy
  cases fx : flagged x with
  | true => exact hn x hx y hy (Or.inl fx)
  | false =>
    cases fy : flagged y with
    | true => exact hn x hx y hy (Or.inr fy)
    | false => exact ho x (hf x fx) y (hf y fy)

/-- What a pair test may skip by boxes: faces lying in disjoint boxes share no point. `P` is any space of points. -/
theorem boxes_apart {P : Type} (inA inB : P → Prop) (boxA boxB : P → Prop)
    (ha : ∀ p, inA p → boxA p) (hb : ∀ p, inB p → boxB p) (hd : ∀ p, boxA p → boxB p → False) :
    ∀ p, inA p → inB p → False :=
  fun p pa pb => hd p (ha p pa) (hb p pb)

/-! ## Within a plane face: a kept loop's side test -/

/-- Even-odd over a list of loops at one probe: `inLoop l` says whether the probe is inside loop `l`. -/
def parity {L : Type} (inLoop : L → Bool) (ls : List L) : Bool :=
  ls.foldr (fun l acc => xor (inLoop l) acc) false

theorem parity_append {L : Type} (inLoop : L → Bool) (a b : List L) :
    parity inLoop (a ++ b) = xor (parity inLoop a) (parity inLoop b) := by
  induction a with
  | nil => simp [parity]
  | cons x xs ih =>
    simp only [parity, List.cons_append, List.foldr_cons] at ih ⊢
    rw [ih, Bool.xor_assoc]

theorem parity_far {L : Type} (inLoop : L → Bool) (ls : List L) (h : ∀ l ∈ ls, inLoop l = false) :
    parity inLoop ls = false := by
  induction ls with
  | nil => rfl
  | cons x xs ih =>
    have hx := h x (by simp)
    have hr := ih (fun l hl => h l (by simp [hl]))
    simp only [parity, List.foldr_cons] at hr ⊢
    rw [hx, hr]
    rfl

/-- A loop whose box a probe is outside of does not hold the probe, so loops new and gone away from it
leave its even-odd answer as it was: `check_sides`' test of a coedge of a kept loop. The rest of what that
test reads (the edge, the surface, the step) is the same by `quiet_loops` and the gap of 0.4: see README. -/
theorem kept_loop_same_answer {L : Type} (inLoop : L → Bool) (kept fresh gone : List L)
    (hf : ∀ l ∈ fresh, inLoop l = false) (hg : ∀ l ∈ gone, inLoop l = false) :
    parity inLoop (kept ++ fresh) = parity inLoop (kept ++ gone) := by
  rw [parity_append, parity_append, parity_far inLoop fresh hf, parity_far inLoop gone hg]

/-! ## Shells: where material can change -/

/-- Material is told by parity along a ray: the hits are the ray's points on the boundary. One assumption about
geometry, Jordan-Brouwer for a closed shell: any ray from the point gives the same answer, so one may be chosen. -/
def material {P : Type} (onB : P → Bool) (ray : List P) : Bool :=
  (ray.filter onB).length % 2 == 1

/-- Boundaries that differ only inside region `R`, and a ray that stays out of `R`: the same answer. So a point from
which such a ray leaves is in the same material before and after: `asked` in `check_shells`. -/
theorem material_same_outside {P : Type} (R : P → Prop)
    (onOld onNew : P → Bool) (ray : List P)
    (hb : ∀ q, ¬ R q → onOld q = onNew q) (hr : ∀ q ∈ ray, ¬ R q) :
    material onOld ray = material onNew ray := by
  have : ray.filter onOld = ray.filter onNew :=
    List.filter_congr (fun q hq => hb q (hr q hq))
  simp [material, this]

/-- A point past a box along one axis has a ray that stays clear of it: along that axis, away. -/
theorem ray_clear_of_box (hi x t : Int) (h : hi < x) (ht : 0 ≤ t) : hi < x + t := by
  omega

/-- A probe outside a shell's box: a ray staying clear of the box meets none of the shell, so it is outside that
shell's material. So an outer skin's face clear of every other shell's box needs no ray at all (`clear` in `check_shells`). -/
theorem outside_box_no_material {P : Type} (onB : P → Bool) (ray : List P)
    (h : ∀ q ∈ ray, onB q = false) : material onB ray = false := by
  have : ray.filter onB = [] := List.filter_eq_nil_iff.mpr (fun q hq => by simp [h q hq])
  simp [material, this]

/-! ## Every rule of `check`, and how it is covered -/

/-- One constructor per `Problem` of `check.rs`. -/
inductive Rule where
  | edgeUses | edgeUsesSameWay | edgeDegenerate
  | faceNoLoops | loopEmpty | loopBroken
  | vertexOffEdge | vertexTolerance | edgeOffFace
  | outerNotFirst | faceWrongSide | loopsCross
  | shellsOverlap
  | duplicateName | euler | shellEuler | vertexFans
  deriving DecidableEq, Repr

/-- How `check_against` covers a rule: tested edge by edge or face by face from a footprint (`full_of_local`,
and within a plane face `kept_loop_same_answer`), run near what changed (`material_same_outside`), or run over
the whole new body as `check` runs it (`wholeOk`, nothing to prove). -/
inductive Cover where
  | edgeFootprint | faceFootprint | nearChange | wholeBody
  deriving DecidableEq, Repr

/-- Exhaustive by the match: a new rule does not compile until it says how it is covered. -/
def cover : Rule → Cover
  | .vertexOffEdge | .vertexTolerance | .edgeOffFace => .edgeFootprint
  | .outerNotFirst | .faceWrongSide | .loopsCross => .faceFootprint
  | .edgeUses | .edgeUsesSameWay | .edgeDegenerate
  | .faceNoLoops | .loopEmpty | .loopBroken
  | .duplicateName | .euler | .shellEuler | .vertexFans => .wholeBody
  | .shellsOverlap => .nearChange

end CheckLocal
