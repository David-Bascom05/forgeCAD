/-!
# The probe fix: `planar_inner` and `beside_partner`, over ℚ

The fix-probe change is on branch claude/repo-review-mbdrlj-fix-probe, a3cc22d:
`kernel/forge-bool/src/boolean.rs`, `interior_point` → `planar_inner`, and `classify_specs` →
`beside_partner`. It replaces stepping across a narrow planar region (G3, G4) in two ways:
- the region's point comes from its own boundary (`planar_inner`);
- a region still within LINEAR of its coplanar partner's boundary is decided by the partner's edge
  (`beside_partner`).

Both are transcribed here over ℚ, statement by statement. The kernel computes the same formulas
in f64; that gap, and the polygonising of curved pieces into chords within LINEAR/4, are the
trusted steps.

## `planar_inner`

The region's loops, flattened into the plane, are rings of points. For each scanline y it collects:
- crossings: the x of every edge with `(a.y < y) != (b.y < y)`, at
  `a.x + (y - a.y)(b.x - a.x)/(b.y - a.y)`;
- spans: the crossings sorted and paired off;
- candidates: each span's midpoint.

The scanlines are the middles of the 32 tallest slabs between successive distinct vertex heights.
The answer is the candidate farthest from the boundary, with that clearance.

**Inside** means the even-odd rule with the same half-open crossing test: an odd number of crossings
strictly left of the point. That is the region's interior for points off its boundary.

- `candidate_inside`: every candidate off the crossings is inside.
- `planarInner_spec`, the probe spec for the fix: if `planar_inner` returns a point with clearance
  > 0, and the clearance is positive only off the boundary, the point is strictly inside the region:
  even-odd inside, and on no boundary segment. Unlike the stepping probe (`Probe.kernel_not_spec`),
  this holds for every region, however narrow.

## `beside_partner`

Cut across the partner's edge E. Let `here` be the region's inward direction from E, and `there`
the partner face P's. Both lie in P's plane, so `here = ±there`. Let `n` be the outward normal of
the face g beyond E.

The other body near E is a wedge of P's half-plane and g's. It is the intersection where E is
convex (`n · there < 0`), the union where E is concave (`n · there > 0`).

`beside_exact`: the code's decision equals the exact position of every point `t · here`, t > 0:
- `here · there > 0` → `On` (inside P);
- otherwise `lean = here · n`, read In if negative, Out if positive, and no decision if zero.

Kernel-checked, core Lean.
-/

namespace Coverage.Inner

structure Q2 where
  x : Rat
  y : Rat
  deriving DecidableEq, Repr, Inhabited

abbrev Ring := List Q2

/-- A ring's edges: (ring[k], ring[(k + 1) % n]). -/
def edges (r : Ring) : List (Q2 × Q2) := r.zip (r.drop 1 ++ r.take 1)

def crosses (y : Rat) (e : Q2 × Q2) : Bool := decide (e.1.y < y) != decide (e.2.y < y)

def xAt (y : Rat) (e : Q2 × Q2) : Rat := e.1.x + (y - e.1.y) * (e.2.x - e.1.x) / (e.2.y - e.1.y)

/-- The x of every edge the scanline y crosses. -/
def crossings (rings : List Ring) (y : Rat) : List Rat :=
  rings.flatMap fun r => ((edges r).filter (crosses y)).map (xAt y)

/-- Even-odd: an odd number of crossings strictly left of the point. -/
def Inside (rings : List Ring) (q : Q2) : Prop :=
  ((crossings rings q.y).filter fun x => decide (x < q.x)).length % 2 = 1

/-- On an edge segment. -/
def OnBoundary (rings : List Ring) (q : Q2) : Prop :=
  ∃ r ∈ rings, ∃ e ∈ edges r, ∃ t : Rat, 0 ≤ t ∧ t ≤ 1 ∧
    q.x = e.1.x + t * (e.2.x - e.1.x) ∧ q.y = e.1.y + t * (e.2.y - e.1.y)

/-- Sorted crossings paired off (`as_chunks::<2>`: a last odd one is dropped). -/
def spans : List Rat → List (Rat × Rat)
  | a :: b :: t => (a, b) :: spans t
  | _ => []

def sortQ (l : List Rat) : List Rat := l.mergeSort fun a b => decide (a ≤ b)

def candidates (rings : List Ring) (y : Rat) : List Q2 :=
  (spans (sortQ (crossings rings y))).map fun ab => ⟨(ab.1 + ab.2) / 2, y⟩

theorem sortQ_sorted (l : List Rat) : (sortQ l).Pairwise fun a b => a ≤ b := by
  have h : (sortQ l).Pairwise fun a b => decide (a ≤ b) = true := by
    apply List.pairwise_mergeSort
    · intro a b c h1 h2; simp only [decide_eq_true_eq] at *; exact Rat.le_trans h1 h2
    · intro a b; by_cases h : a ≤ b <;> simp [h]; exact Rat.le_of_lt (Rat.not_le.mp h)
  exact h.imp fun h => by simpa using h

theorem sortQ_perm (l : List Rat) : (sortQ l).Perm l := List.mergeSort_perm _ _

/-- A span's ends are in the list, in order when it is sorted. -/
theorem spans_mem : ∀ (l : List Rat) (p : Rat × Rat), p ∈ spans l → p.1 ∈ l ∧ p.2 ∈ l
  | a :: b :: t, p, hp => by
    simp only [spans, List.mem_cons] at hp
    rcases hp with rfl | hp
    · simp
    · have := spans_mem t p hp; simp [this]
  | [], _, hp => by simp [spans] at hp
  | [_], _, hp => by simp [spans] at hp

theorem spans_le : ∀ (l : List Rat), l.Pairwise (fun a b => a ≤ b) → ∀ p ∈ spans l, p.1 ≤ p.2
  | a :: b :: t, hs, p, hp => by
    simp only [List.pairwise_cons, List.mem_cons] at hs
    simp only [spans, List.mem_cons] at hp
    rcases hp with rfl | hp
    · exact hs.1 b (Or.inl rfl)
    · exact spans_le t hs.2.2 p hp
  | [], _, _, hp => by simp [spans] at hp
  | [_], _, _, hp => by simp [spans] at hp

/-- In a sorted list, a value strictly between the two of a span has an odd count below it. -/
theorem spans_odd : ∀ (l : List Rat), l.Pairwise (fun a b => a ≤ b) →
    ∀ ab ∈ spans l, ∀ m, ab.1 < m → m < ab.2 → (l.filter fun x => decide (x < m)).length % 2 = 1
  | a :: b :: t, hs, ab, hab, m, h1, h2 => by
    simp only [List.pairwise_cons, List.mem_cons] at hs
    obtain ⟨ha, hb, ht⟩ := hs
    simp only [spans, List.mem_cons] at hab
    rcases hab with rfl | hab
    · simp only at h1 h2
      have hb' : ¬ b < m := fun h => by grind
      have ht' : (t.filter fun x => decide (x < m)) = [] := by
        rw [List.filter_eq_nil_iff]; intro x hx; have := hb x hx; simp; grind
      simp [h1, hb', ht']
    · have ih := spans_odd t ht ab hab m h1 h2
      have hab1 := (spans_mem t ab hab).1
      have am : a < m := by have := ha ab.1 (Or.inr hab1); grind
      have bm : b < m := by have := hb ab.1 hab1; grind
      simp only [List.filter_cons, am, bm, decide_true, ite_true, List.length_cons]
      omega
  | [], _, ab, hab, _, _, _ => by simp [spans] at hab
  | [_], _, ab, hab, _, _, _ => by simp [spans] at hab

/-- **Every candidate off the crossings is inside** (even-odd). -/
theorem candidate_inside (rings : List Ring) (y : Rat) (q : Q2) (hq : q ∈ candidates rings y)
    (hoff : q.x ∉ crossings rings y) : Inside rings q := by
  simp only [candidates, List.mem_map] at hq
  obtain ⟨ab, hab, rfl⟩ := hq
  have hs := sortQ_sorted (crossings rings y)
  have hle := spans_le _ hs ab hab
  have hmem : ab.1 ∈ crossings rings y := (sortQ_perm _).mem_iff.mp (spans_mem _ ab hab).1
  have hne : (ab.1 + ab.2) / 2 ≠ ab.1 := fun h => hoff (by simp only; rw [h]; exact hmem)
  have h1 : ab.1 < (ab.1 + ab.2) / 2 := by grind
  have h2 : (ab.1 + ab.2) / 2 < ab.2 := by grind
  have := spans_odd _ hs ab hab _ h1 h2
  unfold Inside
  simp only
  rw [← ((sortQ_perm (crossings rings y)).filter _).length_eq]
  exact this

theorem t_ge0 (t d c : Rat) (hk : t * d = c) (h : (0 < d ∧ 0 ≤ c) ∨ (d < 0 ∧ c ≤ 0)) : 0 ≤ t := by
  apply Classical.byContradiction; intro hn
  have hn : t < 0 := Rat.not_le.mp hn
  rcases h with ⟨hd, hc⟩ | ⟨hd, hc⟩
  · have := Rat.mul_pos (show 0 < -t by grind) hd; grind
  · have := Rat.mul_pos (show 0 < -t by grind) (show 0 < -d by grind); grind

theorem t_le1 (t d c : Rat) (hk : t * d = c) (h : (0 < d ∧ c ≤ d) ∨ (d < 0 ∧ d ≤ c)) : t ≤ 1 := by
  apply Classical.byContradiction; intro hn
  have hn : 1 < t := Rat.not_le.mp hn
  rcases h with ⟨hd, hc⟩ | ⟨hd, hc⟩
  · have := Rat.mul_pos (show 0 < t - 1 by grind) hd; grind
  · have := Rat.mul_pos (show 0 < t - 1 by grind) (show 0 < -d by grind); grind

/-- A crossing point lies on its edge. -/
theorem crossing_on_boundary (rings : List Ring) (y x : Rat) (hx : x ∈ crossings rings y) :
    OnBoundary rings ⟨x, y⟩ := by
  simp only [crossings, List.mem_flatMap, List.mem_map, List.mem_filter] at hx
  obtain ⟨r, hr, e, ⟨he, hc⟩, rfl⟩ := hx
  refine ⟨r, hr, e, he, (y - e.1.y) / (e.2.y - e.1.y), ?_⟩
  simp only [crosses, bne_iff_ne, ne_eq, decide_eq_decide] at hc
  have hd : e.2.y - e.1.y ≠ 0 := fun h => hc (by grind)
  have key : (y - e.1.y) / (e.2.y - e.1.y) * (e.2.y - e.1.y) = y - e.1.y := Rat.div_mul_cancel hd
  have cases : (0 < e.2.y - e.1.y ∧ 0 ≤ y - e.1.y ∧ y - e.1.y ≤ e.2.y - e.1.y) ∨
      (e.2.y - e.1.y < 0 ∧ y - e.1.y ≤ 0 ∧ e.2.y - e.1.y ≤ y - e.1.y) := by
    by_cases h : e.1.y < y
    · left; have : ¬ e.2.y < y := fun h2 => hc (by simp [h, h2]); grind
    · right; have : e.2.y < y := Classical.byContradiction fun h2 => hc (by simp [h, h2]); grind
  refine ⟨t_ge0 _ _ _ key (by grind), t_le1 _ _ _ key (by grind), ?_, ?_⟩
  · simp only [xAt, Rat.div_def]; grind
  · simp only; grind

/-- `planar_inner`: the 32 tallest slabs between successive distinct vertex heights, each
scanline's candidates, the one of greatest clearance (`clr`, the distance to the boundary).
The scan and the choice are the kernel's (first strictly greater wins). -/
def slabs (rings : List Ring) : List (Rat × Rat) :=
  let hs := ((rings.flatten.map (·.y)).mergeSort fun a b => decide (a ≤ b)).eraseDups
  let ws := (hs.zip hs.tail).map fun (a, b) => (b - a, (a + b) / 2)
  (ws.mergeSort fun p q => decide (q.1 ≤ p.1)).take 32

def planarInner (clr : Q2 → Rat) (rings : List Ring) : Option (Q2 × Rat) :=
  ((slabs rings).flatMap fun s => candidates rings s.2).foldl
    (fun best q => match best with
      | none => some (q, clr q)
      | some b => if clr q > b.2 then some (q, clr q) else some b) none

theorem planarInner_mem (clr : Q2 → Rat) (rings : List Ring) (q : Q2) (c : Rat)
    (h : planarInner clr rings = some (q, c)) :
    c = clr q ∧ ∃ s ∈ slabs rings, q ∈ candidates rings s.2 := by
  unfold planarInner at h
  generalize hl : (slabs rings).flatMap (fun s => candidates rings s.2) = l at h
  have key : ∀ (l : List Q2) (b : Option (Q2 × Rat)),
      (∀ p, b = some p → p.2 = clr p.1 ∧ ∃ s ∈ slabs rings, p.1 ∈ candidates rings s.2) →
      (∀ x ∈ l, ∃ s ∈ slabs rings, x ∈ candidates rings s.2) →
      ∀ p, l.foldl (fun best q => match best with
        | none => some (q, clr q)
        | some b => if clr q > b.2 then some (q, clr q) else some b) b = some p →
        p.2 = clr p.1 ∧ ∃ s ∈ slabs rings, p.1 ∈ candidates rings s.2 := by
    intro l
    induction l with
    | nil => intro b hb _ p hp; exact hb p hp
    | cons x t ih =>
      intro b hb hl p hp
      simp only [List.foldl_cons] at hp
      apply ih _ _ (fun y hy => hl y (List.mem_cons_of_mem _ hy)) p hp
      intro p' hp'
      cases b with
      | none => simp at hp'; subst hp'; exact ⟨rfl, hl x (List.mem_cons_self ..)⟩
      | some b =>
        simp only at hp'
        split at hp'
        · simp at hp'; subst hp'; exact ⟨rfl, hl x (List.mem_cons_self ..)⟩
        · simp at hp'; subst hp'; exact hb b rfl
  have := key l none (by simp) (by rw [← hl]; intro x hx; simp only [List.mem_flatMap] at hx; exact hx) (q, c) h
  exact this

/-- **The probe spec for the fix.** If `planar_inner` returns a point with clearance > 0, and the
clearance is positive only off the boundary, the point is strictly inside the region: even-odd
inside, and on no edge. -/
theorem planarInner_spec (clr : Q2 → Rat) (rings : List Ring)
    (hclr : ∀ q, 0 < clr q → ¬ OnBoundary rings q) (q : Q2) (c : Rat)
    (h : planarInner clr rings = some (q, c)) (hc : 0 < c) :
    Inside rings q ∧ ¬ OnBoundary rings q := by
  obtain ⟨rfl, s, _, hq⟩ := planarInner_mem clr rings q c h
  have hoff := hclr q hc
  have hy : q.y = s.2 := by
    simp only [candidates, List.mem_map] at hq; obtain ⟨_, _, rfl⟩ := hq; rfl
  refine ⟨candidate_inside rings s.2 q hq ?_, hoff⟩
  intro hx
  apply hoff
  have := crossing_on_boundary rings s.2 q.x hx
  rw [← hy] at this
  exact this

/-! ## `beside_partner` -/

/-- A cross-section vector. -/
structure V2 where
  a : Rat
  b : Rat
  deriving DecidableEq, Repr

def dot (u v : V2) : Rat := u.a * v.a + u.b * v.b
def smul (t : Rat) (u : V2) : V2 := ⟨t * u.a, t * u.b⟩

theorem dot_smul (t : Rat) (u v : V2) : dot (smul t u) v = t * dot u v := by
  simp only [dot, smul]; grind

/-- The other body near the partner's edge E (at the origin): inside P's half-space (normal `nP`)
and g's (normal `n`), both where E is convex, either where it is concave. Interior (strict) and
exterior; a point on both planes is neither. -/
inductive Pos | interior | exterior | boundary
  deriving DecidableEq, Repr

def wedge (convex : Bool) (nP n : V2) (v : V2) : Pos :=
  let p := dot nP v
  let g := dot n v
  if convex then
    (if p < 0 ∧ g < 0 then .interior else if 0 < p ∨ 0 < g then .exterior else .boundary)
  else
    (if p < 0 ∨ g < 0 then .interior else if 0 < p ∧ 0 < g then .exterior else .boundary)

inductive Beside | on | off (c : Option Bool)  -- Off(Some true) = In, Off(Some false) = Out
  deriving DecidableEq, Repr

/-- The code's decision (cos = here · there for unit directions in P's plane; `here = ±there`). -/
def decide' (here there n : V2) : Beside :=
  let cos := dot here there
  if cos > 0 then .on
  else
    let lean := dot here n
    .off (if lean < 0 then some true else if 0 < lean then some false else none)

/-- **`beside_partner` decides exactly.** P runs along `there` from E (so `nP · there = 0`); g
leaves E with `n · there < 0` where E is convex and `> 0` where concave. For the region running
along `here = -there` (past the edge, in P's plane), every point `t · here`, t > 0, is interior of
the other body iff the code says In, exterior iff it says Out. Where `here = there` the code says On:
the region lies on P. -/
theorem beside_exact (there nP n : V2) (hP : dot nP there = 0) (hthere : 0 < dot there there)
    (convex : Bool) (hcv : if convex then dot n there < 0 else 0 < dot n there) (t : Rat) (ht : 0 < t) :
    decide' there there n = .on ∧
    (let here := smul (-1) there
     let x := smul t here
     (decide' here there n = .off (some true) ↔ wedge convex nP n x = .interior) ∧
     (decide' here there n = .off (some false) ↔ wedge convex nP n x = .exterior)) := by
  refine ⟨by simp [decide', hthere], ?_⟩
  simp only
  have e1 : dot (smul (-1) there) there = -dot there there := by rw [dot_smul]; grind
  have e2 : dot (smul (-1) there) n = -dot n there := by rw [dot_smul]; simp [dot]; grind
  have e3 : dot nP (smul t (smul (-1) there)) = 0 := by
    have : dot nP (smul t (smul (-1) there)) = t * (-1) * dot nP there := by simp [dot, smul]; grind
    rw [this, hP]; grind
  have e4 : dot n (smul t (smul (-1) there)) = -(t * dot n there) := by simp [dot, smul]; grind
  have hc : ¬ (dot (smul (-1) there) there > 0) := by rw [e1]; grind
  unfold decide' wedge
  simp only [hc, ite_false, e2, e3, e4]
  cases convex with
  | false =>
    -- concave: n · there > 0, so lean = -n·there < 0 (In), and the point is in g's open half-space
    simp only [Bool.false_eq_true, ite_false] at hcv ⊢
    have h1 : -dot n there < 0 := by grind
    have h2 : -(t * dot n there) < 0 := by have := Rat.mul_pos ht hcv; grind
    simp [h1, h2]
  | true =>
    -- convex: n · there < 0, so lean > 0 (Out), and the point is outside g's half-space
    simp only [ite_true] at hcv ⊢
    have h1 : ¬ -dot n there < 0 := by grind
    have h1' : 0 < -dot n there := by grind
    have h2 : 0 < -(t * dot n there) := by
      have := Rat.mul_pos ht (show 0 < -dot n there by grind); grind
    have h2' : ¬ -(t * dot n there) < 0 := by grind
    simp [h1, h1', h2, h2']

end Coverage.Inner
