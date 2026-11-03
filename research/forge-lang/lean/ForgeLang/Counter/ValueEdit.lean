import ForgeLang.Edits

/-!
# Counterexamples: edits addressed to model values instead of source sites

An edit addressed to a site has one meaning (`Edits.lean`). An edit addressed to a *value* in the
model ("make this one 7") has one meaning only when that value's trace is one bare site that drives
nothing else. Each way that fails:

1. Fan-out: a literal in a loop body drives every instance. "Make instance 1 differ" has no site
   edit at all. The fix is an op that changes the source structure first: SplitInstance, which here
   is the list-param form `[w0, w1, w2][i]`; after it every instance has its own site.
2. Two drivers: `a = p + q`. "Make `a` 12" has two site edits with different models.
3. Not invertible: `a = 2 * p`. "Make `a` 3" has no site edit.
-/

namespace ForgeLang.Counter

/-! ## 1. Fan-out -/

def fanDoc : Doc where
  body := [.let_ "w" (.lit 0), .forN "h" "i" (.cst 3) [.feat "b" (.var "w")]]
  σ := fun _ => 10

def fanModel : Model :=
  [([.nm "h", .ix 0, .nm "b"], .site 0), ([.nm "h", .ix 1, .nm "b"], .site 0),
   ([.nm "h", .ix 2, .nm "b"], .site 0)]

theorem fan_run : run 30 fanDoc = .ok (fanModel, []) := rfl

/-- Whatever literal is edited, to whatever value, the three instances stay equal. -/
theorem fan_no_site_edit (q : Site) (v : Int) :
    ∃ x, (run 30 (fanDoc.setLit q v)).map (fun r => r.1.nums (fanDoc.setLit q v).σ) =
      .ok [([.nm "h", .ix 0, .nm "b"], x), ([.nm "h", .ix 1, .nm "b"], x), ([.nm "h", .ix 2, .nm "b"], x)] := by
  rw [setParam_traces v rfl fan_run (by simp)]
  exact ⟨_, rfl⟩

/-- SplitInstance: the same numbers, now one site per instance. -/
def splitDoc : Doc where
  body := [.forN "h" "i" (.cst 3) [.feat "b" (.sel [.lit 10, .lit 11, .lit 12] (.var "i"))]]
  σ := fun _ => 10

theorem split_same_numbers :
    (run 30 splitDoc).map (fun r => r.1.nums splitDoc.σ) = (run 30 fanDoc).map (fun r => r.1.nums fanDoc.σ) := rfl

theorem split_edit_one :
    (run 30 (splitDoc.setLit 11 7)).map (fun r => r.1.nums (splitDoc.setLit 11 7).σ) =
      .ok [([.nm "h", .ix 0, .nm "b"], 10), ([.nm "h", .ix 1, .nm "b"], 7), ([.nm "h", .ix 2, .nm "b"], 10)] := rfl

/-! ## 2. Two drivers -/

def twoDoc (σ : Site → Int) : Doc where
  body := [.feat "a" (.add (.lit 0) (.lit 1)), .feat "b" (.lit 0)]
  σ := σ

def tbl (x y : Int) : Site → Int := fun s => if s = 0 then x else y

/-- "Make `a` 12": two tables do it, and they disagree about `b`. -/
theorem two_meanings :
    (run 20 (twoDoc (tbl 6 6))).map (fun r => r.1.nums (tbl 6 6)) = .ok [([.nm "a"], 12), ([.nm "b"], 6)] ∧
    (run 20 (twoDoc (tbl 4 8))).map (fun r => r.1.nums (tbl 4 8)) = .ok [([.nm "a"], 12), ([.nm "b"], 4)] :=
  ⟨rfl, rfl⟩

/-! ## 3. Not invertible -/

def dblDoc : Doc where
  body := [.feat "a" (.mul (.cst 2) (.lit 0))]
  σ := fun _ => 1

theorem dbl_run : run 20 dblDoc = .ok ([([.nm "a"], .mul (.const 2) (.site 0))], []) := rfl

theorem dbl_unreachable (q : Site) (v : Int) :
    (run 20 (dblDoc.setLit q v)).map (fun r => r.1.nums (dblDoc.setLit q v).σ) ≠ .ok [([.nm "a"], 3)] := by
  rw [setParam_traces v rfl dbl_run (by simp)]
  simp only [Except.map, Model.nums, List.map_cons, List.map_nil, Tr.den, ne_eq]
  intro h
  injection h with h
  simp only [List.cons.injEq, Prod.mk.injEq, true_and, and_true] at h
  omega

end ForgeLang.Counter
