import ForgeBoolCore.Specs

/-!
# A Lean ledger as a Rust ledger (`EncodeSpec`)

The cells are the distinct covectors of the pieces, the facets' sides, the inside entries, the edges
and the verts (`allC`); every covector is replaced by its index there (`idOf`). Within `LFits`
everything fits `u32`, and the Rust ledger reads back as the Lean one (`toL_encode`), is
well-formed, and lists every side.
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce (Cov)

namespace forge_bool_core

def ofS : ForgeExact.DecideOnce.Sgn → Sgn
  | .neg => .Neg
  | .zero => .Zero
  | .pos => .Pos

theorem toS_ofS (x : ForgeExact.DecideOnce.Sgn) : toS (ofS x) = x := by cases x <;> rfl

theorem map_toS_ofS (X : Cov) : (X.map ofS).map toS = X := by
  rw [List.map_map]; conv_rhs => rw [← List.map_id X]
  apply List.map_congr_left; intro x _; exact toS_ofS x

/-- `n` as a `u32` (0 past the range, which `LFits` rules out). -/
def u32 (n : Nat) : U32 := if h : n < 2 ^ 32 then UScalar.ofNatCore n (by simpa using h) else 0#u32

theorem u32_val {n : Nat} (h : n < U32.max) : (u32 n).val = n := by
  have : n < 2 ^ 32 := by have := U32.max_eq; omega
  rw [u32, dif_pos this]; exact UScalar.ofNatCore_val_eq _

/-- A list as a `Vec` (empty past `usize`, which `LFits` rules out). -/
def vec {α : Type} (l : List α) : alloc.vec.Vec α :=
  if h : l.length ≤ Usize.max then alloc.vec.Vec.from l h else alloc.vec.Vec.new α

theorem vec_val {α : Type} {l : List α} (h : l.length < U32.max) : (vec l).val = l := by
  have : l.length ≤ Usize.max := by have := u32_le_usize; omega
  rw [vec, dif_pos this, alloc.vec.Vec.from_val]

section
variable (L : ForgeExact.DecideOnce.Ledger)

/-- The cells: every covector the ledger mentions, sides included, once. -/
def allC : List Cov :=
  ForgeExact.DecideOnce.dedup (L.pieces.map Prod.fst ++ L.facets.flatMap (fun φ => [φ.plus, φ.minus]) ++
    L.inside.map Prod.fst ++ L.edges ++ L.verts)

def idOf (X : Cov) : Nat := (allC L).idxOf X

def encode : Ledger where
  nb := u32 L.nb
  face_body := vec (L.faceBody.map u32)
  face_class := vec (L.faceCls.map u32)
  cells := vec ((allC L).map fun X => vec (X.map ofS))
  pieces := vec (L.pieces.map fun p => ((u32 (idOf L p.1) : CellId), (u32 p.2 : FaceId)))
  inside := vec (L.inside.map fun q => ((u32 (idOf L q.1) : CellId), (u32 q.2 : BodyId)))
  edges := vec (L.edges.map fun X => (u32 (idOf L X) : CellId))
  verts := vec (L.verts.map fun X => (u32 (idOf L X) : CellId))

end

section
variable {L : ForgeExact.DecideOnce.Ledger}

theorem facets_length_le : L.facets.length ≤ L.pieces.length := by
  unfold ForgeExact.DecideOnce.Ledger.facets
  exact (ForgeBoolCore.Dedup.dedup_length_le _).trans (by simp)

theorem allC_length : (allC L).length ≤ 3 * L.pieces.length + L.inside.length + L.edges.length + L.verts.length := by
  unfold allC
  refine (ForgeBoolCore.Dedup.dedup_length_le _).trans ?_
  have : (L.facets.flatMap fun φ => [φ.plus, φ.minus]).length = 2 * L.facets.length := by
    induction L.facets with
    | nil => simp
    | cons x t ih => simp [List.flatMap_cons] at ih ⊢; (try omega)
  have := facets_length_le (L := L)
  simp only [List.length_append, List.length_map]; omega

theorem allC_small (h : LFits L) : (allC L).length < U32.max := by
  have := allC_length (L := L); have := h.size; omega

theorem mem_allC {X : Cov} : X ∈ allC L ↔
    X ∈ L.pieces.map Prod.fst ∨ (∃ φ ∈ L.facets, X = φ.plus ∨ X = φ.minus) ∨ X ∈ L.inside.map Prod.fst ∨
      X ∈ L.edges ∨ X ∈ L.verts := by
  unfold allC
  rw [ForgeExact.DecideOnce.mem_dedup]
  simp only [List.mem_append, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
  constructor
  · rintro ((((h | ⟨φ, hφ, h⟩) | h) | h) | h)
    · exact Or.inl h
    · exact Or.inr (Or.inl ⟨φ, hφ, h⟩)
    · exact Or.inr (Or.inr (Or.inl h))
    · exact Or.inr (Or.inr (Or.inr (Or.inl h)))
    · exact Or.inr (Or.inr (Or.inr (Or.inr h)))
  · rintro (h | ⟨φ, hφ, h⟩ | h | h | h)
    · exact Or.inl (Or.inl (Or.inl (Or.inl h)))
    · exact Or.inl (Or.inl (Or.inl (Or.inr ⟨φ, hφ, h⟩)))
    · exact Or.inl (Or.inl (Or.inr h))
    · exact Or.inl (Or.inr h)
    · exact Or.inr h

theorem allC_width (h : LFits L) : ∀ X ∈ allC L, X.length < U32.max := by
  intro X hX
  rcases mem_allC.mp hX with h1 | ⟨φ, hφ, h2⟩ | h1 | h1 | h1
  · exact h.width X (by simp only [List.mem_append]; exact Or.inl (Or.inl (Or.inl h1)))
  · have hφp : φ ∈ L.pieces.map Prod.fst := by
      unfold ForgeExact.DecideOnce.Ledger.facets at hφ; exact (ForgeExact.DecideOnce.mem_dedup _ _).mp hφ
    have hw := h.width φ (by simp only [List.mem_append]; exact Or.inl (Or.inl (Or.inl hφp)))
    rcases h2 with e | e <;> rw [e] <;>
      simp [ForgeExact.DecideOnce.Cov.plus, ForgeExact.DecideOnce.Cov.minus, hw]
  · exact h.width X (by simp only [List.mem_append]; exact Or.inl (Or.inl (Or.inr h1)))
  · exact h.width X (by simp only [List.mem_append]; exact Or.inl (Or.inr h1))
  · exact h.width X (by simp only [List.mem_append]; exact Or.inr h1)

theorem idOf_lt {X : Cov} (hX : X ∈ allC L) : idOf L X < (allC L).length := List.idxOf_lt_length_iff.mpr hX

theorem cells_encode (h : LFits L) : (encode L).cells.val = (allC L).map fun X => vec (X.map ofS) := by
  show (vec _).val = _
  exact vec_val (by simpa using allC_small h)

theorem cellL_encode (h : LFits L) {X : Cov} (hX : X ∈ allC L) : cellL (encode L) (idOf L X) = X := by
  unfold cellL
  rw [cells_encode h, List.getD_eq_getElem?_getD, List.getElem?_map,
    List.getElem?_eq_getElem (idOf_lt hX), Option.map_some, Option.getD_some]
  have hx : (allC L)[idOf L X]'(idOf_lt hX) = X := List.getElem_idxOf _
  rw [hx, vec_val (by simpa using allC_width h X hX), map_toS_ofS]

end

theorem toL_encode {L : ForgeExact.DecideOnce.Ledger} (h : LFits L) : toL (encode L) = L := by
  have hsz := h.size
  have hsmall := allC_small h
  have hp1 : ∀ p ∈ L.pieces, p.1 ∈ allC L := fun p hp => mem_allC.mpr (Or.inl (List.mem_map_of_mem hp))
  have hi1 : ∀ q ∈ L.inside, q.1 ∈ allC L :=
    fun q hq => mem_allC.mpr (Or.inr (Or.inr (Or.inl (List.mem_map_of_mem hq))))
  have he1 : ∀ X ∈ L.edges, X ∈ allC L := fun X hX => mem_allC.mpr (Or.inr (Or.inr (Or.inr (Or.inl hX))))
  have hv1 : ∀ X ∈ L.verts, X ∈ allC L := fun X hX => mem_allC.mpr (Or.inr (Or.inr (Or.inr (Or.inr hX))))
  have hid : ∀ X ∈ allC L, (u32 (idOf L X)).val = idOf L X := fun X hX => u32_val (by have := idOf_lt hX; omega)
  obtain ⟨hnb0, hnb⟩ := h.nb
  obtain ⟨hfbl, hfcl, hfb, hfc⟩ := h.faces
  obtain ⟨hps, hins⟩ := h.ids
  unfold toL
  congr 1
  · exact u32_val hnb
  · show (vec _).val.map _ = _
    rw [vec_val (by simpa using hfbl), List.map_map]; conv_rhs => rw [← List.map_id L.faceBody]
    apply List.map_congr_left; intro b hb; exact u32_val (hfb b hb)
  · show (vec _).val.map _ = _
    rw [vec_val (by simpa using hfcl), List.map_map]; conv_rhs => rw [← List.map_id L.faceCls]
    apply List.map_congr_left; intro c hc; exact u32_val (hfc c hc)
  · unfold pcs
    show ((vec _).val.map _).map _ = _
    rw [vec_val (by simp; omega), List.map_map, List.map_map]; conv_rhs => rw [← List.map_id L.pieces]
    apply List.map_congr_left; intro p hp
    simp only [Function.comp, id]
    rw [hid _ (hp1 p hp), cellL_encode h (hp1 p hp), u32_val (hps p hp)]
  · show (vec _).val.map _ = _
    rw [vec_val (by simp; omega), List.map_map]; conv_rhs => rw [← List.map_id L.inside]
    apply List.map_congr_left; intro q hq
    simp only [Function.comp, id]
    rw [hid _ (hi1 q hq), cellL_encode h (hi1 q hq), u32_val (hins q hq)]
  · show (vec _).val.map _ = _
    rw [vec_val (by simp; omega), List.map_map]; conv_rhs => rw [← List.map_id L.edges]
    apply List.map_congr_left; intro X hX
    simp only [Function.comp, id]
    rw [hid _ (he1 X hX), cellL_encode h (he1 X hX)]
  · show (vec _).val.map _ = _
    rw [vec_val (by simp; omega), List.map_map]; conv_rhs => rw [← List.map_id L.verts]
    apply List.map_congr_left; intro X hX
    simp only [Function.comp, id]
    rw [hid _ (hv1 X hX), cellL_encode h (hv1 X hX)]

theorem cells_read {L : ForgeExact.DecideOnce.Ledger} (h : LFits L) :
    (encode L).cells.val.map (fun c => c.val.map toS) = allC L := by
  rw [cells_encode h, List.map_map]; conv_rhs => rw [← List.map_id (allC L)]
  apply List.map_congr_left; intro X hX
  simp only [Function.comp, id]; rw [vec_val (by simpa using allC_width h X hX), map_toS_ofS]

theorem encode_wf {L : ForgeExact.DecideOnce.Ledger} (h : LFits L) : LedgerWF (encode L) := by
  have hsz := h.size
  have hsmall := allC_small h
  have hlen := allC_length (L := L)
  have hcl : (encode L).cells.val.length = (allC L).length := by rw [cells_encode h]; simp
  have hidlt : ∀ X ∈ allC L, (u32 (idOf L X)).val < (encode L).cells.val.length := fun X hX => by
    rw [hcl, u32_val (by have := idOf_lt hX; omega)]; exact idOf_lt hX
  obtain ⟨hps, hins⟩ := h.ids
  have hUs := u32_le_usize
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · show 0 < (u32 L.nb).val; rw [u32_val h.nb.2]; exact h.nb.1
  · have hinj : ((encode L).cells.val.map (·.val)) = (allC L).map (fun X => X.map ofS) := by
      rw [cells_encode h, List.map_map]
      apply List.map_congr_left; intro X hX
      simp only [Function.comp]; rw [vec_val (by simpa using allC_width h X hX)]
    rw [hinj]
    apply List.Nodup.map _ (ForgeExact.DecideOnce.nodup_dedup _)
    intro X Y hXY
    have := congrArg (List.map toS) hXY
    rwa [map_toS_ofS, map_toS_ofS] at this
  · intro q hq
    simp only [pcs] at hq
    have : (encode L).pieces.val = L.pieces.map fun p => ((u32 (idOf L p.1) : CellId), (u32 p.2 : FaceId)) :=
      vec_val (by simp; omega)
    rw [this, List.map_map, List.mem_map] at hq
    obtain ⟨p, hp, rfl⟩ := hq
    have hp1 : p.1 ∈ allC L := mem_allC.mpr (Or.inl (List.mem_map_of_mem hp))
    exact ⟨hidlt _ hp1, by simp only [Function.comp]; rw [u32_val (hps p hp)]; exact hps p hp⟩
  · intro q hq
    simp only [ipcs] at hq
    have : (encode L).inside.val = L.inside.map fun q => ((u32 (idOf L q.1) : CellId), (u32 q.2 : BodyId)) :=
      vec_val (by simp; omega)
    rw [this, List.map_map, List.mem_map] at hq
    obtain ⟨p, hp, rfl⟩ := hq
    exact hidlt _ (mem_allC.mpr (Or.inr (Or.inr (Or.inl (List.mem_map_of_mem hp)))))
  · intro e he
    have : (encode L).edges.val = L.edges.map fun X => (u32 (idOf L X) : CellId) := vec_val (by simp; omega)
    rw [this, List.mem_map] at he
    obtain ⟨X, hX, rfl⟩ := he
    exact hidlt _ (mem_allC.mpr (Or.inr (Or.inr (Or.inr (Or.inl hX)))))
  · have : (encode L).edges.val = L.edges.map fun X => (u32 (idOf L X) : CellId) := vec_val (by simp; omega)
    unfold idsOf; rw [this, List.map_map]
    have he1 : ∀ X ∈ L.edges, X ∈ allC L := fun X hX => mem_allC.mpr (Or.inr (Or.inr (Or.inr (Or.inl hX))))
    apply List.Nodup.map_on _ h.edges_nodup
    intro X hX Y hY hXY
    simp only [Function.comp] at hXY
    rw [u32_val (by have := idOf_lt (he1 X hX); omega), u32_val (by have := idOf_lt (he1 Y hY); omega)] at hXY
    have e1 : (allC L)[idOf L X]'(idOf_lt (he1 X hX)) = X := List.getElem_idxOf _
    have e2 : (allC L)[idOf L Y]'(idOf_lt (he1 Y hY)) = Y := List.getElem_idxOf _
    rw [← e1, ← e2]; simp only [hXY]
  · intro e he
    have : (encode L).verts.val = L.verts.map fun X => (u32 (idOf L X) : CellId) := vec_val (by simp; omega)
    rw [this, List.mem_map] at he
    obtain ⟨X, hX, rfl⟩ := he
    exact hidlt _ (mem_allC.mpr (Or.inr (Or.inr (Or.inr (Or.inr hX)))))
  · have h1 : (encode L).pieces.val.length = L.pieces.length := by
      show (vec _).val.length = _; rw [vec_val (by simp; omega)]; simp
    rw [hcl, h1]; omega
  · have h1 : (encode L).edges.val.length = L.edges.length := by
      show (vec _).val.length = _; rw [vec_val (by simp; omega)]; simp
    have h2 : (encode L).verts.val.length = L.verts.length := by
      show (vec _).val.length = _; rw [vec_val (by simp; omega)]; simp
    omega
  · have h1 : (encode L).pieces.val.length = L.pieces.length := by
      show (vec _).val.length = _; rw [vec_val (by simp; omega)]; simp
    rw [hcl, h1]; omega

theorem encode_sides {L : ForgeExact.DecideOnce.Ledger} (h : LFits L) : SidesListed (encode L) := by
  intro φ hφ
  rw [toL_encode h] at hφ
  rw [cells_read h]
  exact ⟨mem_allC.mpr (Or.inr (Or.inl ⟨φ, hφ, Or.inl rfl⟩)), mem_allC.mpr (Or.inr (Or.inl ⟨φ, hφ, Or.inr rfl⟩))⟩

theorem encode_spec_proof : EncodeSpec := fun L h => ⟨encode L, toL_encode h, encode_wf h, encode_sides h⟩

end forge_bool_core
