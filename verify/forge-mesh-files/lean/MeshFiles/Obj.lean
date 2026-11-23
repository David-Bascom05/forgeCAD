import MeshFiles.Text

/-!
# OBJ holds exactly the triangles, and the positions rounded (kernel/forge-mesh/src/obj.rs)

`to_obj` writes, per object, `o NAME`, a `v x y z` per vertex in the object's order, then an
`f i j k` per triangle with the indices counted from 1 across the whole file. Proven, in two
layers:

* `parseLine_render`: each line read back is the line written with its coordinates rounded;
* `readObjs_writeObjs`: the lines read back give every object with its own vertices and exactly
  its triangles (indices back to the object's own, from 0).

Together (`read_write`): an OBJ file reads back as the scene with each position rounded
(`Text.round`) and every triangle the same. The triangles alone decide whether a mesh is closed,
consistently oriented and manifold (`open_edges`, `pinched_vertices`), so the file's mesh is all
three exactly when the written one is.

Normals (`vn`, `f v//vn`) and face groups (`g`) are left out of the model: lines a reader may
skip without changing the mesh. obj.rs `read_obj` numbers each object's vertices in the order its
faces first use them, which is this order for every object `body_objects` makes (it numbers them
so).
-/

namespace MeshFiles

variable {F : Type}

/-- An object as written: its name (whitespace already made `_`), its positions, its triangles
over its own positions from 0. -/
structure Obj (F : Type) where
  name : String
  pos : List (F × F × F)
  tris : List (Nat × Nat × Nat)

inductive Line (F : Type) where
  | o (name : String)
  | v (x y z : F)
  | f (a b c : Nat)

/-- The lines of the objects, each object's indices moved past the vertices before it. -/
def writeObjs : Nat → List (Obj F) → List (Line F)
  | _, [] => []
  | base, ob :: obs =>
    Line.o ob.name :: ob.pos.map (fun p => Line.v p.1 p.2.1 p.2.2) ++
      ob.tris.map (fun t => Line.f (t.1 + base + 1) (t.2.1 + base + 1) (t.2.2 + base + 1)) ++
      writeObjs (base + ob.pos.length) obs

/-- The `v` lines at the head, and what follows them. -/
def takeV : List (Line F) → List (F × F × F) × List (Line F)
  | Line.v x y z :: r => let (vs, rest) := takeV r; ((x, y, z) :: vs, rest)
  | l => ([], l)

def takeF : List (Line F) → List (Nat × Nat × Nat) × List (Line F)
  | Line.f a b c :: r => let (fs, rest) := takeF r; ((a, b, c) :: fs, rest)
  | l => ([], l)

theorem takeV_length (l : List (Line F)) : (takeV l).2.length ≤ l.length := by
  induction l with
  | nil => simp [takeV]
  | cons x r ih => cases x <;> simp [takeV] <;> omega

theorem takeF_length (l : List (Line F)) : (takeF l).2.length ≤ l.length := by
  induction l with
  | nil => simp [takeF]
  | cons x r ih => cases x <;> simp [takeF] <;> omega

/-- obj.rs `read_obj`: an object per `o`, its `v` lines, its `f` lines back over its own
vertices; anything else where an object should start is no OBJ of ours. -/
def readObjs (base : Nat) : List (Line F) → Option (List (Obj F))
  | [] => some []
  | Line.o n :: rest =>
    have h : (takeF (takeV rest).2).2.length < (Line.o n :: rest).length :=
      Nat.lt_of_le_of_lt (Nat.le_trans (takeF_length _) (takeV_length _)) (by simp)
    (readObjs (base + (takeV rest).1.length) (takeF (takeV rest).2).2).map fun obs =>
      ⟨n, (takeV rest).1,
        (takeF (takeV rest).2).1.map fun t => (t.1 - base - 1, t.2.1 - base - 1, t.2.2 - base - 1)⟩
        :: obs
  | _ :: _ => none
termination_by l => l.length

/-- What may follow an object's `v` lines: no `v` line. -/
def NotV : List (Line F) → Prop
  | Line.v .. :: _ => False
  | _ => True

def NotF : List (Line F) → Prop
  | Line.f .. :: _ => False
  | _ => True

theorem takeV_map (ps : List (F × F × F)) (r : List (Line F)) (h : NotV r) :
    takeV (ps.map (fun p => Line.v p.1 p.2.1 p.2.2) ++ r) = (ps, r) := by
  induction ps with
  | nil =>
    simp only [List.map_nil, List.nil_append]
    match r, h with
    | [], _ => rfl
    | Line.o _ :: _, _ => rfl
    | Line.f .. :: _, _ => rfl
  | cons p ps ih => simp [takeV, ih]

theorem takeF_map (ts : List (Nat × Nat × Nat)) (r : List (Line F)) (h : NotF r) :
    takeF (ts.map (fun t => Line.f t.1 t.2.1 t.2.2) ++ r) = (ts, r) := by
  induction ts with
  | nil =>
    simp only [List.map_nil, List.nil_append]
    match r, h with
    | [], _ => rfl
    | Line.o _ :: _, _ => rfl
    | Line.v .. :: _, _ => rfl
  | cons t ts ih => simp [takeF, ih]

theorem writeObjs_head (base : Nat) (obs : List (Obj F)) :
    NotV (writeObjs base obs) ∧ NotF (writeObjs base obs) := by
  cases obs <;> simp [writeObjs, NotV, NotF]

/-- **The OBJ structure claim**: the objects come back, each with its positions and exactly its
triangles over them. -/
theorem readObjs_writeObjs (base : Nat) (obs : List (Obj F)) :
    readObjs base (writeObjs base obs) = some obs := by
  induction obs generalizing base with
  | nil => simp [writeObjs, readObjs]
  | cons ob obs ih =>
    obtain ⟨nv, nf⟩ := writeObjs_head (base + ob.pos.length) obs
    have shifted : ob.tris.map (fun t => Line.f (t.1 + base + 1) (t.2.1 + base + 1)
        (t.2.2 + base + 1)) = (ob.tris.map fun t => (t.1 + base + 1, t.2.1 + base + 1,
        t.2.2 + base + 1)).map (fun t => (Line.f t.1 t.2.1 t.2.2 : Line F)) := by
      simp [List.map_map, Function.comp_def]
    have nvf : NotV (ob.tris.map (fun t => (Line.f (t.1 + base + 1) (t.2.1 + base + 1)
        (t.2.2 + base + 1) : Line F)) ++ writeObjs (base + ob.pos.length) obs) := by
      cases ob.tris with
      | nil => simpa using nv
      | cons _ _ => simp [NotV]
    rw [writeObjs]
    simp only [List.cons_append, List.append_assoc]
    rw [readObjs.eq_def]
    simp only
    rw [takeV_map _ _ nvf, shifted, takeF_map _ _ nf, ih]
    simp only [Option.map_some, Option.some.injEq, List.cons.injEq, and_true]
    have back : ∀ t : Nat × Nat × Nat,
        (t.1 + base + 1 - base - 1, t.2.1 + base + 1 - base - 1, t.2.2 + base + 1 - base - 1) = t :=
      fun t => Prod.ext (by simp only; omega) (Prod.ext (by simp only; omega) (by simp only; omega))
    cases ob
    simp [List.map_map, Function.comp_def, back]

/-! ## The text: each line as its tokens -/

def render (t : Text F) : Line F → List String
  | Line.o n => ["o", n]
  | Line.v x y z => ["v", t.write x, t.write y, t.write z]
  | Line.f a b c => ["f", t.writeN a, t.writeN b, t.writeN c]

def parseLine (t : Text F) : List String → Option (Line F)
  | [k, n] => if k = "o" then some (Line.o n) else none
  | [k, x, y, z] =>
    if k = "v" then
      match t.parse x, t.parse y, t.parse z with
      | some x, some y, some z => some (Line.v x y z)
      | _, _, _ => none
    else if k = "f" then
      match t.parseN x, t.parseN y, t.parseN z with
      | some a, some b, some c => some (Line.f a b c)
      | _, _, _ => none
    else none
  | _ => none

def roundLine (t : Text F) : Line F → Line F
  | Line.v x y z => Line.v (t.round x) (t.round y) (t.round z)
  | l => l

def roundObj (t : Text F) (ob : Obj F) : Obj F :=
  { ob with pos := ob.pos.map fun p => (t.round p.1, t.round p.2.1, t.round p.2.2) }

theorem parseLine_render (t : Text F) (l : Line F) :
    parseLine t (render t l) = some (roundLine t l) := by
  cases l <;> simp [render, parseLine, roundLine, t.parse_write, t.parseN_writeN]

theorem parse_rendered (t : Text F) (l : List (Line F)) :
    (l.map (render t)).mapM (parseLine t) = some (l.map (roundLine t)) := by
  induction l with
  | nil => rfl
  | cons x l ih => simp [List.mapM_cons, parseLine_render, ih]

theorem roundLine_writeObjs (t : Text F) (base : Nat) (obs : List (Obj F)) :
    (writeObjs base obs).map (roundLine t) = writeObjs base (obs.map (roundObj t)) := by
  induction obs generalizing base with
  | nil => rfl
  | cons ob obs ih =>
    simp [writeObjs, roundObj, roundLine, ih, List.map_map, Function.comp_def]

/-- obj.rs `to_obj` as text: each line's tokens. -/
def writeText (t : Text F) (obs : List (Obj F)) : List (List String) :=
  (writeObjs 0 obs).map (render t)

def readText (t : Text F) (lines : List (List String)) : Option (List (Obj F)) :=
  (lines.mapM (parseLine t)).bind (readObjs 0)

/-- **The OBJ claim**: the file reads back as the objects with every position rounded as `num`
rounds it, and every triangle exactly as written. -/
theorem read_write (t : Text F) (obs : List (Obj F)) :
    readText t (writeText t obs) = some (obs.map (roundObj t)) := by
  simp only [readText, writeText, parse_rendered, Option.bind_some, roundLine_writeObjs,
    readObjs_writeObjs]

/-- The triangles are the written ones exactly: whatever decides closed, oriented and manifold
(the triangles alone) decides it the same for the file. -/
theorem read_write_tris (t : Text F) (obs : List (Obj F)) :
    (readText t (writeText t obs)).map (·.map Obj.tris) = some (obs.map Obj.tris) := by
  rw [read_write]; simp [roundObj, List.map_map, Function.comp_def]

end MeshFiles
