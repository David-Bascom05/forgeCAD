/-! Semantics of nominal wants/gives, confirmed bindings and undirected electrical wiring.
The identifier types are independent of implementation indices or JSON display strings.
Positions, package pin numbers and sheet decorations do not participate in electrical union. -/
namespace ForgeLang.Schematic

/-- Guaranteed min/max bounds are independently optional. Typical measurements
    do not enter the overstress decision. All values here are in volts. -/
def violatesBounds (low high : Option Rat) (seenLow seenHigh : Rat) : Bool :=
  low.any (fun l => decide (seenLow < l)) || high.any (fun h => decide (h < seenHigh))

theorem no_declared_bounds_no_violation (lo hi : Rat) :
    violatesBounds none none lo hi = false := rfl

theorem upper_bound_inclusive (limit lo : Rat) :
    violatesBounds none (some limit) lo limit = false := by
  simp [violatesBounds]

/-- A comparison is eligible only when every declared field is present. -/
def eligibleForCheck (fields : List (Option α)) : Bool := fields.all Option.isSome

theorem missing_field_blocks_comparison (before after : List (Option α)) :
    eligibleForCheck (before ++ none :: after) = false := by
  simp [eligibleForCheck, List.all_append]

theorem complete_inputs_eligible (values : List α) :
    eligibleForCheck (values.map some) = true := by
  induction values with
  | nil => rfl
  | cons h t ih => simpa [eligibleForCheck] using ih

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
  | pullUp (rail : VoltageInterval) (resistance : Option (Rat × Rat))
  | pullDown (rail : VoltageInterval) (resistance : Option (Rat × Rat))
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

/-- A selected alternate function supplies the port contract before placement roles are added. -/
def selectedPort (defaultPort : Port) (mode : Option Port) (placed : Port) : Port :=
  effectivePort (mode.getD defaultPort) placed

theorem selected_function_wants (base mode placed : Port) (r : RoleId) :
    r ∈ (selectedPort base (some mode) placed).wants ↔ r ∈ mode.wants ∨ r ∈ placed.wants := by
  exact effective_wants mode placed r

theorem selected_function_gives (base mode placed : Port) (r : RoleId) :
    r ∈ (selectedPort base (some mode) placed).gives ↔ r ∈ mode.gives ∨ r ∈ placed.gives := by
  exact effective_gives mode placed r

/-- Direct signal roles after selecting the active function, adding occurrence roles and
filtering to nominal signal, clock and interface contracts on a fitted driver/receiver pair. -/
inductive SignalCompatibility where
  | compatible | mismatch | unknown
deriving DecidableEq

def signalCompatibility (offered required : List RoleId) : SignalCompatibility :=
  if offered = [] ∨ required = [] then .unknown
  else if required.all (fun r => decide (r ∈ offered)) then .compatible else .mismatch

theorem signal_compatible_iff (offered required : List RoleId) :
    signalCompatibility offered required = .compatible ↔
      offered ≠ [] ∧ required ≠ [] ∧ ∀ r ∈ required, r ∈ offered := by
  by_cases h : offered = [] ∨ required = []
  · rcases h with h | h <;> simp [signalCompatibility, h]
  · simp [signalCompatibility, not_or.mp h, List.all_eq_true]

/-- Once strap input evidence is complete, a datasheet row takes precedence over the default. -/
def strapValue {α : Type} (known : Bool) (row fallback : Option α) : Option α :=
  if known then row.orElse (fun _ => fallback) else none

theorem pinless_strap_default {α : Type} (fallback : α) :
    strapValue true none (some fallback) = some fallback := rfl

theorem strap_row_wins {α : Type} (v : α) (fallback : Option α) :
    strapValue true (some v) fallback = some v := rfl

theorem unresolved_strap {α : Type} (row fallback : Option α) :
    strapValue false row fallback = none := rfl

/-- Object lists enter this model after the reader sorts and deduplicates the qualified IDs. -/
structure FindingKey where
  rule : String
  objects : List String
  deriving DecidableEq

def waived (exceptions : List FindingKey) (finding : FindingKey) : Bool :=
  decide (finding ∈ exceptions)

theorem adding_waiver_preserves_other_finding (xs : List FindingKey) (a b : FindingKey) (h : a ≠ b) :
    waived (a :: xs) b = waived xs b := by
  simp [waived, Ne.symm h]

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

def kindAccepts (wanted actual : String) : Bool :=
  wanted == actual || (wanted == "fet" && (actual == "mosfet" || actual == "jfet"))

theorem custom_kind_reflexive (name : String) : kindAccepts name name = true := by simp [kindAccepts]
example : kindAccepts "fet" "jfet" = true := by decide
example : kindAccepts "mosfet" "jfet" = false := by decide

/-- Imports prepend lexical path segments. Shared role keys have global identity. -/
inductive LibraryRole where
  | localRole (path : List String)
  | sharedRole (key : String)
  deriving DecidableEq

def qualify (namespaceName : String) : LibraryRole → LibraryRole
  | .localRole path => .localRole (namespaceName :: path)
  | .sharedRole key => .sharedRole key

theorem nested_qualification (parent child : String) (path : List String) :
    qualify parent (qualify child (.localRole path)) = .localRole (parent :: child :: path) := rfl
theorem nested_shared_role (parent child key : String) :
    qualify parent (qualify child (.sharedRole key)) = .sharedRole key := rfl

def bridgeMatches (a b : Nat) (want : PathWant) (range : Option NominalBounds) (part : Bridge) : Bool :=
  let matchesType := match want.part with
    | .exactSymbol s => part.symbol == s
    | .classified k => part.partKind.any (kindAccepts k)
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

/-- Renaming printed labels preserves the directed pair when its terminal partition is unchanged. -/
def pairNets (net : Terminal → Nat) (positive negative : Terminal) := (net positive, net negative)

theorem pair_keeps_polarity {before after : Terminal → Nat} (p n : Terminal)
    (hp : before p = after p) (hn : before n = after n) :
    pairNets before p n = pairNets after p n := by simp [pairNets, hp, hn]

def pullWitness (enabled : Bool) (wanted offered : VoltageInterval) (ohmsOK : Bool) :=
  enabled && acceptsVoltage wanted offered && ohmsOK

theorem absent_pull_cannot_supply (w g : VoltageInterval) (r : Bool) :
    pullWitness false w g r = false := rfl

/-- Ordered bus net identities after fitting and resolving both lanes. -/
def i2cConflict (busA busB : Nat × Nat) (addrA addrB : Nat) : Bool :=
  decide (busA = busB ∧ addrA = addrB)

theorem separate_buses_reuse_address (a b : Nat × Nat) (address : Nat) (h : a ≠ b) :
    i2cConflict a b address address = false := by simp [i2cConflict, h]

theorem same_bus_same_address (bus : Nat × Nat) (address : Nat) :
    i2cConflict bus bus address address = true := by simp [i2cConflict]

/-- One address is exclusive among fitted devices on the same ordered pair of bus nets. -/
def fittedAddressConflict (fitA fitB : Bool) (busA busB : Nat × Nat) (addrA addrB : Nat) : Bool :=
  fitA && fitB && i2cConflict busA busB addrA addrB

theorem fitted_address_conflict_iff (fitA fitB : Bool) (busA busB : Nat × Nat) (addrA addrB : Nat) :
    fittedAddressConflict fitA fitB busA busB addrA addrB = true ↔
      fitA = true ∧ fitB = true ∧ busA = busB ∧ addrA = addrB := by
  simp [fittedAddressConflict, i2cConflict, and_assoc]

/-- A declared pin window contains the entire provided interval. Endpoints are inclusive. -/
def windowAccepts (lo hi offeredLo offeredHi : Int) : Bool :=
  decide (lo ≤ offeredLo ∧ offeredHi ≤ hi)

theorem window_accepts_boundary (lo hi : Int) : windowAccepts lo hi lo hi = true := by
  simp [windowAccepts]

theorem window_rejects_partial_overlap (lo hi offeredLo offeredHi : Int)
    (h : offeredLo < lo ∨ hi < offeredHi) : windowAccepts lo hi offeredLo offeredHi = false := by
  simp only [windowAccepts, decide_eq_false_iff_not]
  omega

/-- A unit declaration scales a reading into the attribute type's base unit. -/
def inBaseUnit (value factor : Rat) : Rat := value * factor

theorem base_unit_identity (value : Rat) : inBaseUnit value 1 = value := by
  simp [inBaseUnit]

theorem unit_reading_roundtrip (value factor : Rat) (h : factor ≠ 0) :
    inBaseUnit value factor / factor = value := by
  unfold inBaseUnit
  grind

/-- Appending a valid digit and extracting it recovers both parts in any positive base.
    The Rust loader restricts printable encodings to bases 2 through 36. -/
theorem encoding_digit_roundtrip (base value digit : Nat)
    (positive : 0 < base) (inRange : digit < base) :
    (digit + base * value) / base = value ∧ (digit + base * value) % base = digit := by
  simp [Nat.add_mul_div_left, Nat.add_mul_mod_self_left, Nat.div_eq_of_lt inRange,
    Nat.mod_eq_of_lt inRange, positive]

/-- Two partial table lookups compose, including the reverse path on each admitted pair. -/
theorem conversion_chain_roundtrip {A B C : Type} (ab : A → Option B) (ba : B → Option A)
    (bc : B → Option C) (cb : C → Option B) {a : A} {b : B} {c : C}
    (forwardAB : ab a = some b) (forwardBC : bc b = some c)
    (reverseBA : ba b = some a) (reverseCB : cb c = some b) :
    (ab a).bind bc = some c ∧ (cb c).bind ba = some a := by
  simp [forwardAB, forwardBC, reverseBA, reverseCB]

end ForgeLang.Schematic
