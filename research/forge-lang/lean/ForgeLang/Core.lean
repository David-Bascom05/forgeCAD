/-!
# The core calculus

A document is a program with every safe feature switched on: value functions (recursive), procedures
that make features (recursive), `let`, mutable `set`, conditionals in expressions and statements,
range loops with ids by index, keyed loops with ids by label, list params indexed by a loop variable,
references to features and to loop instances, and expressions anywhere a number goes. Transforms
(`.move(d)`) are arithmetic on a placement value, so `add` stands for them; method chaining is syntax
(`a.f(b)` is `f(a, b)`) and lives in the grammar, not here.

Numbers are carried as *traces*: the arithmetic that made them, over literal sites. A value is
editable when its trace is one bare site. Wherever a number is *observed* (a branch, a loop count, an
index, a comparison) the sites it came from go into the run's control log. That log is what makes
the edit theorems true; `Counter/ControlUntracked.lean` shows what goes wrong without it.

Function and procedure bodies see their parameter and the names bound where they are called
(they read top-level params and lets, as FastCAD functions do). Under single assignment with no
shadowing of top-level names this is the same as lexical scope.

Every call consumes one unit of fuel, so evaluation is a total function (P2 by construction); running
out is the typed error `fuel`, and `Fuel.lean` proves more fuel never changes a result.
-/

namespace ForgeLang

abbrev Name := String

/-- A literal's site. In the language it is a path (statement name, argument position, list index);
here an abstract label. The document's literal table maps sites to numbers. -/
abbrev Site := Nat

/-- Provenance of a number. -/
inductive Tr where
  | site : Site → Tr
  | const : Int → Tr
  | add : Tr → Tr → Tr
  | mul : Tr → Tr → Tr
  deriving DecidableEq, Repr

namespace Tr

def den (σ : Site → Int) : Tr → Int
  | site s => σ s
  | const c => c
  | add a b => a.den σ + b.den σ
  | mul a b => a.den σ * b.den σ

def sites : Tr → List Site
  | site s => [s]
  | const _ => []
  | add a b => a.sites ++ b.sites
  | mul a b => a.sites ++ b.sites

/-- The edit handle: a value is editable exactly when it is one literal, untouched. -/
def handle : Tr → Option Site
  | site s => some s
  | _ => none

end Tr

/-- One step of a feature id: a statement name, a range-loop index, or a keyed-loop label. Ids are
built from these alone, never from evaluation order or values. -/
inductive Seg where
  | nm : Name → Seg
  | ix : Nat → Seg
  | key : Name → Seg
  deriving DecidableEq, Repr

abbrev FId := List Seg
abbrev Model := List (FId × Tr)
abbrev Log := List Site

inductive Expr where
  | lit : Site → Expr
  | cst : Int → Expr
  | var : Name → Expr
  | add : Expr → Expr → Expr
  | mul : Expr → Expr → Expr
  | eq : Expr → Expr → Expr
  | ite : Expr → Expr → Expr → Expr
  /-- `[e0, e1, ...][k]`: a list param indexed by a loop variable. -/
  | sel : List Expr → Expr → Expr
  | call : Name → Expr → Expr
  /-- `x.rel`: a feature (or one made inside it) by its id relative to `x`. -/
  | ref : Name → FId → Expr
  /-- `x[k].rel`: an instance of range loop `x`. -/
  | refIx : Name → Expr → FId → Expr

inductive Stmt where
  | let_ : Name → Expr → Stmt
  /-- Mutable reassignment of a name already bound. -/
  | set : Name → Expr → Stmt
  | feat : Name → Expr → Stmt
  /-- `x = for i in 0..n { body }`: instances `x[0]`, `x[1]`, ... -/
  | forN : Name → Name → Expr → List Stmt → Stmt
  /-- `x = for i in {k1: e1, k2: e2} { body }`: instances `x[k1]`, `x[k2]`. -/
  | forK : Name → Name → List (Name × Expr) → List Stmt → Stmt
  | ifs : Expr → List Stmt → List Stmt → Stmt
  /-- `x = proc(arg)`: the procedure's features under `x`. -/
  | use : Name → Name → Expr → Stmt
  /-- `x = part(c(y1: e1, ..))`: an occurrence of component `c`, its features under `x`. -/
  | occ : Name → Name → List (Name × Expr) → Stmt

inductive Bind where
  | num : Tr → Bind
  | frag : Model → Bind

abbrev Env := List (Name × Bind)

/-- `component c(y1: d1, ..) { body }`: params with defaults, and the names of the defining
document the body may see (its shared params; empty for a component of a used module, whose own
shared params are linked in as params). Functions are in `Defs` and seen everywhere. -/
structure Comp where
  params : List (Name × Expr)
  body : List Stmt
  vis : List Name

structure Defs where
  fns : List (Name × (Name × Expr)) := []
  procs : List (Name × (Name × List Stmt)) := []
  comps : List (Name × Comp) := []
  /-- Record observed sites. `true` is the language; `false` exists only for the counterexample. -/
  track : Bool := true

inductive Err where
  | fuel
  | unbound (x : Name)
  | notNum (x : Name)
  | notFeature (x : Name)
  | dangling (x : Name) (rel : FId)
  | range
  | noFn (f : Name)
  | dupId
  | noComp (c : Name)
  | noParam (c : Name)
  deriving DecidableEq, Repr

abbrev R (α : Type) := Except Err α

instance {ε α : Type} [DecidableEq ε] [DecidableEq α] : DecidableEq (Except ε α)
  | .ok a, .ok b => if h : a = b then isTrue (h ▸ rfl) else isFalse (by intro e; cases e; exact h rfl)
  | .error a, .error b => if h : a = b then isTrue (h ▸ rfl) else isFalse (by intro e; cases e; exact h rfl)
  | .ok _, .error _ => isFalse (by intro e; cases e)
  | .error _, .ok _ => isFalse (by intro e; cases e)

def pre (p : FId) (m : Model) : Model := m.map fun e => (p ++ e.1, e.2)

def obs (D : Defs) (t : Tr) : Log := if D.track then t.sites else []

def lookupNum (ρ : Env) (x : Name) : R Tr :=
  match ρ.lookup x with
  | some (.num t) => .ok t
  | some (.frag _) => .error (.notNum x)
  | none => .error (.unbound x)

def lookupRef (ρ : Env) (x : Name) (rel : FId) : R Tr :=
  match ρ.lookup x with
  | some (.frag m) =>
    match m.lookup rel with
    | some t => .ok t
    | none => .error (.dangling x rel)
  | some (.num _) => .error (.notFeature x)
  | none => .error (.unbound x)

/-- The environment a component body starts from: only the visible names of the caller's. -/
def only (N : List Name) (ρ : Env) : Env := ρ.filter fun e => N.contains e.1

/-- The params a call leaves to their defaults. -/
def dflts (ps args : List (Name × Expr)) : List (Name × Expr) := ps.filter fun p => (args.lookup p.1).isNone

/-- Every argument names a param. -/
def argsOK (ps args : List (Name × Expr)) : Bool := args.all fun a => (ps.lookup a.1).isSome

/-- An index read from a number: negative is an error, never a wrap or a clamp. -/
def toIx (v : Int) : R Nat := if v < 0 then .error .range else .ok v.toNat

mutual

def evalE : Nat → Defs → (Site → Int) → Env → Expr → R (Tr × Log)
  | 0, _, _, _, _ => .error .fuel
  | n + 1, D, σ, ρ, e =>
    match e with
    | .lit s => .ok (.site s, [])
    | .cst c => .ok (.const c, [])
    | .var x => do let t ← lookupNum ρ x; .ok (t, [])
    | .add a b => do
      let (ta, la) ← evalE n D σ ρ a
      let (tb, lb) ← evalE n D σ ρ b
      .ok (.add ta tb, la ++ lb)
    | .mul a b => do
      let (ta, la) ← evalE n D σ ρ a
      let (tb, lb) ← evalE n D σ ρ b
      .ok (.mul ta tb, la ++ lb)
    | .eq a b => do
      let (ta, la) ← evalE n D σ ρ a
      let (tb, lb) ← evalE n D σ ρ b
      .ok (.const (if ta.den σ = tb.den σ then 1 else 0), la ++ lb ++ obs D ta ++ obs D tb)
    | .ite c a b => do
      let (tc, lc) ← evalE n D σ ρ c
      let (t, l) ← evalE n D σ ρ (if tc.den σ ≠ 0 then a else b)
      .ok (t, lc ++ obs D tc ++ l)
    | .sel es k => do
      let (tk, lk) ← evalE n D σ ρ k
      let i ← toIx (tk.den σ)
      match es[i]? with
      | some e' => do
        let (t, l) ← evalE n D σ ρ e'
        .ok (t, lk ++ obs D tk ++ l)
      | none => .error .range
    | .call f a => do
      let (ta, la) ← evalE n D σ ρ a
      match D.fns.lookup f with
      | some (x, body) => do
        let (t, l) ← evalE n D σ ((x, .num ta) :: ρ) body
        .ok (t, la ++ l)
      | none => .error (.noFn f)
    | .ref x rel => do let t ← lookupRef ρ x rel; .ok (t, [])
    | .refIx x k rel => do
      let (tk, lk) ← evalE n D σ ρ k
      let i ← toIx (tk.den σ)
      let t ← lookupRef ρ x (.ix i :: rel)
      .ok (t, lk ++ obs D tk)

/-- A statement returns the bindings it adds (`Δ`), the features it made and what it observed. -/
def execOne : Nat → Defs → (Site → Int) → Env → Stmt → R (Env × Model × Log)
  | 0, _, _, _, _ => .error .fuel
  | n + 1, D, σ, ρ, s =>
    match s with
    | .let_ x e => do
      let (t, l) ← evalE n D σ ρ e
      .ok ([(x, .num t)], [], l)
    | .set x e => do
      let _ ← lookupNum ρ x
      let (t, l) ← evalE n D σ ρ e
      .ok ([(x, .num t)], [], l)
    | .feat x e => do
      let (t, l) ← evalE n D σ ρ e
      .ok ([(x, .frag [([], t)])], [([.nm x], t)], l)
    | .forN x i c body => do
      let (tc, lc) ← evalE n D σ ρ c
      let k ← toIx (tc.den σ)
      let (m, l) ← loopN n D σ ρ i body (List.range k)
      .ok ([(x, .frag m)], pre [.nm x] m, lc ++ obs D tc ++ l)
    | .forK x i kvs body => do
      let (m, l) ← loopK n D σ ρ i body kvs
      .ok ([(x, .frag m)], pre [.nm x] m, l)
    | .ifs c a b => do
      let (tc, lc) ← evalE n D σ ρ c
      let (_, m, l) ← execS n D σ ρ (if tc.den σ ≠ 0 then a else b)
      .ok ([], m, lc ++ obs D tc ++ l)
    | .use x p a => do
      let (ta, la) ← evalE n D σ ρ a
      match D.procs.lookup p with
      | some (y, body) => do
        let (_, m, l) ← execS n D σ ((y, .num ta) :: ρ) body
        .ok ([(x, .frag m)], pre [.nm x] m, la ++ l)
      | none => .error (.noFn p)
    | .occ x c args =>
      match D.comps.lookup c with
      | some C =>
        if argsOK C.params args then do
          let (Δg, lg) ← evalA n D σ ρ args
          let (Δd, ld) ← evalA n D σ (only C.vis ρ) (dflts C.params args)
          let (_, m, l) ← execS n D σ (Δg ++ Δd ++ only C.vis ρ) C.body
          .ok ([(x, .frag m)], pre [.nm x] m, lg ++ ld ++ l)
        else .error (.noParam c)
      | none => .error (.noComp c)

/-- Arguments, each evaluated in the same environment, bound to their param names. -/
def evalA : Nat → Defs → (Site → Int) → Env → List (Name × Expr) → R (Env × Log)
  | 0, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, [] => .ok ([], [])
  | n + 1, D, σ, ρ, (y, e) :: as => do
    let (t, l) ← evalE n D σ ρ e
    let (Δ, l') ← evalA n D σ ρ as
    .ok ((y, .num t) :: Δ, l ++ l')

def execS : Nat → Defs → (Site → Int) → Env → List Stmt → R (Env × Model × Log)
  | 0, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, [] => .ok ([], [], [])
  | n + 1, D, σ, ρ, s :: ss => do
    let (Δ1, m1, l1) ← execOne n D σ ρ s
    let (Δ2, m2, l2) ← execS n D σ (Δ1 ++ ρ) ss
    .ok (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2)

/-- Iterations are independent: each starts from the loop's environment. -/
def loopN : Nat → Defs → (Site → Int) → Env → Name → List Stmt → List Nat → R (Model × Log)
  | 0, _, _, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, _, _, [] => .ok ([], [])
  | n + 1, D, σ, ρ, i, body, k :: ks => do
    let (_, m1, l1) ← execS n D σ ((i, .num (.const k)) :: ρ) body
    let (m2, l2) ← loopN n D σ ρ i body ks
    .ok (pre [.ix k] m1 ++ m2, l1 ++ l2)

def loopK : Nat → Defs → (Site → Int) → Env → Name → List Stmt → List (Name × Expr) → R (Model × Log)
  | 0, _, _, _, _, _, _ => .error .fuel
  | _ + 1, _, _, _, _, _, [] => .ok ([], [])
  | n + 1, D, σ, ρ, i, body, (k, e) :: kvs => do
    let (t, l0) ← evalE n D σ ρ e
    let (_, m1, l1) ← execS n D σ ((i, .num t) :: ρ) body
    let (m2, l2) ← loopK n D σ ρ i body kvs
    .ok (pre [.key k] m1 ++ m2, l0 ++ l1 ++ l2)

end

/-- A document: definitions, a body, and the literal table. Printing puts `σ s` where `lit s` is. -/
structure Doc where
  defs : Defs := {}
  body : List Stmt
  σ : Site → Int

/-- The whole evaluation. Ids must be unique; a clash is a typed error, never "the last one wins". -/
def run (fuel : Nat) (d : Doc) : R (Model × Log) := do
  let (_, m, l) ← execS fuel d.defs d.σ [] d.body
  if (m.map Prod.fst).Nodup then .ok (m, l) else .error .dupId

/-- The numbers a model shows under a literal table. -/
def Model.nums (σ : Site → Int) (m : Model) : List (FId × Int) := m.map fun e => (e.1, e.2.den σ)

def upd (σ : Site → Int) (p : Site) (v : Int) : Site → Int := fun s => if s = p then v else σ s

end ForgeLang
