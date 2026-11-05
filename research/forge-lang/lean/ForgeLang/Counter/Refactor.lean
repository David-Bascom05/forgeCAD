import ForgeLang.Refactor

/-!
# Counterexamples: refactors without their side conditions

ExtractComponent of a block that reads a shared param `W`:
* `extract_naive_unbound`: the file form cannot see `W`: refused (`unbound`). Safe, but the op fails.
* `extract_copy_cuts_link`: copying `W`'s current number into the file as its own param gives the
  same numbers today, but the part's trace now carries the copy, so SetParam `W` moves the inline
  version and not the extracted one. Silent.
* `extract_passes`: passing `W: W` (the rule of `extract_occ`) gives the identical run.

MakeComponent:
* `make_reads_local`: a moved statement reads a document `let` that is not shared: the component
  cannot see it (`unbound`). The design's refusal rule ("an outside statement reads a moved one")
  does not catch this; it needs the converse rule too.
* `make_ids_move`: the moved feature's id changes from `lid` to `L1.lid`.
* `make_breaks_placer`: another file that places this one and refers to `x.lid` now gets
  `dangling`. The in-file check cannot see that reader.
-/

namespace ForgeLang.Counter

def bodyW : List Stmt := [.feat "b" (.add (.var "d") (.var "W"))]

def tblW (w : Int) : Site → Int := fun s => if s = 3 then w else if s = 4 then 5 else 80

def inlineDoc : Doc where
  defs := { comps := [("cup", { params := [("d", .lit 9)], body := bodyW, vis := ["W"] })] }
  body := [.let_ "W" (.lit 3), .occ "c1" "cup" [("d", .lit 1)]]
  σ := tblW 5

def naiveDoc : Doc where
  defs := { comps := [("cupF", { params := [("d", .lit 9)], body := bodyW, vis := [] })] }
  body := [.let_ "W" (.lit 3), .occ "c1" "cupF" [("d", .lit 1)]]
  σ := tblW 5

theorem extract_naive_unbound :
    run 30 inlineDoc = .ok ([([.nm "c1", .nm "b"], .add (.site 1) (.site 3))], []) ∧
    run 30 naiveDoc = .error (.unbound "W") := ⟨rfl, rfl⟩

/-- `W` copied into the file as a param whose default is a new literal (site 4) set to 5. -/
def copyDoc (w : Int) : Doc where
  defs := { comps := [("cupF", { params := [("d", .lit 9), ("W", .lit 4)], body := bodyW, vis := [] })] }
  body := [.let_ "W" (.lit 3), .occ "c1" "cupF" [("d", .lit 1)]]
  σ := tblW w

def inlineDocW (w : Int) : Doc := { inlineDoc with σ := tblW w }

theorem extract_copy_cuts_link :
    -- today: the same number
    (run 30 (inlineDocW 5)).map (fun r => r.1.nums (tblW 5)) = .ok [([.nm "c1", .nm "b"], 85)] ∧
    (run 30 (copyDoc 5)).map (fun r => r.1.nums (tblW 5)) = .ok [([.nm "c1", .nm "b"], 85)] ∧
    -- after SetParam W = 7: the inline part follows, the extracted copy does not
    (run 30 (inlineDocW 7)).map (fun r => r.1.nums (tblW 7)) = .ok [([.nm "c1", .nm "b"], 87)] ∧
    (run 30 (copyDoc 7)).map (fun r => r.1.nums (tblW 7)) = .ok [([.nm "c1", .nm "b"], 85)] :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- Passing `W: W` instead: the same run, trace for trace. -/
def passDoc : Doc where
  defs := { comps := [("cupF", { params := [("d", .lit 9), ("W", .lit 4)], body := bodyW, vis := [] })] }
  body := [.let_ "W" (.lit 3), .occ "c1" "cupF" [("d", .lit 1), ("W", .var "W")]]
  σ := tblW 5

theorem extract_passes : run 30 passDoc = run 30 inlineDoc := rfl

/-! ## MakeComponent -/

def L (vis : List Name) : Comp := { params := [], body := [.feat "lid" (.var "w")], vis := vis }

def beforeMake : Doc where
  defs := { comps := [("L", L [])] }
  body := [.let_ "w" (.lit 0), .feat "lid" (.var "w")]
  σ := fun _ => 4

def afterMake (vis : List Name) : Doc where
  defs := { comps := [("L", L vis)] }
  body := [.let_ "w" (.lit 0), .occ "L1" "L" []]
  σ := fun _ => 4

/-- `w` is a document `let`, not visible to the new component: refused. -/
theorem make_reads_local :
    run 30 beforeMake = .ok ([([.nm "lid"], .site 0)], []) ∧
    run 30 (afterMake []) = .error (.unbound "w") := ⟨rfl, rfl⟩

/-- With `w` visible (shared) it goes through, and the feature's id is now under the occurrence. -/
theorem make_ids_move : run 30 (afterMake ["w"]) = .ok ([([.nm "L1", .nm "lid"], .site 0)], []) := rfl

/-- A file `A` placed by another document that refers to `x.lid`. -/
def placer (aBody : List Stmt) : Doc where
  defs := { comps := [("A", { params := [], body := aBody, vis := [] }),
                      ("L", { params := [], body := [.feat "lid" (.lit 0)], vis := [] })] }
  body := [.occ "x" "A" [], .feat "j" (.ref "x" [.nm "lid"])]
  σ := fun _ => 4

theorem make_breaks_placer :
    run 30 (placer [.feat "lid" (.lit 0)]) =
      .ok ([([.nm "x", .nm "lid"], .site 0), ([.nm "j"], .site 0)], []) ∧
    run 30 (placer [.occ "L1" "L" []]) = .error (.dangling "x" [.nm "lid"]) := ⟨rfl, rfl⟩

end ForgeLang.Counter
