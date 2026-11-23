/-
The G-code interpreter's spec (kernel/forge-machine/src/gcode.rs), at the level of parsed words
in integer units (10^-d mm). A block holds at most one motion code, the distance mode (G90/G91),
the retract mode (G98/G99), G28, the axis words, the arc centre (I, J: the XY plane), R, Q, F and
T. The state is the control's modal state: the position (per axis, unknown until said), the
distance and retract modes, the motion mode, the feed and the active canned cycle.

What is proven here:
- `holds`: in G90 an axis without a word keeps its value; `incremental_eq_absolute`: a run of
  G91 moves does what the same moves written as absolute prefix sums do.
- The canned cycles: a peck's depths are strictly decreasing, between the bottom and R, end at the
  bottom, and number ⌈(R − Z)/Q⌉ (`pecks_*`); every hole ends at the retract level
  (`hole_last`) and never feeds below the bottom (`hole_feeds_above`).
- `home_forgets`: after G28 the homed axes are unknown to the program (their value is the machine's
  reference, which the work offsets place).
- An arc's `full` flag is the control's rule by definition (its end's X and Y are its start's),
  the rule forge-cam's `arcFull_agrees` shows the post's meaning matches.
- `refines_post`: on forge-cam's post language (ForgeCam.Post: G0–G3, X Y Z I J F, T), whatever
  forge-cam's control model reads, this interpreter does the same.
- `posted_program`: with forge-cam's G6 round trip, the interpreter run on what the printer prints
  does exactly the planned motion, less null moves.
-/
import ForgeCam.Post

namespace ForgeMachine

/-- A position: per axis, a value in integer units, or unknown. -/
abbrev Pos (n : ℕ) := Fin n → Option ℤ

/-- Which axis is X, Y and Z. -/
structure Axes (n : ℕ) where
  x : Fin n
  y : Fin n
  z : Fin n

structure Block (n : ℕ) where
  /-- The motion code: 0 1 2 3, 80 (cancel), 81 82 83 84 85 73 (cycles). -/
  g : Option ℕ := none
  /-- G90 (true) or G91 (false). -/
  absolute : Option Bool := none
  /-- G98 (true) or G99 (false). -/
  retractInitial : Option Bool := none
  /-- G17 G18 G19. -/
  plane : Option ℕ := none
  /-- G28. -/
  home : Bool := false
  ax : Pos n := fun _ => none
  i : Option ℤ := none
  j : Option ℤ := none
  k : Option ℤ := none
  r : Option ℤ := none
  q : Option ℤ := none
  /-- A cycle's dwell P (ms). -/
  p : Option ℤ := none
  f : Option ℤ := none
  t : Option ℕ := none

structure Cyc where
  g : ℕ
  z : ℤ
  r : ℤ
  q : ℤ
  dwell : ℤ
  initial : ℤ
  deriving DecidableEq

structure State (n : ℕ) where
  pos : Pos n := fun _ => none
  absolute : Bool := true
  mode : Option ℕ := none
  plane : ℕ := 17
  feed : Option ℤ := none
  retractInitial : Bool := true
  cyc : Option Cyc := none

/-- An arc's centre: absolute plane coordinates, or the radius word. -/
inductive Centre
  | abs (a b : ℤ)
  | r (v : ℤ)
  deriving DecidableEq

inductive Ev (n : ℕ)
  | rapid (dst : Pos n)
  | feed (dst : Pos n) (f : ℤ)
  | arc (dst : Pos n) (plane : ℕ) (c : Centre) (cw full : Bool) (f : ℤ)
  | home (via : Pos n) (axes : List (Fin n))
  | dwell (ms : ℤ)
  | tool (t : ℕ)

/-- The target of a block's axis words: absolute words replace; incremental ones add to a known
value (`none`, a refusal, from an unknown one). -/
def target {n : ℕ} (absolute : Bool) (pos ax : Pos n) : Option (Pos n) :=
  if absolute then some fun k => (ax k).or (pos k)
  else if ∀ k, (ax k).isSome → (pos k).isSome then
    some fun k => match ax k, pos k with
      | some d, some p => some (p + d)
      | _, p => p
  else none

/-! ### Canned cycles -/

/-- A peck's depths from `d` down to the bottom `z`, `s` at a time. -/
def pecks (z s : ℤ) (hs : 0 < s) (d : ℤ) : List ℤ :=
  if _h : z < d then
    let next := max (d - s) z
    next :: pecks z s hs next
  else []
termination_by (d - z).toNat
decreasing_by omega

/-- `put x y h`: the position `p` with X, Y and Z set. -/
def put {n : ℕ} (A : Axes n) (p : Pos n) (x y h : ℤ) : Pos n :=
  Function.update (Function.update (Function.update p A.x (some x)) A.y (some y)) A.z (some h)

/-- The events of one hole at (x, y) from the current height `cur` (forge-post's `expand_hole`;
gcode.rs's cycle arm): to (x, y) at the current height, to R, the cut, back to `back`. `pc` and
`lift` are the peck clearance and the chip-break lift in integer units. -/
def hole {n : ℕ} (A : Axes n) (base : Pos n) (c : Cyc) (cur back pc lift : ℤ) (f : ℤ) (x y : ℤ) :
    List (Ev n) :=
  let at_ := put A base x y
  let cut : List (Ev n) :=
    if c.g = 83 ∨ c.g = 73 then
      if hs : 0 < (if 0 < c.q then c.q else c.r - c.z) then
        (pecks c.z _ hs c.r).flatMap fun d =>
          [.feed (at_ d) f] ++
            (if c.z < d then
              (if c.g = 83 then [.rapid (at_ c.r), .rapid (at_ (d + pc))] else [.rapid (at_ (d + lift))])
            else [])
      else []
    else
      [.feed (at_ c.z) f] ++ (if c.g = 82 ∧ 0 < c.dwell then [.dwell c.dwell] else []) ++
        (if c.g = 85 ∨ c.g = 84 then [.feed (at_ c.r) f] else [])
  [.rapid (at_ cur), .rapid (at_ c.r)] ++ cut ++ [.rapid (at_ back)]

/-- One block. `none` is the interpreter's refusal. -/
def step {n : ℕ} (A : Axes n) (pc lift : ℤ) (σ : State n) (b : Block n) :
    Option (List (Ev n) × State n) :=
  match b.t with
  | some t => some ([.tool t], σ)
  | none =>
  let feed := b.f.or σ.feed
  let absolute := b.absolute.getD σ.absolute
  let ri := b.retractInitial.getD σ.retractInitial
  let plane := b.plane.getD σ.plane
  let σ1 : State n := { σ with feed := feed, absolute := absolute, retractInitial := ri,
                                plane := plane }
  let anyAx := (List.finRange n).any fun k => (b.ax k).isSome
  if b.home then
    match target absolute σ.pos b.ax with
    | none => none
    | some via =>
      let axes := (List.finRange n).filter fun k => (b.ax k).isSome
      if axes = [] then none
      else some ([.home via axes], { σ1 with pos := fun k => if (b.ax k).isSome then none else via k })
  else
  -- the motion code
  let σ2? : Option (State n) :=
    match b.g with
    | none => some σ1
    | some g =>
      if g ≤ 3 then some { σ1 with mode := some g, cyc := none }
      else if g = 80 then some { σ1 with mode := none, cyc := none }
      else if g = 81 ∨ g = 82 ∨ g = 83 ∨ g = 84 ∨ g = 85 ∨ g = 73 then
        match b.ax A.z, b.r, σ.pos A.z with
        | some z, some r, some init =>
          if plane = 17 ∧ absolute ∧ z ≤ r ∧ r ≤ init ∧ 0 ≤ b.q.getD 0 then
            some { σ1 with mode := some 80, cyc := some ⟨g, z, r, b.q.getD 0, b.p.getD 0, init⟩ }
          else none
        | _, _, _ => none
      else none
  match σ2? with
  | none => none
  | some σ2 =>
  if anyAx = false then some ([], σ2) else
  match target absolute σ.pos b.ax with
  | none => none
  | some dst =>
  match σ2.mode, σ2.cyc with
  | some 80, some c =>
    -- a Z word repeats the bottom only with the cycle's own code
    if (b.ax A.z).isSome ∧ b.g = none then none else
    match dst A.x, dst A.y, σ.pos A.z, feed with
    | some x, some y, some cur, some f =>
      let back := if ri then c.initial else c.r
      some (hole A dst c cur back pc lift f x y, { σ2 with pos := put A dst x y back })
    | _, _, _, _ => none
  | some 0, _ => some ([.rapid dst], { σ2 with pos := dst })
  | some 1, _ =>
    match feed with
    | some f => some ([.feed dst f], { σ2 with pos := dst })
    | none => none
  | some g, _ =>
    if g = 2 ∨ g = 3 then
      -- the plane's axes and centre words: G17 X Y (I J), G18 Z X (K I), G19 Y Z (J K)
      let (pa, pb, wa, wb) :=
        if plane = 17 then (A.x, A.y, b.i, b.j)
        else if plane = 18 then (A.z, A.x, b.k, b.i)
        else (A.y, A.z, b.j, b.k)
      match σ.pos pa, σ.pos pb, dst pa, dst pb, feed with
      | some sa, some sb, some ea, some eb, some f =>
        let full := decide (ea = sa) && decide (eb = sb)
        let c? : Option Centre := match b.r, wa, wb with
          | some r, none, none => if r = 0 ∨ full then none else some (.r r)
          | some _, _, _ => none
          | none, none, none => none
          | none, ia, ib => some (.abs (sa + ia.getD 0) (sb + ib.getD 0))
        match c? with
        | none => none
        | some c => some ([.arc dst plane c (g = 2) full f], { σ2 with pos := dst })
      | _, _, _, _, _ => none
    else none
  | none, _ => none

def runFrom {n : ℕ} (A : Axes n) (pc lift : ℤ) : State n → List (Block n) → Option (List (Ev n))
  | _, [] => some []
  | σ, b :: bs => do
    let (es, σ') ← step A pc lift σ b
    let rest ← runFrom A pc lift σ' bs
    pure (es ++ rest)

/-! ### Modal facts -/

/-- In G90 an axis with no word keeps its value. -/
theorem holds {n : ℕ} (pos ax : Pos n) (k : Fin n) (hk : ax k = none) (dst : Pos n)
    (h : target true pos ax = some dst) : dst k = pos k := by
  simp only [target, ite_true, Option.some.injEq] at h
  subst h; simp [hk]

/-- G91 adds to the known position; every axis without a word keeps its value. -/
theorem incremental_target {n : ℕ} (pos ax : Pos n) (dst : Pos n)
    (h : target false pos ax = some dst) (k : Fin n) :
    dst k = match ax k, pos k with
      | some d, some p => some (p + d)
      | _, p => p := by
  simp only [target, Bool.false_eq_true, ite_false] at h
  split_ifs at h with hall
  simp only [Option.some.injEq] at h; subst h; rfl

/-- The absolute form of an incremental target, when every worded axis is known. -/
theorem incremental_eq_absolute {n : ℕ} (pos ax : Pos n)
    (hk : ∀ k, (ax k).isSome → (pos k).isSome) :
    target false pos ax = target true pos (fun k => match ax k, pos k with
      | some d, some p => some (p + d)
      | _, _ => none) := by
  simp only [target, Bool.false_eq_true, ite_false, ite_eq_left hk, ite_true, Option.some.injEq]
  funext k
  have := hk k
  cases hax : ax k <;> cases hp : pos k <;> simp_all

/-! ### Pecks -/

theorem pecks_eq (z s : ℤ) (hs : 0 < s) (d : ℤ) :
    pecks z s hs d = if z < d then max (d - s) z :: pecks z s hs (max (d - s) z) else [] := by
  rw [pecks]; split_ifs <;> rfl

/-- Every peck depth is above the bottom (or at it) and below where it started. -/
theorem pecks_bounds (z s : ℤ) (hs : 0 < s) : ∀ (d x : ℤ), x ∈ pecks z s hs d → z ≤ x ∧ x < d := by
  intro d
  induction h : (d - z).toNat using Nat.strong_induction_on generalizing d with
  | _ m ih =>
    intro x hx
    rw [pecks_eq] at hx
    split_ifs at hx with hzd
    · rcases List.mem_cons.mp hx with rfl | hx
      · exact ⟨le_max_right _ _, by omega⟩
      · have := ih ((max (d - s) z - z).toNat) (by omega) (max (d - s) z) rfl x hx
        exact ⟨this.1, by omega⟩
    · simp at hx

/-- The depths strictly decrease. -/
theorem pecks_sorted (z s : ℤ) (hs : 0 < s) : ∀ d, (pecks z s hs d).Pairwise (· > ·) := by
  intro d
  induction h : (d - z).toNat using Nat.strong_induction_on generalizing d with
  | _ m ih =>
    rw [pecks_eq]
    split_ifs with hzd
    · refine List.pairwise_cons.mpr ⟨fun x hx => ?_, ih _ (by omega) _ rfl⟩
      exact (pecks_bounds z s hs _ x hx).2
    · exact List.Pairwise.nil

/-- The last depth is the bottom. -/
theorem pecks_last (z s : ℤ) (hs : 0 < s) : ∀ d, z < d → (pecks z s hs d).getLast? = some z := by
  intro d
  induction h : (d - z).toNat using Nat.strong_induction_on generalizing d with
  | _ m ih =>
    intro hzd
    rw [pecks_eq, ite_eq_left hzd]
    by_cases hn : z < max (d - s) z
    · rw [List.getLast?_cons, ih _ (by omega) _ rfl hn]; rfl
    · have : max (d - s) z = z := by omega
      rw [this, pecks_eq, ite_eq_right (lt_irrefl z)]; rfl

/-- How many: ⌈(d − z)/s⌉. -/
theorem pecks_length (z s : ℤ) (hs : 0 < s) : ∀ d, z ≤ d →
    ((pecks z s hs d).length : ℤ) = (d - z + s - 1) / s := by
  intro d
  induction h : (d - z).toNat using Nat.strong_induction_on generalizing d with
  | _ m ih =>
    intro hzd
    rw [pecks_eq]
    split_ifs with hlt
    · rw [List.length_cons, Nat.cast_succ,
        show d - z + s - 1 = d - z - 1 + 1 * s by ring, Int.add_mul_ediv_right _ _ hs.ne']
      by_cases hn : d - s ≤ z
      · rw [max_eq_right hn, pecks_eq, ite_eq_right (lt_irrefl z), Int.ediv_eq_zero_of_lt (by omega) (by omega)]
        rfl
      · rw [max_eq_left (by omega), ih _ (by omega) _ rfl (by omega)]
        congr 2; ring
    · have : d = z := by omega
      subst this; simp only [List.length_nil, Nat.cast_zero]
      rw [sub_self, zero_add]
      exact (Int.ediv_eq_zero_of_lt (by omega) (by omega)).symm

/-! ### Holes -/

/-- A hole's last event is the rapid to the retract level. -/
theorem hole_last {n : ℕ} (A : Axes n) (base : Pos n) (c : Cyc) (cur back pc lift f x y : ℤ) :
    (hole A base c cur back pc lift f x y).getLast? = some (.rapid (put A base x y back)) := by
  simp only [hole]
  exact List.getLast?_concat

/-- The feed depths a hole commands, as Z values. -/
def feedDepths {n : ℕ} (A : Axes n) : List (Ev n) → List (Option ℤ)
  | [] => []
  | .feed p _ :: es => p A.z :: feedDepths A es
  | _ :: es => feedDepths A es

theorem feedDepths_append {n : ℕ} (A : Axes n) (a b : List (Ev n)) :
    feedDepths A (a ++ b) = feedDepths A a ++ feedDepths A b := by
  induction a with
  | nil => rfl
  | cons e es ih => cases e <;> simp [feedDepths, ih]

theorem put_z {n : ℕ} (A : Axes n) (p : Pos n) (x y h : ℤ) : put A p x y h A.z = some h := by
  simp [put]

theorem feedDepths_flatMap {n : ℕ} (A : Axes n) {α : Type} (l : List α) (g : α → List (Ev n)) :
    feedDepths A (l.flatMap g) = l.flatMap (fun a => feedDepths A (g a)) := by
  induction l with
  | nil => rfl
  | cons a as ih => simp [List.flatMap_cons, feedDepths_append, ih]

/-- A hole never feeds below its bottom, nor above R (cycles with z ≤ r). -/
theorem hole_feeds_above {n : ℕ} (A : Axes n) (base : Pos n) (c : Cyc) (cur back pc lift f x y : ℤ)
    (hzr : c.z ≤ c.r) :
    ∀ d ∈ feedDepths A (hole A base c cur back pc lift f x y), ∃ v, d = some v ∧ c.z ≤ v ∧ v ≤ c.r := by
  intro d hd
  simp only [hole, feedDepths_append, List.mem_append] at hd
  rcases hd with (hd | hd) | hd
  · simp [feedDepths] at hd
  · split_ifs at hd <;>
      first
      | (rw [feedDepths_flatMap] at hd
         obtain ⟨v, hv, hd⟩ := List.mem_flatMap.mp hd
         have hb := pecks_bounds _ _ _ _ v hv
         refine ⟨v, ?_, hb.1, hb.2.le⟩
         split_ifs at hd <;> simp_all [feedDepths, put_z])
      | (simp [feedDepths, put_z] at hd <;>
         rcases hd with rfl | rfl <;> simp [hzr])
  · simp [feedDepths] at hd

/-! ### G28 and arcs -/

/-- After G28 the homed axes are unknown to the program, the others are at the intermediate
point. -/
theorem home_forgets {n : ℕ} (A : Axes n) (pc lift : ℤ) (σ : State n) (b : Block n)
    (ht : b.t = none) (hh : b.home = true) (es : List (Ev n)) (σ' : State n)
    (h : step A pc lift σ b = some (es, σ')) :
    ∃ via, es = [.home via ((List.finRange n).filter fun k => (b.ax k).isSome)] ∧
      target (b.absolute.getD σ.absolute) σ.pos b.ax = some via ∧
      ∀ k, σ'.pos k = if (b.ax k).isSome then none else via k := by
  simp only [step, ht, hh, ite_true] at h
  split at h
  · simp at h
  · rename_i via hv
    split_ifs at h
    simp only [Option.some.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨via, rfl, hv, fun k => rfl⟩

/-! ### On forge-cam's post language

ForgeCam.Post models forge-post's printer and a control's reading of it: blocks of G0–G3, X Y Z,
I J, F and T, absolute. Embedded here, every block that model's reader accepts, this interpreter
accepts too and does the same: it refines the model. Where they differ, this one knows more: after
a T the model forgets the position and the motion mode (a conservative reading), while the
interpreter keeps them, as a control does (gcode.rs). -/

/-- X Y Z as axes 0 1 2. -/
def xyz : Axes 3 := ⟨0, 1, 2⟩

def embed (b : ForgeCam.Post.Block) : Block 3 :=
  { g := b.g, ax := ![b.x, b.y, b.z], i := b.i, j := b.j, f := b.f, t := b.t }

/-- Our event as the model's motion (none for what it has no word for). -/
def forget : Ev 3 → Option ForgeCam.Post.Motion
  | .rapid p => match p 0, p 1, p 2 with
    | some x, some y, some z => some (.rapid (x, y, z))
    | _, _, _ => none
  | .feed p f => match p 0, p 1, p 2 with
    | some x, some y, some z => some (.feed (x, y, z) f)
    | _, _, _ => none
  | .arc p 17 (.abs ca cb) cw _ f => match p 0, p 1, p 2 with
    | some x, some y, some z => some (.arc (x, y, z) (ca, cb) cw f)
    | _, _, _ => none
  | .arc _ _ _ _ _ _ => none
  | .dwell _ => none
  | .tool t => some (.tool t)
  | .home _ _ => none

/-- The printer's motion codes. -/
def WF (b : ForgeCam.Post.Block) : Prop := ∀ g, b.g = some g → g ≤ 3

/-- What the model's state knows, ours knows the same. -/
structure Rel (st : ForgeCam.Post.State) (σ : State 3) : Prop where
  abs : σ.absolute = true
  plane : σ.plane = 17
  cyc : σ.cyc = none
  mode : ∀ m, st.mode = some m → σ.mode = some m
  modeOk : ∀ m, σ.mode = some m → m ≤ 3
  feed : σ.feed = st.feed
  x : ∀ v, st.x = some v → σ.pos 0 = some v
  y : ∀ v, st.y = some v → σ.pos 1 = some v
  z : ∀ v, st.z = some v → σ.pos 2 = some v

theorem anyAx_embed (b : ForgeCam.Post.Block) :
    ((List.finRange 3).any fun k => ((embed b).ax k).isSome) =
      (b.x.isSome || b.y.isSome || b.z.isSome) := by
  rcases b with ⟨g, x, y, z, i, j, f, t⟩
  cases x <;> cases y <;> cases z <;> rfl

theorem or_rel {x sx px : Option ℤ} {a : ℤ} (h : x.or sx = some a)
    (hr : ∀ v, sx = some v → px = some v) : x.or px = some a := by
  cases x <;> simp_all

set_option maxHeartbeats 4000000 in
/-- **One block.** -/
theorem step_refines (pc lift : ℤ) (st : ForgeCam.Post.State) (σ : State 3)
    (b : ForgeCam.Post.Block) (ms : List ForgeCam.Post.Motion) (st' : ForgeCam.Post.State)
    (hr : Rel st σ) (hw : WF b) (h : ForgeCam.Post.readBlock st b = some (ms, st')) :
    ∃ es σ', step xyz pc lift σ (embed b) = some (es, σ') ∧ es.map forget = ms.map some ∧
      Rel st' σ' := by
  obtain ⟨habs, hplane, hcyc, hmode, hmodeOk, hfeed, hx, hy, hz⟩ := hr
  rcases σ with ⟨pos, absolute, mode, plane, feed, ri, cyc⟩
  simp only at habs hplane hcyc hfeed hmode hmodeOk hx hy hz
  subst habs hplane hcyc hfeed
  rcases b with ⟨g, x, y, z, i, j, f, t⟩
  cases t with
  | some t =>
    simp only [ForgeCam.Post.readBlock, Option.some.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    refine ⟨[.tool t], _, rfl, rfl, ?_⟩
    constructor <;> simp_all
  | none =>
    have hany := anyAx_embed ⟨g, x, y, z, i, j, f, none⟩
    simp only [WF] at hw
    by_cases hno : x = none ∧ y = none ∧ z = none
    · obtain ⟨rfl, rfl, rfl⟩ := hno
      simp only [ForgeCam.Post.readBlock, and_self, ite_true, Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      simp only [Option.isSome_none, Bool.or_false] at hany
      rcases g with _ | g0
      · refine ⟨[], { pos := pos, mode := mode, feed := f.or st.feed, retractInitial := ri }, ?_, rfl, ?_⟩
        · simp only [step, embed] at hany ⊢
          simp [hany]
        · constructor <;> simp_all
      · have hg := hw g0 rfl
        refine ⟨[], { pos := pos, mode := some g0, feed := f.or st.feed, retractInitial := ri }, ?_, rfl, ?_⟩
        · simp only [step, embed] at hany ⊢
          simp [hany, hg]
        · constructor <;> simp_all
    · have hany' : ((List.finRange 3).any fun k =>
          ((embed ⟨g, x, y, z, i, j, f, none⟩).ax k).isSome) = true := by
        rw [hany]; cases x <;> cases y <;> cases z <;> simp_all
      simp only [ForgeCam.Post.readBlock, ite_eq_right hno] at h
      obtain ⟨a, ha⟩ : ∃ a, x.or st.x = some a := by
        cases hxo : x.or st.x <;> simp_all
      obtain ⟨b', hb⟩ : ∃ b', y.or st.y = some b' := by
        cases hyo : y.or st.y <;> simp_all
      obtain ⟨c, hc⟩ : ∃ c, z.or st.z = some c := by
        cases hzo : z.or st.z <;> simp_all
      obtain ⟨m, hm⟩ : ∃ m, g.or st.mode = some m := by
        cases hmo : g.or st.mode <;> simp_all
      rw [ha, hb, hc, hm] at h
      have ha' := or_rel ha hx
      have hb' := or_rel hb hy
      have hc' := or_rel hc hz
      have hgm : g.or mode = some m := by
        cases g <;> simp_all
      have hm3 : m ≤ 3 := by
        cases g <;> simp_all
      let dst : Pos 3 := fun k => (![x, y, z] k).or (pos k)
      have hd0 : dst 0 = some a := ha'
      have hd1 : dst 1 = some b' := hb'
      have hd2 : dst 2 = some c := hc'
      have htarget : target true pos (embed ⟨g, x, y, z, i, j, f, none⟩).ax = some dst := rfl
      -- ours: past the tool and home words and the motion code, the move from `dst`
      -- our motion mode is the model's
      obtain rfl | rfl | rfl | rfl : m = 0 ∨ m = 1 ∨ m = 2 ∨ m = 3 := by omega
      · -- G0
        simp only [Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        refine ⟨[.rapid dst], { pos := dst, mode := some 0, feed := f.or st.feed, retractInitial := ri },
          ?_, ?_, ?_⟩
        · rcases g with _ | g0
          · have hmd : mode = some 0 := hgm
            subst hmd
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz]
          · have : g0 = 0 := by simpa using hgm
            subst this
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz]
        · simp [forget, hd0, hd1, hd2]
        · constructor <;> simp_all
      · -- G1
        obtain ⟨f1, hf1⟩ : ∃ f1, f.or st.feed = some f1 := by
          cases hfo : f.or st.feed <;> simp_all
        simp only [hf1, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        refine ⟨[.feed dst f1], { pos := dst, mode := some 1, feed := f.or st.feed, retractInitial := ri },
          ?_, ?_, ?_⟩
        · rcases g with _ | g0
          · have hmd : mode = some 1 := hgm
            subst hmd
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1]
          · have : g0 = 1 := by simpa using hgm
            subst this
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1]
        · simp [forget, hd0, hd1, hd2]
        · constructor <;> simp_all
      · -- G2
        obtain ⟨sx, hsx⟩ : ∃ sx, st.x = some sx := by
          cases hso : st.x <;> simp [hso] at h ⊢
        obtain ⟨sy, hsy⟩ : ∃ sy, st.y = some sy := by
          cases hso : st.y <;> simp [hso, hsx] at h ⊢
        obtain ⟨i0, rfl⟩ : ∃ i0, i = some i0 := by
          cases i <;> simp [hsx, hsy] at h ⊢
        obtain ⟨j0, rfl⟩ : ∃ j0, j = some j0 := by
          cases j <;> simp [hsx, hsy] at h ⊢
        obtain ⟨f1, hf1⟩ : ∃ f1, f.or st.feed = some f1 := by
          cases hfo : f.or st.feed <;> simp [hsx, hsy, hfo] at h ⊢
        simp [hsx, hsy, hf1] at h
        obtain ⟨rfl, rfl⟩ := h
        have hpx := hx sx hsx
        have hpy := hy sy hsy
        refine ⟨[.arc dst 17 (.abs (sx + i0) (sy + j0)) (decide (2 = 2)) (decide (a = sx) && decide (b' = sy)) f1],
          { pos := dst, mode := some 2, feed := f.or st.feed, retractInitial := ri }, ?_, ?_, ?_⟩
        · rcases g with _ | g0
          · have hmd : mode = some 2 := hgm
            subst hmd
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1, hpx, hpy, hd0, hd1]
          · have : g0 = 2 := by simpa using hgm
            subst this
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1, hpx, hpy, hd0, hd1]
        · simp [forget, hd0, hd1, hd2]
        · constructor <;> simp_all
      · -- G3
        obtain ⟨sx, hsx⟩ : ∃ sx, st.x = some sx := by
          cases hso : st.x <;> simp [hso] at h ⊢
        obtain ⟨sy, hsy⟩ : ∃ sy, st.y = some sy := by
          cases hso : st.y <;> simp [hso, hsx] at h ⊢
        obtain ⟨i0, rfl⟩ : ∃ i0, i = some i0 := by
          cases i <;> simp [hsx, hsy] at h ⊢
        obtain ⟨j0, rfl⟩ : ∃ j0, j = some j0 := by
          cases j <;> simp [hsx, hsy] at h ⊢
        obtain ⟨f1, hf1⟩ : ∃ f1, f.or st.feed = some f1 := by
          cases hfo : f.or st.feed <;> simp [hsx, hsy, hfo] at h ⊢
        simp [hsx, hsy, hf1] at h
        obtain ⟨rfl, rfl⟩ := h
        have hpx := hx sx hsx
        have hpy := hy sy hsy
        refine ⟨[.arc dst 17 (.abs (sx + i0) (sy + j0)) (decide (3 = 2)) (decide (a = sx) && decide (b' = sy)) f1],
          { pos := dst, mode := some 3, feed := f.or st.feed, retractInitial := ri }, ?_, ?_, ?_⟩
        · rcases g with _ | g0
          · have hmd : mode = some 3 := hgm
            subst hmd
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1, hpx, hpy, hd0, hd1]
          · have : g0 = 3 := by simpa using hgm
            subst this
            simp only [step, embed] at hany' htarget ⊢
            simp [hany', htarget, xyz, hf1, hpx, hpy, hd0, hd1]
        · simp [forget, hd0, hd1, hd2]
        · constructor <;> simp_all

/-- **Programs.** Whatever forge-cam's model reads from a block sequence, the interpreter does
too: the same motions, in order. -/
theorem refines_post (pc lift : ℤ) : ∀ (bs : List ForgeCam.Post.Block) (st : ForgeCam.Post.State)
    (σ : State 3) (ms : List ForgeCam.Post.Motion), Rel st σ → (∀ b ∈ bs, WF b) →
    ForgeCam.Post.readFrom st bs = some ms →
    (runFrom xyz pc lift σ (bs.map embed)).map (List.map forget) = some (ms.map some) := by
  intro bs
  induction bs with
  | nil =>
    intro st σ ms _ _ h
    simp only [ForgeCam.Post.readFrom, Option.some.injEq] at h
    subst h; rfl
  | cons b bs ih =>
    intro st σ ms hr hw h
    simp only [ForgeCam.Post.readFrom, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def, Option.some.injEq] at h
    obtain ⟨⟨ms1, st1⟩, h1, rest, h2, rfl⟩ := h
    obtain ⟨es, σ1, hs, hm, hr1⟩ := step_refines pc lift st σ b ms1 st1 hr (hw b (by simp)) h1
    have ih' := ih st1 σ1 rest hr1 (fun b hb => hw b (by simp [hb])) h2
    simp only [List.map_cons, runFrom, hs, Option.bind_eq_bind, Option.bind_some, Option.pure_def]
    cases hrun : runFrom xyz pc lift σ1 (List.map embed bs) with
    | none => rw [hrun] at ih'; simp at ih'
    | some r =>
      rw [hrun] at ih'
      simp only [Option.map_some, Option.some.injEq] at ih' ⊢
      simp [List.map_append, hm, ih']

/-- The printer writes motion codes 0–3 only. -/
theorem print_wf : ∀ (p : List ForgeCam.Post.Rec) (st : ForgeCam.Post.State) (bs : List ForgeCam.Post.Block),
    ForgeCam.Post.printFrom false st p = some bs → ∀ b ∈ bs, WF b := by
  intro p
  induction p with
  | nil =>
    intro st bs h
    simp only [ForgeCam.Post.printFrom, Option.some.injEq] at h
    subst h; simp
  | cons r rs ih =>
    intro st bs h
    simp only [ForgeCam.Post.printFrom, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def, Option.some.injEq] at h
    obtain ⟨⟨bs1, st1⟩, h1, rest, h2, rfl⟩ := h
    intro b hb
    rcases List.mem_append.mp hb with hb | hb
    · cases r with
      | rapid v =>
        simp only [ForgeCam.Post.printOne, ForgeCam.Post.printMove, Option.some.injEq] at h1
        split_ifs at h1 <;> obtain ⟨rfl, rfl⟩ := h1 <;> simp_all [WF]
      | line v f =>
        simp only [ForgeCam.Post.printOne, ForgeCam.Post.printMove, Option.some.injEq] at h1
        split_ifs at h1 <;> obtain ⟨rfl, rfl⟩ := h1 <;> simp_all [WF]
      | arc dst c cw f =>
        simp only [ForgeCam.Post.printOne, ForgeCam.Post.printRec] at h1
        split at h1
        · simp only [Option.some.injEq, Prod.mk.injEq] at h1
          obtain ⟨rfl, rfl⟩ := h1
          simp only [List.mem_singleton] at hb
          subst hb
          intro g hg
          simp only [Option.some.injEq] at hg
          split_ifs at hg <;> omega
        · simp at h1
      | tool t =>
        simp only [ForgeCam.Post.printOne, ForgeCam.Post.printRec, Option.some.injEq] at h1
        obtain ⟨rfl, rfl⟩ := h1
        simp only [List.mem_singleton] at hb
        subst hb
        simp [WF]
    · exact ih st1 rest h2 b hb

/-- **The posted program does what was planned.** forge-cam's printer output, run through this
interpreter from the start state, gives exactly the program's motions less its null moves
(forge-cam's G6 round trip, carried through `refines_post`). -/
theorem posted_program (pc lift : ℤ) (p : List ForgeCam.Post.Rec) (bs : List ForgeCam.Post.Block)
    (h : ForgeCam.Post.print p = some bs) :
    (runFrom xyz pc lift {} (bs.map embed)).map (List.map forget) =
      some ((ForgeCam.Post.normalize (ForgeCam.Post.motions p)).map some) := by
  refine refines_post pc lift bs {} {} _ ?_ (print_wf p {} bs h) (ForgeCam.Post.round_trip p bs h)
  constructor <;> simp

end ForgeMachine
