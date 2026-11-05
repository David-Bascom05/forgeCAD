import ForgeLang.Nested

/-!
# Round 2: nesting and in-context references, concretely

The core calculus already runs a component block that places components (an `occ` in a body) and
an argument that reads another occurrence's geometry (`ref` in an argument): no new construct is
needed. Here, evaluated by `rfl` or by induction:

* `nested_ids`: ids through nested occurrences are path-joined: `box1.lid1.plate`.
* `context_trace`: a number made in a component from a reference argument is driven by the sites
  that shaped the referenced geometry (`ref(occ, name)` over those sites).
* `context_key_needs_ref`: the same call text with the referenced geometry different gives a
  different part: the reference's value is an input, in the key (`memo_sound` covers it, since
  the reference is evaluated where the call is written, like any argument).
* `context_forward_unbound`, `context_loop_unbound`: a reference to an occurrence written below,
  or to the loop being run, is refused (`unbound`): references reach only what is above, so no
  context cycle can be built in the core. The design's `ContextCycle` is that refusal, named.
* `place_cycle_fuel`: a component that places itself (directly; mutually is the same) never
  finishes at any budget. Rule: a placement cycle is a static typed error (`ComponentCycle`),
  found from the definitions, never left to the budget. `rank_acyclic`: a rank certificate
  (every placed component ranks below its placer) rules cycles out.
* `nested_delete_dangles`: DeleteFeature `lid.plate` passes the check inside `lid`'s block, but a
  block that places `lid` reads `lid1.plate`: every occurrence of the placer now fails. Rule: the
  scoped ops' `StillRead` check must look at every block that places the edited component
  (transitively), at references through the occurrence path (`lid1.plate`, `box1.lid1.plate`).
-/

namespace ForgeLang.Counter

def lidC : Comp := { params := [], body := [.feat "plate" (.lit 1)], vis := [] }
def boxC : Comp := { params := [], body := [.feat "wall" (.lit 2), .occ "lid1" "lid" []], vis := [] }

def nestedDoc : Doc where
  defs := { comps := [("lid", lidC), ("box", boxC)] }
  body := [.occ "box1" "box" []]
  σ := fun _ => 5

theorem nested_ids : run 30 nestedDoc =
    .ok ([([.nm "box1", .nm "wall"], .site 2), ([.nm "box1", .nm "lid1", .nm "plate"], .site 1)], []) := rfl

/-! ## In-context references are arguments -/

/-- `lid(rim: ..)`: its plate sits 3 above the rim it is given. -/
def rimLid : Comp := { params := [("rim", .cst 0)], body := [.feat "plate" (.add (.var "rim") (.lit 3))], vis := [] }
/-- `box` sees the shared `w`: its wall is `w`. -/
def wBox : Comp := { params := [], body := [.feat "wall" (.var "w")], vis := ["w"] }

def contextDoc (w : Int) : Doc where
  defs := { comps := [("lid", rimLid), ("box", wBox)] }
  body := [.let_ "w" (.lit 1), .occ "b1" "box" [], .occ "l1" "lid" [("rim", .ref "b1" [.nm "wall"])]]
  σ := fun s => if s = 1 then w else 0

/-- `l1.plate` is driven by `w` (site 1, which shaped `b1.wall`) and its own literal (site 3). -/
theorem context_trace : (run 30 (contextDoc 4)).map (·.1) =
    .ok [([.nm "b1", .nm "wall"], .site 1), ([.nm "l1", .nm "plate"], .add (.site 1) (.site 3))] := rfl

/-- The same call `lid(rim: b1.wall)` with the referenced geometry different is a different part. -/
theorem context_key_needs_ref :
    ((run 30 (contextDoc 4)).map fun r => r.1.nums (contextDoc 4).σ) ≠
    ((run 30 (contextDoc 7)).map fun r => r.1.nums (contextDoc 7).σ) := by decide

/-- A reference to an occurrence written below it: refused. -/
theorem context_forward_unbound :
    run 30 { (contextDoc 4) with body := [.let_ "w" (.lit 1), .occ "l1" "lid" [("rim", .ref "b1" [.nm "wall"])],
      .occ "b1" "box" []] } = .error (.unbound "b1") := rfl

/-- A loop instance referencing the loop it is in: refused (the loop is bound only after it). -/
theorem context_loop_unbound :
    run 40 { (contextDoc 4) with body := [.let_ "w" (.lit 1),
      .forN "x" "i" (.cst 2) [.occ "l" "lid" [("rim", .refIx "x" (.var "i") [.nm "l", .nm "plate"])]]] } =
    .error (.unbound "x") := rfl

/-! ## Placement cycles -/

def selfC : Comp := { params := [], body := [.occ "i" "self" []], vis := [] }
def selfDefs : Defs := { comps := [("self", selfC)] }

theorem self_occ_fuel : ∀ n x σ ρ, execOne n selfDefs σ ρ (.occ x "self" []) = .error .fuel := by
  intro n
  induction n using Nat.strongRecOn with
  | _ n ih =>
    intro x σ ρ
    match n with
    | 0 => rfl
    | 1 => rfl
    | 2 => rfl
    | k + 3 =>
      have h := ih (k + 1) (by omega) "i" σ ([] ++ [] ++ only [] ρ)
      have ha : evalA (k + 2) selfDefs σ ρ [] = .ok ([], []) := by simp [evalA]
      have hb : evalA (k + 2) selfDefs σ (only [] ρ) [] = .ok ([], []) := by simp [evalA]
      have hs : execS (k + 2) selfDefs σ ([] ++ [] ++ only [] ρ) [.occ "i" "self" []] = .error .fuel := by
        rw [execS]; rw [h]; rfl
      rw [execOne]
      simp only [selfDefs, selfC, List.lookup, argsOK, dflts, beq_self_eq_true, List.all_nil, ite_true,
        List.filter_nil] at hs ha hb ⊢
      rw [ha]; simp only [ok_bind]; rw [hb]; simp only [ok_bind]; rw [hs]; rfl

/-- A component placing itself never finishes, at any budget. -/
theorem place_cycle_fuel : ∀ n, run n { defs := selfDefs, body := [.occ "o" "self" []], σ := fun _ => 0 } = .error .fuel := by
  intro n
  match n with
  | 0 => rfl
  | k + 1 => simp [run, execS, self_occ_fuel]

theorem self_reaches : Reach selfDefs "self" "self" := .step (C := selfC) rfl (by simp [selfC, placesB, placesS])

/-! ## A scoped delete under nesting needs the placers' references -/

def readsLid : Comp := { params := [], body := [.occ "lid1" "lid" [], .feat "h" (.ref "lid1" [.nm "plate"])], vis := [] }

def placerDoc (lid : Comp) : Doc where
  defs := { comps := [("lid", lid), ("box", readsLid)] }
  body := [.occ "box1" "box" []]
  σ := fun _ => 5

/-- `lid.plate` deleted: inside `lid` nothing reads it, but `box` reads `lid1.plate`. -/
theorem nested_delete_dangles :
    (run 30 (placerDoc lidC)).map (·.1) =
      .ok [([.nm "box1", .nm "lid1", .nm "plate"], .site 1), ([.nm "box1", .nm "h"], .site 1)] ∧
    run 30 (placerDoc { lidC with body := [] }) = .error (.dangling "lid1" [.nm "plate"]) := ⟨rfl, rfl⟩

end ForgeLang.Counter
