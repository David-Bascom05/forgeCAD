import ForgeLang.Modules

/-!
# Module qualification of component names

A used module's components are linked into `Defs.comps` (`Modules.lean`). Two counterexamples say
how their names must be formed:

* `unqualified_shadow`: two modules that both define `cup`, linked by their bare names: a call of
  `cup` silently takes the first one linked, with no error. Names must be qualified by module
  (`t.cup` / `b.fg#cup`).
* `qualify_string_collides`: qualified as one string `file ++ "#" ++ comp`, two different pairs
  give one name when a part may contain `#`. Safe when component names cannot (they are `ID`
  tokens: letters, digits, `_`), or with structured names (a pair), as ids are (`string_ids_collide`).
-/

namespace ForgeLang.Counter

def linkCupA : Comp := { params := [], body := [.feat "b" (.cst 1)], vis := [] }
def linkCupB : Comp := { params := [], body := [.feat "b" (.cst 2)], vis := [] }

def linkedDoc (L : List (Name × Comp)) : Doc :=
  { defs := linkAll L {}, body := [.occ "o" "cup" []], σ := fun _ => 0 }

theorem unqualified_shadow :
    (run 20 (linkedDoc [("cup", linkCupA), ("cup", linkCupB)])).map (·.1.nums (fun _ => 0)) = .ok [([.nm "o", .nm "b"], 1)] ∧
    (run 20 (linkedDoc [("cup", linkCupB), ("cup", linkCupA)])).map (·.1.nums (fun _ => 0)) = .ok [([.nm "o", .nm "b"], 2)] :=
  ⟨rfl, rfl⟩

def qualify (f c : String) : String := f ++ "#" ++ c

theorem qualify_string_collides : qualify "a#b" "c" = qualify "a" "b#c" ∧ ("a#b", "c") ≠ ("a", "b#c") := by
  refine ⟨rfl, by decide⟩

end ForgeLang.Counter
