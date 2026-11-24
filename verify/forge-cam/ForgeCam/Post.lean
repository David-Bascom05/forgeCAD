/-
G6, the post (research/cam/design.md §5.2; kernel/forge-post): the printed program, read back,
gives the same motions as the program it was printed from, exactly. The obligation is on the
quantised program, whose every coordinate is within half a unit of the last digit of the plan.

The model is forge-post's motion subset in integer units (10^-d mm): rapids, feeds and arcs (with
I, J from the start), tool changes, absolute coordinates (G90). The printer is modal, as
forge-post's: a G word only when the motion mode changes, an axis only when its value changes
(arcs always state X and Y), F only when the feed changes, and a block with nothing but a G is
left out (a null move). The reader keeps the control's modal state.

The round trip is `read (print p) = normalize (motions p)`, where `normalize` drops the null
moves, as forge-post's `round_trip` compares. `print_faulty` is forge-post's printer as of 155e2f2,
which commits the mode before it drops a null move. Its round trip fails, and
`faulty_counterexample` shows the plunge it turns into a rapid.
-/
import Mathlib

namespace ForgeCam.Post

abbrev Q3 := ℤ × ℤ × ℤ

/-- A record of the quantised program. -/
inductive Rec
  | rapid (v : Q3)
  | line (v : Q3) (f : ℤ)
  | arc (dst : Q3) (c : ℤ × ℤ) (cw : Bool) (f : ℤ)
  | tool (t : ℕ)
  deriving DecidableEq, Repr

/-- What the machine does. -/
inductive Motion
  | rapid (v : Q3)
  | feed (v : Q3) (f : ℤ)
  | arc (dst : Q3) (c : ℤ × ℤ) (cw : Bool) (f : ℤ)
  | tool (t : ℕ)
  deriving DecidableEq, Repr

def Rec.motion : Rec → Motion
  | .rapid v => .rapid v
  | .line v f => .feed v f
  | .arc dst c cw f => .arc dst c cw f
  | .tool t => .tool t

def motions (p : List Rec) : List Motion := p.map Rec.motion

/-- The motion less its null moves (a rapid or feed to where the tool already is). -/
def nstep (at_ : Option Q3) (m : Motion) : List Motion × Option Q3 :=
  match m with
  | .rapid v | .feed v _ => if at_ = some v then ([], at_) else ([m], some v)
  | .arc dst _ _ _ => ([m], some dst)
  | .tool _ => ([m], none)

def normalizeFrom : Option Q3 → List Motion → List Motion
  | _, [] => []
  | at_, m :: ms => (nstep at_ m).1 ++ normalizeFrom (nstep at_ m).2 ms

def normalize (ms : List Motion) : List Motion := normalizeFrom none ms

/-- A block: the words it holds, each letter at most once (the dialect's text writes them in
this order). -/
structure Block where
  g : Option ℕ := none
  x : Option ℤ := none
  y : Option ℤ := none
  z : Option ℤ := none
  i : Option ℤ := none
  j : Option ℤ := none
  f : Option ℤ := none
  t : Option ℕ := none
  deriving DecidableEq, Repr

/-- The modal state, the printer's and the control's alike. -/
structure State where
  x : Option ℤ := none
  y : Option ℤ := none
  z : Option ℤ := none
  feed : Option ℤ := none
  mode : Option ℕ := none
  deriving DecidableEq, Repr

/-- A word for a modal value: written iff forced or changed. -/
def modal (force : Bool) (slot : Option ℤ) (v : ℤ) : Option ℤ :=
  if force ∨ slot ≠ some v then some v else none

/-- The position the state knows. -/
def State.pos (s : State) : Option Q3 :=
  match s.x, s.y, s.z with
  | some x, some y, some z => some (x, y, z)
  | _, _, _ => none

/-- An arc or a tool change. `none` is the printer's refusal (an arc from an unknown position). -/
def printRec (st : State) : Rec → Option (List Block × State)
  | .rapid _ | .line _ _ => none  -- printMove's
  | .arc dst c cw f =>
    match st.x, st.y with
    | some sx, some sy =>
      let g := if cw then 2 else 3
      let b : Block := { g := some g, x := some dst.1, y := some dst.2.1,
                         z := modal false st.z dst.2.2, i := some (c.1 - sx), j := some (c.2 - sy),
                         f := modal false st.feed f }
      some ([b], { x := some dst.1, y := some dst.2.1, z := some dst.2.2, feed := some f,
                   mode := some g })
    | _, _ => none
  | .tool t => some ([{ t := some t }], { st with x := none, y := none, z := none, mode := none })

/-- A rapid (g = 0) or feed (g = 1) to v, with the feed for g = 1. `faulty` is forge-post's order
at 155e2f2: the mode committed before a null move is dropped. -/
def printMove (faulty : Bool) (st : State) (g : ℕ) (v : Q3) (f : Option ℤ) :
    List Block × State :=
  let gw := if st.mode ≠ some g then some g else none
  let xw := modal false st.x v.1
  let yw := modal false st.y v.2.1
  let zw := modal false st.z v.2.2
  let fw := match f with
    | some f => modal false st.feed f
    | none => none
  let st' : State := { x := some v.1, y := some v.2.1, z := some v.2.2,
                       feed := match f with | some f => some f | none => st.feed,
                       mode := some g }
  if xw = none ∧ yw = none ∧ zw = none ∧ fw = none then
    ([], if faulty then { st with mode := some g } else st)
  else
    ([{ g := gw, x := xw, y := yw, z := zw, f := fw }], st')

def printOne (faulty : Bool) (st : State) : Rec → Option (List Block × State)
  | .rapid v => some (printMove faulty st 0 v none)
  | .line v f => some (printMove faulty st 1 v (some f))
  | r => printRec st r

def printFrom (faulty : Bool) : State → List Rec → Option (List Block)
  | _, [] => some []
  | st, r :: rs => do
    let (bs, st') ← printOne faulty st r
    let rest ← printFrom faulty st' rs
    pure (bs ++ rest)

/-- The printer (forge-post's `print`, motion records). -/
def print (p : List Rec) : Option (List Block) := printFrom false {} p
def print_faulty (p : List Rec) : Option (List Block) := printFrom true {} p

/-- One block, as the control reads it: `none` is the reader's refusal. -/
def readBlock (st : State) (b : Block) : Option (List Motion × State) :=
  match b.t with
  | some t => some ([.tool t], { st with x := none, y := none, z := none, mode := none })
  | none =>
    let mode := b.g.or st.mode
    let feed := b.f.or st.feed
    if b.x = none ∧ b.y = none ∧ b.z = none then
      some ([], { st with mode := mode, feed := feed })
    else
      let x := b.x.or st.x
      let y := b.y.or st.y
      let z := b.z.or st.z
      let st' : State := { x := x, y := y, z := z, feed := feed, mode := mode }
      match mode, x, y, z with
      | some 0, some x, some y, some z => some ([.rapid (x, y, z)], st')
      | some 1, some x, some y, some z =>
        match feed with
        | some f => some ([.feed (x, y, z) f], st')
        | none => none
      | some g, some x, some y, some z =>
        if g = 2 ∨ g = 3 then
          match st.x, st.y, b.i, b.j, feed with
          | some sx, some sy, some i, some j, some f =>
            some ([.arc (x, y, z) (sx + i, sy + j) (g = 2) f], st')
          | _, _, _, _, _ => none
        else none
      | _, _, _, _ => none

def readFrom : State → List Block → Option (List Motion)
  | _, [] => some []
  | st, b :: bs => do
    let (ms, st') ← readBlock st b
    let rest ← readFrom st' bs
    pure (ms ++ rest)

/-- The reader (forge-post's `read`, motion words). -/
def read (bs : List Block) : Option (List Motion) := readFrom {} bs

/-- The printer's invariant: the position is wholly known or wholly unknown. -/
def Coherent (s : State) : Prop :=
  (s.x = none ∧ s.y = none ∧ s.z = none) ∨ (s.x.isSome ∧ s.y.isSome ∧ s.z.isSome)

theorem modal_none {slot : Option ℤ} {v : ℤ} : modal false slot v = none ↔ slot = some v := by
  unfold modal; by_cases h : slot = some v <;> simp [h]

theorem or_modal (slot : Option ℤ) (v : ℤ) : (modal false slot v).or slot = some v := by
  unfold modal; by_cases h : slot = some v <;> simp [h]

theorem coherent_cases (st : State) (hc : Coherent st) :
    (st.x = none ∧ st.y = none ∧ st.z = none) ∨
    ∃ x y z, st.x = some x ∧ st.y = some y ∧ st.z = some z := by
  rcases hc with h | ⟨hx, hy, hz⟩
  · exact Or.inl h
  · exact Or.inr ⟨_, _, _, Option.some_get hx |>.symm, Option.some_get hy |>.symm,
      Option.some_get hz |>.symm⟩

@[simp] theorem bind_append_eq_map (o : Option (List Motion)) (l : List Motion) :
    (o.bind fun r => some (l ++ r)) = o.map (l ++ ·) := by cases o <;> rfl

theorem readFrom_nil_append (st : State) (rest : List Block) :
    readFrom st ([] ++ rest) = (readFrom st rest).map ([] ++ ·) := by simp

theorem readFrom_single (st st' : State) (b : Block) (ms : List Motion) (rest : List Block)
    (h : readBlock st b = some (ms, st')) :
    readFrom st ([b] ++ rest) = (readFrom st' rest).map (ms ++ ·) := by
  simp only [List.singleton_append, readFrom, h, Option.bind_eq_bind, Option.bind_some]
  cases readFrom st' rest <;> rfl

-- The coherent-state split leaves some branches closed by the first `simp`: the linters see
-- the shared tactic script as unused there.
set_option linter.unusedTactic false in
set_option linter.unnecessarySeqFocus false in
set_option maxHeartbeats 4000000 in
/-- **One record.** The control, reading the printer's blocks for a record from the printer's
own state, ends in the same state and does what the record does, less a null move. -/
theorem step (st : State) (hc : Coherent st) (r : Rec) (bs : List Block) (st' : State)
    (hp : printOne false st r = some (bs, st')) :
    Coherent st' ∧ st'.pos = (nstep st.pos r.motion).2 ∧
    ∀ rest, readFrom st (bs ++ rest) = (readFrom st' rest).map ((nstep st.pos r.motion).1 ++ ·) := by
  rcases st with ⟨x, y, z, feed, mode⟩
  rcases coherent_cases _ hc with ⟨hx, hy, hz⟩ | ⟨x, y, z, hx, hy, hz⟩ <;>
    simp only at hx hy hz <;> subst hx hy hz <;>
  cases r with
  | rapid v =>
    obtain ⟨v1, v2, v3⟩ := v
    simp only [printOne, printMove, modal, Option.some.injEq] at hp
    split_ifs at hp <;> obtain ⟨rfl, rfl⟩ := hp <;>
      refine ⟨?_, ?_, fun rest => ?_⟩ <;> simp_all [Coherent, State.pos, nstep, Rec.motion,
        readFrom, readBlock, bind_append_eq_map]
  | line v f =>
    obtain ⟨v1, v2, v3⟩ := v
    simp only [printOne, printMove, modal, Option.some.injEq] at hp
    split_ifs at hp <;> obtain ⟨rfl, rfl⟩ := hp <;>
      refine ⟨?_, ?_, fun rest => ?_⟩ <;> simp_all [Coherent, State.pos, nstep, Rec.motion,
        readFrom, readBlock, bind_append_eq_map]
  | arc dst c cw f =>
    obtain ⟨d1, d2, d3⟩ := dst
    simp only [printOne, printRec, modal, Option.some.injEq, reduceCtorEq] at hp
    all_goals
      obtain ⟨rfl, rfl⟩ := hp
      refine ⟨?_, ?_, fun rest => ?_⟩ <;> simp_all [Coherent, State.pos, nstep, Rec.motion,
        readFrom, readBlock, bind_append_eq_map] <;> cases cw <;> split_ifs <;> simp_all
  | tool t =>
    simp only [printOne, printRec, Option.some.injEq] at hp
    obtain ⟨rfl, rfl⟩ := hp
    refine ⟨?_, ?_, fun rest => ?_⟩ <;> simp_all [Coherent, State.pos, nstep, Rec.motion,
        readFrom, readBlock, bind_append_eq_map]

theorem roundTrip_from (p : List Rec) : ∀ (st : State) (bs : List Block), Coherent st →
    printFrom false st p = some bs → readFrom st bs = some (normalizeFrom st.pos (motions p)) := by
  induction p with
  | nil =>
    intro st bs _ h
    simp only [printFrom, Option.some.injEq] at h; subst h; rfl
  | cons r rs ih =>
    intro st bs hc h
    simp only [printFrom, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨⟨bs1, st1⟩, h1, rest, h2, rfl⟩ := h
    obtain ⟨hc1, hpos, hread⟩ := step st hc r bs1 st1 h1
    rw [hread, ih st1 rest hc1 h2, hpos]
    rfl

/-- **G6, the round trip.** Whatever the printer prints, the control reads back as exactly the
program's motions, less its null moves. -/
theorem round_trip (p : List Rec) (bs : List Block) (h : print p = some bs) :
    read bs = some (normalize (motions p)) :=
  roundTrip_from p {} bs (Or.inl ⟨rfl, rfl, rfl⟩) h

/-! ### The printer as of 155e2f2 -/

/-- The case the model found: a null feed (the feed unchanged) after a rapid, then a plunge.
(Integer units of 10^-3 mm.) -/
def nullFeedThenPlunge : List Rec :=
  [.tool 1, .rapid (0, 0, 5000), .line (0, 0, 0) 1000, .rapid (0, 0, 5000),
   .line (0, 0, 5000) 1000, .line (0, 0, -3000) 1000]

/-- **The faulty printer fails G6**: the plunge to z = −3 is read back as a rapid. -/
theorem faulty_counterexample :
    (print_faulty nullFeedThenPlunge).bind read =
      some [.tool 1, .rapid (0, 0, 5000), .feed (0, 0, 0) 1000, .rapid (0, 0, 5000),
            .rapid (0, 0, -3000)] ∧
    normalize (motions nullFeedThenPlunge) =
      [.tool 1, .rapid (0, 0, 5000), .feed (0, 0, 0) 1000, .rapid (0, 0, 5000),
       .feed (0, 0, -3000) 1000] := by
  decide

/-- The fixed printer on the same program: read back exactly. -/
theorem fixed_on_counterexample :
    (print nullFeedThenPlunge).bind read = some (normalize (motions nullFeedThenPlunge)) := by
  decide

/-! ### Quantisation: the plan to integer units -/

/-- m is a quantisation of x at d digits: within half a unit of x · 10^d. -/
def IsQuant (d : ℕ) (x : ℚ) (m : ℤ) : Prop := |(m : ℚ) - x * 10 ^ d| ≤ 1 / 2

/-- The quantised value is within half a unit of the last digit of the plan. -/
theorem quant_err {d : ℕ} {x : ℚ} {m : ℤ} (h : IsQuant d x m) :
    |(m : ℚ) / 10 ^ d - x| ≤ 1 / (2 * 10 ^ d) := by
  have hp : (0 : ℚ) < 10 ^ d := by positivity
  have e : (m : ℚ) / 10 ^ d - x = ((m : ℚ) - x * 10 ^ d) / 10 ^ d := by field_simp
  rw [e, abs_div, abs_of_pos hp, div_le_iff₀ hp]
  have : 1 / (2 * 10 ^ d) * (10 : ℚ) ^ d = 1 / 2 := by field_simp
  rw [this]; exact h

/-- The nearest integer, ties to even (forge-post's `quant`). -/
def roundEven (x : ℚ) : ℤ :=
  let f := ⌊x⌋
  let r := x - f
  if r < 1 / 2 then f else if 1 / 2 < r then f + 1 else if f % 2 = 0 then f else f + 1

theorem roundEven_near (x : ℚ) : |(roundEven x : ℚ) - x| ≤ 1 / 2 := by
  have h1 := Int.floor_le x
  have h2 := Int.lt_floor_add_one x
  unfold roundEven
  simp only
  split_ifs with ha hb <;> push_cast <;> rw [abs_le] <;> constructor <;> linarith

/-- forge-post's quantiser is a quantisation. -/
theorem roundEven_isQuant (d : ℕ) (x : ℚ) : IsQuant d x (roundEven (x * 10 ^ d)) :=
  roundEven_near _

/-- A record of the plan, in exact rationals (mm, mm/min). -/
inductive PlanRec
  | rapid (v : ℚ × ℚ × ℚ)
  | line (v : ℚ × ℚ × ℚ) (f : ℚ)
  | arc (dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ)
  | tool (t : ℕ)

/-- One coordinate at d digits. -/
def qc (d : ℕ) (x : ℚ) : ℤ := roundEven (x * 10 ^ d)

def q3 (d : ℕ) (v : ℚ × ℚ × ℚ) : Q3 := (qc d v.1, qc d v.2.1, qc d v.2.2)

/-- The plan in integer units: coordinates at d digits, feeds at e. -/
def quantize (d e : ℕ) : PlanRec → Rec
  | .rapid v => .rapid (q3 d v)
  | .line v f => .line (q3 d v) (qc e f)
  | .arc dst c cw f => .arc (q3 d dst) (qc d c.1, qc d c.2) cw (qc e f)
  | .tool t => .tool t

/-- **G6.** The program printed from the quantised plan reads back as the quantised plan's
motions (less null moves), and every number of it is within half a unit of its last digit of
the plan's. The checks G1–G5 are therefore run on the quantised plan g′, never on g. -/
theorem G6 (d e : ℕ) (plan : List PlanRec) (bs : List Block)
    (h : print (plan.map (quantize d e)) = some bs) :
    read bs = some (normalize (motions (plan.map (quantize d e)))) ∧
    ∀ x : ℚ, |((qc d x : ℤ) : ℚ) / 10 ^ d - x| ≤ 1 / (2 * 10 ^ d) :=
  ⟨round_trip _ bs h, fun x => quant_err (roundEven_isQuant d x)⟩

/-! ### Arc semantics: the vanishing arc

A control reads an arc whose end has the start's X and Y as a full circle (G17; a helix if Z
moves). forge-post's quantiser (0a4c189) turns a *short* arc (less than a half turn, by the cross
test) whose end rounds onto its start into a line. The question is what "onto its start" means:
- the rule as written in 0a4c189 compares all three coordinates (`q3 to = q3 s`). A short helical
  arc whose X, Y round onto the start while Z moves is kept, and the control cuts a full helical
  turn (`arc3_unsafe`): a 0.0002 mm ramp piece becomes a 360° helix of the arc's radius;
- comparing X and Y (`quantizeArc2`) is safe: a posted arc read as a full circle was planned as a
  full turn, or as more than a half turn (`arc2_safe`). -/

/-- The plan's end has the start's X and Y. -/
def xyEq (a b : ℚ × ℚ × ℚ) : Prop := a.1 = b.1 ∧ a.2.1 = b.2.1

instance (a b : ℚ × ℚ × ℚ) : Decidable (xyEq a b) := by unfold xyEq; infer_instance

/-- X and Y round to the same digits. -/
def q2eq (d : ℕ) (a b : ℚ × ℚ × ℚ) : Prop := qc d a.1 = qc d b.1 ∧ qc d a.2.1 = qc d b.2.1

instance (d : ℕ) (a b : ℚ × ℚ × ℚ) : Decidable (q2eq d a b) := by unfold q2eq; infer_instance

/-- forge-post's half-turn test: the end on the turning side of the start (or on its line). -/
def shortArc (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) : Prop :=
  let cr := (s.1 - c.1) * (dst.2.1 - c.2) - (s.2.1 - c.2) * (dst.1 - c.1)
  if cw then cr ≤ 0 else 0 ≤ cr

instance (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) : Decidable (shortArc s dst c cw) := by
  unfold shortArc; infer_instance

/-- The quantiser's arc rule at 0a4c189: all three coordinates compared. -/
def quantizeArc3 (d e : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ) : Rec :=
  if dst ≠ s ∧ q3 d dst = q3 d s ∧ shortArc s dst c cw then .line (q3 d dst) (qc e f)
  else .arc (q3 d dst) (qc d c.1, qc d c.2) cw (qc e f)

/-- The safe rule: X and Y compared (the plane the control decides a full circle in). -/
def quantizeArc2 (d e : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ) : Rec :=
  if ¬ xyEq dst s ∧ q2eq d dst s ∧ shortArc s dst c cw then .line (q3 d dst) (qc e f)
  else .arc (q3 d dst) (qc d c.1, qc d c.2) cw (qc e f)

/-- The control reads a posted record from `start` as a full circle. -/
def readsFull (start : Q3) : Rec → Prop
  | .arc dst _ _ _ => start.1 = dst.1 ∧ start.2.1 = dst.2.1
  | _ => False

/-- **The safe rule never turns a short arc into a full circle**: a posted arc the control reads
as a full circle was planned as a full turn or as more than a half turn. -/
theorem arc2_safe (d e : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ)
    (h : readsFull (q3 d s) (quantizeArc2 d e s dst c cw f)) :
    xyEq dst s ∨ ¬ shortArc s dst c cw := by
  unfold quantizeArc2 at h
  split_ifs at h with hc
  · exact h.elim
  · by_contra hn
    push Not at hn
    apply hc
    refine ⟨hn.1, ?_, hn.2⟩
    simp only [readsFull, q3] at h
    exact ⟨h.1.symm, h.2.symm⟩

/-- **The rule at 0a4c189 is unsafe for helical arcs**: a short arc whose X, Y round onto its
start while its Z rounds elsewhere is posted as an arc, and the control reads it as a full
circle (a full helical turn). -/
theorem arc3_unsafe (d e : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ)
    (hxy : q2eq d dst s) (hz : qc d dst.2.2 ≠ qc d s.2.2) :
    readsFull (q3 d s) (quantizeArc3 d e s dst c cw f) := by
  unfold quantizeArc3
  rw [if_neg (fun h => hz (by have := h.2.1; simp only [q3, Prod.mk.injEq] at this; exact this.2.2))]
  simp only [readsFull, q3]
  exact ⟨hxy.1.symm, hxy.2.symm⟩

/-- forge-post's meant `full` (4bfe2a2): the plan's X, Y end on its start, or they round onto it
over more than a half turn. -/
def meantFull (d : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) : Prop :=
  xyEq dst s ∨ (q2eq d dst s ∧ ¬ shortArc s dst c cw)

/-- **The meant `full` is what the control reads**, for every arc the safe rule posts: the
round trip's `full` on the two sides agree (forge-post 4bfe2a2 compares them). -/
theorem arcFull_agrees (d e : ℕ) (s dst : ℚ × ℚ × ℚ) (c : ℚ × ℚ) (cw : Bool) (f : ℚ)
    {dq : Q3} {cq : ℤ × ℤ} {cwq : Bool} {fq : ℤ}
    (h : quantizeArc2 d e s dst c cw f = .arc dq cq cwq fq) :
    readsFull (q3 d s) (quantizeArc2 d e s dst c cw f) ↔ meantFull d s dst c cw := by
  unfold quantizeArc2 at h ⊢
  split_ifs at h ⊢ with hc
  simp only [readsFull, q3, meantFull]
  constructor
  · intro hr
    have hq : q2eq d dst s := ⟨hr.1.symm, hr.2.symm⟩
    by_cases hx : xyEq dst s
    · exact Or.inl hx
    · exact Or.inr ⟨hq, fun hs => hc ⟨hx, hq, hs⟩⟩
  · rintro (hx | ⟨hq, -⟩)
    · exact ⟨by rw [hx.1], by rw [hx.2]⟩
    · exact ⟨hq.1.symm, hq.2.symm⟩

-- The ramp piece found in forge-post at 99f39bf (fanuc, 3 digits): from (5, 0, 0), ccw about the
-- origin to (5 − 2e-9, 0.0002, −0.5). Short, X and Y round onto the start, Z does not.
#guard decide (shortArc (5, 0, 0) (5 - 2/1000000000, 2/10000, -1/2) (0, 0) false)
#guard decide (q2eq 3 (5 - 2/1000000000, 2/10000, -1/2) (5, 0, 0))
#guard decide (qc 3 (-1/2 : ℚ) ≠ qc 3 0)
#guard quantizeArc3 3 1 (5, 0, 0) (5 - 2/1000000000, 2/10000, -1/2) (0, 0) false 100 ==
  .arc (5000, 0, -500) (0, 0) false 1000
#guard quantizeArc2 3 1 (5, 0, 0) (5 - 2/1000000000, 2/10000, -1/2) (0, 0) false 100 ==
  .line (5000, 0, -500) 1000

end ForgeCam.Post
