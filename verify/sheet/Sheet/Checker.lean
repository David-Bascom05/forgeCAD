import Sheet.Trig

/-!
# The refold checker as executable spec (audit2 §2.0, conditions 1–8)

The input is what the kernel publishes for a sheet, every number a double read as its exact
dyadic value: the gauge, each wall's folded frame and developed place, each bend's numbers and
developed place (`build::Layout`), the flat build's frame of the first wall, and the pairs (flat
vertex, folded vertex of the same name). The checker recomputes the fold from these numbers alone
(refold.rs's model: wall rigidly, zone round its own axis at `rn = BA/θ`) in exact rationals, with
`sin` and `cos` enclosed (Sheet/Trig.lean), and every comparison is three-valued:

* `pass`: proven to hold for the real numbers (`*_sound` theorems);
* `fail`: proven not to hold;
* `undecided`: the enclosure straddles the bound: refused with the locus, never passed.

Frames are taken as published: `Frame.toWorld (x, y, z) = o + x ex + y ey + z ez`,
`Frame.toLocal p = ((p − o)·ex, (p − o)·ey, (p − o)·ez)`, `Place.apply (x, y) = o + x X + y X⊥`;
the spec is the fold *of those numbers*, so no orthonormality is assumed of them.

Conditions:
* **7** `kappa`: `R θ ≤ BA ≤ (R + t) θ`, exact (κ ∈ [0, 1], Rules.lean).
* **3/4** `setback`: the published setback is `ρ sin θ / (1 + cos θ)` within `tol`.
* **5** `continuity`: at both tangent lines, both ends, both skins, the zone's fold is the wall's
  within `tol`.
* **1** `refold`: every flat vertex, developed and folded (by the wall or zone holding it), is its
  folded namesake within `tol`.
* **2** is a theorem (`Bend.iso`, `neutral_radius`), not a check on data: with `ρ` as the kernel
  chooses it the neutral radius is `rn` for every input.
* **6** (overlap and clash) is decided by Region.lean's exact rule and BranchBound.lean's
  certified search on the same inputs.
* **8** (export) by the one rule (Region.lean `closes`, `closes_of_classes`): each contour's
  consecutive pieces meet in one vertex class, i.e. within the spread cap `2L` — exact to rounding
  when no class merges distinct points, within the class's spread when one does.

`tol` is `1e-9` of the part's size, as refold.rs has it: `tol² = 10⁻¹⁸ · max(1, max |v|²)`.
-/

namespace Sheet.Checker

open Sheet.Trig

abbrev V3 := ℚ × ℚ × ℚ
abbrev V2 := ℚ × ℚ

def V3.add (a b : V3) : V3 := (a.1 + b.1, a.2.1 + b.2.1, a.2.2 + b.2.2)
def V3.sub (a b : V3) : V3 := (a.1 - b.1, a.2.1 - b.2.1, a.2.2 - b.2.2)
def V3.smul (k : ℚ) (a : V3) : V3 := (k * a.1, k * a.2.1, k * a.2.2)
def V3.dot (a b : V3) : ℚ := a.1 * b.1 + a.2.1 * b.2.1 + a.2.2 * b.2.2

structure Frame where
  o : V3
  ex : V3
  ey : V3
  ez : V3

def Frame.toWorld (f : Frame) (p : V3) : V3 :=
  f.o.add ((f.ex.smul p.1).add ((f.ey.smul p.2.1).add (f.ez.smul p.2.2)))

def Frame.vecToWorld (f : Frame) (p : V3) : V3 :=
  (f.ex.smul p.1).add ((f.ey.smul p.2.1).add (f.ez.smul p.2.2))

def Frame.toLocal (f : Frame) (p : V3) : V3 :=
  let d := p.sub f.o
  (d.dot f.ex, d.dot f.ey, d.dot f.ez)

structure Place where
  o : V2
  x : V2

def Place.apply (pl : Place) (p : V2) : V2 :=
  (pl.o.1 + pl.x.1 * p.1 - pl.x.2 * p.2, pl.o.2 + pl.x.2 * p.1 + pl.x.1 * p.2)

def Place.local (pl : Place) (q : V2) : V2 :=
  let d : V2 := (q.1 - pl.o.1, q.2 - pl.o.2)
  (d.1 * pl.x.1 + d.2 * pl.x.2, -d.1 * pl.x.2 + d.2 * pl.x.1)

def Place.localVec (pl : Place) (v : V2) : V2 :=
  (v.1 * pl.x.1 + v.2 * pl.x.2, -v.1 * pl.x.2 + v.2 * pl.x.1)

structure WallD where
  id : ℕ
  fold : Frame
  dev : Place

structure BendD where
  joint : ℕ
  a : ℕ
  b : ℕ
  angle : ℚ
  R : ℚ
  BA : ℚ
  s : ℚ
  len : ℚ
  dev : Place

structure Input where
  t : ℚ
  walls : List WallD
  bends : List BendD
  flat0 : Frame
  dev0 : Place
  verts : List (V3 × V3)
  /-- The joints that are hems: their setback is `R + t` (hem.rs), the mold line the fold's
  outside, and wall b is placed by `Bend.Jat` (`zone_BA_hem`). -/
  hems : List ℕ := []

/-! ## Verdicts -/

inductive Verdict
  | pass
  | fail (why : String)
  | undecided (why : String)
  deriving Repr, DecidableEq

def Verdict.and : Verdict → Verdict → Verdict
  | .fail w, _ => .fail w
  | _, .fail w => .fail w
  | .undecided w, _ => .undecided w
  | _, .undecided w => .undecided w
  | .pass, .pass => .pass

def all (vs : List Verdict) : Verdict := vs.foldl Verdict.and .pass

theorem and_pass {a b : Verdict} (h : a.and b = .pass) : a = .pass ∧ b = .pass := by
  cases a <;> cases b <;> simp_all [Verdict.and]

theorem all_pass : ∀ (acc : Verdict) (vs : List Verdict), vs.foldl Verdict.and acc = .pass →
    acc = .pass ∧ ∀ v ∈ vs, v = .pass
  | acc, [], h => ⟨h, fun _ h => nomatch h⟩
  | acc, v :: vs, h => by
      obtain ⟨h1, h2⟩ := all_pass _ vs h
      obtain ⟨h3, h4⟩ := and_pass h1
      exact ⟨h3, fun w hw => (List.mem_cons.mp hw).elim (fun e => e ▸ h4) (h2 w)⟩

/-- Pass iff every part passes. -/
theorem all_spec {vs : List Verdict} (h : all vs = .pass) : ∀ v ∈ vs, v = .pass :=
  (all_pass _ vs h).2

/-! ## Interval points and the distance test -/

abbrev VI := QI × QI × QI

def VI.mem (I : VI) (p : ℝ × ℝ × ℝ) : Prop := I.1.mem p.1 ∧ I.2.1.mem p.2.1 ∧ I.2.2.mem p.2.2

def V3.toR (p : V3) : ℝ × ℝ × ℝ := ((p.1 : ℝ), (p.2.1 : ℝ), (p.2.2 : ℝ))

def dist2R (p q : ℝ × ℝ × ℝ) : ℝ := (p.1 - q.1) ^ 2 + (p.2.1 - q.2.1) ^ 2 + (p.2.2 - q.2.2) ^ 2

/-- An enclosure of `|p − q|²` for `p ∈ I`. -/
def dist2I (I : VI) (q : V3) : QI :=
  let d1 := I.1.sub (QI.pt q.1)
  let d2 := I.2.1.sub (QI.pt q.2.1)
  let d3 := I.2.2.sub (QI.pt q.2.2)
  ((d1.mul d1).add (d2.mul d2)).add (d3.mul d3)

theorem dist2I_mem {I : VI} {p : ℝ × ℝ × ℝ} (h : I.mem p) (q : V3) :
    (dist2I I q).mem (dist2R p q.toR) := by
  have d1 := mem_sub h.1 (mem_pt q.1)
  have d2 := mem_sub h.2.1 (mem_pt q.2.1)
  have d3 := mem_sub h.2.2 (mem_pt q.2.2)
  have := mem_add (mem_add (mem_mul d1 d1) (mem_mul d2 d2)) (mem_mul d3 d3)
  simp only [dist2R, V3.toR, sq]
  exact this

/-- Is the point in `I` within `tol` (given as `tol²`) of `q`? -/
def near (I : VI) (q : V3) (tol2 : ℚ) (what : String) : Verdict :=
  let d := dist2I I q
  if d.2 ≤ tol2 then .pass else if d.1 > tol2 then .fail what else .undecided what

theorem near_sound {I : VI} {p : ℝ × ℝ × ℝ} (h : I.mem p) (q : V3) (tol2 : ℚ) (what : String) :
    (near I q tol2 what = .pass → dist2R p q.toR ≤ tol2) ∧
    (∀ w, near I q tol2 what = .fail w → (tol2 : ℝ) < dist2R p q.toR) := by
  have hm := dist2I_mem h q
  unfold near
  dsimp only
  constructor
  · intro hp
    split_ifs at hp with h1
    exact le_trans hm.2 (by exact_mod_cast h1)
  · intro w hf
    split_ifs at hf with h1 h2
    exact lt_of_lt_of_le (by exact_mod_cast h2) hm.1

def VI.pt (p : V3) : VI := (QI.pt p.1, QI.pt p.2.1, QI.pt p.2.2)

theorem VI.mem_pt (p : V3) : (VI.pt p).mem p.toR := ⟨Trig.mem_pt _, Trig.mem_pt _, Trig.mem_pt _⟩

/-! ## The zone -/

/-- A bend's zone, from the published numbers. -/
structure Zone where
  dev : Place
  ba : ℚ
  len : ℚ
  rn : ℚ
  σ : ℚ
  ρ : ℚ
  fa : Frame
  da : Place

def Zone.ofBend (t : ℚ) (b : BendD) (fa : Frame) (da : Place) : Zone :=
  let θ := |b.angle|
  let σ : ℚ := if b.angle > 0 then 1 else -1
  { dev := b.dev, ba := b.BA, len := b.len, rn := b.BA / θ, σ := σ,
    ρ := if b.angle > 0 then b.R else b.R + t, fa := fa, da := da }

/-- `wall_at`: the wall point at developed `q`, depth `w`. -/
def wallAt (f : Frame) (d : Place) (q : V2) (w : ℚ) : V3 :=
  let l := d.local q
  f.toWorld (l.1, l.2, -w)

/-- The zone's frame at `y`: its tangent point on wall a, its across direction, its normal. -/
def Zone.frame (z : Zone) (y : ℚ) : V3 × V3 × V3 :=
  let t := wallAt z.fa z.da (z.dev.apply (0, y)) 0
  let p1 := z.dev.apply (1, 0)
  let p0 := z.dev.apply (0, 0)
  let across := z.da.localVec (p1.1 - p0.1, p1.2 - p0.2)
  let eu := z.fa.vecToWorld (across.1, across.2, 0)
  (t, eu, z.fa.ez)

/-- The zone point `(u, y, w)` with `sin φ = S`, `cos φ = K`, over ℝ: refold.rs `Zone::at`,
`c + (n (−σ K) + eu S) r` with `c = t + n σ ρ`, `r = ρ + σ w`. -/
def Zone.atR (z : Zone) (y w : ℚ) (S K : ℝ) : ℝ × ℝ × ℝ :=
  let f := z.frame y
  let c := f.1.add (f.2.2.smul (z.σ * z.ρ))
  let r := z.ρ + z.σ * w
  (((c.1 : ℚ) : ℝ) + ((f.2.2.1 * (-z.σ * r) : ℚ) : ℝ) * K + ((f.2.1.1 * r : ℚ) : ℝ) * S,
   ((c.2.1 : ℚ) : ℝ) + ((f.2.2.2.1 * (-z.σ * r) : ℚ) : ℝ) * K + ((f.2.1.2.1 * r : ℚ) : ℝ) * S,
   ((c.2.2 : ℚ) : ℝ) + ((f.2.2.2.2 * (-z.σ * r) : ℚ) : ℝ) * K + ((f.2.1.2.2 * r : ℚ) : ℝ) * S)

/-- The same with `S` and `K` enclosed. -/
def Zone.atI (z : Zone) (y w : ℚ) (S K : QI) : VI :=
  let f := z.frame y
  let c := f.1.add (f.2.2.smul (z.σ * z.ρ))
  let r := z.ρ + z.σ * w
  (((QI.pt c.1).add ((QI.pt (f.2.2.1 * (-z.σ * r))).mul K)).add ((QI.pt (f.2.1.1 * r)).mul S),
   ((QI.pt c.2.1).add ((QI.pt (f.2.2.2.1 * (-z.σ * r))).mul K)).add ((QI.pt (f.2.1.2.1 * r)).mul S),
   ((QI.pt c.2.2).add ((QI.pt (f.2.2.2.2 * (-z.σ * r))).mul K)).add ((QI.pt (f.2.1.2.2 * r)).mul S))

theorem Zone.atI_mem (z : Zone) (y w : ℚ) {S K : QI} {s k : ℝ} (hs : S.mem s) (hk : K.mem k) :
    (z.atI y w S K).mem (z.atR y w s k) :=
  ⟨mem_add (mem_add (Trig.mem_pt _) (mem_mul (Trig.mem_pt _) hk)) (mem_mul (Trig.mem_pt _) hs),
   mem_add (mem_add (Trig.mem_pt _) (mem_mul (Trig.mem_pt _) hk)) (mem_mul (Trig.mem_pt _) hs),
   mem_add (mem_add (Trig.mem_pt _) (mem_mul (Trig.mem_pt _) hk)) (mem_mul (Trig.mem_pt _) hs)⟩

/-- The zone's fold at `(u, y, w)`, enclosed; `none` if the angle is past the enclosure's range. -/
def Zone.at (z : Zone) (u y w : ℚ) : Option VI :=
  (sc (u / z.rn)).map fun r => z.atI y w r.1 r.2

theorem Zone.at_mem (z : Zone) {u y w : ℚ} {I : VI} (h : z.at u y w = some I) :
    I.mem (z.atR y w (Real.sin ((u / z.rn : ℚ) : ℝ)) (Real.cos ((u / z.rn : ℚ) : ℝ))) := by
  unfold Zone.at at h
  cases hs : sc (u / z.rn) with
  | none => rw [hs] at h; cases h
  | some r =>
      rw [hs] at h; cases h
      exact z.atI_mem y w (sc_mem hs).1 (sc_mem hs).2

/-! ## The conditions -/

/-- **7**: `κ ∈ [0, 1]`, exactly. -/
def kappa (t : ℚ) (b : BendD) : Verdict :=
  let θ := |b.angle|
  if b.R * θ ≤ b.BA ∧ b.BA ≤ (b.R + t) * θ then .pass
  else .fail s!"bend {b.joint}: κ outside [0, 1]"

theorem kappa_sound (t : ℚ) (b : BendD) (h : kappa t b = .pass) :
    b.R * |b.angle| ≤ b.BA ∧ b.BA ≤ (b.R + t) * |b.angle| := by
  unfold kappa at h; dsimp only at h; split_ifs at h with h1; exact h1

/-- `|D| ≤ E`, three-valued, for `D` and `E` enclosed. -/
def decide2 (D E : QI) (joint : ℕ) : Verdict :=
  if 0 < E.1 ∧ -E.1 ≤ D.1 ∧ D.2 ≤ E.1 then .pass
  else if D.1 > E.2 ∨ D.2 < -E.2 then .fail s!"bend {joint}: setback off"
  else .undecided s!"bend {joint}: setback undecided"

theorem decide2_pass {D E : QI} {j : ℕ} (h : decide2 D E j = .pass) :
    0 < E.1 ∧ -E.1 ≤ D.1 ∧ D.2 ≤ E.1 := by
  unfold decide2 at h; split_ifs at h with h1 h2 <;> first | exact h1 | cases h

/-- **3/4**: the setback is `ρ sin θ / (1 + cos θ)` within `tol`, decided as
`|s (1 + cos θ) − ρ sin θ| ≤ tol (1 + cos θ)` (no division). -/
def setback (t tol : ℚ) (b : BendD) : Verdict :=
  let θ := |b.angle|
  let ρ := if b.angle > 0 then b.R else b.R + t
  match sc θ with
  | none => .undecided s!"bend {b.joint}: angle out of range"
  | some (S, C) =>
    let opc := (QI.pt 1).add C
    decide2 (((QI.pt b.s).mul opc).sub ((QI.pt ρ).mul S)) ((QI.pt tol).mul opc) b.joint

theorem setback_sound (t tol : ℚ) (b : BendD) (h : setback t tol b = .pass) :
    let θ : ℝ := |b.angle|
    let ρ : ℝ := if b.angle > 0 then b.R else b.R + t
    0 < 1 + Real.cos θ ∧
      |(b.s : ℝ) - ρ * Real.sin θ / (1 + Real.cos θ)| ≤ tol := by
  intro θ ρ
  unfold setback at h
  dsimp only at h
  cases hs : sc |b.angle| with
  | none => rw [hs] at h; cases h
  | some r =>
    obtain ⟨S, C⟩ := r
    rw [hs] at h
    dsimp only at h
    obtain ⟨mS, mC⟩ := sc_mem hs
    have hρ : ((if b.angle > 0 then b.R else b.R + t : ℚ) : ℝ) = ρ := by
      simp only [ρ]; split_ifs <;> push_cast <;> rfl
    have hθ : ((|b.angle| : ℚ) : ℝ) = θ := by simp [θ]
    rw [hθ] at mS mC
    have opc := mem_add (mem_pt 1) mC
    have D := mem_sub (mem_mul (mem_pt b.s) opc) (mem_mul (mem_pt (if b.angle > 0 then b.R else b.R + t)) mS)
    have E := mem_mul (mem_pt tol) opc
    rw [hρ] at D
    obtain ⟨e0, e1, e2⟩ := decide2_pass h
    simp only [QI.mem] at D E
    push_cast at D E
    generalize ((QI.pt b.s).mul ((QI.pt 1).add C)).sub
      ((QI.pt (if b.angle > 0 then b.R else b.R + t)).mul S) = Dq at D e1 e2
    generalize (QI.pt tol).mul ((QI.pt 1).add C) = Eq at E e0 e1 e2
    obtain ⟨D1, D2⟩ := D
    obtain ⟨E1, E2⟩ := E
    have e0' : (0 : ℝ) < Eq.1 := by exact_mod_cast e0
    have e1' : -(Eq.1 : ℝ) ≤ Dq.1 := by exact_mod_cast e1
    have e2' : (Dq.2 : ℝ) ≤ Eq.1 := by exact_mod_cast e2
    have hE : (0 : ℝ) < tol * (1 + Real.cos θ) := by linarith
    have hc0 : 0 ≤ 1 + Real.cos θ := by linarith [Real.neg_one_le_cos θ]
    have hpos : 0 < 1 + Real.cos θ := by
      rcases hc0.lt_or_eq with h' | h'
      · exact h'
      · rw [← h', mul_zero] at hE; exact absurd hE (lt_irrefl 0)
    refine ⟨hpos, ?_⟩
    have e : (b.s : ℝ) - ρ * Real.sin θ / (1 + Real.cos θ) =
        ((b.s : ℝ) * (1 + Real.cos θ) - ρ * Real.sin θ) / (1 + Real.cos θ) := by
      field_simp
    rw [e, abs_le, le_div_iff₀ hpos, div_le_iff₀ hpos]
    constructor <;> nlinarith

/-- **5**: at `u = 0` and `u = BA`, `y = 0` and `y = len`, `w = 0` and `w = t`, the zone's fold
is wall a's and wall b's. -/
def continuity (t tol2 : ℚ) (z : Zone) (fb : Frame) (db : Place) (joint : ℕ) : Verdict :=
  all <| [0, z.len].flatMap fun y => [0, t].flatMap fun w =>
    [(0, z.fa, z.da), (z.ba, fb, db)].map fun (u, f, d) =>
      match z.at u y w with
      | none => .undecided s!"bend {joint}: angle out of range"
      | some I => near I (wallAt f d (z.dev.apply (u, y)) w) tol2 s!"bend {joint}: zone off its wall"

/-- A rational as a float, for messages. -/
def fl (q : ℚ) : Float := Float.ofInt q.num / Float.ofNat q.den

/-- **1**: a flat vertex, developed to `q` at depth `w`, folds (by some wall, or some zone whose
`u` range holds it) to within `tol` of `v`. -/
def refoldVertex (tol2 : ℚ) (walls : List WallD) (zones : List Zone) (q : V2) (w : ℚ) (v : V3) :
    Verdict :=
  let cands : List Verdict :=
    walls.map (fun wd => near (VI.pt (wallAt wd.fold wd.dev q w)) v tol2 "wall") ++
    zones.filterMap fun z =>
      let l := z.dev.local q
      if 0 ≤ l.1 ∧ l.1 ≤ z.ba then
        some ((z.at l.1 l.2 w).elim (.undecided "angle out of range") fun I => near I v tol2 "zone")
      else none
  if cands.any (· == .pass) then .pass
  else if cands.all fun c => match c with | .fail _ => true | _ => false then
    .fail s!"the flat vertex developed to ({fl q.1}, {fl q.2}) at depth {fl w} refolds off its namesake ({fl v.1}, {fl v.2.1}, {fl v.2.2})"
  else .undecided "a flat vertex refolds undecided"

/-- **The refold of a vertex is proven when it passes**: some wall or zone of the published
layout carries the developed point to within `tol` of the folded vertex. -/
theorem refoldVertex_sound (tol2 : ℚ) (walls : List WallD) (zones : List Zone) (q : V2) (w : ℚ)
    (v : V3) (h : refoldVertex tol2 walls zones q w v = .pass) :
    (∃ wd ∈ walls, dist2R (wallAt wd.fold wd.dev q w).toR v.toR ≤ tol2) ∨
    (∃ z ∈ zones, 0 ≤ (z.dev.local q).1 ∧ (z.dev.local q).1 ≤ z.ba ∧
      dist2R (z.atR (z.dev.local q).2 w (Real.sin (((z.dev.local q).1 / z.rn : ℚ) : ℝ))
        (Real.cos (((z.dev.local q).1 / z.rn : ℚ) : ℝ))) v.toR ≤ tol2) := by
  unfold refoldVertex at h
  dsimp only at h
  split_ifs at h with h1
  obtain ⟨c, hc, hpass⟩ := List.any_eq_true.mp h1
  have hpass : c = .pass := by simpa using hpass
  rcases List.mem_append.mp hc with hc | hc
  · obtain ⟨wd, hwd, rfl⟩ := List.mem_map.mp hc
    have := (near_sound (VI.mem_pt (wallAt wd.fold wd.dev q w)) v tol2 "wall").1 hpass
    exact Or.inl ⟨wd, hwd, this⟩
  · obtain ⟨z, hz, hfm⟩ := List.mem_filterMap.mp hc
    split_ifs at hfm with hu
    cases hat : z.at (z.dev.local q).1 (z.dev.local q).2 w with
    | none => rw [hat] at hfm; simp at hfm; rw [← hfm] at hpass; cases hpass
    | some I =>
      rw [hat] at hfm; simp at hfm
      rw [← hfm] at hpass
      have := (near_sound (z.at_mem hat) v tol2 "zone").1 hpass
      exact Or.inr ⟨z, hz, hu.1, hu.2, this⟩

/-- The whole check. -/
def check (inp : Input) : List (String × Verdict) :=
  let size2 := inp.verts.foldl (fun m p => max m (p.2.dot p.2)) 1
  let tol2 := size2 / 10 ^ 18
  -- |v| ≤ |x| + |y| + |z|: a rational bound of refold.rs's size, for the one-dimensional checks
  let size1 := inp.verts.foldl (fun m p => max m (|p.2.1| + |p.2.2.1| + |p.2.2.2|)) 1
  let tol := size1 / 10 ^ 9
  let wallOf (i : ℕ) := inp.walls.find? (·.id == i)
  let zones := inp.bends.filterMap fun b =>
    (wallOf b.a).map fun wa => (b, Zone.ofBend inp.t b wa.fold wa.dev)
  let kap := inp.bends.map fun b => kappa inp.t b
  let sb := inp.bends.map fun b =>
    if inp.hems.contains b.joint then
      (if |b.s - (b.R + inp.t)| ≤ tol then .pass else .fail s!"hem {b.joint}: setback is not R + t")
    else setback inp.t tol b
  let cont := zones.map fun (b, z) =>
    match wallOf b.b with
    | none => .fail s!"bend {b.joint}: no wall b"
    | some wb => continuity inp.t tol2 z wb.fold wb.dev b.joint
  let ref := inp.verts.map fun (f, v) =>
    let p := inp.flat0.toLocal f
    let q := inp.dev0.apply (p.1, p.2.1)
    refoldVertex tol2 inp.walls (zones.map (·.2)) q (-p.2.2) v
  [("7 kappa", all kap), ("3/4 setback", all sb), ("5 continuity", all cont),
    ("1 refold", all ref)]

end Sheet.Checker
