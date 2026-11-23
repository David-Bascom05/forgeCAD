import MeshFiles.Text

/-!
# The 3MF model part holds exactly the scene, positions rounded (kernel/forge-mesh/src/threemf.rs)

`model_xml` writes, per object, `<object id=K name=.. pid pindex>` (K = first + its index), a
`<vertex x y z/>` per vertex in its order, a `<triangle v1 v2 v3/>` per triangle over its own
vertices from 0, `</object>`; then per placement `<item objectid=K transform=.. partnumber=../>`.
Proven:

* `readModel_writeModel`: the elements read back give every object (name, material, vertices,
  exactly its triangles) and every item (which object, its transform, its name);
* `parseEl_render`: each element read back from its attribute values is the element written with
  its coordinates rounded (`Text.round`), so `read_write`: the model part reads back as the scene
  with each position rounded and everything else exact.

Assumed, not proven: XML escaping (`esc`, `unesc`) gives a name back as written; attributes keep
their order (the reader looks them up by name, which is weaker than needed). The ZIP package
around the part, and DEFLATE in it, are outside this model (README: an interim differential).
-/

namespace MeshFiles

variable {F : Type}

structure MObj (F : Type) where
  name : String
  mat : Option Nat
  pos : List (F × F × F)
  tris : List (Nat × Nat × Nat)

structure MItem (F : Type) where
  obj : Nat
  /-- None: the identity, written as no attribute. -/
  m : Option (List F)
  name : String

inductive El (F : Type) where
  | object (id : Nat) (name : String) (mat : Option Nat)
  | vertex (x y z : F)
  | triangle (a b c : Nat)
  | objectEnd
  | item (id : Nat) (m : Option (List F)) (name : String)

def writeObjects (first : Nat) : List (MObj F) → List (El F)
  | [] => []
  | ob :: obs =>
    El.object first ob.name ob.mat :: ob.pos.map (fun p => El.vertex p.1 p.2.1 p.2.2) ++
      ob.tris.map (fun t => El.triangle t.1 t.2.1 t.2.2) ++ El.objectEnd ::
      writeObjects (first + 1) obs

/-- threemf.rs `model_xml`'s resources and build, as elements. -/
def writeModel (first : Nat) (obs : List (MObj F)) (its : List (MItem F)) : List (El F) :=
  writeObjects first obs ++ its.map fun it => El.item (first + it.obj) it.m it.name

def takeVx : List (El F) → List (F × F × F) × List (El F)
  | El.vertex x y z :: r => let (vs, rest) := takeVx r; ((x, y, z) :: vs, rest)
  | l => ([], l)

def takeTri : List (El F) → List (Nat × Nat × Nat) × List (El F)
  | El.triangle a b c :: r => let (fs, rest) := takeTri r; ((a, b, c) :: fs, rest)
  | l => ([], l)

theorem takeVx_length (l : List (El F)) : (takeVx l).2.length ≤ l.length := by
  induction l with
  | nil => simp [takeVx]
  | cons x r ih => cases x <;> simp [takeVx] <;> omega

theorem takeTri_length (l : List (El F)) : (takeTri l).2.length ≤ l.length := by
  induction l with
  | nil => simp [takeTri]
  | cons x r ih => cases x <;> simp [takeTri] <;> omega

/-- The items, each object id back to the object's index (`first` the first id). -/
def readItems (first n : Nat) : List (El F) → Option (List (MItem F))
  | [] => some []
  | El.item id m name :: rest =>
    if first ≤ id ∧ id < first + n then
      (readItems first n rest).map (⟨id - first, m, name⟩ :: ·)
    else none
  | _ :: _ => none

/-- threemf.rs `read_model`: objects in order (each its vertices, triangles, end), then items. -/
def readObjects (first : Nat) (acc : List (MObj F)) : List (El F) →
    Option (List (MObj F) × List (MItem F))
  | El.object id name mat :: rest =>
    match hm : (takeTri (takeVx rest).2).2 with
    | El.objectEnd :: r2 =>
      have : r2.length < (El.object id name mat :: rest).length := by
        have e := congrArg List.length hm
        have l1 := takeTri_length (takeVx rest).2
        have l2 := takeVx_length rest
        simp only [List.length_cons] at e ⊢
        omega
      if id = first + acc.length then
        readObjects first (acc ++ [⟨name, mat, (takeVx rest).1, (takeTri (takeVx rest).2).1⟩]) r2
      else none
    | _ => none
  | l => (readItems first acc.length l).map (acc, ·)
termination_by l => l.length

def readModel (first : Nat) (els : List (El F)) : Option (List (MObj F) × List (MItem F)) :=
  readObjects first [] els

def NotVx : List (El F) → Prop
  | El.vertex .. :: _ => False
  | _ => True

def NotTri : List (El F) → Prop
  | El.triangle .. :: _ => False
  | _ => True

theorem takeVx_map (ps : List (F × F × F)) (r : List (El F)) (h : NotVx r) :
    takeVx (ps.map (fun p => El.vertex p.1 p.2.1 p.2.2) ++ r) = (ps, r) := by
  induction ps with
  | nil =>
    simp only [List.map_nil, List.nil_append]
    match r, h with
    | [], _ => rfl
    | El.object .. :: _, _ => rfl
    | El.triangle .. :: _, _ => rfl
    | El.objectEnd :: _, _ => rfl
    | El.item .. :: _, _ => rfl
  | cons p ps ih => simp [takeVx, ih]

theorem takeTri_map (ts : List (Nat × Nat × Nat)) (r : List (El F)) (h : NotTri r) :
    takeTri (ts.map (fun t => El.triangle t.1 t.2.1 t.2.2) ++ r) = (ts, r) := by
  induction ts with
  | nil =>
    simp only [List.map_nil, List.nil_append]
    match r, h with
    | [], _ => rfl
    | El.object .. :: _, _ => rfl
    | El.vertex .. :: _, _ => rfl
    | El.objectEnd :: _, _ => rfl
    | El.item .. :: _, _ => rfl
  | cons t ts ih => simp [takeTri, ih]

theorem readItems_write (first n : Nat) (its : List (MItem F)) (h : ∀ it ∈ its, it.obj < n) :
    readItems first n (its.map fun it => El.item (first + it.obj) it.m it.name) = some its := by
  induction its with
  | nil => rfl
  | cons it its ih =>
    have hit := h it (by simp)
    simp only [List.map_cons, readItems]
    have hin : first ≤ first + it.obj ∧ first + it.obj < first + n := by omega
    simp only [hin, and_self, ↓reduceIte, ih (fun x hx => h x (by simp [hx]))]
    simp

theorem readObjects_write (first : Nat) (acc obs : List (MObj F)) (its : List (MItem F))
    (h : ∀ it ∈ its, it.obj < acc.length + obs.length) :
    readObjects first acc (writeObjects (first + acc.length) obs ++
      its.map fun it => El.item (first + it.obj) it.m it.name) = some (acc ++ obs, its) := by
  induction obs generalizing acc with
  | nil =>
    simp only [writeObjects, List.nil_append, List.append_nil]
    cases its with
    | nil => simp [readObjects.eq_def, readItems]
    | cons it its =>
      rw [readObjects.eq_def]
      have r := readItems_write first acc.length (it :: its) (by simpa using h)
      simp only [List.map_cons] at r ⊢
      rw [r]
      rfl
  | cons ob obs ih =>
    simp only [writeObjects, List.cons_append, List.append_assoc]
    rw [readObjects.eq_def]
    simp only
    have nvx : NotVx (ob.tris.map (fun t => (El.triangle t.1 t.2.1 t.2.2 : El F)) ++
        El.objectEnd :: (writeObjects (first + acc.length + 1) obs ++
          its.map fun it => El.item (first + it.obj) it.m it.name)) := by
      cases ob.tris <;> simp [NotVx]
    rw [takeVx_map _ _ nvx, takeTri_map _ _ (by simp [NotTri])]
    have := ih (acc ++ [⟨ob.name, ob.mat, ob.pos, ob.tris⟩])
      (by intro it hit; have := h it hit; simp at this ⊢; omega)
    simp only [List.length_append, List.length_singleton, ← Nat.add_assoc] at this
    dsimp only
    simp only [↓reduceIte, this, List.append_assoc, List.singleton_append]

/-- **The 3MF structure claim**: the model's elements read back as every object and every item,
when each item names one of the objects. -/
theorem readModel_writeModel (first : Nat) (obs : List (MObj F)) (its : List (MItem F))
    (h : ∀ it ∈ its, it.obj < obs.length) :
    readModel first (writeModel first obs its) = some (obs, its) := by
  have := readObjects_write first [] obs its (by simpa using h)
  simpa [readModel, writeModel] using this

/-! ## The attribute values: each element as its tag and values -/

/-- `t` writes positions (`num`, 1e-6), `tm` transforms (`matrix_num`, 1e-12). -/
def renderEl (t tm : Text F) : El F → List String
  | El.object id name mat => ["object", t.writeN id, name] ++ (mat.map t.writeN).toList
  | El.vertex x y z => ["vertex", t.write x, t.write y, t.write z]
  | El.triangle a b c => ["triangle", t.writeN a, t.writeN b, t.writeN c]
  | El.objectEnd => ["/object"]
  | El.item id m name => ["item", t.writeN id, name] ++ (m.map fun m => m.map tm.write).getD []

def parseNums (tm : Text F) : List String → Option (List F)
  | [] => some []
  | x :: xs => match tm.parse x, parseNums tm xs with
    | some y, some ys => some (y :: ys)
    | _, _ => none

def parseEl (t tm : Text F) : List String → Option (El F)
  | ["object", id, name] => (t.parseN id).map (El.object · name none)
  | ["object", id, name, mat] =>
    match t.parseN id, t.parseN mat with
    | some id, some mat => some (El.object id name (some mat))
    | _, _ => none
  | ["vertex", x, y, z] =>
    match t.parse x, t.parse y, t.parse z with
    | some x, some y, some z => some (El.vertex x y z)
    | _, _, _ => none
  | ["triangle", a, b, c] =>
    match t.parseN a, t.parseN b, t.parseN c with
    | some a, some b, some c => some (El.triangle a b c)
    | _, _, _ => none
  | ["/object"] => some El.objectEnd
  | "item" :: id :: name :: m =>
    match t.parseN id, m with
    | some id, [] => some (El.item id none name)
    | some id, m => (parseNums tm m).map (El.item id · name)
    | none, _ => none
  | _ => none

def roundEl (t tm : Text F) : El F → El F
  | El.vertex x y z => El.vertex (t.round x) (t.round y) (t.round z)
  | El.item id m name => El.item id (m.map (·.map tm.round)) name
  | e => e

theorem parseNums_write (tm : Text F) (m : List F) :
    parseNums tm (m.map tm.write) = some (m.map tm.round) := by
  induction m with
  | nil => rfl
  | cons x m ih => simp [parseNums, tm.parse_write, ih]

/-- Each element read back is the element written, its numbers rounded. A transform is written
with all twelve entries (`IDENTITY` is written as no transform, `m = none`). -/
theorem parseEl_renderEl (t tm : Text F) (e : El F)
    (hm : ∀ id m name, e = El.item id (some m) name → m ≠ []) :
    parseEl t tm (renderEl t tm e) = some (roundEl t tm e) := by
  cases e with
  | object id name mat => cases mat <;> simp [renderEl, parseEl, roundEl, t.parseN_writeN]
  | vertex x y z => simp [renderEl, parseEl, roundEl, t.parse_write]
  | triangle a b c => simp [renderEl, parseEl, roundEl, t.parseN_writeN]
  | objectEnd => simp [renderEl, parseEl, roundEl]
  | item id m name =>
    cases m with
    | none => simp [renderEl, parseEl, roundEl, t.parseN_writeN]
    | some m =>
      have ne := hm id m name rfl
      cases m with
      | nil => exact absurd rfl ne
      | cons x xs =>
        have p := parseNums_write tm (x :: xs)
        simp only [List.map_cons] at p
        simp [renderEl, parseEl, roundEl, t.parseN_writeN, p]

def roundMObj (t : Text F) (ob : MObj F) : MObj F :=
  { ob with pos := ob.pos.map fun p => (t.round p.1, t.round p.2.1, t.round p.2.2) }

def roundMItem (tm : Text F) (it : MItem F) : MItem F :=
  { it with m := it.m.map (·.map tm.round) }

theorem roundEl_writeObjects (t tm : Text F) (first : Nat) (obs : List (MObj F)) :
    (writeObjects first obs).map (roundEl t tm) = writeObjects first (obs.map (roundMObj t)) := by
  induction obs generalizing first with
  | nil => rfl
  | cons ob obs ih =>
    simp [writeObjects, roundMObj, roundEl, ih, List.map_map, Function.comp_def]

theorem roundEl_writeModel (t tm : Text F) (first : Nat) (obs : List (MObj F))
    (its : List (MItem F)) :
    (writeModel first obs its).map (roundEl t tm) =
      writeModel first (obs.map (roundMObj t)) (its.map (roundMItem tm)) := by
  simp [writeModel, roundEl_writeObjects, roundEl, roundMItem, List.map_map, Function.comp_def]

theorem no_item_in_writeObjects (first : Nat) (obs : List (MObj F)) (e : El F)
    (he : e ∈ writeObjects first obs) : ∀ id m name, e ≠ El.item id m name := by
  induction obs generalizing first with
  | nil => simp [writeObjects] at he
  | cons ob obs ih =>
    intro id m name heq
    subst heq
    simp only [writeObjects, List.cons_append, List.mem_cons, List.mem_append, List.mem_map]
      at he
    rcases he with he | (⟨_, _, he⟩ | ⟨_, _, he⟩) | he | he
    · cases he
    · cases he
    · cases he
    · cases he
    · exact ih (first + 1) he id m name rfl

theorem parse_renderedEls (t tm : Text F) (l : List (El F))
    (hm : ∀ e ∈ l, ∀ id m name, e = El.item id (some m) name → m ≠ []) :
    (l.map (renderEl t tm)).mapM (parseEl t tm) = some (l.map (roundEl t tm)) := by
  induction l with
  | nil => rfl
  | cons x l ih =>
    simp [List.mapM_cons, parseEl_renderEl t tm x (hm x (by simp)),
      ih (fun e he => hm e (by simp [he]))]

/-- **The 3MF claim**: the model part's elements, written and read back, give every object with
its positions rounded (`num`) and exactly its triangles, name and material, and every item with
its object and its transform rounded (`matrix_num`). -/
theorem read_write3mf (t tm : Text F) (first : Nat) (obs : List (MObj F)) (its : List (MItem F))
    (h : ∀ it ∈ its, it.obj < obs.length)
    (hm : ∀ it ∈ its, ∀ m, it.m = some m → m ≠ []) :
    (((writeModel first obs its).map (renderEl t tm)).mapM (parseEl t tm)).bind
        (readModel first) =
      some (obs.map (roundMObj t), its.map (roundMItem tm)) := by
  rw [parse_renderedEls]
  · rw [Option.bind_some, roundEl_writeModel, readModel_writeModel]
    simpa [roundMItem] using h
  · intro e he id m name heq
    subst heq
    simp only [writeModel, List.mem_append, List.mem_map] at he
    rcases he with he | ⟨it, hit, heq⟩
    · exact absurd rfl (no_item_in_writeObjects first obs _ he id (some m) name)
    · simp only [El.item.injEq] at heq
      exact hm it hit m heq.2.1

end MeshFiles
