import MeshFiles.Bytes

/-!
# Binary STL holds exactly the triangles written (kernel/forge-mesh/src/stl.rs)

`to_binary_stl` writes an 80-byte header, the triangle count as a little-endian u32, then per
triangle its facet normal and three corners, each three f32s, and two spare bytes: 50 bytes.
`read_stl` reads them back. Proven: for any header of 80 bytes and fewer than 2³² triangles, the
reader gives back exactly the triangles written, corners and normals, in order; given only that
an f32 written as four bytes reads back as itself (`Fixed`, Rust's bit casts).

The corners are the mesh's positions rounded to f32 (`as f32`): the file holds that rounding
exactly. Which corners are the same point, and so the welded mesh's topology, is what an STL
reader recovers by equal coordinates: equal f64 positions give equal f32s, so shared vertices
stay shared; distinct positions that round to one f32 would merge (the tests check the welded
mesh is the same size).
-/

namespace MeshFiles

variable {F : Type}

abbrev P3 (F : Type) := F × F × F

/-- One facet: its normal and its corners, as written. -/
structure Facet (F : Type) where
  n : P3 F
  a : P3 F
  b : P3 F
  c : P3 F

def encP (c : Fixed F 4) (p : P3 F) : List Nat := c.enc p.1 ++ c.enc p.2.1 ++ c.enc p.2.2

def encFacet (c : Fixed F 4) (t : Facet F) : List Nat :=
  encP c t.n ++ encP c t.a ++ encP c t.b ++ encP c t.c ++ [0, 0]

/-- stl.rs `to_binary_stl`: header, count, the facets. -/
def writeStl (c : Fixed F 4) (header : List Nat) (ts : List (Facet F)) : List Nat :=
  header ++ u32le ts.length ++ ts.flatMap (encFacet c)

def decF (c : Fixed F 4) (l : List Nat) (k : Nat) : F := c.dec ((l.drop (4 * k)).take 4)

def decP (c : Fixed F 4) (l : List Nat) (k : Nat) : P3 F :=
  (decF c l k, decF c l (k + 1), decF c l (k + 2))

/-- One 50-byte record. -/
def decFacet (c : Fixed F 4) (l : List Nat) : Facet F :=
  ⟨decP c l 0, decP c l 3, decP c l 6, decP c l 9⟩

def readFacets (c : Fixed F 4) : Nat → List Nat → Option (List (Facet F))
  | 0, [] => some []
  | 0, _ :: _ => none
  | k + 1, l =>
    if l.length < 50 then none
    else (readFacets c k (l.drop 50)).map (decFacet c (l.take 50) :: ·)

/-- stl.rs `read_stl`, binary: the header skipped, the count read, exactly that many records. -/
def readStl (c : Fixed F 4) (bytes : List Nat) : Option (List (Facet F)) :=
  match readU32 (bytes.drop 80) with
  | some (n, rest) => readFacets c n rest
  | none => none

theorem encP_length (c : Fixed F 4) (p : P3 F) : (encP c p).length = 12 := by
  simp [encP, c.len]

theorem encFacet_length (c : Fixed F 4) (t : Facet F) : (encFacet c t).length = 50 := by
  simp [encFacet, encP_length]

theorem decF_encP (c : Fixed F 4) (p : P3 F) (r : List Nat) :
    decF c (encP c p ++ r) 0 = p.1 ∧ decF c (encP c p ++ r) 1 = p.2.1 ∧
      decF c (encP c p ++ r) 2 = p.2.2 := by
  simp only [decF, encP, List.append_assoc]
  refine ⟨?_, ?_, ?_⟩
  · simp [take_of_length (c.len _), c.dec_enc]
  · rw [show 4 * 1 = 4 from rfl, drop_of_length (c.len _), take_of_length (c.len _), c.dec_enc]
  · rw [show 4 * 2 = 4 + 4 from rfl, ← List.drop_drop, drop_of_length (c.len _),
      drop_of_length (c.len _), take_of_length (c.len _), c.dec_enc]

theorem decP_shift (c : Fixed F 4) (l r : List Nat) (k : Nat) (h : l.length = 12) :
    decP c (l ++ r) (k + 3) = decP c r k := by
  simp only [decP, decF]
  have e : ∀ j, 4 * (k + j + 3) = 12 + 4 * (k + j) := fun j => by omega
  refine Prod.ext ?_ (Prod.ext ?_ ?_) <;> simp only
  · rw [show k + 3 = k + 0 + 3 from rfl, e 0, ← List.drop_drop, drop_of_length h]; rfl
  · rw [show k + 3 + 1 = k + 1 + 3 by omega, e 1, ← List.drop_drop, drop_of_length h]
  · rw [show k + 3 + 2 = k + 2 + 3 by omega, e 2, ← List.drop_drop, drop_of_length h]

theorem decP_encP (c : Fixed F 4) (p : P3 F) (r : List Nat) : decP c (encP c p ++ r) 0 = p := by
  obtain ⟨h1, h2, h3⟩ := decF_encP c p r
  simp only [decP, h1, h2, h3]

theorem decFacet_encFacet (c : Fixed F 4) (t : Facet F) : decFacet c (encFacet c t) = t := by
  have l := encP_length c
  simp only [decFacet, encFacet, List.append_assoc]
  congr 1
  · exact decP_encP c _ _
  · rw [decP_shift c _ _ 0 (l _)]; exact decP_encP c _ _
  · rw [show (6 : Nat) = 3 + 3 from rfl, decP_shift c _ _ 3 (l _), decP_shift c _ _ 0 (l _)]
    exact decP_encP c _ _
  · rw [show (9 : Nat) = 6 + 3 from rfl, decP_shift c _ _ 6 (l _),
      show (6 : Nat) = 3 + 3 from rfl, decP_shift c _ _ 3 (l _), decP_shift c _ _ 0 (l _)]
    exact decP_encP c _ _

theorem readFacets_flatMap (c : Fixed F 4) (ts : List (Facet F)) :
    readFacets c ts.length (ts.flatMap (encFacet c)) = some ts := by
  induction ts with
  | nil => rfl
  | cons t ts ih =>
    have hl := encFacet_length c t
    have hlt : ¬ ((encFacet c t ++ ts.flatMap (encFacet c)).length < 50) := by
      simp only [List.length_append, hl]; omega
    simp only [List.length_cons, List.flatMap_cons, readFacets, hlt, ↓reduceIte,
      drop_of_length hl, ih, take_of_length hl, decFacet_encFacet]
    rfl

/-- **The binary STL claim**: any 80-byte header and fewer than 2³² facets read back exactly. -/
theorem readStl_writeStl (c : Fixed F 4) (header : List Nat) (ts : List (Facet F))
    (hh : header.length = 80) (hn : ts.length < 4294967296) :
    readStl c (writeStl c header ts) = some ts := by
  simp only [readStl, writeStl, List.append_assoc, drop_of_length hh, readU32_u32le _ hn]
  exact readFacets_flatMap c ts

/-- And the file's size is what a reader checks it by (stl.rs `binary_len`). -/
theorem writeStl_length (c : Fixed F 4) (header : List Nat) (ts : List (Facet F))
    (hh : header.length = 80) : (writeStl c header ts).length = 84 + 50 * ts.length := by
  simp only [writeStl, List.length_append, hh, u32le_length, List.length_flatMap,
    encFacet_length]
  induction ts with
  | nil => simp
  | cons t ts ih => simp [List.map_cons, List.sum_cons]; omega

end MeshFiles
