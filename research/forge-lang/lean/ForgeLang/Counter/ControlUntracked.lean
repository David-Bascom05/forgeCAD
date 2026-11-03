import ForgeLang.Edits

/-!
# Counterexample: provenance without control dependencies

```
param c = 1
feat a = c ? 5 : 7
```
`a` shows 5 and its trace is the bare literal `5`: it looks editable through that literal alone.
But `c` also drives it. If the run does not log what branches observe (`track := false`), editing
`c` changes a value that claimed a different single driver, and SetParam is not a re-read of the
traces: P3 (one driver) and P4 (round trip by re-reading) both fail. With logging, `c`'s site is in
the log, so the theorems simply do not apply to it: it is a structural param and its edit is a re-run.
-/

namespace ForgeLang.Counter

def ctlDoc (track : Bool) : Doc where
  defs := { track := track }
  body := [.let_ "c" (.lit 0), .feat "a" (.ite (.var "c") (.lit 1) (.lit 2))]
  σ := fun s => if s = 0 then 1 else if s = 1 then 5 else 7

/-- Without logging: `a`'s trace is the bare site 1, the log is empty... -/
theorem untracked_run : run 20 (ctlDoc false) = .ok ([([.nm "a"], .site 1)], []) := rfl

/-- ...yet editing site 0, which is in neither `a`'s trace nor the log, moves `a` to another site. -/
theorem untracked_edit : run 20 ((ctlDoc false).setLit 0 0) = .ok ([([.nm "a"], .site 2)], []) := rfl

theorem untracked_breaks_sole_driver :
    (0 : Site) ∉ (Tr.site 1).sites ∧ (0 : Site) ∉ ([] : Log) ∧
      Model.nums (ctlDoc false).σ [([.nm "a"], .site 1)] = [([.nm "a"], 5)] ∧
      Model.nums ((ctlDoc false).setLit 0 0).σ [([.nm "a"], .site 2)] = [([.nm "a"], 7)] := by
  refine ⟨by decide, by simp, rfl, rfl⟩

/-- With logging the observed site is recorded, which is what `setParam_traces` needs. -/
theorem tracked_run : run 20 (ctlDoc true) = .ok ([([.nm "a"], .site 1)], [0]) := rfl

end ForgeLang.Counter
