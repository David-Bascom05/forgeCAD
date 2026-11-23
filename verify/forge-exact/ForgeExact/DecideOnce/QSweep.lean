import ForgeExact.DecideOnce.QVert3

/-!
# Q1 L2: the sweep's events are vertices and line samples

The sweep direction is `x` (after the global rotation and shear, curved-l2-plan.md §2). Its
events are where a slab's delineability can fail (LemmaD's `NoCrit`):
* **a curve's turning points**: points of `q_j = q_k = 0` where `∇q_j`, `∇q_k` and `eₓ` are
  dependent. That determinant is the `x`-component of `∇q_j × ∇q_k`, a product of linear forms:
  a **quadric** (`critQ`, `critQ_ev`). So a turning point is a vertex of the three quadrics
  `q_j, q_k, critQ q_j q_k`, found and certified by QVert (`onVert_spec`, `bezout3_spec`), and
  their completeness is the vertex proposer's;
* **a surface's critical points**: points of `q = 0` where `∇q ∥ eₓ`, i.e. `∂_y q = ∂_z q = 0`:
  two planes (`gradPlaneY`, `gradPlaneZ`). With `q` they are again a vertex of three quadrics;
* the **vertices** of three carriers themselves.

So every event is a vertex sample, and the event values are their `x`-roots: `Num`s, ordered by
`Num.cmp`, with rational walls or sample planes between them by `QRef.between`.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- The gradient's `y`- and `z`-components, as affine forms `(a, b, c, d)` = `a x + b y + c z + d`. -/
def gradY (q : Quadric) : Rat × Rat × Rat × Rat := (q.xy, 2 * q.yy, q.yz, q.ly)
def gradZ (q : Quadric) : Rat × Rat × Rat × Rat := (q.xz, q.yz, 2 * q.zz, q.lz)

/-- The product of two affine forms, as a quadric. -/
def affMul (f g : Rat × Rat × Rat × Rat) : Quadric :=
  let (a, b, c, d) := f
  let (a', b', c', d') := g
  ⟨a * a', b * b', c * c', a * b' + b * a', a * c' + c * a', b * c' + c * b',
   a * d' + d * a', b * d' + d * b', c * d' + d * c', d * d'⟩

/-- The difference of two quadrics. -/
def qsub (p q : Quadric) : Quadric :=
  ⟨p.xx - q.xx, p.yy - q.yy, p.zz - q.zz, p.xy - q.xy, p.xz - q.xz, p.yz - q.yz,
   p.lx - q.lx, p.ly - q.ly, p.lz - q.lz, p.c - q.c⟩

/-- `(∇q_j × ∇q_k)ₓ = ∂_y q_j ∂_z q_k − ∂_z q_j ∂_y q_k`: zero exactly at the turning points of
`q_j ∩ q_k` in the `x`-direction. -/
def critQ (qj qk : Quadric) : Quadric := qsub (affMul (gradY qj) (gradZ qk)) (affMul (gradZ qj) (gradY qk))

/-- `∂_y q = 0` and `∂_z q = 0` as planes (quadrics of degree 1). -/
def gradPlaneY (q : Quadric) : Quadric :=
  let (a, b, c, d) := gradY q
  ⟨0, 0, 0, 0, 0, 0, a, b, c, d⟩
def gradPlaneZ (q : Quadric) : Quadric :=
  let (a, b, c, d) := gradZ q
  ⟨0, 0, 0, 0, 0, 0, a, b, c, d⟩

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- An affine form's value. -/
def affEv (f : Rat × Rat × Rat × Rat) (p : PK K) : K :=
  M.q f.1 * p.1 + M.q f.2.1 * p.2.1 + M.q f.2.2.1 * p.2.2 + M.q f.2.2.2

/-- The gradient's components in the model. -/
def dyK (q : Quadric) (p : PK K) : K :=
  M.q q.xy * p.1 + (1 + 1) * M.q q.yy * p.2.1 + M.q q.yz * p.2.2 + M.q q.ly
def dzK (q : Quadric) (p : PK K) : K :=
  M.q q.xz * p.1 + M.q q.yz * p.2.1 + (1 + 1) * M.q q.zz * p.2.2 + M.q q.lz

theorem q_two : M.q 2 = 1 + 1 := by
  rw [show (2 : Rat) = 1 + 1 by grind, M.q_add, M.q_one]

theorem affEv_gradY (q : Quadric) (p : PK K) : affEv M (gradY q) p = dyK M q p := by
  simp only [affEv, gradY, dyK, M.q_mul, q_two]

theorem affEv_gradZ (q : Quadric) (p : PK K) : affEv M (gradZ q) p = dzK M q p := by
  simp only [affEv, gradZ, dzK, M.q_mul, q_two]

theorem affMul_ev (f g : Rat × Rat × Rat × Rat) (p : PK K) :
    Quadric.evK M (affMul f g) p = affEv M f p * affEv M g p := by
  obtain ⟨a, b, c, d⟩ := f
  obtain ⟨a', b', c', d'⟩ := g
  simp only [affMul, Quadric.evK, affEv, M.q_add, M.q_mul]
  grind

theorem qsub_ev (p q : Quadric) (x : PK K) :
    Quadric.evK M (qsub p q) x = Quadric.evK M p x - Quadric.evK M q x := by
  simp only [qsub, Quadric.evK, M.q_sub]
  grind

/-- **The turning-point quadric is the cross product's `x`-component.** -/
theorem critQ_ev (qj qk : Quadric) (p : PK K) :
    Quadric.evK M (critQ qj qk) p = dyK M qj p * dzK M qk p - dzK M qj p * dyK M qk p := by
  rw [critQ, qsub_ev, affMul_ev, affMul_ev, affEv_gradY, affEv_gradZ, affEv_gradY, affEv_gradZ]

theorem gradPlaneY_ev (q : Quadric) (p : PK K) : Quadric.evK M (gradPlaneY q) p = dyK M q p := by
  simp only [gradPlaneY, gradY, Quadric.evK, dyK, M.q_zero, M.q_mul, q_two]
  grind

theorem gradPlaneZ_ev (q : Quadric) (p : PK K) : Quadric.evK M (gradPlaneZ q) p = dzK M q p := by
  simp only [gradPlaneZ, gradZ, Quadric.evK, dzK, M.q_zero, M.q_mul, q_two]
  grind

/-- At a rational point, `dyK`, `dzK` are exact-curved's gradient components (`Quadric.grad`). -/
theorem dyK_grad (q : Quadric) (p : V3) :
    dyK M q (M.q p.x, M.q p.y, M.q p.z) = M.q (q.grad p).y := by
  simp only [dyK, Quadric.grad, M.q_add, M.q_mul, q_two]
  grind

theorem dzK_grad (q : Quadric) (p : V3) :
    dzK M q (M.q p.x, M.q p.y, M.q p.z) = M.q (q.grad p).z := by
  simp only [dzK, Quadric.grad, M.q_add, M.q_mul, q_two]
  grind

end Model

/-! ## The events, assembled and certified

`eventTriples`: the three kinds of triple (three carriers; a pair with its `critQ`; a carrier
with its two gradient planes). `proposeTriple` sends a triple with a plane to
`proposeVertPlaneAny` (a non-plane first, the plane last) and one without to `proposeVert3`;
three planes are not handled yet (`none`, `Skip::Budget`). `eventsOf` sorts, dedups and separates
the `x`-roots (untrusted) and then checks:
- every raw root is `Num.cmp`-equal to an event;
- every event is a rational or a checked isolation;
- consecutive events are separated by rationals.

`planesOf` puts a rational sample plane between consecutive events and one outside each end.
* `eventsOf_complete`: every point of a triple's three zero sets has its `x` among the events;
* `planes_between`: the sample plane of two consecutive events lies strictly between them.
-/

def isPlaneQ (q : Quadric) : Bool :=
  q.xx == 0 && q.yy == 0 && q.zz == 0 && q.xy == 0 && q.xz == 0 && q.yz == 0

theorem isPlaneQ_iff {q : Quadric} (h : isPlaneQ q = true) :
    q.xx = 0 ∧ q.yy = 0 ∧ q.zz = 0 ∧ q.xy = 0 ∧ q.xz = 0 ∧ q.yz = 0 := by
  simp only [isPlaneQ, Bool.and_eq_true, beq_iff_eq] at h
  exact ⟨h.1.1.1.1.1, h.1.1.1.1.2, h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

def proposeTriple (a b c : Quadric) (fuel : Nat) : Option (List VSample) :=
  if !isPlaneQ a && !isPlaneQ b && !isPlaneQ c then proposeVert3 a b c fuel
  else if isPlaneQ c && !isPlaneQ a then proposeVertPlaneAny a b c fuel
  else if isPlaneQ c && !isPlaneQ b then proposeVertPlaneAny b a c fuel
  else if isPlaneQ b && !isPlaneQ a then proposeVertPlaneAny a c b fuel
  else if isPlaneQ b && !isPlaneQ c then proposeVertPlaneAny c a b fuel
  else if isPlaneQ a && !isPlaneQ b then proposeVertPlaneAny b c a fuel
  else if isPlaneQ a && !isPlaneQ c then proposeVertPlaneAny c b a fuel
  else none

def tri : List Quadric → List (Quadric × Quadric × Quadric)
  | [] => []
  | a :: rest =>
    ((rest.zipIdx.flatMap fun (b, i) => (rest.drop (i + 1)).map fun c => (a, b, c))) ++ tri rest

def pairs : List Quadric → List (Quadric × Quadric)
  | [] => []
  | a :: rest => rest.map (fun b => (a, b)) ++ pairs rest

def eventTriples (qs : List Quadric) : List (Quadric × Quadric × Quadric) :=
  tri qs ++ (pairs qs).map (fun (a, b) => (a, b, critQ a b)) ++
    qs.map fun q => (q, gradPlaneY q, gradPlaneZ q)

/-- A constant (no `x`, `y`, `z` terms at all). -/
def isConstQ (q : Quadric) : Bool := isPlaneQ q && q.lx == 0 && q.ly == 0 && q.lz == 0

/-- The determinant of three planes' normals, and Cramer's numerator for `x`. -/
def det3Q (a b c : Quadric) : Rat :=
  a.lx * (b.ly * c.lz - b.lz * c.ly) - a.ly * (b.lx * c.lz - b.lz * c.lx) +
    a.lz * (b.lx * c.ly - b.ly * c.lx)
def numXQ (a b c : Quadric) : Rat :=
  -(a.c * (b.ly * c.lz - b.lz * c.ly) - a.ly * (b.c * c.lz - b.lz * c.c) +
    a.lz * (b.c * c.ly - b.ly * c.c))

/-- The `x` of every common point of a triple, proposed: a nonzero constant has no zeros; three
planes meet at Cramer's point (if their normals are independent); otherwise the curved
proposers. `none`: not decided (a singular triple of planes, or fuel spent). -/
def proposeTripleX (a b c : Quadric) (fuel : Nat) : Option (List Num) :=
  if (isConstQ a && a.c != 0) || (isConstQ b && b.c != 0) || (isConstQ c && c.c != 0) then some []
  else if isPlaneQ a && isPlaneQ b && isPlaneQ c then
    if det3Q a b c != 0 then some [.rat (numXQ a b c / det3Q a b c)] else none
  else (proposeTriple a b c fuel).map (List.map VSample.x)

def rawEvents (qs : List Quadric) (fuel : Nat) : Option (List Num) :=
  ((eventTriples qs).mapM fun (t : Quadric × Quadric × Quadric) =>
      proposeTripleX t.1 t.2.1 t.2.2 fuel).map List.flatten

/-- Refine consecutive numbers until their intervals separate (untrusted). -/
def separate : Nat → Num → List Num → List Num
  | _, u, [] => [u]
  | 0, u, v :: rest => u :: separate 0 v rest
  | k + 1, u, v :: rest =>
    if u.hi < v.lo then u :: separate (k + 1) v rest
    else separate k u.bisect (v.bisect :: rest)

def isoOkN : Num → Bool
  | .rat _ => true
  | .iso r => r.ok

def sepOk : List Num → Bool
  | u :: v :: rest => decide (u.hi < v.lo) && sepOk (v :: rest)
  | _ => true

def spreadE : List Num → List Num
  | [] => []
  | u :: rest => separate 400 u rest

/-- The certificate check: every raw event equals a listed one, every listed one is a checked
isolation, and consecutive ones are separated. -/
def checkEvents (raw E : List Num) : Bool :=
  raw.all (fun n => E.any fun e => Num.cmp 200 n e == some .eq) && E.all isoOkN && sepOk E

def pickEvents (raw : List Num) : Option (List Num) :=
  ((QRef.sortNums raw.toArray).map Array.toList).bind fun sorted =>
    if checkEvents raw (spreadE sorted) then some (spreadE sorted) else none

def eventsOf (qs : List Quadric) (fuel : Nat) : Option (List Num) :=
  (rawEvents qs fuel).bind pickEvents

theorem pickEvents_spec {raw E : List Num} (h : pickEvents raw = some E) :
    checkEvents raw E = true := by
  unfold pickEvents at h
  cases hs : (QRef.sortNums raw.toArray).map Array.toList with
  | none => rw [hs] at h; cases h
  | some sorted =>
    rw [hs, Option.bind_some] at h
    by_cases hc : checkEvents raw (spreadE sorted) = true
    · rw [if_pos hc] at h; cases h; exact hc
    · rw [if_neg hc] at h; cases h

/-- A rational sample plane between consecutive events, and one outside each end. -/
def planesOf : List Num → List Rat
  | [] => [0]
  | u :: rest =>
    (u.lo - 1) :: ((u :: rest).zip rest).map (fun (a, b) => (a.hi + b.lo) / 2) ++
      [((u :: rest).getLast?.map Num.hi).getD u.hi + 1]

theorem mapM_mem {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {ls : List β}, l.mapM f = some ls → ∀ a ∈ l, ∃ r, f a = some r ∧ r ∈ ls
  | [], _, _, a, ha => nomatch ha
  | x :: xs, ls, h, a, ha => by
    simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    rcases List.mem_cons.mp ha with rfl | ha
    · exact ⟨r, hr, List.mem_cons_self ..⟩
    · obtain ⟨r', h1, h2⟩ := mapM_mem hrs a ha
      exact ⟨r', h1, List.mem_cons_of_mem _ h2⟩

theorem sepOk_zip : ∀ {l : List Num}, sepOk l = true → ∀ p ∈ l.zip l.tail, p.1.hi < p.2.lo
  | [], _, p, hp => by simp at hp
  | [_], _, p, hp => by simp at hp
  | u :: v :: rest, h, p, hp => by
    simp only [sepOk, Bool.and_eq_true, decide_eq_true_eq] at h
    simp only [List.tail_cons, List.zip_cons_cons, List.mem_cons] at hp
    rcases hp with rfl | hp
    · exact h.1
    · exact sepOk_zip h.2 p (by simpa using hp)

section Model2

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem proposeTriple_complete {a b c : Quadric} {fuel : Nat} {vs : List VSample}
    (h : proposeTriple a b c fuel = some vs) {x y z : K}
    (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ v ∈ vs, Denotes M v.x x := by
  unfold proposeTriple at h
  by_cases h1 : (!isPlaneQ a && !isPlaneQ b && !isPlaneQ c) = true
  · rw [if_pos h1] at h
    obtain ⟨v, hv, hd, _⟩ := proposeVert3_complete M h ha hb hc; exact ⟨v, hv, hd⟩
  rw [if_neg h1] at h
  by_cases h2 : (isPlaneQ c && !isPlaneQ a) = true
  · rw [if_pos h2] at h
    simp only [Bool.and_eq_true] at h2
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h2.1) h ha hb hc
    exact ⟨v, hv, hd⟩
  rw [if_neg h2] at h
  by_cases h3 : (isPlaneQ c && !isPlaneQ b) = true
  · rw [if_pos h3] at h
    simp only [Bool.and_eq_true] at h3
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h3.1) h hb ha hc
    exact ⟨v, hv, hd⟩
  rw [if_neg h3] at h
  by_cases h4 : (isPlaneQ b && !isPlaneQ a) = true
  · rw [if_pos h4] at h
    simp only [Bool.and_eq_true] at h4
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h4.1) h ha hc hb
    exact ⟨v, hv, hd⟩
  rw [if_neg h4] at h
  by_cases h5 : (isPlaneQ b && !isPlaneQ c) = true
  · rw [if_pos h5] at h
    simp only [Bool.and_eq_true] at h5
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h5.1) h hc ha hb
    exact ⟨v, hv, hd⟩
  rw [if_neg h5] at h
  by_cases h6 : (isPlaneQ a && !isPlaneQ b) = true
  · rw [if_pos h6] at h
    simp only [Bool.and_eq_true] at h6
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h6.1) h hb hc ha
    exact ⟨v, hv, hd⟩
  rw [if_neg h6] at h
  by_cases h7 : (isPlaneQ a && !isPlaneQ c) = true
  · rw [if_pos h7] at h
    simp only [Bool.and_eq_true] at h7
    obtain ⟨v, hv, hd, _⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h7.1) h hc hb ha
    exact ⟨v, hv, hd⟩
  rw [if_neg h7] at h
  cases h

theorem plane_ev {q : Quadric} (h : isPlaneQ q = true) (x y z : K) :
    Quadric.evK M q (x, y, z) = M.q q.lx * x + M.q q.ly * y + M.q q.lz * z + M.q q.c := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := isPlaneQ_iff h
  simp only [Quadric.evK, h1, h2, h3, h4, h5, h6, Model.q_zero]; grind

theorem const_ne {q : Quadric} (h : (isConstQ q && q.c != 0) = true) (x y z : K) :
    Quadric.evK M q (x, y, z) ≠ 0 := by
  simp only [isConstQ, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq] at h
  obtain ⟨⟨⟨⟨hp, h1⟩, h2⟩, h3⟩, hc⟩ := h
  rw [plane_ev M hp, h1, h2, h3]
  simp only [Model.q_zero]
  intro h0
  have : M.q q.c = M.q 0 := by rw [Model.q_zero]; grind
  exact hc ((Model.q_eq M).mp this)

theorem cramer3 {a b c : Quadric} (ha : isPlaneQ a = true) (hb : isPlaneQ b = true)
    (hc : isPlaneQ c = true) (hd : det3Q a b c ≠ 0) {x y z : K}
    (ea : Quadric.evK M a (x, y, z) = 0) (eb : Quadric.evK M b (x, y, z) = 0)
    (ec : Quadric.evK M c (x, y, z) = 0) : x = M.q (numXQ a b c / det3Q a b c) := by
  rw [plane_ev M ha] at ea; rw [plane_ev M hb] at eb; rw [plane_ev M hc] at ec
  have key : M.q (det3Q a b c) * x = M.q (numXQ a b c) := by
    simp only [det3Q, numXQ, Model.q_add, Model.q_sub, Model.q_mul, Model.q_neg]
    grind
  have hq : M.q (numXQ a b c / det3Q a b c) * M.q (det3Q a b c) = M.q (numXQ a b c) := by
    rw [← Model.q_mul]; congr 1; exact Rat.div_mul_cancel hd
  have hd' : M.q (det3Q a b c) ≠ 0 := fun h0 =>
    hd ((Model.q_eq M).mp (by rw [h0, Model.q_zero]))
  grind

theorem proposeTripleX_complete {a b c : Quadric} {fuel : Nat} {xs : List Num}
    (h : proposeTripleX a b c fuel = some xs) {x y z : K}
    (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ n ∈ xs, Denotes M n x := by
  unfold proposeTripleX at h
  by_cases h1 : ((isConstQ a && a.c != 0) || (isConstQ b && b.c != 0) ||
      (isConstQ c && c.c != 0)) = true
  · exfalso
    simp only [Bool.or_eq_true] at h1
    rcases h1 with (h1 | h1) | h1
    · exact const_ne M h1 x y z ha
    · exact const_ne M h1 x y z hb
    · exact const_ne M h1 x y z hc
  rw [if_neg h1] at h
  by_cases h2 : (isPlaneQ a && isPlaneQ b && isPlaneQ c) = true
  · rw [if_pos h2] at h
    simp only [Bool.and_eq_true] at h2
    by_cases h3 : (det3Q a b c != 0) = true
    · rw [if_pos h3] at h
      cases h
      simp only [bne_iff_ne, ne_eq] at h3
      exact ⟨_, List.mem_singleton_self _, cramer3 M h2.1.1 h2.1.2 h2.2 h3 ha hb hc⟩
    · rw [if_neg h3] at h; cases h
  rw [if_neg h2] at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨vs, hvs, rfl⟩ := h
  obtain ⟨v, hv, hd⟩ := proposeTriple_complete M hvs ha hb hc
  exact ⟨v.x, List.mem_map_of_mem hv, hd⟩

theorem rawEvents_complete {qs : List Quadric} {fuel : Nat} {raw : List Num}
    (h : rawEvents qs fuel = some raw) {a b c : Quadric} (ht : (a, b, c) ∈ eventTriples qs)
    {x y z : K} (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ n ∈ raw, Denotes M n x := by
  unfold rawEvents at h
  cases hm : (eventTriples qs).mapM (fun (t : Quadric × Quadric × Quadric) =>
      proposeTripleX t.1 t.2.1 t.2.2 fuel) with
  | none => rw [hm] at h; cases h
  | some ls =>
    rw [hm] at h
    simp only [Option.map_some, Option.some.injEq] at h
    subst h
    obtain ⟨r, hr, hrl⟩ := mapM_mem hm (a, b, c) ht
    obtain ⟨n, hn, hd⟩ := proposeTripleX_complete M hr ha hb hc
    exact ⟨n, List.mem_flatten.mpr ⟨_, hrl, hn⟩, hd⟩

/-- **Every event point's `x` is an event.** -/
theorem eventsOf_complete {qs : List Quadric} {fuel : Nat} {E : List Num}
    (h : eventsOf qs fuel = some E) {a b c : Quadric} (ht : (a, b, c) ∈ eventTriples qs)
    {x y z : K} (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ e ∈ E, Denotes M e x := by
  unfold eventsOf at h
  cases hr : rawEvents qs fuel with
  | none => rw [hr] at h; cases h
  | some raw =>
    rw [hr, Option.bind_some] at h
    have hck := pickEvents_spec h
    simp only [checkEvents, Bool.and_eq_true, List.all_eq_true, List.any_eq_true,
      beq_iff_eq] at hck
    obtain ⟨⟨hcov, hiso⟩, _⟩ := hck
    obtain ⟨n, hn, hd⟩ := rawEvents_complete M hr ht ha hb hc
    obtain ⟨e, he, hcmp⟩ := hcov n hn
    have hden : ∃ t, Denotes M e t := by
      have := hiso e he
      cases e with
      | rat q => exact ⟨M.q q, rfl⟩
      | iso r => exact Iso.denotes_of_ok M r this
    obtain ⟨t, ht⟩ := hden
    have := cmp_spec M 200 hd ht hcmp
    simp only [CmpIs] at this
    subst this
    exact ⟨e, he, ht⟩

/-- The listed events are separated (`sepOk`), for `planes_between`. -/
theorem eventsOf_sep {qs : List Quadric} {fuel : Nat} {E : List Num}
    (h : eventsOf qs fuel = some E) : sepOk E = true := by
  unfold eventsOf at h
  cases hr : rawEvents qs fuel with
  | none => rw [hr] at h; cases h
  | some raw =>
    rw [hr, Option.bind_some] at h
    have hck := pickEvents_spec h
    simp only [checkEvents, Bool.and_eq_true] at hck
    exact hck.2

/-- **The sample plane of two consecutive events lies strictly between them.** -/
theorem planes_between {E : List Num} (hs : sepOk E = true) {u v : Num} (huv : (u, v) ∈ E.zip E.tail)
    {s t : K} (hu : Denotes M u s) (hv : Denotes M v t) :
    s < M.q ((u.hi + v.lo) / 2) ∧ M.q ((u.hi + v.lo) / 2) < t := by
  have hsep := sepOk_zip hs (u, v) huv
  have h1 := Denotes.le_hi M hu
  have h2 := Denotes.lo_le M hv
  have m1 : M.q u.hi < M.q ((u.hi + v.lo) / 2) := M.q_lt.mpr (by grind)
  have m2 : M.q ((u.hi + v.lo) / 2) < M.q v.lo := M.q_lt.mpr (by grind)
  exact ⟨Std.lt_of_le_of_lt h1 m1, Std.lt_of_lt_of_le m2 h2⟩

end Model2

-- the unit circle (sphere ∩ z = 0) turns in x at (±1, 0, 0): critQ there is 2y, and the vertex
-- (1, 0, 0) certifies on sphere, plane and critQ
#guard critQ sphere1 planeZ == ⟨0, 0, 0, 0, 0, 0, 0, 2, 0, 0⟩
#guard vAt100.onVert sphere1 planeZ (critQ sphere1 planeZ)
-- the unit sphere and the planes y = 0, z = 0: events at x = ±1 (vertices and the circle's turns)
#guard (eventsOf [sphere1, planeY0, planeZ] 30).map (·.map fun n => (n.lo, n.hi)) = some [(-1, -1), (1, 1)]
#guard (eventsOf [sphere1, planeY0, planeZ] 30).map planesOf = some [-2, 0, 2]

end ForgeExact.DecideOnce.Curved
