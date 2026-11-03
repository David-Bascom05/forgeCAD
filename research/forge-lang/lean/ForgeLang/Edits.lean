import ForgeLang.Weak

/-!
# P1, P2, P3, P4 (SetParam), P5 (SetParam)

All corollaries of `sim`. The hypothesis everywhere is the one the theorem cannot do without:
the edited literal was not observed (`p ∉ L`, where `L` is the run's control log). A literal that
was observed is a *structural* param (a count, a flag, an index); editing it is still one meaning
(evaluation is a function), but its effect is a re-run, not a re-read: `Loops.lean` covers what stays
stable then.
-/

namespace ForgeLang

/-! ## Fuel: P2 and the fuel-independence half of P1 -/

theorem execS_mono {n m D σ ρ x r} (h : execS n D σ ρ x = .ok r) (hm : n ≤ m) :
    execS m D σ ρ x = .ok r := (sim n).s (.inr fun _ => rfl) h (fun _ _ => rfl) hm

theorem execOne_mono {n m D σ ρ x r} (h : execOne n D σ ρ x = .ok r) (hm : n ≤ m) :
    execOne m D σ ρ x = .ok r := (sim n).one (.inr fun _ => rfl) h (fun _ _ => rfl) hm

theorem loopN_mono {n m D σ ρ i b ks r} (h : loopN n D σ ρ i b ks = .ok r) (hm : n ≤ m) :
    loopN m D σ ρ i b ks = .ok r := (sim n).ln (.inr fun _ => rfl) h (fun _ _ => rfl) hm

theorem loopK_mono {n m D σ ρ i b kvs r} (h : loopK n D σ ρ i b kvs = .ok r) (hm : n ≤ m) :
    loopK m D σ ρ i b kvs = .ok r := (sim n).lk (.inr fun _ => rfl) h (fun _ _ => rfl) hm

theorem execS_unique {n n' D σ ρ x r r'} (h : execS n D σ ρ x = .ok r) (h' : execS n' D σ ρ x = .ok r') :
    r = r' := by
  have a := execS_mono h (Nat.le_max_left n n')
  have b := execS_mono h' (Nat.le_max_right n n')
  rw [a] at b; exact Except.ok.inj b

theorem execOne_unique {n n' D σ ρ x r r'} (h : execOne n D σ ρ x = .ok r)
    (h' : execOne n' D σ ρ x = .ok r') : r = r' := by
  have a := execOne_mono h (Nat.le_max_left n n')
  have b := execOne_mono h' (Nat.le_max_right n n')
  rw [a] at b; exact Except.ok.inj b

theorem loopN_unique {n n' D σ ρ i b ks r r'} (h : loopN n D σ ρ i b ks = .ok r)
    (h' : loopN n' D σ ρ i b ks = .ok r') : r = r' := by
  have a := loopN_mono h (Nat.le_max_left n n')
  have b' := loopN_mono h' (Nat.le_max_right n n')
  rw [a] at b'; exact Except.ok.inj b'

theorem loopK_unique {n n' D σ ρ i b kvs r r'} (h : loopK n D σ ρ i b kvs = .ok r)
    (h' : loopK n' D σ ρ i b kvs = .ok r') : r = r' := by
  have a := loopK_mono h (Nat.le_max_left n n')
  have b' := loopK_mono h' (Nat.le_max_right n n')
  rw [a] at b'; exact Except.ok.inj b'

theorem run_ok {n : Nat} {d : Doc} {m : Model} {l : Log} :
    run n d = .ok (m, l) ↔
      ∃ Δ, execS n d.defs d.σ [] d.body = .ok (Δ, m, l) ∧ (m.map Prod.fst).Nodup := by
  unfold run
  constructor
  · intro h
    rw [bind_ok] at h
    obtain ⟨⟨Δ, m', l'⟩, he, h⟩ := h
    simp only at h
    split at h
    · cases h; exact ⟨Δ, he, by assumption⟩
    · cases h
  · rintro ⟨Δ, he, hn⟩
    rw [he]
    simp [hn]

/-- P2: a run that finished keeps its result with any larger budget. Together with `run` being a
total function (a `def`, checked by Lean), the budget is the only thing a result can depend on
besides the document, and only as "finished or `fuel`". -/
theorem run_mono {n m : Nat} {d : Doc} {r} (h : run n d = .ok r) (hm : n ≤ m) : run m d = .ok r := by
  obtain ⟨mm, l⟩ := r
  obtain ⟨Δ, he, hn⟩ := run_ok.1 h
  exact run_ok.2 ⟨Δ, execS_mono he hm, hn⟩

/-- P1: one meaning. Any two budgets that finish give the same model. -/
theorem run_unique {n n' : Nat} {d : Doc} {r r'} (h : run n d = .ok r) (h' : run n' d = .ok r') :
    r = r' := by
  have a := run_mono h (Nat.le_max_left n n')
  have b := run_mono h' (Nat.le_max_right n n')
  rw [a] at b; exact Except.ok.inj b

/-- P1: in a model, an id names one feature. -/
theorem run_ids_unique {n : Nat} {d : Doc} {m : Model} {l : Log} (h : run n d = .ok (m, l)) :
    (m.map Prod.fst).Nodup := by
  obtain ⟨_, _, hn⟩ := run_ok.1 h; exact hn

/-! ## SetParam on a literal nothing observed -/

def Doc.setLit (d : Doc) (p : Site) (v : Int) : Doc := { d with σ := upd d.σ p v }

/-- The model-level SetParam: re-read every trace with the new table. No re-run. -/
def Model.setParam (m : Model) (σ : Site → Int) (p : Site) (v : Int) : List (FId × Int) :=
  m.nums (upd σ p v)

theorem agree_upd {σ : Site → Int} {p : Site} {v : Int} {L : Log} (hp : p ∉ L) :
    Agree σ (upd σ p v) L := by
  intro s hs
  have : s ≠ p := fun e => hp (e ▸ hs)
  simp [upd, this]

/-- The edited document has the same traces, ids and control log. -/
theorem setParam_traces {n : Nat} {d : Doc} {m : Model} {L : Log} {p : Site} (v : Int)
    (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) :
    run n (d.setLit p v) = .ok (m, L) := by
  obtain ⟨Δ, he, hn⟩ := run_ok.1 h
  exact run_ok.2 ⟨Δ, (sim n).s (.inl htr) he (agree_upd hp) (Nat.le_refl n), hn⟩

/-- P4 for SetParam: edit the text then evaluate = edit the model. -/
theorem setParam_roundtrip {n : Nat} {d : Doc} {m : Model} {L : Log} {p : Site} (v : Int)
    (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) :
    (run n (d.setLit p v)).map (fun r => r.1.nums (d.setLit p v).σ) = .ok (m.setParam d.σ p v) := by
  rw [setParam_traces v htr h hp]; rfl

/-- P5 for SetParam: every id is unchanged. -/
theorem setParam_ids {n : Nat} {d : Doc} {m : Model} {L : Log} {p : Site} (v : Int)
    (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) :
    (run n (d.setLit p v)).map (fun r => r.1.map Prod.fst) = .ok (m.map Prod.fst) := by
  rw [setParam_traces v htr h hp]; rfl

/-- P3, the handle is honest: a value whose trace is the bare site `p` becomes exactly `v`. -/
theorem handle_honest {m : Model} {σ : Site → Int} {i : FId} {p : Site} (v : Int)
    (hi : (i, Tr.site p) ∈ m) : (i, v) ∈ m.setParam σ p v := by
  simp only [Model.setParam, Model.nums, List.mem_map]
  exact ⟨_, hi, by simp [Tr.den, upd]⟩

/-- P3, one driver: no literal outside a value's trace changes it. -/
theorem sole_driver {m : Model} {σ : Site → Int} {i : FId} {t : Tr} {q : Site} (w : Int)
    (hi : (i, t) ∈ m) (hq : q ∉ t.sites) : (i, t.den σ) ∈ m.setParam σ q w := by
  simp only [Model.setParam, Model.nums, List.mem_map]
  exact ⟨_, hi, by
    simp only [Prod.mk.injEq, true_and]
    exact (Tr.den_congr t (agree_upd hq)).symm⟩

/-- A driven value has no handle: its trace is not one bare site. -/
theorem driven_no_handle {t : Tr} (h : ∀ s, t ≠ .site s) : t.handle = none := by
  cases t with
  | site s => exact absurd rfl (h s)
  | _ => rfl

end ForgeLang
