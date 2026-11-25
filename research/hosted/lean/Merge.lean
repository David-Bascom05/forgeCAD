import ForgeLang.Edits

/-!
# Concurrent SetParam, in the ForgeLang core model (research/forge-lang/lean)

The hosted design orders ops on the server and re-applies a late op to the head it lands on.
For SetParam this file proves what that order may and may not change:

* `setLit_comm`: two SetParams on different sites commute: whichever the server orders first,
  the document is the same.
* `setLit_lww`: two SetParams on the same site: the later one wins and the earlier leaves no
  trace (last writer wins, exactly).
* `concurrent_setParam_traces`: if neither site was observed by the run (neither is a structural
  param), the merged document evaluates to the same traces, ids and control log as the base: the
  merge can change values only, never the feature list or a branch.

Compile from research/forge-lang/lean: `lake env lean ../../hosted/lean/Merge.lean`.
-/

namespace ForgeLang

theorem upd_comm {σ : Site → Int} {p q : Site} (h : p ≠ q) (v w : Int) :
    upd (upd σ p v) q w = upd (upd σ q w) p v := by
  funext s
  by_cases hq : s = q
  · subst hq; simp [upd, h.symm]
  · by_cases hp : s = p
    · subst hp; simp [upd, hq]
    · simp [upd, hp, hq]

theorem upd_upd {σ : Site → Int} {p : Site} (v w : Int) : upd (upd σ p v) p w = upd σ p w := by
  funext s; by_cases hs : s = p <;> simp [upd, hs]

theorem setLit_comm (d : Doc) {p q : Site} (h : p ≠ q) (v w : Int) :
    (d.setLit p v).setLit q w = (d.setLit q w).setLit p v := by
  cases d; simp only [Doc.setLit, upd_comm h]

theorem setLit_lww (d : Doc) (p : Site) (v w : Int) :
    (d.setLit p v).setLit p w = d.setLit p w := by
  cases d; simp only [Doc.setLit, upd_upd]

theorem concurrent_setParam_traces {n : Nat} {d : Doc} {m : Model} {L : Log} {p q : Site}
    (v w : Int) (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) (hq : q ∉ L) :
    run n ((d.setLit p v).setLit q w) = .ok (m, L) := by
  have h1 := setParam_traces v htr h hp
  exact setParam_traces (d := d.setLit p v) w htr h1 hq

end ForgeLang
