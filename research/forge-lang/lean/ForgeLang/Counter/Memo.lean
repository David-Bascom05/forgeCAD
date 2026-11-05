import ForgeLang.Memo

/-!
# Counterexamples: memo keys that are too weak

`Memo.memo_sound` keys a call by its body and definitions, the literal numbers in their code and
the input numbers (arguments, defaults and the visible shared params), and stores the entry
*parametric*. Each part is needed:

* `concrete_cache_wrong`: an entry stored with the first call's traces, keyed by argument values.
  `c1 = part(cup(d: small))`, `c2 = part(cup(d: big))`, `small = big = 80`. Today's numbers agree,
  but `c2.b` carries `small`'s site: its handle is `small`, and SetParam on `big` leaves it at 80
  while the real document moves it to 90.
* `key_needs_shared`: the same call in two documents whose shared `W` differs.
* `key_needs_literals`: the same skeleton, a different literal number inside the body.
* `key_needs_defs`: the same body, a different function it calls.

In each case the calls agree on everything else in the key and the models differ, so a cache
that left that part out would hand one document the other's part.
-/

namespace ForgeLang.Counter

def cup1 : Comp := { params := [("d", .lit 9)], body := [.feat "b" (.var "d")], vis := ["W"] }

def twoCups : Doc where
  defs := { comps := [("cup", cup1)] }
  body := [.let_ "small" (.lit 1), .let_ "big" (.lit 2),
    .occ "c1" "cup" [("d", .var "small")], .occ "c2" "cup" [("d", .var "big")]]
  σ := fun s => if s = 9 then 50 else 80

def twoCupsModel : Model := [([.nm "c1", .nm "b"], .site 1), ([.nm "c2", .nm "b"], .site 2)]

theorem twoCups_run : run 30 twoCups = .ok (twoCupsModel, []) := rfl

/-- What a value-keyed cache of concrete results gives: `c2` is `c1`'s entry. -/
def concreteCached : Model := [([.nm "c1", .nm "b"], .site 1), ([.nm "c2", .nm "b"], .site 1)]

theorem concrete_cache_wrong :
    -- today the numbers agree, so the cache looks right
    twoCupsModel.nums twoCups.σ = concreteCached.nums twoCups.σ ∧
    -- but `c2.b`'s handle is `small`, not `big`
    concreteCached.lookup [.nm "c2", .nm "b"] = some (.site 1) ∧
    -- and after SetParam `big` = 90 the document moves `c2`, the cached traces re-read do not
    (run 30 (twoCups.setLit 2 90)).map (fun r => r.1.nums (upd twoCups.σ 2 90)) =
      .ok [([.nm "c1", .nm "b"], 80), ([.nm "c2", .nm "b"], 90)] ∧
    concreteCached.nums (upd twoCups.σ 2 90) = [([.nm "c1", .nm "b"], 80), ([.nm "c2", .nm "b"], 80)] := by
  refine ⟨rfl, rfl, rfl, rfl⟩

/-! ## Cross-run keys -/

def cupW : Comp := { params := [("d", .lit 9)], body := [.feat "b" (.add (.var "d") (.var "W"))], vis := ["W"] }

/-- One document, a shared `W` at site 3. -/
def withW (w : Int) : Doc where
  defs := { comps := [("cup", cupW)] }
  body := [.let_ "W" (.lit 3), .occ "c1" "cup" [("d", .lit 1)]]
  σ := fun s => if s = 3 then w else 80

/-- Same component text, same argument value, different `W`: different part. -/
theorem key_needs_shared :
    (run 30 (withW 1)).map (fun r => r.1.nums (withW 1).σ) = .ok [([.nm "c1", .nm "b"], 81)] ∧
    (run 30 (withW 2)).map (fun r => r.1.nums (withW 2).σ) = .ok [([.nm "c1", .nm "b"], 82)] :=
  ⟨rfl, rfl⟩

def cupLit : Comp := { params := [("d", .lit 9)], body := [.feat "b" (.mul (.var "d") (.lit 5))], vis := [] }

def withLit (k : Int) : Doc where
  defs := { comps := [("cup", cupLit)] }
  body := [.occ "c1" "cup" [("d", .lit 1)]]
  σ := fun s => if s = 5 then k else 80

/-- Same skeleton (the text with holes), same argument: different part. The key must hash the
literal numbers, not just the tree. -/
theorem key_needs_literals :
    (withLit 2).defs = (withLit 3).defs ∧ (withLit 2).body = (withLit 3).body ∧
    (run 30 (withLit 2)).map (fun r => r.1.nums (withLit 2).σ) = .ok [([.nm "c1", .nm "b"], 160)] ∧
    (run 30 (withLit 3)).map (fun r => r.1.nums (withLit 3).σ) = .ok [([.nm "c1", .nm "b"], 240)] :=
  ⟨rfl, rfl, rfl, rfl⟩

def cupFn : Comp := { params := [("d", .lit 9)], body := [.feat "b" (.call "f" (.var "d"))], vis := [] }

def withFn (fb : Expr) : Doc where
  defs := { fns := [("f", ("x", fb))], comps := [("cup", cupFn)] }
  body := [.occ "c1" "cup" [("d", .lit 1)]]
  σ := fun _ => 80

/-- Same component text, same argument, a different function body it calls: different part. -/
theorem key_needs_defs :
    (run 30 (withFn (.var "x"))).map (fun r => r.1.nums (fun _ => 80)) = .ok [([.nm "c1", .nm "b"], 80)] ∧
    (run 30 (withFn (.add (.var "x") (.var "x")))).map (fun r => r.1.nums (fun _ => 80)) =
      .ok [([.nm "c1", .nm "b"], 160)] :=
  ⟨rfl, rfl⟩

end ForgeLang.Counter
