import ForgeLang.LL1.Proof

/-!
# The table parser and the recursive descent, one function

Two parsers of any grammar, both from the same `choose` (TABLE) and the same tables (FIRST,
NULLABLE) and `emptyRule` (EMPTY), errors included:

* `run`: the stack machine of `syntax.rs`'s `Parser::run` (the grammar part: no actions, no
  `chain` shortcuts, no value stack). On a nonterminal it expands by the table; where the table
  has nothing it notes the miss, with the expected set `expected_set` gives (FIRST of the stack
  from the top to the first symbol that cannot be empty), and goes on in *missed mode*: rules
  from the table where it has one, else the nonterminal's empty rule (EMPTY), never a shift, until
  it must stop. The error is the miss's position and set, every nonterminal missed on the way
  (where `stmtsrest` and `stmts` make Rust's "end of statement" and "a value") and the symbol it
  stopped on (Rust's `what(stop)`).
* `rdS`/`rdF`: recursive descent, a function per symbol, with one inherited value: `fk`, the FIRST
  of what follows the call (`firstSeq` of the rest of the rule, and the caller's `fk` when that
  rest can be empty). Missed mode is the descent going on with the miss noted, as each function
  returns into its caller.

`rd_eq_run` (fuel for fuel, both ways): on every token string the two give the same answer: the
same rules in the same order when they accept, the same error (position, expected set, missed
nonterminals, stop symbol) when they refuse. No hypothesis on the grammar: not even LL(1).
`rd_accepts_parse`: on accepted input with no miss, the answer is the proven `parse`'s tree
(`Proof.lean`), so `unambiguous`, `roundtrip` and `lossless` hold of it.

This is the refinement theorem for `src/syntax/fast.rs`: what a one-pass parser must compute to
be the table parser on every input, accepted and refused. The Rust code is tied to the model by
`tests/parse_diff.rs`, not by proof.
-/

namespace ForgeLang.LL1.Driver

variable (G : Grammar) (T : Tables)

/-- table.rs's EMPTY: the first rule of `A` whose right side can be empty. -/
def emptyRule (A : Nat) : Option Nat :=
  (List.range G.rules.length).find? fun i =>
    match G.rules[i]? with
    | some (B, β) => B == A && T.nullSeq β
    | none => false

structure St where
  w : List Tok
  pos : Nat
  /-- The rules expanded, the last first. -/
  out : List Nat
  /-- After a miss: its expected set and the nonterminals missed so far. -/
  miss : Option (List Tok × List Nat)

structure Err where
  pos : Nat
  set : List Tok
  missed : List Nat
  stop : Sym

inductive Res where
  | accept (out : List Nat)
  | error (e : Err)

/-- At a nonterminal: the rule to expand by, the state with the miss noted, or the error. `fk` is
FIRST of what lies below `A` on the stack. -/
def pick (A : Nat) (fk : List Tok) (st : St) : Except Err (Nat × St) :=
  match choose G T A (la st.w) with
  | some r => .ok (r, st)
  | none =>
    let m : List Tok × List Nat := match st.miss with
      | none => (T.firstSeq [.n A] ++ (if T.nullSym (.n A) then fk else []), [A])
      | some (s, ms) => (s, ms ++ [A])
    match emptyRule G T A with
    | some r => .ok (r, { st with miss := some m })
    | none => .error ⟨st.pos, m.1, m.2, .n A⟩

/-- At a terminal: shift it, unless a miss came first or it is not the lookahead. -/
def shiftT (a : Tok) (st : St) : Except Err St :=
  match st.miss with
  | none => if la st.w = some a then .ok { st with w := st.w.tail, pos := st.pos + 1 }
    else .error ⟨st.pos, [a], [], .t a⟩
  | some (s, ms) => .error ⟨st.pos, s, ms, .t a⟩

/-- Expand by rule `r`: its right side and the state that records it. -/
def expand (r : Nat) (st : St) : Option (List Sym × St) :=
  (G.rules[r]?).map fun p => (p.2, { st with out := r :: st.out })

/-! ## The stack machine -/

abbrev Config := List Sym × St

def step : Config → Config ⊕ Res
  | ([], st) => .inr (.accept st.out)
  | (.t a :: k, st) => match shiftT a st with
    | .ok st' => .inl (k, st')
    | .error e => .inr (.error e)
  | (.n A :: k, st) => match pick G T A (T.firstSeq k) st with
    | .ok (r, st') => match expand G r st' with
      | some (α, st'') => .inl (α ++ k, st'')
      | none => .inr (.error ⟨st.pos, [], [], .n A⟩)
    | .error e => .inr (.error e)

def run : Nat → Config → Option Res
  | 0, _ => none
  | n + 1, c => match step G T c with
    | .inl c' => run n c'
    | .inr r => some r

/-! ## The recursive descent -/

mutual
def rdS : Nat → Sym → List Tok → St → Option (Except Err St)
  | 0, _, _, _ => none
  | _ + 1, .t a, _, st => some (shiftT a st)
  | n + 1, .n A, fk, st => match pick G T A fk st with
    | .ok (r, st') => match expand G r st' with
      | some (α, st'') => rdF n α fk st''
      | none => some (.error ⟨st.pos, [], [], .n A⟩)
    | .error e => some (.error e)
  termination_by n => (n, 0)
def rdF : Nat → List Sym → List Tok → St → Option (Except Err St)
  | _, [], _, st => some (.ok st)
  | n, X :: α, fk, st =>
    match rdS n X (T.firstSeq α ++ (if T.nullSeq α then fk else [])) st with
    | some (.ok st') => rdF n α fk st'
    | r => r
  termination_by n α => (n, α.length + 1)
end

/-- The whole input: the start symbol, nothing below it. -/
def init (w : List Tok) : St := ⟨w, 0, [], none⟩

def rd (n : Nat) (w : List Tok) : Option Res :=
  match rdS G T n (.n G.start) [] (init w) with
  | some (.ok st) => some (.accept st.out)
  | some (.error e) => some (.error e)
  | none => none

/-! ## Steps -/

/-- `s` steps from `c` to `c'`, none of them finishing. -/
def steps : Nat → Config → Option Config
  | 0, c => some c
  | s + 1, c => match step G T c with
    | .inl c' => steps s c'
    | .inr _ => none

variable {G T}

theorem run_steps : ∀ (s : Nat) {c c' : Config}, steps G T s c = some c' →
    (∀ j, run G T (j + s) c = run G T j c') ∧ ∀ j, j ≤ s → run G T j c = none
  | 0, c, c', h => by
    simp only [steps, Option.some.injEq] at h; subst h
    exact ⟨fun _ => rfl, fun j hj => by obtain rfl : j = 0 := by omega
                                        rfl⟩
  | s + 1, c, c', h => by
    simp only [steps] at h
    split at h
    · rename_i c1 hc
      obtain ⟨h1, h2⟩ := run_steps s h
      refine ⟨fun j => ?_, fun j hj => ?_⟩
      · rw [show j + (s + 1) = (j + s) + 1 by omega, run, hc]; exact h1 j
      · cases j with
        | zero => rfl
        | succ j => rw [run, hc]; exact h2 j (by omega)
    · cases h

theorem steps_add : ∀ (s : Nat) {s' : Nat} {c c' c'' : Config}, steps G T s c = some c' →
    steps G T s' c' = some c'' → steps G T (s + s') c = some c''
  | 0, _, c, c', c'', h, h' => by simp only [steps, Option.some.injEq] at h; subst h; simpa using h'
  | s + 1, s', c, c', c'', h, h' => by
    simp only [steps] at h
    rw [show s + 1 + s' = (s + s') + 1 by omega, steps]
    split at h
    · exact steps_add s h h'
    · cases h

theorem run_some_mono : ∀ (n : Nat) {c : Config} {r}, run G T n c = some r → ∀ m, n ≤ m → run G T m c = some r
  | 0, _, _, h, _, _ => by simp [run] at h
  | n + 1, c, r, h, m, hm => by
    obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
    rw [run] at h ⊢
    split at h
    · exact run_some_mono n h m (by omega)
    · exact h

theorem run_none_anti {n m : Nat} {c : Config} (h : run G T n c = none) (hm : m ≤ n) : run G T m c = none := by
  cases hr : run G T m c with
  | none => rfl
  | some r => rw [run_some_mono m hr n hm] at h; cases h

theorem firstSeq_append (α k : List Sym) :
    T.firstSeq (α ++ k) = T.firstSeq α ++ (if T.nullSeq α then T.firstSeq k else []) := by
  induction α with
  | nil => simp [Tables.firstSeq, Tables.nullSeq]
  | cons X α ih =>
    have ih' : T.firstSeq (α ++ k) = T.firstSeq α ++ (if α.all T.nullSym then T.firstSeq k else []) := ih
    cases h : T.nullSym X <;> simp [Tables.firstSeq, Tables.nullSeq, h, ih']

/-! ## The simulation -/

/-- What `rdS n X` says, the machine does from `X` on top of `k`. -/
def SimS (G : Grammar) (T : Tables) (n : Nat) : Prop :=
  ∀ (X : Sym) (k : List Sym) (st : St),
    (∀ st', rdS G T n X (T.firstSeq k) st = some (.ok st') → ∃ s, steps G T s (X :: k, st) = some (k, st')) ∧
    (∀ e, rdS G T n X (T.firstSeq k) st = some (.error e) → ∃ s, run G T s (X :: k, st) = some (.error e)) ∧
    (rdS G T n X (T.firstSeq k) st = none → run G T n (X :: k, st) = none)

def SimF (G : Grammar) (T : Tables) (n : Nat) : Prop :=
  ∀ (α : List Sym) (k : List Sym) (st : St),
    (∀ st', rdF G T n α (T.firstSeq k) st = some (.ok st') → ∃ s, steps G T s (α ++ k, st) = some (k, st')) ∧
    (∀ e, rdF G T n α (T.firstSeq k) st = some (.error e) → ∃ s, run G T s (α ++ k, st) = some (.error e)) ∧
    (rdF G T n α (T.firstSeq k) st = none → run G T n (α ++ k, st) = none)

theorem simF_of (n : Nat) (hS : SimS G T n) : SimF G T n := by
  intro α
  induction α with
  | nil =>
    intro k st
    refine ⟨fun st' h => ⟨0, ?_⟩, fun e h => ?_, fun h => ?_⟩
    · simp only [rdF, Option.some.injEq, Except.ok.injEq] at h; subst h; rfl
    · simp [rdF] at h
    · simp [rdF] at h
  | cons X α ih =>
    intro k st
    have hfk : T.firstSeq α ++ (if T.nullSeq α then T.firstSeq k else []) = T.firstSeq (α ++ k) :=
      (firstSeq_append α k).symm
    obtain ⟨hok, herr, hnone⟩ := hS X (α ++ k) st
    rw [← hfk] at hok herr hnone
    simp only [List.cons_append]
    cases h1 : rdS G T n X (T.firstSeq α ++ (if T.nullSeq α then T.firstSeq k else [])) st with
    | none =>
      have e1 : rdF G T n (X :: α) (T.firstSeq k) st = none := by rw [rdF, h1]
      refine ⟨fun st' h => (by rw [e1] at h; cases h), fun e h => (by rw [e1] at h; cases h), fun _ => hnone h1⟩
    | some x =>
      cases x with
      | error e0 =>
        have e1 : rdF G T n (X :: α) (T.firstSeq k) st = some (.error e0) := by rw [rdF, h1]
        refine ⟨fun st' h => (by rw [e1] at h; cases h), fun e h => ?_, fun h => (by rw [e1] at h; cases h)⟩
        rw [e1] at h; cases h; exact herr e0 h1
      | ok st1 =>
        have e1 : rdF G T n (X :: α) (T.firstSeq k) st = rdF G T n α (T.firstSeq k) st1 := by rw [rdF, h1]
        obtain ⟨s1, hs1⟩ := hok st1 h1
        rw [e1]
        refine ⟨fun st' h => ?_, fun e h => ?_, fun h => ?_⟩
        · obtain ⟨s2, hs2⟩ := (ih k st1).1 st' h
          exact ⟨s1 + s2, steps_add s1 hs1 hs2⟩
        · obtain ⟨s2, hs2⟩ := (ih k st1).2.1 e h
          exact ⟨s2 + s1, by rw [(run_steps s1 hs1).1 s2]; exact hs2⟩
        · have hn := (ih k st1).2.2 h
          by_cases hle : n ≤ s1
          · exact (run_steps s1 hs1).2 n hle
          · obtain ⟨j, rfl⟩ : ∃ j, n = j + s1 := ⟨n - s1, by omega⟩
            rw [(run_steps s1 hs1).1 j]
            exact run_none_anti hn (by omega)

theorem simS : ∀ n, SimS G T n := by
  intro n
  induction n with
  | zero =>
    intro X k st
    refine ⟨fun st' h => ?_, fun e h => ?_, fun _ => rfl⟩ <;> simp [rdS] at h
  | succ n ih =>
    have hF := simF_of n ih
    intro X k st
    cases X with
    | t a =>
      refine ⟨fun st' h => ⟨1, ?_⟩, fun e h => ⟨1, ?_⟩, fun h => ?_⟩
      · simp only [rdS, Option.some.injEq] at h
        simp [steps, step, h]
      · simp only [rdS, Option.some.injEq] at h
        simp [run, step, h]
      · simp [rdS] at h
    | n A =>
      refine ⟨fun st' h => ?_, fun e h => ?_, fun h => ?_⟩
      · rw [rdS] at h
        split at h
        · rename_i r st1 hp
          split at h
          · rename_i α st2 hx
            obtain ⟨s, hs⟩ := (hF α k st2).1 st' h
            exact ⟨s + 1, by rw [Nat.add_comm, steps_add 1 (c' := (α ++ k, st2)) (by simp [steps, step, hp, hx]) hs]⟩
          · cases h
        · cases h
      · rw [rdS] at h
        split at h
        · rename_i r st1 hp
          split at h
          · rename_i α st2 hx
            obtain ⟨s, hs⟩ := (hF α k st2).2.1 e h
            exact ⟨s + 1, by rw [run, step, hp]; simp only [hx]; exact hs⟩
          · rename_i hx
            simp only [Option.some.injEq, Except.error.injEq] at h; subst h
            exact ⟨1, by simp [run, step, hp, hx]⟩
        · rename_i e' hp
          simp only [Option.some.injEq, Except.error.injEq] at h; subst h
          exact ⟨1, by simp [run, step, hp]⟩
      · rw [rdS] at h
        split at h
        · rename_i r st1 hp
          split at h
          · rename_i α st2 hx
            rw [run, step, hp]; simp only [hx]
            exact (hF α k st2).2.2 h
          · cases h
        · cases h

/-- The two parsers are one function: what the descent answers with some fuel, the machine
answers with some fuel, and the reverse; each answer is unique (more fuel never changes it). -/
theorem rd_eq_run (w : List Tok) (r : Res) :
    (∃ n, rd G T n w = some r) ↔ (∃ n, run G T n ([.n G.start], init w) = some r) := by
  constructor
  · rintro ⟨n, h⟩
    obtain ⟨hok, herr, -⟩ := simS (G := G) (T := T) n (.n G.start) [] (init w)
    unfold rd at h
    split at h
    · rename_i st hs
      simp only [Option.some.injEq] at h; subst h
      obtain ⟨s, hst⟩ := hok st (by simpa [Tables.firstSeq] using hs)
      exact ⟨1 + s, by rw [(run_steps s hst).1 1]; rfl⟩
    · rename_i e hs
      simp only [Option.some.injEq] at h; subst h
      exact herr e (by simpa [Tables.firstSeq] using hs)
    · cases h
  · rintro ⟨n, h⟩
    obtain ⟨hok, herr, hnone⟩ := simS (G := G) (T := T) n (.n G.start) [] (init w)
    refine ⟨n, ?_⟩
    unfold rd
    have hfk : T.firstSeq [] = [] := rfl
    rw [hfk] at hok herr hnone
    cases hr : rdS G T n (.n G.start) [] (init w) with
    | none => rw [hnone hr] at h; cases h
    | some x =>
      cases x with
      | ok st =>
        obtain ⟨s, hs⟩ := hok st hr
        have h1 : run G T (1 + s) ([.n G.start], init w) = some (.accept st.out) := by
          rw [(run_steps s hs).1 1]; rfl
        have := run_some_mono _ h (max n (1 + s)) (by omega)
        rw [run_some_mono _ h1 (max n (1 + s)) (by omega)] at this
        simpa using this
      | error e =>
        obtain ⟨s, hs⟩ := herr e hr
        have := run_some_mono _ h (max n s) (by omega)
        rw [run_some_mono _ hs (max n s) (by omega)] at this
        simpa using this

/-! ## Accepted input: the proven parser's tree -/

mutual
def preT : Tree → List Nat
  | .leaf _ => []
  | .node r f => r :: preF f
def preF : Forest → List Nat
  | .nil => []
  | .cons t f => preT t ++ preF f
end

theorem pick_cases {A fk st r st'} (h : pick G T A fk st = .ok (r, st')) :
    (choose G T A (la st.w) = some r ∧ st' = st) ∨ st'.miss.isSome := by
  unfold pick at h
  cases hc : choose G T A (la st.w) with
  | some r' =>
    simp only [hc, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact .inl ⟨rfl, rfl⟩
  | none =>
    right
    simp only [hc] at h
    cases he : emptyRule G T A with
    | none => cases hm : st.miss <;> simp [he, hm] at h
    | some r' => cases hm : st.miss <;> simp [he, hm] at h <;> (obtain ⟨-, rfl⟩ := h; simp)

theorem pick_miss {A fk st r st'} (h : pick G T A fk st = .ok (r, st')) (hm : st'.miss = none) :
    choose G T A (la st.w) = some r ∧ st' = st := by
  rcases pick_cases h with h | h
  · exact h
  · simp [hm] at h

theorem rdF_cons_ok {n X α fk st st'} (h : rdF G T n (X :: α) fk st = some (.ok st')) :
    ∃ st1, rdS G T n X (T.firstSeq α ++ (if T.nullSeq α then fk else [])) st = some (.ok st1) ∧
      rdF G T n α fk st1 = some (.ok st') := by
  rw [rdF] at h
  cases h1 : rdS G T n X (T.firstSeq α ++ (if T.nullSeq α then fk else [])) st with
  | none => rw [h1] at h; cases h
  | some x =>
    cases x with
    | ok st1 => rw [h1] at h; exact ⟨st1, rfl, h⟩
    | error e => rw [h1] at h; cases h

/-- Once a miss is noted it stays. -/
theorem miss_stays : ∀ n,
    (∀ X fk st st', rdS G T n X fk st = some (.ok st') → st.miss.isSome → st'.miss.isSome) ∧
    (∀ α fk st st', rdF G T n α fk st = some (.ok st') → st.miss.isSome → st'.miss.isSome) := by
  intro n
  induction n with
  | zero =>
    refine ⟨fun X fk st st' h => by simp [rdS] at h, ?_⟩
    intro α
    induction α with
    | nil => intro fk st st' h hm; simp only [rdF, Option.some.injEq, Except.ok.injEq] at h; subst h; exact hm
    | cons X α ih => intro fk st st' h; simp [rdF, rdS] at h
  | succ n ih =>
    have hS : ∀ X fk st st', rdS G T (n + 1) X fk st = some (.ok st') → st.miss.isSome → st'.miss.isSome := by
      intro X fk st st' h hm
      cases X with
      | t a =>
        simp only [rdS, Option.some.injEq] at h
        unfold shiftT at h
        cases hmi : st.miss with
        | none => simp [hmi] at hm
        | some p => simp [hmi] at h
      | n A =>
        rw [rdS] at h
        split at h
        · rename_i r st1 hp
          split at h
          · rename_i α st2 hx
            apply ih.2 α fk st2 st' h
            unfold expand at hx
            cases hg : G.rules[r]? with
            | none => simp [hg] at hx
            | some p =>
              simp only [hg, Option.map_some, Option.some.injEq, Prod.mk.injEq] at hx
              rw [← hx.2]
              rcases pick_cases hp with ⟨-, rfl⟩ | h'
              · exact hm
              · exact h'
          · cases h
        · cases h
    refine ⟨hS, ?_⟩
    intro α
    induction α with
    | nil => intro fk st st' h hm; simp only [rdF, Option.some.injEq, Except.ok.injEq] at h; subst h; exact hm
    | cons X α ihα =>
      intro fk st st' h hm
      obtain ⟨st1, h1, h⟩ := rdF_cons_ok h
      exact ihα fk st1 st' h (hS X _ st st1 h1 hm)

/-- With no miss, the descent builds a parse tree: its leaves the tokens it took, its rules
(preorder) the ones it output. -/
theorem rd_tree : ∀ n,
    (∀ X fk st st', rdS G T n X fk st = some (.ok st') → st'.miss = none →
      ∃ t, VT G t X ∧ t.yield ++ st'.w = st.w ∧ st'.out = (preT t).reverse ++ st.out) ∧
    (∀ α fk st st', rdF G T n α fk st = some (.ok st') → st'.miss = none →
      ∃ f, VF G f α ∧ f.yield ++ st'.w = st.w ∧ st'.out = (preF f).reverse ++ st.out) := by
  intro n
  induction n with
  | zero =>
    refine ⟨fun X fk st st' h => by simp [rdS] at h, ?_⟩
    intro α
    induction α with
    | nil =>
      intro fk st st' h _
      simp only [rdF, Option.some.injEq, Except.ok.injEq] at h; subst h
      exact ⟨.nil, .nil, rfl, by simp [preF]⟩
    | cons X α ih => intro fk st st' h; simp [rdF, rdS] at h
  | succ n ih =>
    have hS : ∀ X fk st st', rdS G T (n + 1) X fk st = some (.ok st') → st'.miss = none →
        ∃ t, VT G t X ∧ t.yield ++ st'.w = st.w ∧ st'.out = (preT t).reverse ++ st.out := by
      intro X fk st st' h hm
      cases X with
      | t a =>
        simp only [rdS, Option.some.injEq] at h
        unfold shiftT at h
        cases hmi : st.miss with
        | some p => simp [hmi] at h
        | none =>
          simp only [hmi] at h
          split at h
          · rename_i hla
            simp only [Except.ok.injEq] at h; subst h
            cases hw : st.w with
            | nil => simp [la, hw] at hla
            | cons b w' =>
              simp only [la, hw, List.head?_cons, Option.some.injEq] at hla; subst hla
              exact ⟨.leaf b, .leaf b, by simp [Tree.yield], by simp [preT]⟩
          · cases h
      | n A =>
        rw [rdS] at h
        split at h
        · rename_i r st1 hp
          split at h
          · rename_i α st2 hx
            have hm1 : st1.miss = none := by
              cases h1 : st1.miss with
              | none => rfl
              | some p =>
                have : st2.miss = st1.miss := by
                  unfold expand at hx
                  cases hg : G.rules[r]? with
                  | none => simp [hg] at hx
                  | some q => simp only [hg, Option.map_some, Option.some.injEq, Prod.mk.injEq] at hx; rw [← hx.2]
                have := (miss_stays n).2 α fk st2 st' h (by simp [this, h1])
                simp [hm] at this
            obtain ⟨hc, rfl⟩ := pick_miss hp hm1
            obtain ⟨β, hr⟩ := choose_rule hc
            unfold expand at hx
            simp only [hr, Option.map_some, Option.some.injEq, Prod.mk.injEq] at hx
            obtain ⟨rfl, rfl⟩ := hx
            obtain ⟨f, hf, hy, ho⟩ := (ih.2) β fk _ st' h hm
            exact ⟨.node r f, .node hr hf, by simpa [Tree.yield] using hy, by simp [ho, preT]⟩
          · cases h
        · cases h
    refine ⟨hS, ?_⟩
    intro α
    induction α with
    | nil =>
      intro fk st st' h _
      simp only [rdF, Option.some.injEq, Except.ok.injEq] at h; subst h
      exact ⟨.nil, .nil, rfl, by simp [preF]⟩
    | cons X α ihα =>
      intro fk st st' h hm
      obtain ⟨st1, h1, h⟩ := rdF_cons_ok h
      · obtain ⟨f, hf, hy, ho⟩ := ihα fk st1 st' h hm
        have hm1 : st1.miss = none := by
          cases h2 : st1.miss with
          | none => rfl
          | some p =>
            have := (miss_stays (n + 1)).2 α fk st1 st' h (by simp [h2])
            simp [hm] at this
        obtain ⟨t, ht, hy', ho'⟩ := hS X _ st st1 h1 hm1
        exact ⟨.cons t f, .cons ht hf, by simp [Forest.yield, hy, hy'], by simp [ho, ho', preF]⟩

/-- An input the descent (so the table parser) accepts with no miss and nothing left over is one
the proven `parse` accepts, with the tree whose rules the descent output; under `check`, the only
tree of that input (`unambiguous`). -/
theorem rd_accepts_parse (hc : check G T = true) {n : Nat} {w : List Tok} {st : St}
    (h : rdS G T n (.n G.start) [] (init w) = some (.ok st)) (hm : st.miss = none) (hw : st.w = []) :
    ∃ t, parse G T t.size (.n G.start) w = some (t, []) ∧ st.out = (preT t).reverse := by
  obtain ⟨t, ht, hy, ho⟩ := (rd_tree n).1 _ _ _ _ h hm
  simp only [hw, List.append_nil, init] at hy ho
  refine ⟨t, ?_, by simpa using ho⟩
  rw [← hy]
  exact roundtrip hc ht

end ForgeLang.LL1.Driver
