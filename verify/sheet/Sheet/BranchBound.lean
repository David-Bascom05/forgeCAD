import Mathlib.Topology.MetricSpace.Pseudo.Defs
import Mathlib.Data.Real.Basic
import Mathlib.Tactic

/-!
# The bend simulator's branch and bound, sound (kernel/forge-brake/src/bb.rs)

forge-brake certifies, for a piece of the part against a tool (or against another piece), over the
motion parameter `s` (the fold angle `φ`, or the ram height):
* `Clear lower`: the signed distance is at least `lower` at **every** point of the piece through
  the whole motion, and `lower ≥ limit − res`;
* `Hit q`: `q` is a real point of the piece at a real motion parameter whose distance is
  **proven** below `limit` (a collision, or a clearance violation);
* `Undecided lower`: the budget ran out; `lower` is what was proven, nothing more.

This file states those claims and proves the search sound **for any order of work** (bb.rs pops a
heap by `lb`; the proof does not care), given the contracts of its primitives:

* `encl`: a cell's lower bound is sound: `∀ x ∈ c, D x → lower c ≤ d x` (bb.rs `lower`, built
  from the interval enclosures; `lipschitz_lower` proves the default `sd(mid) − hd` sound when `d`
  is 1-Lipschitz and the cell lies in the ball of radius `hd` round `mid`);
* `upper`: a point bound is sound: `d q ≤ up q` (`sd_bounds(at).1`);
* `cover`: the halves of a split cover the cell (bb.rs splits at one `mid` both halves share);
* `mayHold c = false → ∀ x ∈ c, ¬ D x` (cells off the piece are dropped);
* comparisons are exact (over ℝ here; in the kernel, on outward-rounded intervals or exact
  rationals, never a bare `f64` comparison of rounded quantities).

The interval primitives enter only through `encl`. `Encl` is the contract a corrected
`Interval::sin` (the trace session's fix) must meet; `Encl.comp` carries it through a formula.

**The rule this file fixes for bb.rs**: a leaf (a cell too small to split) that cannot be cleared
only lowers `lower`; when the work runs out the verdict is `Clear` only if `lower ≥ limit − res`,
otherwise `Undecided` — bb.rs today returns `Clear{lower: min(lower, upper)}` there unchecked.
-/

namespace Sheet.BB

/-! ## Interval contracts -/

/-- A closed interval of ℝ. -/
structure Ival where
  lo : ℝ
  hi : ℝ

def Ival.mem (I : Ival) (x : ℝ) : Prop := I.lo ≤ x ∧ x ≤ I.hi

/-- The enclosure contract of an interval extension `F` of `f`. -/
def Encl (f : ℝ → ℝ) (F : Ival → Ival) : Prop := ∀ I x, I.mem x → (F I).mem (f x)

theorem Encl.comp {f g : ℝ → ℝ} {F G : Ival → Ival} (hf : Encl f F) (hg : Encl g G) :
    Encl (g ∘ f) (G ∘ F) := fun I x hx => hg (F I) (f x) (hf I x hx)

/-- Interval addition, outward: any upper bound of the exact sum's upper end and lower bound of
its lower end (the kernel's `next_up`/`next_down` of the rounded sums) keeps the enclosure. -/
def Ival.addOut (rd ru : ℝ → ℝ) (I J : Ival) : Ival := ⟨rd (I.lo + J.lo), ru (I.hi + J.hi)⟩

theorem Ival.addOut_mem (rd ru : ℝ → ℝ) (hrd : ∀ r, rd r ≤ r) (hru : ∀ r, r ≤ ru r)
    {I J : Ival} {x y : ℝ} (hx : I.mem x) (hy : J.mem y) : (Ival.addOut rd ru I J).mem (x + y) :=
  ⟨le_trans (hrd _) (by linarith [hx.1, hy.1]), le_trans (by linarith [hx.2, hy.2]) (hru _)⟩

/-- The constant `c` as an interval is sound only when `c` is the exact value: bb.rs's
`Mul<f64>`/`Add<f64>` take a rounded `f64` constant (such as `1/BA`) as exact; the true constant
must be enclosed instead. -/
theorem const_encl_needs_exact (c c' : ℝ) (h : (⟨c', c'⟩ : Ival).mem c) : c = c' :=
  le_antisymm h.2 h.1

/-! ## Lower bounds of a cell -/

/-- **The 1-Lipschitz lower bound** (bb.rs `lower`'s default, `sd(mid) − hd`): sound when the
distance is 1-Lipschitz, the cell is in the ball of radius `hd` round `mid`, and the point bound
at `mid` is a lower bound. -/
theorem lipschitz_lower {X : Type} [PseudoMetricSpace X] (d : X → ℝ)
    (hd1 : ∀ x y, |d x - d y| ≤ dist x y) (mid x : X) (hd lo : ℝ) (hx : dist x mid ≤ hd)
    (hlo : lo ≤ d mid) : lo - hd ≤ d x := by
  have := hd1 mid x
  rw [dist_comm] at this
  have := (abs_le.mp this).2
  linarith

/-! ## The search -/

variable {X C : Type}

/-- What the search is given. -/
structure Problem (X C : Type) where
  /-- The true signed distance at a point (of the piece, at a motion parameter). -/
  d : X → ℝ
  /-- The points that are the piece's (the domain). -/
  D : X → Prop
  /-- Cell membership. -/
  mem : C → X → Prop
  lower : C → ℝ
  witness : C → Option X
  up : X → ℝ
  split : C → List C
  leaf : C → Bool
  mayHold : C → Bool
  /-- `encl`: the cell's lower bound is sound. -/
  encl : ∀ c x, mem c x → D x → lower c ≤ d x
  /-- `upper`: the point bound is sound, at points of the piece. -/
  upper : ∀ q, D q → d q ≤ up q
  /-- The witness is a point of the piece (any real point; it need not lie in the cell). -/
  witD : ∀ c q, witness c = some q → D q
  /-- The halves cover the cell. -/
  cover : ∀ c x, mem c x → ∃ c' ∈ split c, mem c' x
  /-- Cells off the piece are dropped. -/
  drop : ∀ c x, mayHold c = false → mem c x → ¬ D x

inductive Verdict (X : Type)
  | clear (lower : ℝ)
  | hit (q : X)
  | undecided (lower : ℝ)

/-- One search, by fuel: `work` the cells still open, `lo` the least lower bound of the cells
closed as leaves. `pick` reorders the work at every step (bb.rs: a heap by `lb`); any order. -/
def search (pb : Problem X C) (pick : List C → List C) (limit res : ℝ) [DecidablePred (· ≥ limit)]
    [∀ r : ℝ, Decidable (r < limit)] [∀ r : ℝ, Decidable (r ≥ limit - res)] :
    ℕ → List C → ℝ → Verdict X
  | _, [], lo => if lo ≥ limit - res then .clear lo else .undecided lo
  | 0, c :: rest, lo =>
      let m := (c :: rest).foldl (fun m c => min m (pb.lower c)) lo
      if m ≥ limit - res then .clear m else .undecided m
  | n + 1, w, lo =>
      match pick w with
      | [] => if lo ≥ limit - res then .clear lo else .undecided lo
      | c :: rest =>
        if pb.lower c ≥ limit then search pb pick limit res n rest lo
        else match pb.witness c with
          | some q => if pb.up q < limit then .hit q else
              if pb.leaf c then search pb pick limit res n rest (min lo (pb.lower c))
              else search pb pick limit res n (rest ++ (pb.split c).filter pb.mayHold) lo
          | none =>
              if pb.leaf c then search pb pick limit res n rest (min lo (pb.lower c))
              else search pb pick limit res n (rest ++ (pb.split c).filter pb.mayHold) lo

/-- The invariant: every point of the piece in a root cell is in an open cell, or proven at least
`limit` (closed clear), or at least `lo` (closed as a leaf). -/
def Inv (pb : Problem X C) (roots : List C) (limit : ℝ) (w : List C) (lo : ℝ) : Prop :=
  ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → (∃ c ∈ w, pb.mem c x) ∨ min limit lo ≤ pb.d x

theorem foldl_min_le (pb : Problem X C) : ∀ (w : List C) (m : ℝ),
    (w.foldl (fun m c => min m (pb.lower c)) m ≤ m) ∧
      ∀ c ∈ w, w.foldl (fun m c => min m (pb.lower c)) m ≤ pb.lower c
  | [], m => ⟨le_rfl, fun _ h => nomatch h⟩
  | c :: w, m => by
      obtain ⟨h1, h2⟩ := foldl_min_le pb w (min m (pb.lower c))
      refine ⟨le_trans h1 (min_le_left _ _), fun c' hc' => ?_⟩
      rcases List.mem_cons.mp hc' with rfl | hc'
      · exact le_trans h1 (min_le_right _ _)
      · exact h2 c' hc'

/-- **Soundness of the search**, for every order of work (`pick` any permutation):
* `clear lo` ⇒ every point of the piece in a root cell is at distance at least `min limit lo`,
  and that is at least `limit − res`;
* `hit q` ⇒ `q` is a point of the piece at distance below `limit`;
* `undecided lo` ⇒ every such point is at distance at least `min limit lo` (what was proven). -/
theorem search_sound (pb : Problem X C) (pick : List C → List C)
    (hpick : ∀ w, (pick w).Perm w) (limit res : ℝ) (hres : 0 ≤ res)
    [DecidablePred (· ≥ limit)] [∀ r : ℝ, Decidable (r < limit)]
    [∀ r : ℝ, Decidable (r ≥ limit - res)] (roots : List C) :
    ∀ (n : ℕ) (w : List C) (lo : ℝ), Inv pb roots limit w lo →
      match search pb pick limit res n w lo with
      | .clear l => limit - res ≤ min limit l ∧
          ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit l ≤ pb.d x
      | .hit q => pb.D q ∧ pb.d q < limit
      | .undecided l =>
          ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit l ≤ pb.d x := by
  intro n
  induction n with
  | zero =>
    intro w lo hinv
    cases w with
    | nil =>
      simp only [search]
      have hall : ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit lo ≤ pb.d x :=
        fun x hD hr => (hinv x hD hr).resolve_left (by simp)
      split_ifs with h
      · exact ⟨le_min (by linarith) h, hall⟩
      · exact hall
    | cons c rest =>
      simp only [search]
      obtain ⟨hm, hmc⟩ := foldl_min_le pb (c :: rest) lo
      set m := (c :: rest).foldl (fun m c => min m (pb.lower c)) lo
      have hall : ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit m ≤ pb.d x := by
        intro x hD hr
        rcases hinv x hD hr with ⟨c', hc', hx⟩ | h
        · exact le_trans (min_le_right _ _) (le_trans (hmc c' hc') (pb.encl c' x hx hD))
        · exact le_trans (min_le_min_left _ hm) h
      split_ifs with h
      · exact ⟨le_min (by linarith) h, hall⟩
      · exact hall
  | succ n ih =>
    intro w lo hinv
    cases w with
    | nil =>
      simp only [search]
      have hall : ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit lo ≤ pb.d x :=
        fun x hD hr => (hinv x hD hr).resolve_left (by simp)
      split_ifs with h
      · exact ⟨le_min (by linarith) h, hall⟩
      · exact hall
    | cons c0 rest0 =>
    simp only [search]
    set w := c0 :: rest0 with hw0
    have hinvp : Inv pb roots limit (pick w) lo := by
      intro x hD hr
      rcases hinv x hD hr with ⟨c, hc, hx⟩ | h
      · exact Or.inl ⟨c, (hpick w).mem_iff.mpr hc, hx⟩
      · exact Or.inr h
    -- the cases of the step, each keeping the invariant
    have close_clear : ∀ c rest, pick w = c :: rest → pb.lower c ≥ limit →
        Inv pb roots limit rest lo := by
      intro c rest hw hc x hD hr
      rcases hinvp x hD hr with ⟨c', hc', hx⟩ | h
      · rw [hw] at hc'
        rcases List.mem_cons.mp hc' with rfl | hc'
        · exact Or.inr (le_trans (min_le_left _ _) (le_trans hc (pb.encl _ x hx hD)))
        · exact Or.inl ⟨c', hc', hx⟩
      · exact Or.inr h
    have close_leaf : ∀ c rest, pick w = c :: rest →
        Inv pb roots limit rest (min lo (pb.lower c)) := by
      intro c rest hw x hD hr
      rcases hinvp x hD hr with ⟨c', hc', hx⟩ | h
      · rw [hw] at hc'
        rcases List.mem_cons.mp hc' with rfl | hc'
        · exact Or.inr (le_trans (min_le_right _ _)
            (le_trans (min_le_right _ _) (pb.encl _ x hx hD)))
        · exact Or.inl ⟨c', hc', hx⟩
      · exact Or.inr (le_trans (min_le_min_left _ (min_le_left _ _)) h)
    have do_split : ∀ c rest, pick w = c :: rest →
        Inv pb roots limit (rest ++ (pb.split c).filter pb.mayHold) lo := by
      intro c rest hw x hD hr
      rcases hinvp x hD hr with ⟨c', hc', hx⟩ | h
      · rw [hw] at hc'
        rcases List.mem_cons.mp hc' with rfl | hc'
        · obtain ⟨c'', hc'', hx''⟩ := pb.cover _ x hx
          have hm : pb.mayHold c'' = true := by
            by_contra hm; exact pb.drop c'' x (by simpa using hm) hx'' hD
          exact Or.inl ⟨c'', List.mem_append_right _ (List.mem_filter.mpr ⟨hc'', hm⟩), hx''⟩
        · exact Or.inl ⟨c', List.mem_append_left _ hc', hx⟩
      · exact Or.inr h
    cases hw : pick w with
    | nil =>
      have hall : ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit lo ≤ pb.d x := by
        intro x hD hr
        rcases hinvp x hD hr with ⟨c, hc, _⟩ | h
        · rw [hw] at hc; exact absurd hc (by simp)
        · exact h
      simp only
      split_ifs with h
      · exact ⟨le_min (by linarith) h, hall⟩
      · exact hall
    | cons c rest =>
      simp only
      by_cases h1 : pb.lower c ≥ limit
      · rw [if_pos h1]; exact ih rest lo (close_clear c rest hw h1)
      · rw [if_neg h1]
        cases hq : pb.witness c with
        | some q =>
          simp only
          by_cases h2 : pb.up q < limit
          · rw [if_pos h2]
            exact ⟨pb.witD c q hq, lt_of_le_of_lt (pb.upper q (pb.witD c q hq)) h2⟩
          · rw [if_neg h2]
            by_cases h3 : pb.leaf c = true
            · rw [if_pos h3]; exact ih rest _ (close_leaf c rest hw)
            · rw [if_neg h3]; exact ih _ lo (do_split c rest hw)
        | none =>
          simp only
          by_cases h3 : pb.leaf c = true
          · rw [if_pos h3]; exact ih rest _ (close_leaf c rest hw)
          · rw [if_neg h3]; exact ih _ lo (do_split c rest hw)

/-- The start: the roots that may hold are the work, nothing closed (`lo = limit`). -/
theorem inv_start (pb : Problem X C) (roots : List C) (limit : ℝ) :
    Inv pb roots limit (roots.filter pb.mayHold) limit := by
  intro x hD ⟨r, hr, hx⟩
  left
  refine ⟨r, List.mem_filter.mpr ⟨hr, ?_⟩, hx⟩
  by_contra hm; exact pb.drop r x (by simpa using hm) hx hD

/-- **The certificate**: a run from the roots is sound. A `clear` verdict means the clearance
`limit − res` holds at every point of the piece through the whole motion. -/
theorem certify_sound (pb : Problem X C) (pick : List C → List C)
    (hpick : ∀ w, (pick w).Perm w) (limit res : ℝ) (hres : 0 ≤ res)
    [DecidablePred (· ≥ limit)] [∀ r : ℝ, Decidable (r < limit)]
    [∀ r : ℝ, Decidable (r ≥ limit - res)] (roots : List C) (budget : ℕ) :
    match search pb pick limit res budget (roots.filter pb.mayHold) limit with
    | .clear l => limit - res ≤ min limit l ∧
        ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit l ≤ pb.d x
    | .hit q => pb.D q ∧ pb.d q < limit
    | .undecided l => ∀ x, pb.D x → (∃ r ∈ roots, pb.mem r x) → min limit l ≤ pb.d x :=
  search_sound pb pick hpick limit res hres roots budget _ _ (inv_start pb roots limit)

end Sheet.BB
