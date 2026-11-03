import ForgeLang.Edits

/-!
# Counterexamples: id schemes that move ids under edits that did not target them

The core builds ids from statement names, range indices and entry labels. Each alternative below
is a real design option (several are what FastCAD does) and each moves the id of a feature the
edit did not touch, or gives two features one id.
-/

namespace ForgeLang.Counter

def idsOf (r : R (Model × Log)) : R (List FId) := r.map fun r => r.1.map Prod.fst

/-! ## Index ids over a list: inserting an element shifts every later instance

`for p in [a, b] { feat b = p }` with ids `h[0]`, `h[1]`. Insert a new element in front: the
feature made from `b`'s literal (site 2) was `h[1]` and is now `h[2]`, and `h[1]` now holds
another literal. A reference to `h[1]` silently changes target. -/

def listDoc : Doc where
  body := [.forN "h" "i" (.cst 2) [.feat "b" (.sel [.lit 1, .lit 2] (.var "i"))]]
  σ := fun s => s
def listIns : Doc where
  body := [.forN "h" "i" (.cst 3) [.feat "b" (.sel [.lit 0, .lit 1, .lit 2] (.var "i"))]]
  σ := fun s => s

theorem index_shift :
    run 30 listDoc = .ok ([([.nm "h", .ix 0, .nm "b"], .site 1), ([.nm "h", .ix 1, .nm "b"], .site 2)], []) ∧
    run 30 listIns = .ok ([([.nm "h", .ix 0, .nm "b"], .site 0), ([.nm "h", .ix 1, .nm "b"], .site 1),
      ([.nm "h", .ix 2, .nm "b"], .site 2)], []) := ⟨rfl, rfl⟩

/-- The same with labels: the old instances keep their ids and their literals. -/
def keyDoc : Doc where
  body := [.forK "h" "p" [("x", .lit 1), ("y", .lit 2)] [.feat "b" (.var "p")]]
  σ := fun s => s
def keyIns : Doc where
  body := [.forK "h" "p" [("w", .lit 0), ("x", .lit 1), ("y", .lit 2)] [.feat "b" (.var "p")]]
  σ := fun s => s

theorem key_stable :
    run 30 keyDoc = .ok ([([.nm "h", .key "x", .nm "b"], .site 1), ([.nm "h", .key "y", .nm "b"], .site 2)], []) ∧
    run 30 keyIns = .ok ([([.nm "h", .key "w", .nm "b"], .site 0), ([.nm "h", .key "x", .nm "b"], .site 1),
      ([.nm "h", .key "y", .nm "b"], .site 2)], []) := ⟨rfl, rfl⟩

/-! ## Ids by evaluation order (a counter, or the position in the output)

A conditional feature toggles; the feature after it did not change, but its position did. Under
name ids it keeps its id. FastCAD's `part ball[] = [for ... if ... : ...]` numbering (`ball_1`,
`ball_2`, ...) is position in the filtered output: changing the filter renumbers later parts. -/

def byPosition (m : Model) : List (Nat × Tr) := (List.range m.length).zip (m.map Prod.snd)

def condDoc (c : Int) : Doc where
  body := [.ifs (.lit 0) [.feat "x" (.lit 1)] [], .feat "y" (.lit 2)]
  σ := fun s => if s = 0 then c else s

theorem position_ids_shift :
    (run 20 (condDoc 1)).map (fun r => byPosition r.1) = .ok [(0, .site 1), (1, .site 2)] ∧
    (run 20 (condDoc 0)).map (fun r => byPosition r.1) = .ok [(0, .site 2)] := ⟨rfl, rfl⟩

theorem name_ids_stable :
    idsOf (run 20 (condDoc 1)) = .ok [[.nm "x"], [.nm "y"]] ∧ idsOf (run 20 (condDoc 0)) = .ok [[.nm "y"]] :=
  ⟨rfl, rfl⟩

/-- Filter in a comprehension: `[for i in 0..3 if i != k : feat b]`. Instance from `i = 2` is output
position 1 when `k = 0` and position 2 when `k = 1`; its index id is `h[2]` either way. -/
def filtDoc (k : Int) : Doc where
  body := [.forN "h" "i" (.cst 3) [.ifs (.eq (.var "i") (.lit 0)) [] [.feat "b" (.var "i")]]]
  σ := fun _ => k

theorem filter_renumbers :
    (run 30 (filtDoc 0)).map (fun r => byPosition r.1) = .ok [(0, .const 1), (1, .const 2)] ∧
    (run 30 (filtDoc 1)).map (fun r => byPosition r.1) = .ok [(0, .const 0), (1, .const 2)] ∧
    idsOf (run 30 (filtDoc 0)) = .ok [[.nm "h", .ix 1, .nm "b"], [.nm "h", .ix 2, .nm "b"]] ∧
    idsOf (run 30 (filtDoc 1)) = .ok [[.nm "h", .ix 0, .nm "b"], [.nm "h", .ix 2, .nm "b"]] :=
  ⟨rfl, rfl, rfl, rfl⟩

/-! ## Ids by value: a SetParam renames the feature it changed

`for x in [10, 20]` keyed by the element's value. Editing 20 to 25 renames `h[20]` to `h[25]`:
every reference to it dangles though the user only changed a number. -/

def byValue (σ : Site → Int) (m : Model) : List (Int × Tr) := m.map fun e => (e.2.den σ, e.2)

def valDoc : Doc where
  body := [.forN "h" "i" (.cst 2) [.feat "b" (.sel [.lit 0, .lit 1] (.var "i"))]]
  σ := fun s => if s = 0 then 10 else 20

theorem value_ids_move :
    (run 30 valDoc).map (fun r => (byValue valDoc.σ r.1).map Prod.fst) = .ok [10, 20] ∧
    (run 30 (valDoc.setLit 1 25)).map (fun r => (byValue (valDoc.setLit 1 25).σ r.1).map Prod.fst) =
      .ok [10, 25] ∧
    idsOf (run 30 valDoc) = .ok [[.nm "h", .ix 0, .nm "b"], [.nm "h", .ix 1, .nm "b"]] ∧
    idsOf (run 30 (valDoc.setLit 1 25)) = .ok [[.nm "h", .ix 0, .nm "b"], [.nm "h", .ix 1, .nm "b"]] :=
  ⟨by decide, by decide, rfl, rfl⟩

/-! ## No call path: two calls, one id -/

def bossDefs : Defs := { procs := [("boss", ("d", [.feat "b" (.var "d")]))] }
def callDoc : Doc where
  defs := bossDefs
  body := [.use "l" "boss" (.lit 0), .use "r" "boss" (.lit 1)]
  σ := fun s => s

theorem call_path_ids : idsOf (run 20 callDoc) = .ok [[.nm "l", .nm "b"], [.nm "r", .nm "b"]] := rfl

/-- Strip the call path and the two features share an id: the model is no longer a function of id. -/
theorem no_call_path_clash : ¬ ([[.nm "b"], [.nm "b"]] : List FId).Nodup := by decide

/-! ## Ids as strings: built names collide

FastCAD's `part ball[]` makes `ball_1`, `ball_2`, ...; a separate `part ball_1` collides with it.
Structured ids cannot. -/

theorem string_ids_collide : ("ball_" ++ toString (1 : Nat)) = "ball_1" := by decide
theorem structured_ids_distinct : ([.nm "ball", .ix 1] : FId) ≠ [.nm "ball_1"] := by decide

/-! ## References to loop instances when the count changes

In the core, `h[2].b` with only two instances is the typed error `dangling`, and `h[-1]` is
`range`. A relative reference ("the last one") instead re-targets silently. -/

def refDoc (n : Int) (k : Int) : Doc where
  body := [.forN "h" "i" (.lit 0) [.feat "b" (.var "i")], .feat "r" (.refIx "h" (.cst k) [.nm "b"])]
  σ := fun _ => n

theorem ref_dangles :
    (run 30 (refDoc 3 2)).map (fun r => r.1.getLast?) = .ok (some ([.nm "r"], .const 2)) ∧
    run 30 (refDoc 2 2) = .error (.dangling "h" [.ix 2, .nm "b"]) ∧
    run 30 (refDoc 3 (-1)) = .error .range := ⟨rfl, rfl, rfl⟩

def lastOf (m : Model) : Option Tr := (m.getLast?).map Prod.snd

/-- "The last instance" of `h` is a different feature at each count, and no error says so. -/
theorem relative_ref_retargets :
    (run 30 (refDoc 3 0)).map (fun r => lastOf (r.1.filter fun e => e.1.head? = some (.nm "h"))) =
      .ok (some (.const 2)) ∧
    (run 30 (refDoc 4 0)).map (fun r => lastOf (r.1.filter fun e => e.1.head? = some (.nm "h"))) =
      .ok (some (.const 3)) := ⟨rfl, rfl⟩

/-! ## Selecting by world position (FastCAD `fillet(at=[x, y, z])`)

The selection is "whatever is nearest this point". Moving an unrelated feature changes what is
selected; no error. Placement by world coordinates (`.move([x, y, z])`) is a value like any other
and touches no property; *selection* by coordinates is a reference whose target is a function of
every feature's geometry. -/

def nearest (pts : List (Name × Int)) (q : Int) : Option Name :=
  (pts.foldl (fun (best : Option (Name × Int)) p =>
    match best with
    | none => some p
    | some b => if (p.2 - q).natAbs < (b.2 - q).natAbs then some p else some b) none).map Prod.fst

theorem nearest_retargets :
    nearest [("a", 0), ("b", 10)] 4 = some "a" ∧ nearest [("a", 0), ("b", 3)] 4 = some "b" := by decide

end ForgeLang.Counter
