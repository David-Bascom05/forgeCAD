/-! Semantics of nominal wants/gives, confirmed bindings and undirected electrical wiring.
The identifier types are independent of implementation indices or JSON display strings.
Positions, package pin numbers and sheet decorations do not participate in electrical union. -/
namespace ForgeLang.Schematic

structure RoleId where
  name : String
deriving DecidableEq

/-- A dimensioned rating belongs only to the constructors that accept it in the reader.
Numeric magnitude is abstract here; the Rust reader checks finite bounds and positive capacitance. -/
structure Millivolts where
  value : Rat
deriving DecidableEq

structure Nanofarads where
  value : Rat
deriving DecidableEq

/-- Inclusive bounds, in the same voltage unit on both sides of a pin connection. -/
structure VoltageInterval where
  lo : Rat
  hi : Rat
  ordered : lo ≤ hi
deriving DecidableEq

def VoltageInterval.contains (r : VoltageInterval) (v : Rat) : Prop := r.lo ≤ v ∧ v ≤ r.hi

def acceptsVoltage (want give : VoltageInterval) : Bool :=
  decide (want.lo ≤ give.lo ∧ give.hi ≤ want.hi)

theorem acceptsVoltage_iff (want give : VoltageInterval) :
    acceptsVoltage want give = true ↔ ∀ v, give.contains v → want.contains v := by
  simp only [acceptsVoltage, decide_eq_true_eq]
  constructor
  · intro ⟨lo, hi⟩ v ⟨a, b⟩
    exact ⟨Rat.le_trans lo a, Rat.le_trans b hi⟩
  · intro h
    exact ⟨(h give.lo ⟨Rat.le_refl, give.ordered⟩).1,
           (h give.hi ⟨give.ordered, Rat.le_refl⟩).2⟩

inductive RoleSpec where
  | power (voltage : Option Millivolts)
  | powerRange (voltage : VoltageInterval)
  | reference (voltage : Option Millivolts)
  | bypass (capacitance : Option Nanofarads)
  | groundReturn | signal | clock | bias | termination | feedback | protection | interfaceRole
deriving DecidableEq

abbrev Registry := RoleId → Option RoleSpec

structure Terminal where
  occurrence : List String
  pin : String
deriving DecidableEq

structure Port where
  wants : List RoleId
  gives : List RoleId
deriving DecidableEq

/-- Roles on a placement add to the reusable library pin. Membership is set-like. -/
def effectivePort (library placement : Port) : Port :=
  ⟨library.wants ++ placement.wants, library.gives ++ placement.gives⟩

theorem effective_wants (library placement : Port) (r : RoleId) :
    r ∈ (effectivePort library placement).wants ↔ r ∈ library.wants ∨ r ∈ placement.wants := by
  simp [effectivePort]

theorem effective_gives (library placement : Port) (r : RoleId) :
    r ∈ (effectivePort library placement).gives ↔ r ∈ library.gives ∨ r ∈ placement.gives := by
  simp [effectivePort]

structure Binding where
  consumer : Terminal
  provider : Terminal
  role : RoleId
deriving DecidableEq

def Admissible (ports : Terminal → Port) (b : Binding) : Prop :=
  b.consumer ≠ b.provider ∧ b.role ∈ (ports b.consumer).wants ∧ b.role ∈ (ports b.provider).gives

instance (p : Terminal → Port) (b : Binding) : Decidable (Admissible p b) :=
  inferInstanceAs (Decidable (_ ∧ _ ∧ _))

def checkBinding (ports : Terminal → Port) (b : Binding) : Bool := decide (Admissible ports b)

theorem binding_sound_complete (ports : Terminal → Port) (b : Binding) :
    checkBinding ports b = true ↔ Admissible ports b := by simp [checkBinding]

theorem missing_give_refused (ports : Terminal → Port) (b : Binding)
    (h : b.role ∉ (ports b.provider).gives) : checkBinding ports b = false := by
  simp [checkBinding, Admissible, h]

/-- Resolve a proposed connection to one declared, typed role shared by its two pins. -/
def resolveBinding (types : Registry) (ports : Terminal → Port) (b : Binding) : Option RoleSpec :=
  if checkBinding ports b then types b.role else none

theorem resolved_binding_typed (types : Registry) (ports : Terminal → Port) (b : Binding) (spec : RoleSpec) :
    resolveBinding types ports b = some spec ↔ Admissible ports b ∧ types b.role = some spec := by
  by_cases h : Admissible ports b <;> simp [resolveBinding, checkBinding, h]

theorem undeclared_role_refused (types : Registry) (ports : Terminal → Port) (b : Binding)
    (h : types b.role = none) : resolveBinding types ports b = none := by
  simp [resolveBinding, h]

abbrev Wire := Terminal × Terminal

def touches (e : Wire) (a b : Terminal) : Prop :=
  (e.1 = a ∧ e.2 = b) ∨ (e.1 = b ∧ e.2 = a)

theorem reverse_wire (e : Wire) (a b : Terminal) :
    touches (e.2,e.1) a b ↔ touches e a b := by
  simp only [touches]
  constructor <;> intro h <;> rcases h with ⟨x,y⟩ | ⟨x,y⟩
  · exact Or.inr ⟨y,x⟩
  · exact Or.inl ⟨y,x⟩
  · exact Or.inr ⟨y,x⟩
  · exact Or.inl ⟨y,x⟩

abbrev Adj (w : List Wire) (a b : Terminal) := ∃ e ∈ w, touches e a b

inductive Connected (w : List Wire) (a : Terminal) : Terminal → Prop where
  | here : Connected w a a
  | step {x y} : Connected w a x → Adj w x y → Connected w a y

theorem same_adjacency {u v : List Wire} (eqv : ∀ a b, Adj u a b ↔ Adj v a b)
    {a b} (h : Connected u a b) : Connected v a b := by
  induction h with
  | here => exact .here
  | step _ edge ih => exact .step ih ((eqv _ _).mp edge)

theorem reverse_gesture {e : Wire} {ws : List Wire} {a b} :
    Connected ((e.2,e.1)::ws) a b ↔ Connected (e::ws) a b := by
  have h : ∀ x y, Adj ((e.2,e.1)::ws) x y ↔ Adj (e::ws) x y := by
    intro x y
    simp [Adj, reverse_wire]
  exact ⟨same_adjacency h, same_adjacency (fun x y => (h x y).symm)⟩

structure Drawing where
  wires : List Wire
  bindings : List Binding
  pose : Terminal → Int × Int

def confirm (d : Drawing) (p : Terminal → Port) (b : Binding) : Option Drawing :=
  if checkBinding p b then some {d with bindings := b :: d.bindings} else none

theorem confirm_preserves_wiring {d d' : Drawing} {p b} (h : confirm d p b = some d') :
    d'.wires = d.wires ∧ Admissible p b := by
  unfold confirm at h
  split at h
  next valid => cases h; exact ⟨rfl, (binding_sound_complete p b).mp valid⟩
  next => contradiction

theorem move_preserves_intent (d : Drawing) (pos : Terminal → Int × Int) :
    ({d with pose := pos} : Drawing).bindings = d.bindings ∧
    ({d with pose := pos} : Drawing).wires = d.wires := ⟨rfl,rfl⟩

/-- A fitted bridge represents one occurrence with both terminals and a nominal value.
    The nets are the already partitioned electrical graph. -/
structure Bridge where
  symbol : String
  partKind : Option String
  path : Option String
  defaultPath : Bool
  fitted : Bool
  left : Nat
  right : Nat
  value : Option Rat

abbrev NominalBounds := Rat × Rat

inductive PartType where
  | exactSymbol (name : String)
  | classified (name : String)

structure PathWant where
  part : PartType
  path : Option String

def bridgeMatches (a b : Nat) (want : PathWant) (range : Option NominalBounds) (part : Bridge) : Bool :=
  let matchesType := match want.part with
    | .exactSymbol s => part.symbol == s
    | .classified k => part.partKind == some k
  let matchesPath := match want.path with
    | none => part.defaultPath
    | some name => part.path == some name
  a != b && part.fitted && matchesType && matchesPath &&
    ((part.left == a && part.right == b) || (part.path.isNone && part.left == b && part.right == a)) &&
    match range with
    | none => true
    | some r => part.value.any fun v => decide (r.1 ≤ v ∧ v ≤ r.2)

def directConnection (a b : Terminal) (net : Terminal → Nat) : Bool :=
  a != b && net a == net b

theorem direct_connection_iff (a b : Terminal) (net : Terminal → Nat) :
    directConnection a b net = true ↔ a ≠ b ∧ net a = net b := by simp [directConnection]

theorem unpopulated_bridge_refused (a b : Nat) (s : PathWant) (r : Option NominalBounds) (p : Bridge)
    (h : p.fitted = false) : bridgeMatches a b s r p = false := by simp [bridgeMatches, h]

theorem short_bypasses_bridge (a : Nat) (s : PathWant) (r : Option NominalBounds) (p : Bridge) :
    bridgeMatches a a s r p = false := by simp [bridgeMatches]

theorem wrong_entry_refused (a b : Nat) (s : PathWant) (r : Option NominalBounds) (p : Bridge)
    (name : String) (h : p.path = some name) (wrong : p.left ≠ a) :
    bridgeMatches a b s r p = false := by simp [bridgeMatches, h, wrong]

theorem one_bridge_witness (a b : Nat) (s : PathWant) (r : Option NominalBounds) (parts : List Bridge) :
    parts.any (bridgeMatches a b s r) = true ↔ ∃ p ∈ parts, bridgeMatches a b s r p = true := by simp

end ForgeLang.Schematic
