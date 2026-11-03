import ForgeLang.Struct

/-!
# Counterexamples: rebinding (mutable `set`, or a second `let` of a name)

Rebinding does not break P1, P2, P3 or SetParam: a reassigned name is a new trace and the theorems
in `Edits.lean` hold for documents with `set`. What it breaks is the structural edits. Removing a
binding from what a later statement sees can uncover an *older* binding of the same name, so the
later statement runs fine and means something else. In single assignment the same edit can only
fail with `unbound`.

It also makes params dishonest: a param that is later reassigned has a literal that drives nothing.
-/

namespace ForgeLang.Counter

def tbl12 : Site → Int := fun s => if s = 0 then 1 else 2

/-! ## MoveFeature changes a feature silently -/

def mutDoc : Doc := { body := [.let_ "a" (.lit 0), .set "a" (.lit 1), .feat "f" (.var "a")], σ := tbl12 }
def mutMoved : Doc := { body := [.let_ "a" (.lit 0), .feat "f" (.var "a"), .set "a" (.lit 1)], σ := tbl12 }

theorem move_silent :
    run 20 mutDoc = .ok ([([.nm "f"], .site 1)], []) ∧ run 20 mutMoved = .ok ([([.nm "f"], .site 0)], []) :=
  ⟨rfl, rfl⟩

/-- The hypothesis `move_sound` needs is exactly what fails. -/
theorem move_hyp_fails : ¬ FreshAfter (.set "a" (.lit 1)) [.let_ "a" (.lit 0)] := by
  intro h; exact h "a" rfl (by simp [binders, Stmt.binder])

/-! ## DeleteFeature changes a feature silently -/

def shadowDoc : Doc := { body := [.let_ "a" (.lit 0), .let_ "a" (.lit 1), .feat "f" (.var "a")], σ := tbl12 }
def shadowDel : Doc := { body := [.let_ "a" (.lit 0), .feat "f" (.var "a")], σ := tbl12 }

/-- The deleted statement made no feature, yet `f` changed. -/
theorem delete_silent :
    run 20 shadowDoc = .ok ([([.nm "f"], .site 1)], []) ∧ run 20 shadowDel = .ok ([([.nm "f"], .site 0)], []) :=
  ⟨rfl, rfl⟩

/-- In single assignment the same deletion is refused with a typed error. -/
def singleDel : Doc := { body := [.feat "f" (.var "a")], σ := tbl12 }
theorem delete_refused : run 20 singleDel = .error (.unbound "a") := rfl

/-! ## A reassigned param drives nothing -/

theorem dead_param (v : Int) :
    run 20 (mutDoc.setLit 0 v) = .ok ([([.nm "f"], .site 1)], []) :=
  setParam_traces v rfl (by rfl) (by simp)

end ForgeLang.Counter
