import Sheet.Fold

/-!
# Refold of a strip tree: `fold ∘ develop = folded`, by induction (audit2 §2.0, theorem `refold`)

A sheet is a tree of walls joined by bends. A wall is named by its path from the root: the list
of bends crossed to reach it, root first. Each bend carries its numbers (`Bend`) and its two joint
frames, rigid motions of the developed plane (build.rs:1082-1206):

* `ja`: wall `a`'s own coordinates to its joint frame (mold line `x = 0`, the wall at `x ≤ 0`);
* `jb`: wall `b`'s own coordinates to its joint frame (mold line `x' = 0`, the wall at `x' ≥ 0`).

The **folded part** is a placement `M` of every wall (its own coordinates and depth, `(q, -w)`,
to ℝ³) that obeys the joint condition at each bend: the two walls' mold lines coincide and `b` is
turned by `θ` about them (`Bend.J`). Nothing else is assumed of `M`.

The kernel computes, from the flat's numbers alone:
* the **development** `dev`: `dev b = dev a · ja⁻¹ · shift (BA − 2 s) · jb`
  (build.rs:1187 `dev[a].then(ja).then(shift(ba − 2s)).then(jb⁻¹)`, read as maps of points);
* the **fold**: wall `b`'s placement continued from wall `a`'s across the bend, and each zone
  folded round its axis in wall `a`'s frame.

`refold`: the fold of every wall is the folded part's placement, and on every tangent line the
zone's fold and the wall's agree, so `F` is one map on the flat domain.
-/

namespace Sheet

open Real

abbrev V2 := ℝ × ℝ
abbrev V3 := ℝ × ℝ × ℝ

/-- A rigid motion of the developed plane, lifted to ℝ³ (depth untouched). -/
def lift (g : V2 ≃ V2) (p : V3) : V3 := ((g (p.1, p.2.1)).1, (g (p.1, p.2.1)).2, p.2.2)

theorem lift_symm_lift (g : V2 ≃ V2) (p : V3) : lift g.symm (lift g p) = p := by
  simp [lift]

/-- The shift of the developed plane by `d` across the bend. -/
def shift (d : ℝ) : V2 ≃ V2 where
  toFun p := (p.1 + d, p.2)
  invFun p := (p.1 - d, p.2)
  left_inv p := by simp
  right_inv p := by simp

/-- A bend of the tree: its numbers and its two joint frames. -/
structure Edge where
  bend : Bend
  ja : V2 ≃ V2
  jb : V2 ≃ V2

/-- Wall `b`'s placement continued from wall `a`'s across the edge. -/
noncomputable def Edge.step (e : Edge) (Ma : V3 → V3) : V3 → V3 :=
  fun p => Ma (lift e.ja.symm (e.bend.J (lift e.jb p)))

/-- Wall `b`'s development from wall `a`'s: `dev a · ja⁻¹ · shift (BA − 2 s) · jb`. -/
noncomputable def Edge.dstep (e : Edge) (Pa : V2 ≃ V2) : V2 ≃ V2 :=
  (e.jb.trans (shift (e.bend.BA - 2 * e.bend.s))).trans (e.ja.symm.trans Pa)

/-- The fold placement of the wall at `path`, from the root's placement `M0`. -/
noncomputable def foldPlace (M0 : V3 → V3) (path : List Edge) : V3 → V3 :=
  path.foldl (fun M e => e.step M) M0

/-- The development of the wall at `path`, from the root's `P0`. -/
noncomputable def develop (P0 : V2 ≃ V2) (path : List Edge) : V2 ≃ V2 :=
  path.foldl (fun P e => e.dstep P) P0

theorem foldPlace_snoc (M0 : V3 → V3) (path : List Edge) (e : Edge) :
    foldPlace M0 (path ++ [e]) = e.step (foldPlace M0 path) := by
  simp [foldPlace, List.foldl_append]

theorem develop_snoc (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) :
    develop P0 (path ++ [e]) = e.dstep (develop P0 path) := by
  simp [develop, List.foldl_append]

/-- A tree: a set of paths closed under prefixes (every wall's parent is a wall). -/
structure Tree where
  walls : Set (List Edge)
  root : [] ∈ walls
  parent : ∀ p e, p ++ [e] ∈ walls → p ∈ walls

/-- A folded part on the tree: each wall's placement, with the root's `M0`, obeying the joint
condition at every bend. -/
structure Folded (T : Tree) where
  M : List Edge → V3 → V3
  joint : ∀ p e, p ++ [e] ∈ T.walls → M (p ++ [e]) = e.step (M p)

/-- **`refold`, placements**: the fold computed from the flat's numbers is the folded part's
placement, at every wall of the tree. -/
theorem refold_place (T : Tree) (X : Folded T) :
    ∀ path ∈ T.walls, foldPlace (X.M []) path = X.M path := by
  intro path
  induction path using List.reverseRecOn with
  | nil => intro _; rfl
  | append_singleton p e ih =>
      intro h
      rw [foldPlace_snoc, ih (T.parent p e h), X.joint p e h]

/-- The flat-to-folded map on the wall at `path`: undo its development, then place it. -/
noncomputable def Fwall (M0 : V3 → V3) (P0 : V2 ≃ V2) (path : List Edge) (z : V2) (w : ℝ) : V3 :=
  foldPlace M0 path (((develop P0 path).symm z).1, ((develop P0 path).symm z).2, -w)

/-- A flat point's coordinates in the joint frame of `e`, a bend out of the wall at `path`. -/
noncomputable def jointCoords (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) (z : V2) : V2 :=
  e.ja ((develop P0 path).symm z)

/-- The flat-to-folded map on the zone of `e`, a bend out of the wall at `path`: in the joint
frame of wall `a`, the zone point `(x + s, y, w)` folded round its axis, then placed with `a`. -/
noncomputable def Fzone (M0 : V3 → V3) (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) (z : V2)
    (w : ℝ) : V3 :=
  let q := jointCoords P0 path e z
  foldPlace M0 path (lift e.ja.symm (e.bend.zone (q.1 + e.bend.s) q.2 w))

/-- Wall `b`'s own coordinates of a flat point, in its joint frame: wall `a`'s joint coordinates
moved back by `BA − 2 s`. -/
theorem jb_develop (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) (z : V2) :
    e.jb ((develop P0 (path ++ [e])).symm z) =
      ((jointCoords P0 path e z).1 - (e.bend.BA - 2 * e.bend.s), (jointCoords P0 path e z).2) := by
  rw [develop_snoc]
  simp [Edge.dstep, jointCoords, shift, Equiv.trans_apply]

/-- **Glue at `u = 0`**: on wall `a`'s tangent line the zone's fold is wall `a`'s. -/
theorem glue_a (M0 : V3 → V3) (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) (z : V2) (w : ℝ)
    (hz : (jointCoords P0 path e z).1 = -e.bend.s) :
    Fzone M0 P0 path e z w = Fwall M0 P0 path z w := by
  simp only [Fzone, Fwall]
  rw [hz, neg_add_cancel, e.bend.zone_zero]
  congr 1
  simp only [lift]
  rw [← hz]
  simp [jointCoords]

/-- **Glue at `u = BA`**: on wall `b`'s tangent line the zone's fold is wall `b`'s. -/
theorem glue_b (M0 : V3 → V3) (P0 : V2 ≃ V2) (path : List Edge) (e : Edge) (z : V2) (w : ℝ)
    (hz : (jointCoords P0 path e z).1 = e.bend.BA - e.bend.s) :
    Fzone M0 P0 path e z w = Fwall M0 P0 (path ++ [e]) z w := by
  simp only [Fzone, Fwall]
  rw [foldPlace_snoc]
  simp only [Edge.step]
  congr 1
  have hb := jb_develop P0 path e z
  rw [hz] at hb
  rw [hz, show e.bend.BA - e.bend.s + e.bend.s = e.bend.BA by ring, e.bend.zone_BA]
  congr 2
  have h1 : (lift e.jb (((develop P0 (path ++ [e])).symm z).1,
      ((develop P0 (path ++ [e])).symm z).2, -w)) =
      (e.bend.BA - e.bend.s - (e.bend.BA - 2 * e.bend.s), (jointCoords P0 path e z).2, -w) := by
    simp only [lift, Prod.mk.eta, hb]
  rw [h1]
  ext <;> simp; ring

/-- **`refold`** (§2.0 item 1): on every wall of the tree, the flat developed and then folded is
the folded part (`fold ∘ develop = folded`), and on both tangent lines of every bend the zone's
fold agrees with its walls', so the fold is one well-defined map on the flat domain. -/
theorem refold (T : Tree) (X : Folded T) (P0 : V2 ≃ V2) :
    (∀ path ∈ T.walls, ∀ q : V2, ∀ w : ℝ,
      Fwall (X.M []) P0 path (develop P0 path q) w = X.M path (q.1, q.2, -w)) ∧
    (∀ path e, path ++ [e] ∈ T.walls → ∀ z w,
      ((jointCoords P0 path e z).1 = -e.bend.s →
        Fzone (X.M []) P0 path e z w = Fwall (X.M []) P0 path z w) ∧
      ((jointCoords P0 path e z).1 = e.bend.BA - e.bend.s →
        Fzone (X.M []) P0 path e z w = Fwall (X.M []) P0 (path ++ [e]) z w)) := by
  refine ⟨fun path h q w => ?_, fun path e _ z w => ⟨glue_a _ _ _ _ _ _, glue_b _ _ _ _ _ _⟩⟩
  simp only [Fwall, Equiv.symm_apply_apply]
  rw [refold_place T X path h]

end Sheet
