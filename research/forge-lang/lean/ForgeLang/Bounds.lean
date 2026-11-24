import ForgeLang.Edits
import ForgeLang.Errors

/-!
# Where values enter the kernel: the magnitude bound

`interface/forge-lang/src/builtins.rs` (`within_bounds`, in `Args::bind`) refuses a number past
`eval::MAX_MAGNITUDE` (1e12; angles `MAX_ANGLE`) where a builtin that is not arithmetic takes it:
a feature's or a sketch's argument. Arithmetic, user functions and the pure builtins (`sqrt` ..
`len`) may carry larger values. In the core calculus a feature's number is its trace in the model,
so the bound is a check on the finished model's numbers, `Bounded`, made after the run: `runK`.

It is not an observation. A check inside evaluation would compare a value, and every literal it
reads would become structural (a data edit to 1e13 would re-read fine and re-run as an error). Made
on the model's numbers, a re-read checks the same numbers a re-run does (`setParam_bounded`), so
P2, P4 and P5 hold for `runK` as they do for `run`. One bound `B` stands for each argument's own
(lengths, angles, counts).

The other new limits are fuel or `range`: statements, features and nesting run out as `Fuel` in
Rust (the core's single fuel counts calls; any further count of work is fuel of the same kind and
`run_mono`'s argument is unchanged), and a range or index past `maxList` is `range` (`toIx`).

Nesting is counted at calls only (`eval.rs`, `call_fn_here`): a call adds its callee body's static
depth (`syntax::body_depth`, a level per statement and expression it nests) to the run's nest, and
is refused (`Fuel`, "nested evaluation") past `nestBudget`; a statement itself the parser bounds
at `maxDepth` (`fast::MAX_DEPTH`). `stack_depth_le` is the bound this gives the evaluator's
recursion: at most `maxDepth + nestBudget` = 600 levels, the bound the per-expression count it
replaces gave.
-/

namespace ForgeLang

/-- Every number of the model within `B` in magnitude, under literal table `σ`. -/
def Bounded (B : Nat) (σ : Site → Int) (m : Model) : Bool :=
  m.all fun e => (e.2.den σ).natAbs ≤ B

/-- What the kernel's boundary can refuse besides evaluation's own errors. -/
inductive KErr where
  | eval (e : Err)
  | tooLarge
  deriving DecidableEq, Repr

/-- The run as the kernel takes it: the model, then its numbers within `B`. -/
def runK (B : Nat) (fuel : Nat) (d : Doc) : Except KErr (Model × Log) :=
  match run fuel d with
  | .error e => .error (.eval e)
  | .ok (m, l) => if Bounded B d.σ m then .ok (m, l) else .error .tooLarge

theorem runK_of_run {B n : Nat} {d : Doc} {m : Model} {l : Log} (h : run n d = .ok (m, l)) :
    runK B n d = if Bounded B d.σ m then .ok (m, l) else .error .tooLarge := by
  simp [runK, h]

/-- P2 for `runK`: a run the kernel took keeps its result under any larger budget. -/
theorem runK_mono {B n m : Nat} {d : Doc} {r} (h : runK B n d = .ok r) (hm : n ≤ m) :
    runK B m d = .ok r := by
  unfold runK at h ⊢
  cases hr : run n d with
  | error e => simp [hr] at h
  | ok mr =>
    obtain ⟨mm, l⟩ := mr
    rw [run_mono hr hm]
    simpa [hr] using h

/-- P1 for `runK`: any two budgets that finish give the same model. -/
theorem runK_unique {B n n' : Nat} {d : Doc} {r r'} (h : runK B n d = .ok r) (h' : runK B n' d = .ok r') :
    r = r' := by
  have a := runK_mono h (Nat.le_max_left n n')
  have b := runK_mono h' (Nat.le_max_right n n')
  rw [a] at b; exact Except.ok.inj b

/-- P2 for `runK`: a refusal other than running out of fuel stays under any larger budget. -/
theorem runK_error_stable {B n m : Nat} {d : Doc} {er : KErr} (h : runK B n d = .error er)
    (hf : er ≠ .eval .fuel) (hm : n ≤ m) : runK B m d = .error er := by
  unfold runK at h ⊢
  cases hr : run n d with
  | error e =>
    simp [hr] at h
    subst h
    have he : e ≠ .fuel := fun x => hf (x ▸ rfl)
    simp [run_error_stable hr he hm]
  | ok mr =>
    obtain ⟨mm, l⟩ := mr
    rw [run_mono hr hm]
    simpa [hr] using h

/-- P4 for SetParam through the kernel's boundary: a data edit (an unobserved literal) re-reads
the model and checks the bound on the re-read numbers; that is what a re-run gives. -/
theorem setParam_bounded {B n : Nat} {d : Doc} {m : Model} {L : Log} {p : Site} (v : Int)
    (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) :
    runK B n (d.setLit p v) =
      if Bounded B (upd d.σ p v) m then .ok (m, L) else .error .tooLarge := by
  rw [runK_of_run (setParam_traces v htr h hp)]
  rfl

/-- P5 for SetParam through the boundary: when the edit is taken, every id is unchanged. -/
theorem setParam_bounded_ids {B n : Nat} {d : Doc} {m : Model} {L : Log} {p : Site} (v : Int)
    (htr : d.defs.track = true) (h : run n d = .ok (m, L)) (hp : p ∉ L) {r}
    (hk : runK B n (d.setLit p v) = .ok r) : r.1.map Prod.fst = m.map Prod.fst := by
  rw [setParam_bounded v htr h hp] at hk
  split at hk
  · cases hk; rfl
  · cases hk

/-- The bound reads only the model's numbers: two tables that give the model the same numbers are
refused or taken alike. -/
theorem bounded_congr {B : Nat} {σ σ' : Site → Int} {m : Model} (h : m.nums σ = m.nums σ') :
    Bounded B σ m = Bounded B σ' m := by
  induction m with
  | nil => rfl
  | cons e es ih =>
    simp only [Model.nums, List.map_cons, List.cons.injEq] at h
    simp only [Bounded, List.all_cons] at ih ⊢
    have hd : e.2.den σ = e.2.den σ' := congrArg Prod.snd h.1
    rw [hd, ih h.2]

/-! ## The stack-depth bound, nesting counted at calls -/

/-- A statement's depth, as the parser bounds it (`syntax::fast::MAX_DEPTH`). -/
def maxDepth : Nat := 400

/-- What the bodies of the calls being evaluated may nest, in sum (`eval::NEST_BUDGET`). -/
def nestBudget : Nat := 200

/-- One call being evaluated: its callee body's static depth, and how deep in that body the
evaluation is (at the next call, or where it is now). -/
structure Frame where
  body : Nat
  here : Nat

/-- The frames' body depths, summed: the run's nest. -/
def bodySum : List Frame → Nat
  | [] => 0
  | f :: fs => f.body + bodySum fs

/-- How deep the evaluation is in all frames' bodies together. -/
def atSum : List Frame → Nat
  | [] => 0
  | f :: fs => f.here + atSum fs

/-- The call guard: a callee of body depth `d`, called at nest `n`, is taken when the nest stays
within the budget, the nest then `n + d`; else the call is refused. -/
def admit (n d : Nat) : Option Nat :=
  if n + d ≤ nestBudget then some (n + d) else none

/-- Calls taken one inside the next from nest `n`, each with its callee's body depth: the nest
after the innermost, or none for a refusal. -/
def admits : Nat → List Nat → Option Nat
  | n, [] => some n
  | n, d :: ds => (admit n d).bind (admits · ds)

/-- A taken call keeps the nest within the budget, and the nest is what it was plus the body's. -/
theorem admit_some {n d n' : Nat} (h : admit n d = some n') : n' = n + d ∧ n' ≤ nestBudget := by
  unfold admit at h
  split at h
  · cases h; exact ⟨rfl, by assumption⟩
  · cases h

/-- Calls taken from a nest within the budget: the nest is the sum of their bodies' depths on top
of it, and within the budget. -/
theorem admits_some {n n' : Nat} {ds : List Nat} (hn : n ≤ nestBudget) (h : admits n ds = some n') :
    n' = n + ds.foldr (· + ·) 0 ∧ n' ≤ nestBudget := by
  induction ds generalizing n with
  | nil => cases h; exact ⟨by simp, hn⟩
  | cons d ds ih =>
    simp only [admits] at h
    cases ha : admit n d with
    | none => rw [ha] at h; cases h
    | some m =>
      rw [ha] at h
      have ⟨hm, hb⟩ := admit_some ha
      have ⟨h1, h2⟩ := ih hb h
      refine ⟨?_, h2⟩
      simp only [List.foldr_cons]
      omega

/-- The stack the guard allows: from the document's nest 0, every call stack it takes has its
bodies' depths in sum within the budget. -/
theorem admits_zero {n' : Nat} {fs : List Frame} (h : admits 0 (fs.map Frame.body) = some n') :
    bodySum fs ≤ nestBudget := by
  have ⟨h1, h2⟩ := admits_some (Nat.zero_le _) h
  have hs : ∀ gs : List Frame, (gs.map Frame.body).foldr (· + ·) 0 = bodySum gs := by
    intro gs
    induction gs with
    | nil => rfl
    | cons g gs ih => simp only [List.map_cons, List.foldr_cons, bodySum, ih]
  rw [hs] at h1
  omega

/-- How deep in its frames an evaluation is is at most their bodies' depths. -/
theorem atSum_le {fs : List Frame} (h : ∀ f ∈ fs, f.here ≤ f.body) : atSum fs ≤ bodySum fs := by
  induction fs with
  | nil => exact Nat.le_refl 0
  | cons f fs ih =>
    have hf := h f (List.mem_cons_self ..)
    have hr := ih fun g hg => h g (List.mem_cons_of_mem _ hg)
    simp only [atSum, bodySum]
    omega

/-- The evaluator's recursion: the top statement's depth where its call is (`top`, within the
parser's bound) and each frame's depth in its body, with the frames the guard took, is at most
`maxDepth + nestBudget`: 600 levels, the per-expression bound's, counted only at calls. -/
theorem stack_depth_le {top n' : Nat} {fs : List Frame} (htop : top ≤ maxDepth)
    (hat : ∀ f ∈ fs, f.here ≤ f.body) (h : admits 0 (fs.map Frame.body) = some n') :
    top + atSum fs ≤ 600 := by
  have h1 := atSum_le hat
  have h2 := admits_zero h
  simp only [maxDepth, nestBudget] at htop h2
  omega

end ForgeLang
