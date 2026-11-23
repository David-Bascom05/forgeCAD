import ForgeExact.DecideOnce.Boxes
import ForgeExact.DecideOnce.Ledger
import ForgeExact.DecideOnce.Nef
import ForgeExact.Jordan

/-!
# `Exact` for every box family: (c) as a theorem

The box generator (`boxLedger K boxes`, `Boxes.lean`) builds a ledger on the grid carriers
`x = 0 .. K`, `y = 0 .. K`, `z = 0 .. K` (`gridPs K`) from any finite list of boxes on the grid.
`box_exact` proves every field of `Exact` for it, for every such family, with no hypothesis but
that each box has positive extent on every axis and lies in `0 .. K` (`Valid`):

* cells are the doubled-coordinate cells (`DCell`): `covCell_eq_covOf`, `realised_eq`, `cov_inj`;
* `compat`: the open box is a union of full cells (`box_compat`);
* pieces, facets, sides and the witness are the box faces' cells, decided by `holds`
  (`box_pieceFacet`, `box_oneFace`, `box_pieceReal`, `box_witness`);
* `boundary` / `complete`: a facet bounds a box exactly when it is one of its face cells
  (`box_boundary`, `box_complete`);
* `edges` / `verts`: the edge and vertex cells the generator lists are exactly the realised cells
  of dimension 1 and 0 below a facet (`box_edges`, `box_verts`, from `isEdge_iff`, `isVert_iff`).

`box_correct` and `box_refused_iff` are then (c) for every box family: each valid answer is the exact
regularised boolean of the open boxes, and the zero-thickness refusal is exactly non-manifoldness.
-/

namespace ForgeExact.DecideOnce.BoxEx

open ForgeExact ForgeExact.Ref

/-! ## One axis -/

def sig (a : Int) (k : Nat) : Sgn :=
  if a < 2 * (k : Int) then .neg else if a = 2 * (k : Int) then .zero else .pos

theorem axSigns_eq (K : Nat) (a : Int) : axSigns K a = (List.range (K + 1)).map (sig a) := rfl

theorem len_axSigns (K : Nat) (a : Int) : (axSigns K a).length = K + 1 := by simp [axSigns_eq]

theorem get_axSigns (K : Nat) (a : Int) {k : Nat} (hk : k < K + 1) : (axSigns K a)[k]? = some (sig a k) := by
  simp [axSigns_eq, hk]

/-- The coordinate `q` lies in the doubled cell `a` of the axis `0 .. K`. -/
def cell1 (K : Nat) (a : Int) (q : Rat) : Prop := ∀ k, k < K + 1 → sgnQ (q - ((k : Int) : Rat)) = sig a k

theorem cast_two_mul (a : Int) (k : Nat) : (a : Rat) / 2 - ((k : Int) : Rat) = ((a - 2 * (k : Int) : Int) : Rat) / 2 := by
  simp only [Rat.intCast_sub, Rat.intCast_mul]
  have : ((2 : Int) : Rat) = 2 := rfl
  rw [this]; grind

theorem half_pos (z : Int) : 0 < (z : Rat) / 2 ↔ 0 < z := by
  rw [Rat.div_def, Rat.mul_comm, Jordan.mul_pos_iff_left (Rat.inv_pos.mpr (by decide))]
  exact Rat.intCast_lt_intCast (a := 0)

theorem half_neg (z : Int) : (z : Rat) / 2 < 0 ↔ z < 0 := by
  have := half_pos (-z)
  simp only [Rat.intCast_neg] at this
  constructor
  · intro h; have : 0 < -(z : Rat) / 2 := by grind
    have h2 := (half_pos (-z)).mp (by simp only [Rat.intCast_neg]; exact this)
    omega
  · intro h; have h2 := (half_pos (-z)).mpr (by omega); simp only [Rat.intCast_neg] at h2; grind

theorem sgnQ_half (z : Int) : sgnQ ((z : Rat) / 2) = if z < 0 then .neg else if z = 0 then .zero else .pos := by
  unfold sgnQ
  by_cases h1 : 0 < z
  · have := (half_pos z).mpr h1
    have h3 : ¬z < 0 := by omega
    have h4 : z ≠ 0 := by omega
    simp [this, h3, h4]
  · by_cases h2 : z < 0
    · have := (half_neg z).mpr h2
      have : ¬(z : Rat) / 2 > 0 := by grind
      simp [*]
    · have hz : z = 0 := by omega
      subst hz
      have : ((0 : Int) : Rat) / 2 = 0 := by rw [Rat.div_def]; simp
      rw [this]; simp

theorem half_cell (K : Nat) (a : Int) : cell1 K a ((a : Rat) / 2) := by
  intro k _
  rw [cast_two_mul, sgnQ_half]
  unfold sig
  by_cases h1 : a < 2 * (k : Int)
  · have : a - 2 * (k : Int) < 0 := by omega
    simp [h1, this]
  · by_cases h2 : a = 2 * (k : Int)
    · simp [h1, h2]
    · have h3 : ¬a - 2 * (k : Int) < 0 := by omega
      have h4 : a - 2 * (k : Int) ≠ 0 := by omega
      simp [h1, h2, h3, h4]

theorem ic_lt {a b : Int} : (a : Rat) < (b : Rat) ↔ a < b := Rat.intCast_lt_intCast
theorem ic_le {a b : Int} : (a : Rat) ≤ (b : Rat) ↔ a ≤ b := Rat.intCast_le_intCast
theorem ic_eq {a b : Int} : (a : Rat) = (b : Rat) ↔ a = b := Rat.intCast_inj

theorem sgnQ_sub_pos {q r : Rat} : sgnQ (q - r) = .pos ↔ r < q := by rw [sgnQ_pos_iff]; grind
theorem sgnQ_sub_neg {q r : Rat} : sgnQ (q - r) = .neg ↔ q < r := by rw [sgnQ_neg_iff]; grind
theorem sgnQ_sub_zero {q r : Rat} : sgnQ (q - r) = .zero ↔ q = r := by rw [sgnQ_zero_iff]; grind

/-- The doubled cell of a coordinate, clamped to the axis `0 .. K`. -/
def dbl1 (K : Nat) (q : Rat) : Int :=
  if q < 0 then -1
  else if ((K : Int) : Rat) < q then 2 * (K : Int) + 1
  else if ((q.floor : Int) : Rat) = q then 2 * q.floor else 2 * q.floor + 1

theorem dbl1_range (K : Nat) (q : Rat) : -1 ≤ dbl1 K q ∧ dbl1 K q ≤ 2 * (K : Int) + 1 := by
  unfold dbl1
  have f1 := Rat.floor_le q
  have f2 := Rat.lt_floor_add_one q
  rw [Rat.intCast_add] at f2
  split
  · omega
  · split
    · omega
    · rename_i h1 h2
      have h0 : (0 : Rat) ≤ q := by grind
      have hf0 : 0 ≤ q.floor := by
        have : ((-1 : Int) : Rat) < (q.floor : Rat) := by
          have : ((-1 : Int) : Rat) = -1 := rfl
          rw [this]; have : ((1 : Int) : Rat) = 1 := rfl; grind
        have := ic_lt.mp this; omega
      have hfK : q.floor ≤ (K : Int) := by
        have : (q.floor : Rat) ≤ ((K : Int) : Rat) := by grind
        exact ic_le.mp this
      split <;> omega

theorem dbl1_cell (K : Nat) (q : Rat) : cell1 K (dbl1 K q) q := by
  intro k hk
  have f1 := Rat.floor_le q
  have f2 := Rat.lt_floor_add_one q
  rw [Rat.intCast_add] at f2
  have one : ((1 : Int) : Rat) = 1 := rfl
  rw [one] at f2
  have hkK : (k : Int) ≤ (K : Int) := by omega
  have hkK' : ((k : Int) : Rat) ≤ ((K : Int) : Rat) := ic_le.mpr hkK
  have hk0 : (0 : Rat) ≤ ((k : Int) : Rat) := by
    have := ic_le.mpr (show (0 : Int) ≤ (k : Int) by omega); exact this
  unfold dbl1 sig
  split
  · rename_i h; rw [sgnQ_sub_neg.mpr (by grind)]; simp; omega
  · split
    · rename_i h1 h2; rw [sgnQ_sub_pos.mpr (by grind)]
      have : ¬(2 * (K : Int) + 1 < 2 * (k : Int)) := by omega
      have : ¬(2 * (K : Int) + 1 = 2 * (k : Int)) := by omega
      simp [*]
    · split
      · rename_i h1 h2 h3
        -- q is the integer f
        by_cases hlt : q.floor < (k : Int)
        · have : q < ((k : Int) : Rat) := by rw [← h3]; exact ic_lt.mpr hlt
          rw [sgnQ_sub_neg.mpr this]; simp; omega
        · by_cases heq : q.floor = (k : Int)
          · have : q = ((k : Int) : Rat) := by rw [← h3, heq]
            rw [sgnQ_sub_zero.mpr this]; simp [heq]
          · have : ((k : Int) : Rat) < q := by rw [← h3]; exact ic_lt.mpr (by omega)
            rw [sgnQ_sub_pos.mpr this]
            have : ¬(2 * q.floor < 2 * (k : Int)) := by omega
            have : ¬(2 * q.floor = 2 * (k : Int)) := by omega
            simp [*]
      · rename_i h1 h2 h3
        have hlt' : (q.floor : Rat) < q := by grind
        by_cases hlt : q.floor < (k : Int)
        · have : q < ((k : Int) : Rat) := by
            have : ((q.floor + 1 : Int) : Rat) ≤ ((k : Int) : Rat) := ic_le.mpr (by omega)
            rw [Rat.intCast_add, one] at this; grind
          rw [sgnQ_sub_neg.mpr this]; simp; omega
        · have : ((k : Int) : Rat) < q := by
            have : ((k : Int) : Rat) ≤ (q.floor : Rat) := ic_le.mpr (by omega)
            grind
          rw [sgnQ_sub_pos.mpr this]
          have : ¬(2 * q.floor + 1 < 2 * (k : Int)) := by omega
          have : ¬(2 * q.floor + 1 = 2 * (k : Int)) := by omega
          simp [*]

/-- On the axis's range, the sign pattern determines the doubled cell. -/
theorem sig_inj (K : Nat) {a b : Int} (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1)
    (hb : -1 ≤ b ∧ b ≤ 2 * (K : Int) + 1) (h : ∀ k, k < K + 1 → sig a k = sig b k) : a = b := by
  apply Classical.byContradiction
  intro hne
  -- the larger one, at the plane just below or at it, differs
  have key : ∀ a b : Int, -1 ≤ a → b ≤ 2 * (K : Int) + 1 → a < b →
      ∃ k, k < K + 1 ∧ sig a k ≠ sig b k := by
    intro a b ha hb hab
    have hb0 : 0 ≤ b := by omega
    by_cases he : b % 2 = 0
    · refine ⟨(b / 2).toNat, ?_, ?_⟩
      · omega
      · unfold sig
        have e : 2 * (((b / 2).toNat : Nat) : Int) = b := by omega
        rw [e]; simp [hab]
    · refine ⟨((b - 1) / 2).toNat, ?_, ?_⟩
      · omega
      · unfold sig
        have e : 2 * ((((b - 1) / 2).toNat : Nat) : Int) = b - 1 := by omega
        rw [e]
        have : ¬(b < b - 1) := by omega
        have : ¬(b = b - 1) := by omega
        by_cases h1 : a < b - 1
        · simp [*]
        · have : a = b - 1 := by omega
          simp [*]
  rcases Int.lt_or_gt_of_ne hne with hlt | hlt
  · obtain ⟨k, hk, hd⟩ := key a b ha.1 hb.2 hlt
    exact hd (h k hk)
  · obtain ⟨k, hk, hd⟩ := key b a hb.1 ha.2 hlt
    exact hd (h k hk).symm

theorem cell1_unique (K : Nat) {a b : Int} {q : Rat} (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1)
    (hb : -1 ≤ b ∧ b ≤ 2 * (K : Int) + 1) (h1 : cell1 K a q) (h2 : cell1 K b q) : a = b :=
  sig_inj K ha hb fun k hk => (h1 k hk).symm.trans (h2 k hk)

/-- Within a box's ends, the coordinate's order is the doubled cell's. -/
theorem lo_iff (K : Nat) {a : Int} {q : Rat} (h : cell1 K a q) {lo : Nat} (hlo : lo < K + 1) :
    ((lo : Int) : Rat) < q ↔ 2 * (lo : Int) < a := by
  have := h lo hlo
  rw [← sgnQ_sub_pos, this]
  unfold sig
  by_cases h1 : a < 2 * (lo : Int) <;> by_cases h2 : a = 2 * (lo : Int) <;> simp [h1, h2] <;> omega

theorem hi_iff (K : Nat) {a : Int} {q : Rat} (h : cell1 K a q) {hi : Nat} (hhi : hi < K + 1) :
    q < ((hi : Int) : Rat) ↔ a < 2 * (hi : Int) := by
  have := h hi hhi
  rw [← sgnQ_sub_neg, this]
  unfold sig
  by_cases h1 : a < 2 * (hi : Int) <;> by_cases h2 : a = 2 * (hi : Int) <;> simp [h1, h2]

/-! ## Three axes -/

def unitV : Nat → V3
  | 0 => ⟨1, 0, 0⟩
  | 1 => ⟨0, 1, 0⟩
  | _ => ⟨0, 0, 1⟩

def crd : Nat → V3 → Rat
  | 0, x => x.x
  | 1, x => x.y
  | _, x => x.z

def dc : Nat → DCell → Int
  | 0, c => c.1
  | 1, c => c.2.1
  | _, c => c.2.2

/-- The carriers of one axis: the planes `x_ax = k`, `k = 0 .. K`. -/
def axPl (K ax : Nat) : List Plane := (List.range (K + 1)).map fun (k : Nat) => ⟨unitV ax, ((k : Int) : Rat)⟩

/-- The box family's carriers, in the order `cov` lists its signs. -/
def gridPs (K : Nat) : List Plane := axPl K 0 ++ axPl K 1 ++ axPl K 2

theorem eval_unit (ax : Nat) (c : Rat) (x : V3) : (⟨unitV ax, c⟩ : Plane).eval x = crd ax x - c := by
  match ax with
  | 0 => simp [Plane.eval, V3.dot, unitV, crd]; grind
  | 1 => simp [Plane.eval, V3.dot, unitV, crd]; grind
  | _ + 2 => simp [Plane.eval, V3.dot, unitV, crd]; grind

theorem covCell_map : ∀ (ls : List Nat) (P : Nat → Plane) (s : Nat → Sgn) (x : V3),
    covCell (ls.map P) (ls.map s) x ↔ ∀ k ∈ ls, sgnQ ((P k).eval x) = s k
  | [], _, _, _ => by simp [covCell]
  | k :: ls, P, s, x => by
    simp only [List.map_cons, covCell, List.mem_cons, forall_eq_or_imp]
    rw [covCell_map ls P s x]

theorem covCell_append : ∀ {p1 p2 : List Plane} {t1 t2 : Cov} {x : V3}, t1.length = p1.length →
    (covCell (p1 ++ p2) (t1 ++ t2) x ↔ covCell p1 t1 x ∧ covCell p2 t2 x)
  | [], _, [], _, _, _ => by simp [covCell]
  | p :: p1, p2, t :: t1, t2, x, h => by
    simp only [List.cons_append, covCell]
    rw [covCell_append (by simpa using h)]
    simp [and_assoc]
  | [], _, _ :: _, _, _, h => by simp at h
  | _ :: _, _, [], _, _, h => by simp at h

theorem covCell_ax (K ax : Nat) (a : Int) (x : V3) :
    covCell (axPl K ax) (axSigns K a) x ↔ cell1 K a (crd ax x) := by
  unfold axPl
  rw [axSigns_eq, covCell_map]
  simp only [List.mem_range, eval_unit]
  rfl

theorem covCell_cov (K : Nat) (c : DCell) (x : V3) :
    covCell (gridPs K) (cov K c) x ↔ cell1 K c.1 x.x ∧ cell1 K c.2.1 x.y ∧ cell1 K c.2.2 x.z := by
  unfold gridPs cov
  rw [covCell_append (by simp [len_axSigns, axPl]), covCell_append (by simp [len_axSigns, axPl])]
  rw [covCell_ax, covCell_ax, covCell_ax]
  simp [crd, and_assoc]

def dblV (K : Nat) (x : V3) : DCell := (dbl1 K x.x, dbl1 K x.y, dbl1 K x.z)

theorem covCell_eq_covOf : ∀ {ps : List Plane} {T : Cov} {x : V3}, covCell ps T x → T = covOf ps x
  | [], [], _, _ => rfl
  | p :: ps, t :: T, x, ⟨h1, h2⟩ => by
    simp only [covOf, List.map_cons, List.cons.injEq]
    exact ⟨h1.symm, covCell_eq_covOf h2⟩
  | [], _ :: _, _, h => h.elim
  | _ :: _, [], _, h => h.elim

theorem dblV_cell (K : Nat) (x : V3) : covCell (gridPs K) (cov K (dblV K x)) x :=
  (covCell_cov K _ x).mpr ⟨dbl1_cell K x.x, dbl1_cell K x.y, dbl1_cell K x.z⟩

/-- **Every cell of the grid is a doubled cell's.** -/
theorem realised_eq (K : Nat) {T : Cov} {x : V3} (h : covCell (gridPs K) T x) : T = cov K (dblV K x) :=
  (covCell_eq_covOf h).trans (covCell_eq_covOf (dblV_cell K x)).symm

/-- In the axes' range. -/
def InR (K : Nat) (c : DCell) : Prop :=
  (-1 ≤ c.1 ∧ c.1 ≤ 2 * (K : Int) + 1) ∧ (-1 ≤ c.2.1 ∧ c.2.1 ≤ 2 * (K : Int) + 1) ∧
    (-1 ≤ c.2.2 ∧ c.2.2 ≤ 2 * (K : Int) + 1)

theorem dblV_inR (K : Nat) (x : V3) : InR K (dblV K x) :=
  ⟨dbl1_range K x.x, dbl1_range K x.y, dbl1_range K x.z⟩

theorem axSigns_inj (K : Nat) {a b : Int} (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1)
    (hb : -1 ≤ b ∧ b ≤ 2 * (K : Int) + 1) (h : axSigns K a = axSigns K b) : a = b := by
  apply sig_inj K ha hb
  intro k hk
  have := congrArg (fun l => l[k]?) h
  simp only [get_axSigns K _ hk, Option.some.injEq] at this
  exact this

theorem cov_inj (K : Nat) {c c' : DCell} (hc : InR K c) (hc' : InR K c') (h : cov K c = cov K c') :
    c = c' := by
  unfold cov at h
  have l1 := len_axSigns K c.1
  have l2 := len_axSigns K c.2.1
  have l1' := len_axSigns K c'.1
  have l2' := len_axSigns K c'.2.1
  rw [List.append_assoc, List.append_assoc] at h
  obtain ⟨e1, h⟩ := List.append_inj h (by rw [l1, l1'])
  obtain ⟨e2, e3⟩ := List.append_inj h (by rw [l2, l2'])
  obtain ⟨c1, c2, c3⟩ := c
  obtain ⟨d1, d2, d3⟩ := c'
  simp only [Prod.mk.injEq]
  exact ⟨axSigns_inj K hc.1 hc'.1 e1, axSigns_inj K hc.2.1 hc'.2.1 e2, axSigns_inj K hc.2.2 hc'.2.2 e3⟩

def pt (c : DCell) : V3 := ⟨(c.1 : Rat) / 2, (c.2.1 : Rat) / 2, (c.2.2 : Rat) / 2⟩

theorem pt_cell (K : Nat) (c : DCell) : covCell (gridPs K) (cov K c) (pt c) :=
  (covCell_cov K c _).mpr ⟨half_cell K _, half_cell K _, half_cell K _⟩

theorem cell_dbl (K : Nat) {c : DCell} {x : V3} (h : covCell (gridPs K) (cov K c) x) (hc : InR K c) :
    dblV K x = c :=
  cov_inj K (dblV_inR K x) hc ((realised_eq K h).symm)

/-! ## One axis's covector, as a list -/

theorem axSigns_succ (K : Nat) (a : Int) : axSigns (K + 1) a = axSigns K a ++ [sig a (K + 1)] := by
  rw [axSigns_eq, axSigns_eq, List.range_succ, List.map_append]; rfl

theorem zero_mem_axSigns (K : Nat) (a : Int) :
    Sgn.zero ∈ axSigns K a ↔ ∃ k, k < K + 1 ∧ a = 2 * (k : Int) := by
  rw [axSigns_eq]
  simp only [List.mem_map, List.mem_range]
  constructor
  · rintro ⟨k, hk, h⟩
    refine ⟨k, hk, ?_⟩
    unfold sig at h
    split at h
    · cases h
    · split at h
      · assumption
      · cases h
  · rintro ⟨k, hk, rfl⟩
    exact ⟨k, hk, by unfold sig; simp⟩

theorem count_axSigns_even (K k : Nat) (hk : k < K + 1) : (axSigns K (2 * (k : Int))).count .zero = 1 := by
  induction K with
  | zero =>
    have : k = 0 := by omega
    subst this; simp [axSigns_eq, sig]
  | succ K ih =>
    rw [axSigns_succ, List.count_append]
    by_cases h : k < K + 1
    · rw [ih h]
      have hs : sig (2 * (k : Int)) (K + 1) = .neg := by unfold sig; simp only [ite_eq_left_iff]; omega
      rw [hs]; simp
    · have hk' : k = K + 1 := by omega
      subst hk'
      have : (axSigns K (2 * ((K + 1 : Nat) : Int))).count .zero = 0 := by
        apply List.count_eq_zero.mpr
        intro hm
        obtain ⟨j, hj, he⟩ := (zero_mem_axSigns K _).mp hm
        omega
      rw [this]; simp [sig]

theorem count_axSigns_odd (K : Nat) (a : Int) (ha : a % 2 = 1) : (axSigns K a).count .zero = 0 := by
  apply List.count_eq_zero.mpr
  intro hm
  obtain ⟨j, _, he⟩ := (zero_mem_axSigns K _).mp hm
  omega

theorem count_axSigns_out (K : Nat) (a : Int) (ha : a < 0 ∨ 2 * (K : Int) < a) :
    (axSigns K a).count .zero = 0 := by
  apply List.count_eq_zero.mpr
  intro hm
  obtain ⟨j, hj, he⟩ := (zero_mem_axSigns K _).mp hm
  omega

theorem idx_axSigns_even (K k : Nat) (hk : k < K + 1) : (axSigns K (2 * (k : Int))).idxOf .zero = k := by
  induction K with
  | zero =>
    have : k = 0 := by omega
    subst this; simp [axSigns_eq, sig]
  | succ K ih =>
    rw [axSigns_succ, List.idxOf_append]
    by_cases h : k < K + 1
    · have hm : Sgn.zero ∈ axSigns K (2 * (k : Int)) := (zero_mem_axSigns K _).mpr ⟨k, h, rfl⟩
      simp only [hm, ite_true]; exact ih h
    · have hk' : k = K + 1 := by omega
      subst hk'
      have hm : Sgn.zero ∉ axSigns K (2 * ((K + 1 : Nat) : Int)) := by
        intro hm
        obtain ⟨j, hj, he⟩ := (zero_mem_axSigns K _).mp hm
        omega
      simp only [hm, ite_false, len_axSigns]
      simp [sig]

theorem set_axSigns (K k : Nat) (hk : k < K + 1) (d : Int) (hd : d = 1 ∨ d = -1) :
    (axSigns K (2 * (k : Int))).set k (if d = 1 then .pos else .neg) = axSigns K (2 * (k : Int) + d) := by
  apply List.ext_getElem
  · simp [len_axSigns]
  · intro j h1 h2
    simp only [List.getElem_set, axSigns_eq, List.getElem_map, List.getElem_range]
    unfold sig
    by_cases hj : k = j
    · subst hj
      rcases hd with rfl | rfl
      · have : ¬(2 * (k : Int) + 1 < 2 * (k : Int)) := by omega
        have : ¬(2 * (k : Int) + 1 = 2 * (k : Int)) := by omega
        simp [*]
      · have : (2 * (k : Int) + -1 < 2 * (k : Int)) := by omega
        simp [*]
    · simp only [hj, ite_false]
      by_cases h3 : (k : Int) < j
      · have e1 : 2 * (k : Int) < 2 * (j : Int) := by omega
        have e2 : 2 * (k : Int) + d < 2 * (j : Int) := by omega
        simp only [e1, e2, ite_true]
      · have e1 : ¬(2 * (k : Int) < 2 * (j : Int)) := by omega
        have e2 : ¬(2 * (k : Int) = 2 * (j : Int)) := by omega
        have e3 : ¬(2 * (k : Int) + d < 2 * (j : Int)) := by omega
        have e4 : ¬(2 * (k : Int) + d = 2 * (j : Int)) := by omega
        simp only [e1, e2, e3, e4, ite_false]

theorem zip_map_same {α β γ : Type} (l : List α) (f : α → β) (g : α → γ) :
    (l.map f).zip (l.map g) = l.map fun x => (f x, g x) := by
  induction l with
  | nil => rfl
  | cons x l ih => simp [ih]

theorem le_axSigns (K : Nat) (a' a : Int) :
    Cov.le (axSigns K a') (axSigns K a) = true ↔ ∀ k, k < K + 1 → sig a' k = .zero ∨ sig a' k = sig a k := by
  unfold Cov.le
  rw [axSigns_eq, axSigns_eq, zip_map_same]
  simp only [List.length_map, List.length_range, beq_self_eq_true, Bool.true_and, List.all_map,
    List.all_eq_true, List.mem_range, Function.comp_def, Bool.or_eq_true, beq_iff_eq]

theorem le_append {X1 X2 Y1 Y2 : Cov} (h : X1.length = Y1.length) (h2 : X2.length = Y2.length) :
    Cov.le (X1 ++ X2) (Y1 ++ Y2) = (Cov.le X1 Y1 && Cov.le X2 Y2) := by
  unfold Cov.le
  rw [List.zip_append h, List.all_append]
  simp only [List.length_append, h, h2, beq_self_eq_true, Bool.true_and]

theorem le_cov (K : Nat) (c' c : DCell) :
    Cov.le (cov K c') (cov K c) = true ↔
      Cov.le (axSigns K c'.1) (axSigns K c.1) = true ∧ Cov.le (axSigns K c'.2.1) (axSigns K c.2.1) = true ∧
      Cov.le (axSigns K c'.2.2) (axSigns K c.2.2) = true := by
  unfold cov
  rw [le_append (by simp [len_axSigns]) (by simp [len_axSigns]),
    le_append (by simp [len_axSigns]) (by simp [len_axSigns])]
  simp [Bool.and_eq_true, and_assoc]

/-- The face order on one axis: equal, or an even cell beside an odd one. -/
theorem le1_iff (K : Nat) {a' a : Int} (ha' : -1 ≤ a' ∧ a' ≤ 2 * (K : Int) + 1)
    (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1) :
    (∀ k, k < K + 1 → sig a' k = .zero ∨ sig a' k = sig a k) ↔
      a' = a ∨ (a' % 2 = 0 ∧ (a = a' + 1 ∨ a = a' - 1)) := by
  constructor
  · intro h
    by_cases he : a' % 2 = 0
    · -- a' = 2m on a plane; its neighbours bound a
      have hm0 : 0 ≤ a' := by omega
      have hmK : a' ≤ 2 * (K : Int) := by omega
      have lower : a' - 1 ≤ a := by
        by_cases hm : a' = 0
        · omega
        · have hj := h ((a' / 2 - 1).toNat) (by omega)
          unfold sig at hj
          have e : 2 * (((a' / 2 - 1).toNat : Nat) : Int) = a' - 2 := by omega
          rw [e] at hj
          have : ¬(a' < a' - 2) := by omega
          have : ¬(a' = a' - 2) := by omega
          simp only [*, ite_false, reduceCtorEq, false_or] at hj
          by_cases h1 : a < a' - 2
          · simp [h1] at hj
          · by_cases h2 : a = a' - 2
            · simp [h2] at hj
            · omega
      have upper : a ≤ a' + 1 := by
        by_cases hm : a' = 2 * (K : Int)
        · omega
        · have hj := h ((a' / 2 + 1).toNat) (by omega)
          unfold sig at hj
          have e : 2 * (((a' / 2 + 1).toNat : Nat) : Int) = a' + 2 := by omega
          rw [e] at hj
          have : (a' < a' + 2) := by omega
          simp only [*, ite_true, reduceCtorEq, false_or] at hj
          by_cases h1 : a < a' + 2
          · omega
          · simp [h1] at hj
            split at hj <;> cases hj
      omega
    · left
      apply sig_inj K ha' ha
      intro k hk
      rcases h k hk with h0 | h0
      · unfold sig at h0
        split at h0
        · cases h0
        · split at h0
          · omega
          · cases h0
      · exact h0
  · rintro (rfl | ⟨he, h1 | h1⟩) <;> intro k hk
    · right; rfl
    · subst h1; unfold sig
      by_cases e1 : a' < 2 * (k : Int)
      · by_cases e2 : a' + 1 < 2 * (k : Int)
        · right; simp [e1, e2]
        · left; omega
      · by_cases e2 : a' = 2 * (k : Int)
        · left; simp [e1, e2]
        · right
          have e3 : ¬(a' + 1 < 2 * (k : Int)) := by omega
          have e4 : ¬(a' + 1 = 2 * (k : Int)) := by omega
          simp only [e1, e2, e3, e4, ite_false]
    · subst h1; unfold sig
      by_cases e2 : a' = 2 * (k : Int)
      · left; simp [e2]
      · right
        by_cases e1 : a' < 2 * (k : Int)
        · have e3 : a' - 1 < 2 * (k : Int) := by omega
          simp only [e1, e3, ite_true]
        · have e3 : ¬(a' - 1 < 2 * (k : Int)) := by omega
          have e4 : ¬(a' - 1 = 2 * (k : Int)) := by omega
          simp only [e1, e2, e3, e4, ite_false]

/-! ## Facets of the grid: one even coordinate -/

theorem count_axis (K : Nat) {a : Int} (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1) :
    (axSigns K a).count .zero = if a % 2 = 0 then 1 else 0 := by
  split
  · rename_i he
    have : a = 2 * (((a / 2).toNat : Nat) : Int) := by omega
    rw [this]; exact count_axSigns_even K _ (by omega)
  · exact count_axSigns_odd K a (by omega)

theorem zero_nmem_odd (K : Nat) {a : Int} (ha : a % 2 = 1) : Sgn.zero ∉ axSigns K a := by
  intro hm; obtain ⟨j, _, he⟩ := (zero_mem_axSigns K _).mp hm; omega

theorem zero_mem_even (K : Nat) {a : Int} (ha : -1 ≤ a ∧ a ≤ 2 * (K : Int) + 1) (he : a % 2 = 0) :
    Sgn.zero ∈ axSigns K a := (zero_mem_axSigns K _).mpr ⟨(a / 2).toNat, by omega, by omega⟩

/-- A doubled cell with exactly one even coordinate, on axis `ax`. -/
def OneEven (c : DCell) (ax : Nat) : Prop :=
  ax < 3 ∧ dc ax c % 2 = 0 ∧ ∀ o, o < 3 → o ≠ ax → dc o c % 2 = 1

theorem isFacet_cov (K : Nat) {c : DCell} (hc : InR K c) {ax : Nat} (h : OneEven c ax) :
    (cov K c).isFacet = true := by
  obtain ⟨hax, he, ho⟩ := h
  unfold Cov.isFacet cov
  simp only [List.count_append, count_axis K hc.1, count_axis K hc.2.1, count_axis K hc.2.2, beq_iff_eq]
  match ax, hax with
  | 0, _ => have := ho 1 (by omega) (by omega); have := ho 2 (by omega) (by omega)
            simp only [dc] at *; simp [he, *]
  | 1, _ => have := ho 0 (by omega) (by omega); have := ho 2 (by omega) (by omega)
            simp only [dc] at *; simp [he, *]
  | 2, _ => have := ho 0 (by omega) (by omega); have := ho 1 (by omega) (by omega)
            simp only [dc] at *; simp [he, *]

theorem zeroIdx_cov (K : Nat) {c : DCell} (hc : InR K c) {ax : Nat} (h : OneEven c ax) :
    (cov K c).zeroIdx = ax * (K + 1) + (dc ax c / 2).toNat := by
  obtain ⟨hax, he, ho⟩ := h
  have hc' := hc
  obtain ⟨⟨r1, r1'⟩, ⟨r2, r2'⟩, ⟨r3, r3'⟩⟩ := hc'
  unfold Cov.zeroIdx cov
  match ax, hax with
  | 0, _ =>
    have h1 := ho 1 (by omega) (by omega); have h2 := ho 2 (by omega) (by omega)
    simp only [dc] at *
    rw [List.idxOf_append, if_pos (List.mem_append_left _ (zero_mem_even K hc.1 he)), List.idxOf_append,
      if_pos (zero_mem_even K hc.1 he)]
    have e : c.1 = 2 * (((c.1 / 2).toNat : Nat) : Int) := by omega
    rw [e, idx_axSigns_even K _ (by omega)]; simp; omega
  | 1, _ =>
    have h1 := ho 0 (by omega) (by omega); have h2 := ho 2 (by omega) (by omega)
    simp only [dc] at *
    have n1 := zero_nmem_odd K h1
    rw [List.idxOf_append, if_pos (List.mem_append_right _ (zero_mem_even K hc.2.1 he)),
      List.idxOf_append, if_neg n1, len_axSigns]
    have e : c.2.1 = 2 * (((c.2.1 / 2).toNat : Nat) : Int) := by omega
    rw [e, idx_axSigns_even K _ (by omega)]; simp; omega
  | 2, _ =>
    have h1 := ho 0 (by omega) (by omega); have h2 := ho 1 (by omega) (by omega)
    simp only [dc] at *
    have n12 : Sgn.zero ∉ axSigns K c.1 ++ axSigns K c.2.1 := by
      simp only [List.mem_append, not_or]; exact ⟨zero_nmem_odd K h1, zero_nmem_odd K h2⟩
    rw [List.idxOf_append, if_neg n12, List.length_append, len_axSigns, len_axSigns]
    have e : c.2.2 = 2 * (((c.2.2 / 2).toNat : Nat) : Int) := by omega
    rw [e, idx_axSigns_even K _ (by omega)]; simp; omega

/-- A side of a facet of the grid is the doubled cell moved off its even axis. -/
theorem side_cov (K : Nat) {c : DCell} (hc : InR K c) {ax : Nat} (h : OneEven c ax) (d : Int)
    (hd : d = 1 ∨ d = -1) :
    (cov K c).set (cov K c).zeroIdx (if d = 1 then .pos else .neg) = cov K (bump ax d c) := by
  rw [zeroIdx_cov K hc h]
  obtain ⟨hax, he, ho⟩ := h
  have hc' := hc
  obtain ⟨⟨r1, r1'⟩, ⟨r2, r2'⟩, ⟨r3, r3'⟩⟩ := hc'
  unfold cov
  match ax, hax with
  | 0, _ =>
    simp only [dc] at *
    obtain ⟨k, hk, e⟩ : ∃ k : Nat, k < K + 1 ∧ c.1 = 2 * (k : Int) := ⟨(c.1 / 2).toNat, by omega, by omega⟩
    have hidx : (c.1 / 2).toNat = k := by omega
    simp only [bump]
    rw [hidx, List.append_assoc, List.set_append_left _ _ (by simp [len_axSigns]; omega), Nat.zero_mul,
      Nat.zero_add, e, set_axSigns K k hk d hd, List.append_assoc]
  | 1, _ =>
    simp only [dc] at *
    obtain ⟨k, hk, e⟩ : ∃ k : Nat, k < K + 1 ∧ c.2.1 = 2 * (k : Int) :=
      ⟨(c.2.1 / 2).toNat, by omega, by omega⟩
    have hidx : (c.2.1 / 2).toNat = k := by omega
    simp only [bump]
    rw [hidx, List.set_append_left _ _ (by simp [len_axSigns]; omega),
      List.set_append_right _ _ (by simp [len_axSigns])]
    simp only [len_axSigns, Nat.one_mul, Nat.add_sub_cancel_left]
    rw [e, set_axSigns K k hk d hd]
  | 2, _ =>
    simp only [dc] at *
    obtain ⟨k, hk, e⟩ : ∃ k : Nat, k < K + 1 ∧ c.2.2 = 2 * (k : Int) :=
      ⟨(c.2.2 / 2).toNat, by omega, by omega⟩
    have hidx : (c.2.2 / 2).toNat = k := by omega
    simp only [bump]
    rw [hidx, List.set_append_right _ _ (by simp [len_axSigns]; omega)]
    simp only [List.length_append, len_axSigns]
    rw [show 2 * (K + 1) + k - (K + 1 + (K + 1)) = k by omega, e, set_axSigns K k hk d hd]

/-! ## Boxes -/

/-- A box on the grid `0 .. K`: `lo < hi ≤ K` on every axis. -/
def ValidBox (K : Nat) (b : BoxN) : Prop := ∀ o, o < 3 → (b.ax o).1 < (b.ax o).2 ∧ (b.ax o).2 ≤ K

/-- The open box. -/
def boxSet (b : BoxN) : Pred V3 := fun x =>
  ∀ o, o < 3 → ((((b.ax o).1 : Nat) : Int) : Rat) < crd o x ∧ crd o x < ((((b.ax o).2 : Nat) : Int) : Rat)

theorem holds_iff (b : BoxN) (c : DCell) :
    b.holds c = true ↔ ∀ o, o < 3 → 2 * (((b.ax o).1 : Nat) : Int) < dc o c ∧ dc o c < 2 * (((b.ax o).2 : Nat) : Int) := by
  unfold BoxN.holds
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · rintro ⟨⟨h0, h1⟩, h2⟩ o ho
    match o, ho with
    | 0, _ => exact h0
    | 1, _ => exact h1
    | 2, _ => exact h2
  · intro h
    exact ⟨⟨h 0 (by omega), h 1 (by omega)⟩, h 2 (by omega)⟩

theorem cell_ax (K : Nat) {c : DCell} {x : V3} (hx : covCell (gridPs K) (cov K c) x) (o : Nat) (ho : o < 3) :
    cell1 K (dc o c) (crd o x) := by
  have := (covCell_cov K c x).mp hx
  match o, ho with
  | 0, _ => exact this.1
  | 1, _ => exact this.2.1
  | 2, _ => exact this.2.2

/-- **A box's membership is its doubled cell's.** -/
theorem boxSet_iff (K : Nat) {b : BoxN} (hb : ValidBox K b) {c : DCell} {x : V3}
    (hx : covCell (gridPs K) (cov K c) x) : boxSet b x ↔ b.holds c = true := by
  rw [holds_iff]
  unfold boxSet
  apply forall_congr'; intro o
  apply imp_congr_right; intro ho
  have hc := cell_ax K hx o ho
  rw [lo_iff K hc (by have := hb o ho; omega), hi_iff K hc (by have := hb o ho; omega)]

theorem mem_odds (lo hi : Nat) (h : lo ≤ hi) (u : Int) :
    u ∈ odds lo hi ↔ u % 2 = 1 ∧ 2 * (lo : Int) < u ∧ u < 2 * (hi : Int) := by
  unfold odds
  simp only [List.mem_map, List.mem_range]
  constructor
  · rintro ⟨t, ht, rfl⟩; omega
  · intro hu; exact ⟨(((u - 1) / 2) - lo).toNat, by omega, by omega⟩

/-- A box's face value: `lo` for an even side, `hi` for an odd one. -/
def endv (b : BoxN) (s : Nat) : Nat := if s % 2 = 0 then (b.ax (s / 2)).1 else (b.ax (s / 2)).2

/-- Inside the box's range on axis `o`, on an odd (open) cell. -/
def InAx (b : BoxN) (o : Nat) (a : Int) : Prop :=
  a % 2 = 1 ∧ 2 * (((b.ax o).1 : Nat) : Int) < a ∧ a < 2 * (((b.ax o).2 : Nat) : Int)

theorem mem_faceCells {K : Nat} {b : BoxN} (hb : ValidBox K b) {s : Nat} (hs : s < 6) (c : DCell) :
    c ∈ faceCells b s ↔ dc (s / 2) c = 2 * ((endv b s : Nat) : Int) ∧
      ∀ o, o < 3 → o ≠ s / 2 → InAx b o (dc o c) := by
  have v0 := hb 0 (by omega)
  have v1 := hb 1 (by omega)
  have v2 := hb 2 (by omega)
  obtain ⟨c1, c2, c3⟩ := c
  match s, hs with
  | 0, _ | 1, _ =>
    simp only [faceCells, endv, place, List.mem_flatMap, List.mem_map, BoxN.ax] at *
    simp only [Nat.reduceDiv, Nat.reduceMod, Nat.reduceAdd, Nat.lt_irrefl, ite_true, ite_false,
      Nat.zero_lt_one, Nat.one_lt_two, show (1 : Nat) < 2 by decide, show ¬((2 : Nat) < 1) by decide] at *
    simp only [mem_odds _ _ (Nat.le_of_lt v1.1), mem_odds _ _ (Nat.le_of_lt v2.1), Prod.mk.injEq]
    constructor
    · rintro ⟨u, hu, v, hv, rfl, rfl, rfl⟩
      refine ⟨by simp [dc], fun o ho hne => ?_⟩
      match o, ho with
      | 0, _ => exact absurd rfl hne
      | 1, _ => exact ⟨hu.1, hu.2.1, hu.2.2⟩
      | 2, _ => exact ⟨hv.1, hv.2.1, hv.2.2⟩
    · rintro ⟨h0, ho⟩
      have h1 := ho 1 (by omega) (by omega)
      have h2 := ho 2 (by omega) (by omega)
      simp only [dc, InAx, BoxN.ax] at h0 h1 h2
      exact ⟨c2, h1, c3, h2, h0.symm, rfl, rfl⟩
  | 2, _ | 3, _ =>
    simp only [faceCells, endv, place, List.mem_flatMap, List.mem_map, BoxN.ax] at *
    simp only [Nat.reduceDiv, Nat.reduceMod, Nat.reduceAdd, ite_true, ite_false,
      show ¬((2 : Nat) < 0) by decide] at *
    simp only [mem_odds _ _ (Nat.le_of_lt v0.1), mem_odds _ _ (Nat.le_of_lt v2.1), Prod.mk.injEq]
    constructor
    · rintro ⟨u, hu, v, hv, rfl, rfl, rfl⟩
      refine ⟨by simp [dc], fun o ho hne => ?_⟩
      match o, ho with
      | 0, _ => exact ⟨hu.1, hu.2.1, hu.2.2⟩
      | 1, _ => exact absurd rfl hne
      | 2, _ => exact ⟨hv.1, hv.2.1, hv.2.2⟩
    · rintro ⟨h0, ho⟩
      have h1 := ho 0 (by omega) (by omega)
      have h2 := ho 2 (by omega) (by omega)
      simp only [dc, InAx, BoxN.ax] at h0 h1 h2
      exact ⟨c1, h1, c3, h2, rfl, h0.symm, rfl⟩
  | 4, _ | 5, _ =>
    simp only [faceCells, endv, place, List.mem_flatMap, List.mem_map, BoxN.ax] at *
    simp only [Nat.reduceDiv, Nat.reduceMod, Nat.reduceAdd, ite_true, ite_false,
      show ((0 : Nat) < 1) by decide] at *
    simp only [mem_odds _ _ (Nat.le_of_lt v0.1), mem_odds _ _ (Nat.le_of_lt v1.1), Prod.mk.injEq]
    constructor
    · rintro ⟨u, hu, v, hv, rfl, rfl, rfl⟩
      refine ⟨by simp [dc], fun o ho hne => ?_⟩
      match o, ho with
      | 0, _ => exact ⟨hu.1, hu.2.1, hu.2.2⟩
      | 1, _ => exact ⟨hv.1, hv.2.1, hv.2.2⟩
      | 2, _ => exact absurd rfl hne
    · rintro ⟨h0, ho⟩
      have h1 := ho 0 (by omega) (by omega)
      have h2 := ho 1 (by omega) (by omega)
      simp only [dc, InAx, BoxN.ax] at h0 h1 h2
      exact ⟨c1, h1, c2, h2, rfl, rfl, h0.symm⟩

theorem dc_bump (ax o : Nat) (hax : ax < 3) (ho : o < 3) (d : Int) (c : DCell) :
    dc o (bump ax d c) = if o = ax then dc o c + d else dc o c := by
  obtain ⟨c1, c2, c3⟩ := c
  match ax, hax, o, ho with
  | 0, _, 0, _ | 0, _, 1, _ | 0, _, 2, _ | 1, _, 0, _ | 1, _, 1, _ | 1, _, 2, _
  | 2, _, 0, _ | 2, _, 1, _ | 2, _, 2, _ => simp [bump, dc]

/-- **The flip.** Across a facet of the grid on axis `ax`, a box's membership changes iff the
facet lies in one of the box's two faces on that axis. -/
theorem flip_iff {K : Nat} {b : BoxN} (hb : ValidBox K b) {c : DCell} {ax : Nat} (h : OneEven c ax) :
    (b.holds (bump ax 1 c) ≠ b.holds (bump ax (-1) c)) ↔
      ∃ s, s < 6 ∧ s / 2 = ax ∧ c ∈ faceCells b s := by
  obtain ⟨hax, he, ho⟩ := h
  have hb' := hb ax hax
  have key : ∀ d : Int, (b.holds (bump ax d c) = true ↔
      (2 * (((b.ax ax).1 : Nat) : Int) < dc ax c + d ∧ dc ax c + d < 2 * (((b.ax ax).2 : Nat) : Int)) ∧
      ∀ o, o < 3 → o ≠ ax → 2 * (((b.ax o).1 : Nat) : Int) < dc o c ∧ dc o c < 2 * (((b.ax o).2 : Nat) : Int)) := by
    intro d
    rw [holds_iff]
    constructor
    · intro h'
      refine ⟨?_, fun o ho' hne => ?_⟩
      · have := h' ax hax; rw [dc_bump ax ax hax hax, if_pos rfl] at this; exact this
      · have := h' o ho'; rw [dc_bump ax o hax ho', if_neg hne] at this; exact this
    · rintro ⟨h1, h2⟩ o ho'
      rw [dc_bump ax o hax ho']
      split
      · rename_i e; subst e; exact h1
      · exact h2 o ho' (by assumption)
  have faces : ∀ s, s < 6 → s / 2 = ax → (c ∈ faceCells b s ↔
      dc ax c = 2 * ((endv b s : Nat) : Int) ∧ ∀ o, o < 3 → o ≠ ax → InAx b o (dc o c)) := by
    intro s hs hsa
    rw [mem_faceCells hb hs, hsa]
  constructor
  · intro hne
    have hothers : ∀ o, o < 3 → o ≠ ax → InAx b o (dc o c) := by
      intro o ho' hne'
      apply Classical.byContradiction
      intro hno
      apply hne
      have hf : ∀ d : Int, b.holds (bump ax d c) = false := by
        intro d
        cases hh : b.holds (bump ax d c)
        · rfl
        · exact absurd ⟨ho o ho' hne', ((key d).mp hh).2 o ho' hne'⟩ hno
      rw [hf 1, hf (-1)]
    cases h1 : b.holds (bump ax 1 c) <;> cases h2 : b.holds (bump ax (-1) c)
    · rw [h1, h2] at hne; exact absurd rfl hne
    · -- the minus side is in, the plus side out: on the hi face
      have := (key (-1)).mp h2
      have hnot : ¬(2 * (((b.ax ax).1 : Nat) : Int) < dc ax c + 1 ∧ dc ax c + 1 < 2 * (((b.ax ax).2 : Nat) : Int)) :=
        fun hh => by
          have := (key 1).mpr ⟨hh, this.2⟩; rw [h1] at this; cases this
      refine ⟨2 * ax + 1, by omega, by omega, (faces _ (by omega) (by omega)).mpr ⟨?_, hothers⟩⟩
      unfold endv
      simp only [show (2 * ax + 1) % 2 = 1 by omega, show (2 * ax + 1) / 2 = ax by omega]
      simp only [Nat.one_ne_zero, ite_false]
      omega
    · have := (key 1).mp h1
      have hnot : ¬(2 * (((b.ax ax).1 : Nat) : Int) < dc ax c + -1 ∧ dc ax c + -1 < 2 * (((b.ax ax).2 : Nat) : Int)) :=
        fun hh => by
          have := (key (-1)).mpr ⟨hh, this.2⟩; rw [h2] at this; cases this
      refine ⟨2 * ax, by omega, by omega, (faces _ (by omega) (by omega)).mpr ⟨?_, hothers⟩⟩
      unfold endv
      simp only [show (2 * ax) % 2 = 0 by omega, show (2 * ax) / 2 = ax by omega, ite_true]
      omega
    · rw [h1, h2] at hne; exact absurd rfl hne
  · rintro ⟨s, hs, hsa, hc⟩
    obtain ⟨hd, hothers⟩ := (faces s hs hsa).mp hc
    have hin : ∀ o, o < 3 → o ≠ ax →
        2 * (((b.ax o).1 : Nat) : Int) < dc o c ∧ dc o c < 2 * (((b.ax o).2 : Nat) : Int) :=
      fun o ho' hne => ⟨(hothers o ho' hne).2.1, (hothers o ho' hne).2.2⟩
    unfold endv at hd
    rw [hsa] at hd
    split at hd
    · -- the lo face: the plus side is in, the minus side out
      have h1 : b.holds (bump ax 1 c) = true := (key 1).mpr ⟨by omega, hin⟩
      have h2 : b.holds (bump ax (-1) c) = false := by
        cases hh : b.holds (bump ax (-1) c)
        · rfl
        · have := ((key (-1)).mp hh).1; omega
      rw [h1, h2]; decide
    · have h1 : b.holds (bump ax 1 c) = false := by
        cases hh : b.holds (bump ax 1 c)
        · rfl
        · have := ((key 1).mp hh).1; omega
      have h2 : b.holds (bump ax (-1) c) = true := (key (-1)).mpr ⟨by omega, hin⟩
      rw [h1, h2]; decide

theorem det_unit (i j k : Nat) (hi : i < 3) (hj : j < 3) (hk : k < 3) :
    det3 (unitV i) (unitV j) (unitV k) ≠ 0 ↔ (i ≠ j ∧ j ≠ k ∧ i ≠ k) := by
  match i, hi, j, hj, k, hk with
  | 0, _, 0, _, 0, _ | 0, _, 0, _, 1, _ | 0, _, 0, _, 2, _ | 0, _, 1, _, 0, _ | 0, _, 1, _, 1, _
  | 0, _, 1, _, 2, _ | 0, _, 2, _, 0, _ | 0, _, 2, _, 1, _ | 0, _, 2, _, 2, _
  | 1, _, 0, _, 0, _ | 1, _, 0, _, 1, _ | 1, _, 0, _, 2, _ | 1, _, 1, _, 0, _ | 1, _, 1, _, 1, _
  | 1, _, 1, _, 2, _ | 1, _, 2, _, 0, _ | 1, _, 2, _, 1, _ | 1, _, 2, _, 2, _
  | 2, _, 0, _, 0, _ | 2, _, 0, _, 1, _ | 2, _, 0, _, 2, _ | 2, _, 1, _, 0, _ | 2, _, 1, _, 1, _
  | 2, _, 1, _, 2, _ | 2, _, 2, _, 0, _ | 2, _, 2, _, 1, _ | 2, _, 2, _, 2, _ =>
    simp [det3, V3.dot, V3.cross, unitV] <;> grind
theorem cross_unit (i j : Nat) (hi : i < 3) (hj : j < 3) :
    ((unitV i).cross (unitV j)).isZero = false ↔ i ≠ j := by
  match i, hi, j, hj with
  | 0, _, 0, _ | 0, _, 1, _ | 0, _, 2, _ | 1, _, 0, _ | 1, _, 1, _ | 1, _, 2, _
  | 2, _, 0, _ | 2, _, 1, _ | 2, _, 2, _ => simp [V3.cross, V3.isZero, unitV] <;> grind

/-! ## The box ledger, as doubled cells -/

theorem getD_in {α : Type} {l : List α} {i : Nat} {d : α} (h : i < l.length) : l.getD i d = l[i] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]; rfl

theorem getD_out {α : Type} {l : List α} {i : Nat} {d : α} (h : l.length ≤ i) : l.getD i d = d := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none h]; rfl

/-- The default box `boxLedger` reads past the list: empty, on the grid. -/
def dflt : BoxN := ⟨(0, 0, 0), (0, 0, 0)⟩

def boxAt (boxes : List BoxN) (i : Nat) : BoxN := boxes.getD i dflt

/-- Every box of the family is a box on the grid. -/
def Valid (K : Nat) (boxes : List BoxN) : Prop := ∀ b ∈ boxes, ValidBox K b

theorem ends_le {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) (i : Nat) (o : Nat) (ho : o < 3) :
    ((boxAt boxes i).ax o).1 ≤ K ∧ ((boxAt boxes i).ax o).2 ≤ K := by
  unfold boxAt
  by_cases hi : i < boxes.length
  · rw [getD_in hi]
    have := hv _ (List.getElem_mem hi) o ho; omega
  · rw [getD_out (by omega)]
    match o, ho with
    | 0, _ | 1, _ | 2, _ => simp [dflt, BoxN.ax]

theorem valid_at {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) {i : Nat} (hi : i < boxes.length) :
    ValidBox K (boxAt boxes i) := by
  unfold boxAt; rw [getD_in hi]; exact hv _ (List.getElem_mem hi)

/-- **Every box's membership is its doubled cell's** (the default box too: it is empty). -/
theorem boxSet_iff' (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) (i : Nat) {c : DCell} {x : V3}
    (hx : covCell (gridPs K) (cov K c) x) : boxSet (boxAt boxes i) x ↔ (boxAt boxes i).holds c = true := by
  rw [holds_iff]
  unfold boxSet
  apply forall_congr'; intro o
  apply imp_congr_right; intro ho
  have hc := cell_ax K hx o ho
  have he := ends_le hv i o ho
  rw [lo_iff K hc (by omega), hi_iff K hc (by omega)]

theorem mem_boxPieces (boxes : List BoxN) (c : DCell) (f : Nat) :
    (c, f) ∈ boxPieces boxes ↔ ∃ i, i < boxes.length ∧ ∃ s, s < 6 ∧ f = 6 * i + s ∧ c ∈ faceCells (boxAt boxes i) s := by
  unfold boxPieces boxAt dflt
  simp only [List.mem_flatMap, List.mem_range, List.mem_map, Prod.mk.injEq]
  constructor
  · rintro ⟨i, hi, s, hs, c', hc', rfl, rfl⟩; exact ⟨i, hi, s, hs, rfl, hc'⟩
  · rintro ⟨i, hi, s, hs, rfl, hc⟩; exact ⟨i, hi, s, hs, c, hc, rfl, rfl⟩

/-- What a piece cell is. -/
theorem piece_facts {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} {f : Nat}
    (h : (c, f) ∈ boxPieces boxes) :
    f / 6 < boxes.length ∧ f % 6 < 6 ∧ c ∈ faceCells (boxAt boxes (f / 6)) (f % 6) ∧
      OneEven c (f % 6 / 2) ∧ InR K c ∧ dc (f % 6 / 2) c = 2 * ((endv (boxAt boxes (f / 6)) (f % 6) : Nat) : Int) := by
  obtain ⟨i, hi, s, hs, rfl, hc⟩ := (mem_boxPieces boxes c f).mp h
  have e1 : (6 * i + s) / 6 = i := by omega
  have e2 : (6 * i + s) % 6 = s := by omega
  rw [e1, e2]
  have hb := valid_at hv hi
  obtain ⟨hd, ho⟩ := (mem_faceCells hb hs c).mp hc
  have hax : s / 2 < 3 := by omega
  refine ⟨hi, hs, hc, ⟨hax, by omega, fun o ho' hne => (ho o ho' hne).1⟩, ?_, hd⟩
  have hend : endv (boxAt boxes i) s ≤ K := by
    unfold endv; have := hb (s / 2) hax; split <;> omega
  have hr : ∀ o, o < 3 → -1 ≤ dc o c ∧ dc o c ≤ 2 * (K : Int) + 1 := by
    intro o ho'
    by_cases e : o = s / 2
    · subst e; omega
    · have := (ho o ho' e); have := hb o ho'; unfold InAx at *; omega
  exact ⟨hr 0 (by omega), hr 1 (by omega), hr 2 (by omega)⟩

theorem evenAx_eq {c : DCell} {ax : Nat} (h : OneEven c ax) : evenAx c = ax := by
  obtain ⟨hax, he, ho⟩ := h
  obtain ⟨c1, c2, c3⟩ := c
  unfold evenAx
  match ax, hax with
  | 0, _ => simp only [dc] at he; simp [he]
  | 1, _ =>
    have := ho 0 (by omega) (by omega); simp only [dc] at he this
    simp [he, show ¬(c1 % 2 = 0) by omega]
  | 2, _ =>
    have h0 := ho 0 (by omega) (by omega); have h1 := ho 1 (by omega) (by omega)
    simp only [dc] at he h0 h1
    simp [show ¬(c1 % 2 = 0) by omega, show ¬(c2 % 2 = 0) by omega]

theorem inR_bump {K : Nat} {c : DCell} (hc : InR K c) {ax : Nat} (h : OneEven c ax) (d : Int)
    (hd : d = 1 ∨ d = -1) : InR K (bump ax d c) := by
  obtain ⟨hax, he, _⟩ := h
  obtain ⟨⟨r1, r1'⟩, ⟨r2, r2'⟩, ⟨r3, r3'⟩⟩ := hc
  obtain ⟨c1, c2, c3⟩ := c
  match ax, hax with
  | 0 , _ => simp only [dc] at he; simp only [bump, InR] at *; omega
  | 1 , _ => simp only [dc] at he; simp only [bump, InR] at *; omega
  | 2 , _ => simp only [dc] at he; simp only [bump, InR] at *; omega

/-! ## The ledger's tables -/

open Ledger

def boxA (boxes : List BoxN) : Nat → Pred V3 := fun i => boxSet (boxAt boxes i)

theorem bodyOf_box (K : Nat) (boxes : List BoxN) {f : Nat} (hf : f < 6 * boxes.length) :
    (boxLedger K boxes).bodyOf f = f / 6 := by
  unfold Ledger.bodyOf boxLedger
  simp only
  rw [getD_in (by simp; omega)]
  simp

theorem clsOf_box (K : Nat) (boxes : List BoxN) {f : Nat} (hf : f < 6 * boxes.length) :
    (boxLedger K boxes).clsOf f = (f % 6 / 2) * (K + 1) + endv (boxAt boxes (f / 6)) (f % 6) := by
  unfold Ledger.clsOf boxLedger
  simp only
  rw [getD_in (by simp; omega)]
  simp [endv, boxAt, dflt]

theorem mem_facets_box (K : Nat) (boxes : List BoxN) (φ : Cov) :
    φ ∈ (boxLedger K boxes).facets ↔ ∃ c f, (c, f) ∈ boxPieces boxes ∧ φ = cov K c := by
  rw [mem_facets]
  unfold boxLedger
  simp only [List.mem_map, Prod.mk.injEq]
  constructor
  · rintro ⟨f, ⟨c, f'⟩, h, rfl, rfl⟩; exact ⟨c, f', h, rfl⟩
  · rintro ⟨c, f, h, rfl⟩; exact ⟨f, ⟨c, f⟩, h, rfl, rfl⟩

theorem plus_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} {f : Nat}
    (h : (c, f) ∈ boxPieces boxes) : (cov K c).plus = cov K (bump (f % 6 / 2) 1 c) := by
  obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv h
  have := side_cov K h2 h1 1 (Or.inl rfl)
  simp only [ite_true] at this
  exact this

theorem minus_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} {f : Nat}
    (h : (c, f) ∈ boxPieces boxes) : (cov K c).minus = cov K (bump (f % 6 / 2) (-1) c) := by
  obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv h
  have := side_cov K h2 h1 (-1) (Or.inr rfl)
  simp only [show ¬((-1 : Int) = 1) by decide, ite_false] at this
  exact this

theorem mem_topes_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) (T : Cov) :
    T ∈ (boxLedger K boxes).topes ↔ ∃ c f, (c, f) ∈ boxPieces boxes ∧
      (T = cov K (bump (f % 6 / 2) 1 c) ∨ T = cov K (bump (f % 6 / 2) (-1) c)) := by
  unfold Ledger.topes
  rw [mem_dedup]
  simp only [List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
  constructor
  · rintro ⟨φ, hφ, h⟩
    obtain ⟨c, f, hp, rfl⟩ := (mem_facets_box K boxes φ).mp hφ
    rw [plus_box K hv hp, minus_box K hv hp] at h
    exact ⟨c, f, hp, h⟩
  · rintro ⟨c, f, hp, h⟩
    refine ⟨cov K c, (mem_facets_box K boxes _).mpr ⟨c, f, hp, rfl⟩, ?_⟩
    rw [plus_box K hv hp, minus_box K hv hp]; exact h

/-- The side cells the generator classifies. -/
def sidesOf (boxes : List BoxN) : List DCell :=
  dedup ((dedup ((boxPieces boxes).map Prod.fst)).flatMap fun c => [bump (evenAx c) 1 c, bump (evenAx c) (-1) c])

theorem mem_sidesOf {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) (t : DCell) :
    t ∈ sidesOf boxes ↔ ∃ c f, (c, f) ∈ boxPieces boxes ∧
      (t = bump (f % 6 / 2) 1 c ∨ t = bump (f % 6 / 2) (-1) c) := by
  unfold sidesOf
  rw [mem_dedup]
  simp only [List.mem_flatMap, mem_dedup, List.mem_map, List.mem_cons, List.mem_nil_iff, or_false]
  constructor
  · rintro ⟨c, ⟨⟨c', f⟩, hp, rfl⟩, h⟩
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1] at h
    exact ⟨c', f, hp, h⟩
  · rintro ⟨c, f, hp, h⟩
    refine ⟨c, ⟨(c, f), hp, rfl⟩, ?_⟩
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1]; exact h

theorem mem_inside_box (K : Nat) (boxes : List BoxN) (T : Cov) (i : Nat) :
    (T, i) ∈ (boxLedger K boxes).inside ↔ ∃ t, t ∈ sidesOf boxes ∧ T = cov K t ∧ i < boxes.length ∧
      (boxAt boxes i).holds t = true := by
  unfold boxLedger sidesOf boxAt dflt
  simp only [List.mem_flatMap, List.mem_map, List.mem_filter, List.mem_range, Prod.mk.injEq]
  constructor
  · rintro ⟨t, ht, i', ⟨hi, hh⟩, rfl, rfl⟩; exact ⟨t, ht, rfl, hi, hh⟩
  · rintro ⟨t, ht, rfl, hi, hh⟩; exact ⟨t, ht, i, ⟨hi, hh⟩, rfl, rfl⟩

theorem memT_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) {t : DCell} (ht : InR K t) (i : Nat) :
    memT (gridPs K) (boxA boxes) (cov K t) i ↔ (boxAt boxes i).holds t = true := by
  constructor
  · rintro ⟨x, hx, hA⟩; exact (boxSet_iff' K hv i hx).mp hA
  · intro h; exact ⟨pt t, pt_cell K t, (boxSet_iff' K hv i (pt_cell K t)).mpr h⟩

/-! ## The `Exact` fields -/

theorem oneEven_unique {c : DCell} {a b : Nat} (ha : OneEven c a) (hb : OneEven c b) : a = b := by
  apply Classical.byContradiction; intro hne
  have := ha.2.2 b hb.1 (Ne.symm hne)
  have := hb.2.1
  omega

theorem gridPs_nonzero (K : Nat) : ∀ p ∈ gridPs K, p.n ≠ ⟨0, 0, 0⟩ := by
  intro p hp
  unfold gridPs axPl at hp
  simp only [List.mem_append, List.mem_map, List.mem_range] at hp
  rcases hp with (⟨k, _, rfl⟩ | ⟨k, _, rfl⟩) | ⟨k, _, rfl⟩ <;> simp [unitV]

theorem box_compat (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ T ∈ fulls (gridPs K).length, ∀ i, (covCell (gridPs K) T).Sub (boxA boxes i) ∨
      ∀ x, covCell (gridPs K) T x → ¬boxA boxes i x := by
  intro T _ i
  have key : ∀ x, covCell (gridPs K) T x → (boxA boxes i x ↔ (boxAt boxes i).holds (dblV K x) = true) :=
    fun x hx => boxSet_iff' K hv i (dblV_cell K x)
  by_cases h : ∃ x, covCell (gridPs K) T x ∧ boxA boxes i x
  · obtain ⟨x, hx, hA⟩ := h
    left
    intro y hy
    have e : dblV K y = dblV K x :=
      cov_inj K (dblV_inR K y) (dblV_inR K x) ((realised_eq K hy).symm.trans (realised_eq K hx))
    rw [key y hy, e, ← key x hx]; exact hA
  · exact Or.inr fun x hx hA => h ⟨x, hx, hA⟩

theorem box_pieceFacet (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ p ∈ (boxLedger K boxes).pieces, p.1.isFacet = true ∧ p.1.zeroIdx = (boxLedger K boxes).clsOf p.2 ∧
      (boxLedger K boxes).bodyOf p.2 < (boxLedger K boxes).nb := by
  intro p hp
  unfold boxLedger at hp
  simp only [List.mem_map] at hp
  obtain ⟨⟨c, f⟩, hcf, rfl⟩ := hp
  obtain ⟨hi, hs, _, h1, h2, h3⟩ := piece_facts hv hcf
  have hf : f < 6 * boxes.length := by omega
  refine ⟨isFacet_cov K h2 h1, ?_, ?_⟩
  · simp only
    rw [zeroIdx_cov K h2 h1, clsOf_box K boxes hf, h3]
    congr 1; omega
  · rw [bodyOf_box K boxes hf]; exact hi

theorem box_oneFace (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ p ∈ (boxLedger K boxes).pieces, ∀ q ∈ (boxLedger K boxes).pieces, q.1 = p.1 →
      (boxLedger K boxes).bodyOf q.2 = (boxLedger K boxes).bodyOf p.2 → q.2 = p.2 := by
  intro p hp q hq e1 e2
  unfold boxLedger at hp hq
  simp only [List.mem_map] at hp hq
  obtain ⟨⟨c, f⟩, hcf, rfl⟩ := hp
  obtain ⟨⟨c', f'⟩, hcf', rfl⟩ := hq
  obtain ⟨hi, hs, _, h1, h2, h3⟩ := piece_facts hv hcf
  obtain ⟨hi', hs', _, h1', h2', h3'⟩ := piece_facts hv hcf'
  simp only at e1 e2 ⊢
  have ec : c' = c := cov_inj K h2' h2 e1
  subst ec
  rw [bodyOf_box K boxes (by omega), bodyOf_box K boxes (by omega)] at e2
  have hax := oneEven_unique h1' h1
  rw [e2, hax] at h3'
  rw [h3] at h3'
  have hend : endv (boxAt boxes (f / 6)) (f' % 6) = endv (boxAt boxes (f / 6)) (f % 6) := by omega
  have hb := valid_at hv hi (f % 6 / 2) (by omega)
  have hss : f' % 6 = f % 6 := by
    unfold endv at hend
    rw [hax] at hend
    split at hend <;> split at hend <;> omega
  omega

theorem box_pieceReal (K : Nat) (boxes : List BoxN) :
    ∀ p ∈ (boxLedger K boxes).pieces, Realised (gridPs K) p.1 := by
  intro p hp
  unfold boxLedger at hp
  simp only [List.mem_map] at hp
  obtain ⟨⟨c, f⟩, _, rfl⟩ := hp
  exact ⟨pt c, pt_cell K c⟩

theorem box_witness (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ T ∈ (boxLedger K boxes).topes, ∀ i, i < (boxLedger K boxes).nb →
      ((boxLedger K boxes).inside.contains (T, i) = true ↔ memT (gridPs K) (boxA boxes) T i) := by
  intro T hT i hi
  obtain ⟨c, f, hp, hT⟩ := (mem_topes_box K hv T).mp hT
  obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv hp
  obtain ⟨t0, ht0, hd⟩ : ∃ t0, T = cov K t0 ∧ ∃ d : Int, (d = 1 ∨ d = -1) ∧ t0 = bump (f % 6 / 2) d c := by
    rcases hT with h | h
    · exact ⟨_, h, 1, Or.inl rfl, rfl⟩
    · exact ⟨_, h, -1, Or.inr rfl, rfl⟩
  obtain ⟨d, hd1, rfl⟩ := hd
  have hR := inR_bump h2 h1 d hd1
  subst ht0
  rw [memT_box K hv hR, List.contains_iff_mem, mem_inside_box]
  constructor
  · rintro ⟨t, ht, e, _, hh⟩
    obtain ⟨c', f', hp', ht'⟩ := (mem_sidesOf hv t).mp ht
    obtain ⟨_, _, _, h1', h2', _⟩ := piece_facts hv hp'
    have hRt : InR K t := by
      rcases ht' with rfl | rfl
      · exact inR_bump h2' h1' 1 (Or.inl rfl)
      · exact inR_bump h2' h1' (-1) (Or.inr rfl)
    rw [cov_inj K hR hRt e]; exact hh
  · intro hh
    refine ⟨_, (mem_sidesOf hv _).mpr ⟨c, f, hp, ?_⟩, rfl, hi, hh⟩
    rcases hd1 with rfl | rfl
    · exact Or.inl rfl
    · exact Or.inr rfl

/-- The flip, in the ledger's terms: body `i`'s membership flips across the facet of a doubled cell
with one even coordinate iff the cell lies in one of box `i`'s faces. -/
theorem flip_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} (hc : InR K c)
    {ax : Nat} (h : OneEven c ax) {i : Nat} (hi : i < boxes.length) :
    Flip (gridPs K) (boxA boxes) (cov K c) i ↔ ∃ s, s < 6 ∧ s / 2 = ax ∧ c ∈ faceCells (boxAt boxes i) s := by
  unfold Flip Cov.plus Cov.minus
  have e1 := side_cov K hc h 1 (Or.inl rfl)
  have e2 := side_cov K hc h (-1) (Or.inr rfl)
  simp only [ite_true, show ¬((-1 : Int) = 1) by decide, ite_false] at e1 e2
  rw [e1, e2, memT_box K hv (inR_bump hc h 1 (Or.inl rfl)), memT_box K hv (inR_bump hc h (-1) (Or.inr rfl)),
    ← flip_iff (valid_at hv hi) h]
  cases (boxAt boxes i).holds (bump ax 1 c) <;> cases (boxAt boxes i).holds (bump ax (-1) c) <;> simp

theorem box_boundary (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ φ ∈ (boxLedger K boxes).facets, ∀ i, i < (boxLedger K boxes).nb →
      (Flip (gridPs K) (boxA boxes) φ i ↔ (boxLedger K boxes).isPieceOf i φ = true) := by
  intro φ hφ i hi
  have hi' : i < boxes.length := hi
  obtain ⟨c, f, hp, rfl⟩ := (mem_facets_box K boxes φ).mp hφ
  obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv hp
  rw [flip_box K hv h2 h1 hi]
  unfold Ledger.isPieceOf
  rw [List.any_eq_true]
  constructor
  · rintro ⟨s, hs, hsa, hcs⟩
    have hm : (c, 6 * i + s) ∈ boxPieces boxes := (mem_boxPieces boxes c _).mpr ⟨i, hi, s, hs, rfl, hcs⟩
    refine ⟨(cov K c, 6 * i + s), ?_, ?_⟩
    · unfold boxLedger; exact List.mem_map.mpr ⟨_, hm, rfl⟩
    · simp only [beq_self_eq_true, Bool.true_and, beq_iff_eq]
      rw [bodyOf_box K boxes (by omega)]; omega
  · rintro ⟨q, hq, hqq⟩
    unfold boxLedger at hq
    simp only [List.mem_map] at hq
    obtain ⟨⟨c', f'⟩, hp', rfl⟩ := hq
    obtain ⟨hi', hs', hcs', h1', h2', _⟩ := piece_facts hv hp'
    simp only [Bool.and_eq_true, beq_iff_eq] at hqq
    obtain ⟨e1, e2⟩ := hqq
    have ec : c' = c := cov_inj K h2' h2 e1
    subst ec
    rw [bodyOf_box K boxes (by omega)] at e2
    subst e2
    exact ⟨f' % 6, hs', oneEven_unique h1' h1, hcs'⟩

/-- A grid facet of a doubled cell in range has exactly one even coordinate. -/
theorem oneEven_of_facet (K : Nat) {c : DCell} (hc : InR K c) (h : (cov K c).isFacet = true) :
    ∃ ax, OneEven c ax := by
  unfold Cov.isFacet cov at h
  simp only [List.count_append, count_axis K hc.1, count_axis K hc.2.1, count_axis K hc.2.2, beq_iff_eq] at h
  obtain ⟨c1, c2, c3⟩ := c
  simp only at h
  by_cases e1 : c1 % 2 = 0 <;> by_cases e2 : c2 % 2 = 0 <;> by_cases e3 : c3 % 2 = 0 <;>
    simp only [e1, e2, e3, ite_true, ite_false] at h <;> try omega
  · exact ⟨0, by omega, by simp [dc, e1], fun o ho hne => by
      match o, ho with
      | 0, _ => exact absurd rfl hne
      | 1, _ => simp [dc]; omega
      | 2, _ => simp [dc]; omega⟩
  · exact ⟨1, by omega, by simp [dc, e2], fun o ho hne => by
      match o, ho with
      | 0, _ => simp [dc]; omega
      | 1, _ => exact absurd rfl hne
      | 2, _ => simp [dc]; omega⟩
  · exact ⟨2, by omega, by simp [dc, e3], fun o ho hne => by
      match o, ho with
      | 0, _ => simp [dc]; omega
      | 1, _ => simp [dc]; omega
      | 2, _ => exact absurd rfl hne⟩

theorem box_complete (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ φ, Realised (gridPs K) φ → φ.isFacet = true → ∀ i, i < (boxLedger K boxes).nb →
      Flip (gridPs K) (boxA boxes) φ i → φ ∈ (boxLedger K boxes).facets := by
  intro φ ⟨x, hx⟩ hfa i hi hfl
  have e := realised_eq K hx
  subst e
  have hR := dblV_inR K x
  obtain ⟨ax, hax⟩ := oneEven_of_facet K hR hfa
  obtain ⟨s, hs, _, hcs⟩ := (flip_box K hv hR hax hi).mp hfl
  exact (mem_facets_box K boxes _).mpr ⟨_, 6 * i + s,
    (mem_boxPieces boxes _ _).mpr ⟨i, hi, s, hs, rfl, hcs⟩, rfl⟩

/-! ## Dimension on the grid: count the even coordinates -/

def ev (a : Int) : Nat := if a % 2 = 0 then 1 else 0
def nEven (c : DCell) : Nat := ev c.1 + ev c.2.1 + ev c.2.2

theorem getD3 {α : Type} (A B C : List α) (n : Nat) (hA : A.length = n) (hB : B.length = n) (d : α)
    (j : Nat) : (A ++ B ++ C).getD j d =
      if j < n then A.getD j d else if j < 2 * n then B.getD (j - n) d else C.getD (j - 2 * n) d := by
  simp only [List.getD_eq_getElem?_getD]
  rw [List.append_assoc]
  split
  · rw [List.getElem?_append_left (by omega)]
  · rw [List.getElem?_append_right (by omega), hA]
    split
    · rw [List.getElem?_append_left (by omega)]
    · rw [List.getElem?_append_right (by omega), hB]
      congr 2; omega

theorem getD_axSigns (K : Nat) (a : Int) {k : Nat} (hk : k < K + 1) : (axSigns K a).getD k .pos = sig a k := by
  rw [List.getD_eq_getElem?_getD, get_axSigns K a hk]; rfl

theorem sig_zero (a : Int) (k : Nat) : sig a k = .zero ↔ a = 2 * (k : Int) := by
  unfold sig; split
  · simp only [reduceCtorEq, false_iff]; omega
  · split <;> simp_all

/-- An entry of the grid covector is zero iff its axis's doubled coordinate is on that plane. -/
theorem entry_zero (K : Nat) (c : DCell) {j : Nat} (hj : j < 3 * (K + 1)) :
    (cov K c).getD j .pos = .zero ↔ dc (j / (K + 1)) c = 2 * ((j % (K + 1) : Nat) : Int) := by
  unfold cov
  rw [getD3 _ _ _ (K + 1) (len_axSigns K _) (len_axSigns K _)]
  split
  · rename_i h1
    rw [getD_axSigns K _ h1, sig_zero, Nat.div_eq_of_lt h1, Nat.mod_eq_of_lt h1]; rfl
  · split
    · rename_i h1 h2
      rw [getD_axSigns K _ (by omega), sig_zero]
      have e1 : j / (K + 1) = 1 := by
        apply Nat.div_eq_of_lt_le <;> omega
      have e2 : j % (K + 1) = j - (K + 1) := by
        rw [Nat.mod_eq_sub_mul_div, e1]; omega
      rw [e1, e2]; rfl
    · rename_i h1 h2
      rw [getD_axSigns K _ (by omega), sig_zero]
      have e1 : j / (K + 1) = 2 := by
        apply Nat.div_eq_of_lt_le <;> omega
      have e2 : j % (K + 1) = j - 2 * (K + 1) := by
        rw [Nat.mod_eq_sub_mul_div, e1]; omega
      rw [e1, e2]; rfl

theorem grid_getD (K : Nat) {j : Nat} (hj : j < 3 * (K + 1)) :
    (gridPs K).getD j default = ⟨unitV (j / (K + 1)), (((j % (K + 1) : Nat) : Int) : Rat)⟩ := by
  unfold gridPs
  rw [getD3 _ _ _ (K + 1) (by simp [axPl]) (by simp [axPl])]
  have gl : ∀ ax k, k < K + 1 → (axPl K ax).getD k default = ⟨unitV ax, ((k : Int) : Rat)⟩ := by
    intro ax k hk
    rw [List.getD_eq_getElem?_getD]; simp [axPl, hk]
  split
  · rename_i h1
    rw [gl 0 j h1, Nat.div_eq_of_lt h1, Nat.mod_eq_of_lt h1]
  · split
    · rw [gl 1 _ (by omega)]
      have e1 : j / (K + 1) = 1 := by apply Nat.div_eq_of_lt_le <;> omega
      have e2 : j % (K + 1) = j - (K + 1) := by rw [Nat.mod_eq_sub_mul_div, e1]; omega
      rw [e1, e2]
    · rw [gl 2 _ (by omega)]
      have e1 : j / (K + 1) = 2 := by apply Nat.div_eq_of_lt_le <;> omega
      have e2 : j % (K + 1) = j - 2 * (K + 1) := by rw [Nat.mod_eq_sub_mul_div, e1]; omega
      rw [e1, e2]

theorem len_cov (K : Nat) (c : DCell) : (cov K c).length = 3 * (K + 1) := by
  simp [cov, len_axSigns]; omega

theorem mem_zeros (K : Nat) (c : DCell) (j : Nat) :
    j ∈ zerosOf (cov K c) ↔ j < 3 * (K + 1) ∧ dc (j / (K + 1)) c = 2 * ((j % (K + 1) : Nat) : Int) := by
  unfold zerosOf
  simp only [List.mem_filter, List.mem_range, len_cov, beq_iff_eq]
  constructor
  · rintro ⟨h1, h2⟩; exact ⟨h1, (entry_zero K c h1).mp h2⟩
  · rintro ⟨h1, h2⟩; exact ⟨h1, (entry_zero K c h1).mpr h2⟩

/-- The zero of axis `ax` of a doubled cell even there. -/
theorem zero_of_even (K : Nat) {c : DCell} (hc : InR K c) {ax : Nat} (hax : ax < 3) (he : dc ax c % 2 = 0) :
    ax * (K + 1) + (dc ax c / 2).toNat ∈ zerosOf (cov K c) ∧
      (ax * (K + 1) + (dc ax c / 2).toNat) / (K + 1) = ax := by
  have hr : -1 ≤ dc ax c ∧ dc ax c ≤ 2 * (K : Int) + 1 := by
    obtain ⟨h1, h2, h3⟩ := hc
    match ax, hax with
    | 0, _ => exact h1
    | 1, _ => exact h2
    | 2, _ => exact h3
  have hk : (dc ax c / 2).toNat < K + 1 := by omega
  have e1 : (ax * (K + 1) + (dc ax c / 2).toNat) / (K + 1) = ax := by
    rw [Nat.add_comm, Nat.add_mul_div_right _ _ (by omega), Nat.div_eq_of_lt hk]; omega
  have e2 : (ax * (K + 1) + (dc ax c / 2).toNat) % (K + 1) = (dc ax c / 2).toNat := by
    rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hk]
  have hle : ax * (K + 1) ≤ 2 * (K + 1) := Nat.mul_le_mul_right (K + 1) (show ax ≤ 2 by omega)
  refine ⟨(mem_zeros K c _).mpr ⟨by omega, ?_⟩, e1⟩
  rw [e1, e2]; omega

theorem even_of_zero (K : Nat) {c : DCell} {j : Nat} (h : j ∈ zerosOf (cov K c)) :
    j / (K + 1) < 3 ∧ dc (j / (K + 1)) c % 2 = 0 := by
  obtain ⟨h1, h2⟩ := (mem_zeros K c j).mp h
  refine ⟨?_, by omega⟩
  apply Nat.div_lt_of_lt_mul; omega

theorem isVert_iff (K : Nat) {c : DCell} (hc : InR K c) :
    IsVert (gridPs K) (cov K c) ↔ nEven c = 3 := by
  unfold IsVert nEven ev
  constructor
  · rintro ⟨a, ha, b, hb, d, hd, hdet⟩
    obtain ⟨ja, ea⟩ := even_of_zero K ha
    obtain ⟨jb, eb⟩ := even_of_zero K hb
    obtain ⟨jd, ed⟩ := even_of_zero K hd
    have ha' := (mem_zeros K c a).mp ha
    have hb' := (mem_zeros K c b).mp hb
    have hd' := (mem_zeros K c d).mp hd
    rw [grid_getD K ha'.1, grid_getD K hb'.1, grid_getD K hd'.1] at hdet
    obtain ⟨n1, n2, n3⟩ := (det_unit _ _ _ ja jb jd).mp hdet
    -- three distinct axes below 3: each of 0, 1, 2
    have all : ∀ o, o < 3 → dc o c % 2 = 0 := by
      intro o ho
      revert ea eb ed ja jb jd n1 n2 n3
      generalize a / (K + 1) = x
      generalize b / (K + 1) = y
      generalize d / (K + 1) = z
      intro ea eb ed ja jb jd n1 n2 n3
      have : o = x ∨ o = y ∨ o = z := by omega
      rcases this with rfl | rfl | rfl <;> assumption
    have h0 := all 0 (by omega); have h1 := all 1 (by omega); have h2 := all 2 (by omega)
    simp only [dc] at h0 h1 h2
    simp [h0, h1, h2]
  · intro h
    obtain ⟨c1, c2, c3⟩ := c
    simp only at h
    by_cases h0 : c1 % 2 = 0 <;> by_cases h1 : c2 % 2 = 0 <;> by_cases h2 : c3 % 2 = 0 <;>
      simp only [h0, h1, h2, ite_true, ite_false] at h <;> try omega
    obtain ⟨z0, e0⟩ := zero_of_even K hc (ax := 0) (by omega) h0
    obtain ⟨z1, e1⟩ := zero_of_even K hc (ax := 1) (by omega) h1
    obtain ⟨z2, e2⟩ := zero_of_even K hc (ax := 2) (by omega) h2
    refine ⟨_, z0, _, z1, _, z2, ?_⟩
    rw [grid_getD K ((mem_zeros K _ _).mp z0).1, grid_getD K ((mem_zeros K _ _).mp z1).1,
      grid_getD K ((mem_zeros K _ _).mp z2).1, e0, e1, e2]
    exact (det_unit 0 1 2 (by omega) (by omega) (by omega)).mpr ⟨by omega, by omega, by omega⟩

theorem isEdge_iff (K : Nat) {c : DCell} (hc : InR K c) :
    IsEdge (gridPs K) (cov K c) ↔ nEven c = 2 := by
  unfold IsEdge
  rw [isVert_iff K hc]
  constructor
  · rintro ⟨hn, a, ha, b, hb, hcr⟩
    obtain ⟨ja, ea⟩ := even_of_zero K ha
    obtain ⟨jb, eb⟩ := even_of_zero K hb
    rw [grid_getD K ((mem_zeros K _ _).mp ha).1, grid_getD K ((mem_zeros K _ _).mp hb).1] at hcr
    have hne := (cross_unit _ _ ja jb).mp hcr
    unfold nEven ev at hn ⊢
    obtain ⟨c1, c2, c3⟩ := c
    have two : ∀ o, o < 3 → o = a / (K + 1) ∨ o = b / (K + 1) ∨ True := by intro o _; simp
    -- two distinct axes are even, not all three
    revert ea eb
    generalize a / (K + 1) = x at *
    generalize b / (K + 1) = y at *
    intro ea eb
    match x, ja, y, jb with
    | 0, _, 0, _ | 1, _, 1, _ | 2, _, 2, _ => exact absurd rfl hne
    | 0, _, 1, _ | 1, _, 0, _ =>
      simp only [dc] at ea eb; simp only [ea, eb] at hn ⊢; by_cases e : c3 % 2 = 0 <;> simp_all
    | 0, _, 2, _ | 2, _, 0, _ =>
      simp only [dc] at ea eb; simp only [ea, eb] at hn ⊢; by_cases e : c2 % 2 = 0 <;> simp_all
    | 1, _, 2, _ | 2, _, 1, _ =>
      simp only [dc] at ea eb; simp only [ea, eb] at hn ⊢; by_cases e : c1 % 2 = 0 <;> simp_all
  · intro h
    refine ⟨by omega, ?_⟩
    unfold nEven ev at h
    obtain ⟨c1, c2, c3⟩ := c
    simp only at h
    by_cases e1 : c1 % 2 = 0 <;> by_cases e2 : c2 % 2 = 0 <;> by_cases e3 : c3 % 2 = 0 <;>
      simp only [e1, e2, e3, ite_true, ite_false] at h <;> try omega
    · obtain ⟨z0, f0⟩ := zero_of_even K hc (ax := 0) (by omega) (by simpa [dc] using e1)
      obtain ⟨z1, f1⟩ := zero_of_even K hc (ax := 1) (by omega) (by simpa [dc] using e2)
      refine ⟨_, z0, _, z1, ?_⟩
      rw [grid_getD K ((mem_zeros K _ _).mp z0).1, grid_getD K ((mem_zeros K _ _).mp z1).1, f0, f1]
      exact (cross_unit 0 1 (by omega) (by omega)).mpr (by omega)
    · obtain ⟨z0, f0⟩ := zero_of_even K hc (ax := 0) (by omega) (by simpa [dc] using e1)
      obtain ⟨z1, f1⟩ := zero_of_even K hc (ax := 2) (by omega) (by simpa [dc] using e3)
      refine ⟨_, z0, _, z1, ?_⟩
      rw [grid_getD K ((mem_zeros K _ _).mp z0).1, grid_getD K ((mem_zeros K _ _).mp z1).1, f0, f1]
      exact (cross_unit 0 2 (by omega) (by omega)).mpr (by omega)
    · obtain ⟨z0, f0⟩ := zero_of_even K hc (ax := 1) (by omega) (by simpa [dc] using e2)
      obtain ⟨z1, f1⟩ := zero_of_even K hc (ax := 2) (by omega) (by simpa [dc] using e3)
      refine ⟨_, z0, _, z1, ?_⟩
      rw [grid_getD K ((mem_zeros K _ _).mp z0).1, grid_getD K ((mem_zeros K _ _).mp z1).1, f0, f1]
      exact (cross_unit 1 2 (by omega) (by omega)).mpr (by omega)

/-! ## Edges and vertices of the box ledger -/

theorem inR_dc {K : Nat} {c : DCell} (hc : InR K c) (a : Nat) (ha : a < 3) :
    -1 ≤ dc a c ∧ dc a c ≤ 2 * (K : Int) + 1 := by
  obtain ⟨h1, h2, h3⟩ := hc
  match a, ha with
  | 0, _ => exact h1
  | 1, _ => exact h2
  | 2, _ => exact h3

/-- The face order between two grid cells, axis by axis. -/
theorem le_rel (K : Nat) {c' c : DCell} (hc' : InR K c') (hc : InR K c) :
    Cov.le (cov K c') (cov K c) = true ↔ ∀ a, a < 3 →
      dc a c' = dc a c ∨ (dc a c' % 2 = 0 ∧ (dc a c = dc a c' + 1 ∨ dc a c = dc a c' - 1)) := by
  rw [le_cov, le_axSigns, le_axSigns, le_axSigns,
    le1_iff K hc'.1 hc.1, le1_iff K hc'.2.1 hc.2.1, le1_iff K hc'.2.2 hc.2.2]
  constructor
  · rintro ⟨h0, h1, h2⟩ a ha
    match a, ha with
    | 0, _ => exact h0
    | 1, _ => exact h1
    | 2, _ => exact h2
  · intro h; exact ⟨h 0 (by omega), h 1 (by omega), h 2 (by omega)⟩

theorem inR_bump_other {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} {f : Nat}
    (hp : (c, f) ∈ boxPieces boxes) {o : Nat} (ho : o < 3) (hne : o ≠ f % 6 / 2) (d : Int)
    (hd : d = 1 ∨ d = -1) : InR K (bump o d c) ∧ dc o c % 2 = 1 := by
  obtain ⟨hi, hs, hcs, h1, h2, _⟩ := piece_facts hv hp
  obtain ⟨_, hothers⟩ := (mem_faceCells (valid_at hv hi) hs c).mp hcs
  have hin := hothers o ho hne
  have hb := valid_at hv hi o ho
  unfold InAx at hin
  refine ⟨?_, hin.1⟩
  have r := fun a ha => inR_dc h2 a ha
  have key : ∀ a, a < 3 → -1 ≤ dc a (bump o d c) ∧ dc a (bump o d c) ≤ 2 * (K : Int) + 1 := by
    intro a ha
    rw [dc_bump o a ho ha]
    split
    · rename_i e; subst e; omega
    · exact r a ha
  exact ⟨key 0 (by omega), key 1 (by omega), key 2 (by omega)⟩

theorem nEven_dc (c : DCell) : nEven c = ev (dc 0 c) + ev (dc 1 c) + ev (dc 2 c) := rfl

theorem dcell_ext {c c' : DCell} (h : ∀ a, a < 3 → dc a c = dc a c') : c = c' := by
  obtain ⟨c1, c2, c3⟩ := c
  obtain ⟨d1, d2, d3⟩ := c'
  have h0 := h 0 (by omega); have h1 := h 1 (by omega); have h2 := h 2 (by omega)
  simp only [dc] at h0 h1 h2
  simp [h0, h1, h2]

/-- The shape of an edge cell beside a facet cell. -/
theorem edge_shape {c c' : DCell} {ax : Nat} (h : OneEven c ax)
    (hrel : ∀ a, a < 3 → dc a c' = dc a c ∨ (dc a c' % 2 = 0 ∧ (dc a c = dc a c' + 1 ∨ dc a c = dc a c' - 1)))
    (hn : nEven c' = 2) :
    ∃ o d, o < 3 ∧ o ≠ ax ∧ (d = 1 ∨ d = -1) ∧ c' = bump o d c ∧ (o = (ax + 1) % 3 ∨ o = (ax + 2) % 3) := by
  obtain ⟨hax, he, ho⟩ := h
  have r0 := hrel 0 (by omega); have r1 := hrel 1 (by omega); have r2 := hrel 2 (by omega)
  have p0 := ho 0 (by omega); have p1 := ho 1 (by omega); have p2 := ho 2 (by omega)
  obtain ⟨c1, c2, c3⟩ := c
  obtain ⟨d1, d2, d3⟩ := c'
  unfold nEven ev at hn
  simp only [dc] at r0 r1 r2 p0 p1 p2 hn he ⊢
  match ax, hax with
  | 0, _ =>
    simp only [dc] at he
    have q0 : d1 % 2 = 0 := by omega
    by_cases q1 : d2 % 2 = 0
    · have q2 : ¬(d3 % 2 = 0) := by intro q2; simp [q0, q1, q2] at hn
      exact ⟨1, d2 - c2, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩
    · have q2 : d3 % 2 = 0 := by
        apply Classical.byContradiction; intro q2; simp [q0, q1, q2] at hn
      exact ⟨2, d3 - c3, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩
  | 1, _ =>
    simp only [dc] at he
    have q0 : d2 % 2 = 0 := by omega
    by_cases q1 : d1 % 2 = 0
    · have q2 : ¬(d3 % 2 = 0) := by intro q2; simp [q0, q1, q2] at hn
      exact ⟨0, d1 - c1, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩
    · have q2 : d3 % 2 = 0 := by
        apply Classical.byContradiction; intro q2; simp [q0, q1, q2] at hn
      exact ⟨2, d3 - c3, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩
  | 2, _ =>
    simp only [dc] at he
    have q0 : d3 % 2 = 0 := by omega
    by_cases q1 : d1 % 2 = 0
    · have q2 : ¬(d2 % 2 = 0) := by intro q2; simp [q0, q1, q2] at hn
      exact ⟨0, d1 - c1, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩
    · have q2 : d2 % 2 = 0 := by
        apply Classical.byContradiction; intro q2; simp [q0, q1, q2] at hn
      exact ⟨1, d2 - c2, by omega, by omega, by omega, by simp only [bump, Prod.mk.injEq]; omega, by omega⟩

/-- And of a vertex cell beside a facet cell: moved by one on both other axes. -/
theorem vert_shape {c c' : DCell} {ax : Nat} (h : OneEven c ax)
    (hrel : ∀ a, a < 3 → dc a c' = dc a c ∨ (dc a c' % 2 = 0 ∧ (dc a c = dc a c' + 1 ∨ dc a c = dc a c' - 1)))
    (hn : nEven c' = 3) :
    ∃ d1 d2 : Int, (d1 = 1 ∨ d1 = -1) ∧ (d2 = 1 ∨ d2 = -1) ∧
      c' = bump ((ax + 2) % 3) d2 (bump ((ax + 1) % 3) d1 c) := by
  obtain ⟨hax, he, ho⟩ := h
  have r0 := hrel 0 (by omega); have r1 := hrel 1 (by omega); have r2 := hrel 2 (by omega)
  have p0 := ho 0 (by omega); have p1 := ho 1 (by omega); have p2 := ho 2 (by omega)
  obtain ⟨c1, c2, c3⟩ := c
  obtain ⟨d1, d2, d3⟩ := c'
  unfold nEven ev at hn
  simp only [dc] at r0 r1 r2 p0 p1 p2 hn he ⊢
  by_cases q1 : d1 % 2 = 0 <;> by_cases q2 : d2 % 2 = 0 <;> by_cases q3 : d3 % 2 = 0 <;>
    simp only [q1, q2, q3, ite_true, ite_false] at hn <;> try omega
  match ax, hax with
  | 0, _ =>
    simp only at he; have := p1 (by decide); have := p2 (by decide)
    exact ⟨d2 - c2, d3 - c3, by omega, by omega, by simp only [Nat.reduceAdd, Nat.reduceMod, bump, Prod.mk.injEq]; omega⟩
  | 1, _ =>
    simp only at he; have := p0 (by decide); have := p2 (by decide)
    exact ⟨d3 - c3, d1 - c1, by omega, by omega, by simp only [Nat.reduceAdd, Nat.reduceMod, bump, Prod.mk.injEq]; omega⟩
  | 2, _ =>
    simp only at he; have := p0 (by decide); have := p1 (by decide)
    exact ⟨d1 - c1, d2 - c2, by omega, by omega, by simp only [Nat.reduceAdd, Nat.reduceMod, bump, Prod.mk.injEq]; omega⟩

theorem mem_edges_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) (e : Cov) :
    e ∈ (boxLedger K boxes).edges ↔ ∃ c f, (c, f) ∈ boxPieces boxes ∧ ∃ o d,
      (o = (f % 6 / 2 + 1) % 3 ∨ o = (f % 6 / 2 + 2) % 3) ∧ (d = 1 ∨ d = -1) ∧ e = cov K (bump o d c) := by
  unfold boxLedger
  simp only [List.mem_map, mem_dedup, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
  constructor
  · rintro ⟨c', ⟨c, ⟨⟨c0, f⟩, hp, rfl⟩, hc'⟩, rfl⟩
    simp only at hc'
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1] at hc'
    refine ⟨c0, f, hp, ?_⟩
    rcases hc' with rfl | rfl | rfl | rfl
    · exact ⟨_, 1, Or.inl rfl, Or.inl rfl, rfl⟩
    · exact ⟨_, -1, Or.inl rfl, Or.inr rfl, rfl⟩
    · exact ⟨_, 1, Or.inr rfl, Or.inl rfl, rfl⟩
    · exact ⟨_, -1, Or.inr rfl, Or.inr rfl, rfl⟩
  · rintro ⟨c, f, hp, o, d, ho, hd, rfl⟩
    refine ⟨bump o d c, ⟨c, ⟨⟨c, f⟩, hp, rfl⟩, ?_⟩, rfl⟩
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1]
    rcases ho with rfl | rfl <;> rcases hd with rfl | rfl <;> simp

theorem mem_verts_box (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) (v : Cov) :
    v ∈ (boxLedger K boxes).verts ↔ ∃ c f, (c, f) ∈ boxPieces boxes ∧ ∃ d1 d2 : Int,
      (d1 = 1 ∨ d1 = -1) ∧ (d2 = 1 ∨ d2 = -1) ∧
      v = cov K (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) := by
  unfold boxLedger
  simp only [List.mem_map, mem_dedup, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
  constructor
  · rintro ⟨c', ⟨c, ⟨⟨c0, f⟩, hp, rfl⟩, hc'⟩, rfl⟩
    simp only at hc'
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1] at hc'
    refine ⟨c0, f, hp, ?_⟩
    rcases hc' with rfl | rfl | rfl | rfl
    · exact ⟨1, 1, Or.inl rfl, Or.inl rfl, rfl⟩
    · exact ⟨1, -1, Or.inl rfl, Or.inr rfl, rfl⟩
    · exact ⟨-1, 1, Or.inr rfl, Or.inl rfl, rfl⟩
    · exact ⟨-1, -1, Or.inr rfl, Or.inr rfl, rfl⟩
  · rintro ⟨c, f, hp, d1, d2, h1, h2, rfl⟩
    refine ⟨_, ⟨c, ⟨⟨c, f⟩, hp, rfl⟩, ?_⟩, rfl⟩
    rw [evenAx_eq (piece_facts hv hp).2.2.2.1]
    rcases h1 with rfl | rfl <;> rcases h2 with rfl | rfl <;> simp

theorem nEven_bump {o : Nat} (ho : o < 3) {d : Int} (hd : d = 1 ∨ d = -1) {c : DCell}
    (hodd : dc o c % 2 = 1) : nEven (bump o d c) = nEven c + 1 := by
  obtain ⟨c1, c2, c3⟩ := c
  match o, ho with
  | 0, _ | 1, _ | 2, _ =>
    simp only [dc] at hodd
    simp only [nEven, bump, ev]
    rcases hd with rfl | rfl <;> (repeat' split) <;> omega

theorem nEven_one {c : DCell} {ax : Nat} (h : OneEven c ax) : nEven c = 1 := by
  obtain ⟨hax, he, ho⟩ := h
  have p0 := fun h => ho 0 (by omega) h
  have p1 := fun h => ho 1 (by omega) h
  have p2 := fun h => ho 2 (by omega) h
  obtain ⟨c1, c2, c3⟩ := c
  match ax, hax with
  | 0, _ | 1, _ | 2, _ =>
    simp only [dc] at he p0 p1 p2
    simp only [nEven, ev]
    have := p0; have := p1; have := p2
    simp_all <;> omega

/-- A piece cell moved by one on another axis: two even coordinates, and below the piece. -/
theorem bumped_edge {K : Nat} {boxes : List BoxN} (hv : Valid K boxes) {c : DCell} {f : Nat}
    (hp : (c, f) ∈ boxPieces boxes) {o : Nat} (ho : o = (f % 6 / 2 + 1) % 3 ∨ o = (f % 6 / 2 + 2) % 3)
    {d : Int} (hd : d = 1 ∨ d = -1) :
    InR K (bump o d c) ∧ nEven (bump o d c) = 2 ∧ Cov.le (cov K (bump o d c)) (cov K c) = true := by
  obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv hp
  have hax := h1.1
  have hne : o ≠ f % 6 / 2 := by omega
  have ho3 : o < 3 := by omega
  obtain ⟨hR, hodd⟩ := inR_bump_other hv hp ho3 hne d hd
  refine ⟨hR, ?_, (le_rel K hR h2).mpr fun a ha => ?_⟩
  · rw [nEven_bump ho3 hd hodd, nEven_one h1]
  · rw [dc_bump o a ho3 ha]
    split
    · rename_i e; subst e; right; exact ⟨by omega, by omega⟩
    · left; rfl

theorem box_edges (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ e, e ∈ (boxLedger K boxes).edges ↔ Realised (gridPs K) e ∧ IsEdge (gridPs K) e ∧
      ∃ φ ∈ (boxLedger K boxes).facets, Cov.le e φ = true := by
  intro e
  rw [mem_edges_box K hv]
  constructor
  · rintro ⟨c, f, hp, o, d, ho, hd, rfl⟩
    obtain ⟨hR, hn, hle⟩ := bumped_edge hv hp ho hd
    exact ⟨⟨pt _, pt_cell K _⟩, (isEdge_iff K hR).mpr hn, cov K c,
      (mem_facets_box K boxes _).mpr ⟨c, f, hp, rfl⟩, hle⟩
  · rintro ⟨⟨x, hx⟩, hE, φ, hφ, hle⟩
    have e1 := realised_eq K hx
    subst e1
    have hR := dblV_inR K x
    obtain ⟨c, f, hp, rfl⟩ := (mem_facets_box K boxes φ).mp hφ
    obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv hp
    obtain ⟨o, d, _, _, hd, he, ho⟩ := edge_shape h1 ((le_rel K hR h2).mp hle) ((isEdge_iff K hR).mp hE)
    exact ⟨c, f, hp, o, d, ho, hd, by rw [he]⟩

theorem box_verts (K : Nat) {boxes : List BoxN} (hv : Valid K boxes) :
    ∀ v, v ∈ (boxLedger K boxes).verts ↔ Realised (gridPs K) v ∧ IsVert (gridPs K) v ∧
      ∃ φ ∈ (boxLedger K boxes).facets, Cov.le v φ = true := by
  intro v
  rw [mem_verts_box K hv]
  constructor
  · rintro ⟨c, f, hp, d1, d2, h1, h2, rfl⟩
    obtain ⟨_, _, _, ho, hc, _⟩ := piece_facts hv hp
    have hax := ho.1
    -- the first move is an edge cell; the second, on the third axis, another
    obtain ⟨hR1, hn1, hle1⟩ := bumped_edge hv hp (Or.inl rfl) h1
    have o3 : (f % 6 / 2 + 2) % 3 < 3 := by omega
    have hne : (f % 6 / 2 + 2) % 3 ≠ (f % 6 / 2 + 1) % 3 := by omega
    have hodd : dc ((f % 6 / 2 + 2) % 3) (bump ((f % 6 / 2 + 1) % 3) d1 c) % 2 = 1 := by
      rw [dc_bump _ _ (by omega) o3, if_neg hne]
      exact (inR_bump_other hv hp o3 (by omega) d2 h2).2
    have hR : InR K (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) := by
      have b := (inR_bump_other hv hp o3 (by omega) d2 h2).1
      have key : ∀ a, a < 3 → dc a (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) =
          dc a (bump ((f % 6 / 2 + 2) % 3) d2 c) ∨
          dc a (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) =
          dc a (bump ((f % 6 / 2 + 1) % 3) d1 c) := by
        intro a ha
        rw [dc_bump _ a o3 ha, dc_bump _ a o3 ha, dc_bump _ a (by omega) ha]
        split <;> split <;> simp_all
      have r1 := fun a ha => inR_dc b a ha
      have r2 := fun a ha => inR_dc hR1 a ha
      have r : ∀ a, a < 3 → -1 ≤ dc a (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) ∧
          dc a (bump ((f % 6 / 2 + 2) % 3) d2 (bump ((f % 6 / 2 + 1) % 3) d1 c)) ≤ 2 * (K : Int) + 1 := by
        intro a ha
        rcases key a ha with e | e <;> rw [e]
        · exact r1 a ha
        · exact r2 a ha
      exact ⟨r 0 (by omega), r 1 (by omega), r 2 (by omega)⟩
    refine ⟨⟨pt _, pt_cell K _⟩, (isVert_iff K hR).mpr ?_, cov K c,
      (mem_facets_box K boxes _).mpr ⟨c, f, hp, rfl⟩, ?_⟩
    · rw [nEven_bump o3 h2 hodd, hn1]
    · apply (le_rel K hR hc).mpr
      intro a ha
      rw [dc_bump _ a o3 ha, dc_bump _ a (by omega) ha]
      have hc1 := (inR_bump_other hv hp (show (f % 6 / 2 + 1) % 3 < 3 by omega) (by omega) d1 h1).2
      have hc2 := (inR_bump_other hv hp o3 (by omega) d2 h2).2
      split
      · rename_i e; subst e
        right
        rw [if_neg hne]
        exact ⟨by omega, by omega⟩
      · split
        · rename_i _ e; subst e; right; exact ⟨by omega, by omega⟩
        · left; rfl
  · rintro ⟨⟨x, hx⟩, hV, φ, hφ, hle⟩
    have e1 := realised_eq K hx
    subst e1
    have hR := dblV_inR K x
    obtain ⟨c, f, hp, rfl⟩ := (mem_facets_box K boxes φ).mp hφ
    obtain ⟨_, _, _, h1, h2, _⟩ := piece_facts hv hp
    obtain ⟨d1, d2, hd1, hd2, he⟩ := vert_shape h1 ((le_rel K hR h2).mp hle) ((isVert_iff K hR).mp hV)
    exact ⟨c, f, hp, d1, d2, hd1, hd2, by rw [he]⟩

/-- **`Exact` for every box family.** For any finite list of boxes on the grid `0 .. K`, the ledger
the generator builds is exact for the grid's carriers and the open boxes: so (b) and (c) hold of
it as theorems, with no hypothesis left. -/
theorem box_exact (K : Nat) (boxes : List BoxN) (hv : Valid K boxes) :
    Exact (boxLedger K boxes) (gridPs K) (boxA boxes) where
  nonzero := gridPs_nonzero K
  compat := box_compat K hv
  pieceFacet := box_pieceFacet K hv
  oneFace := box_oneFace K hv
  pieceReal := box_pieceReal K boxes
  witness := box_witness K hv
  boundary := box_boundary K hv
  complete := box_complete K hv
  edges := box_edges K hv
  verts := box_verts K hv

/-- **(c) for every box family.** The core on the generated ledger: each valid answer is the exact
regularised boolean of the open boxes, with its faces the exact boundary facets. -/
theorem box_correct (K : Nat) (boxes : List BoxN) (hv : Valid K boxes) (rule : Rule) {b : Body}
    (hb : core (boxLedger K boxes) rule = .ok b) :
    DecideOnce.Valid (boxLedger K boxes) ((boxLedger K boxes).keepTab rule) b ∧
    (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ XBoundary (boxLedger K boxes) (gridPs K) (boxA boxes) rule φ) ∧
    reg ((decomp (box_exact K boxes hv)).kept fun T =>
        xkeep (boxLedger K boxes) (gridPs K) (boxA boxes) rule T = true) =
      reg (combine (ruleOp rule) (opnds (boxLedger K boxes) (boxA boxes))) :=
  correct_ok (box_exact K boxes hv) rule hb

/-- **The refusal is exact for every box family.** The core refuses as zero-thickness iff the exact
boolean of the open boxes is non-manifold at an edge or vertex of the grid. -/
theorem box_refused_iff (K : Nat) (boxes : List BoxN) (hv : Valid K boxes) (rule : Rule) :
    (∃ x, core (boxLedger K boxes) rule = .refused (.zeroThickness x)) ↔
      XNonManifold (boxLedger K boxes) (gridPs K) (boxA boxes) rule :=
  zeroThickness_iff (box_exact K boxes hv) rule

end ForgeExact.DecideOnce.BoxEx
