import ForgeExact.Flat

/-!
# The one tolerance rule (research/kernel-audit/README.md §6.1), executable and proven

The reference applies this to the dyadic inputs before the exact boolean. These are the very
functions the executable runs.

1. **Carriers** (`Carrier`: a face's plane, its box and its name). Two carriers are
   tolerance-equal (`tolEq`) iff the region where their faces' boxes, grown by LINEAR, overlap is
   not empty and each plane is within LINEAR of the other at the projection onto it of every
   corner of `R`, the hull of those grown boxes (`extent`: over the overlap alone, two faces
   meeting at an edge would always be equal). (The affine distance to `T` is largest over `S ∩ R` at a vertex, and the
   projected corners span a superset of `S ∩ R`: the test is conservative.)
2. **Classes** (`cls`): the transitive closure of `tolEq` over the carriers, which is what a
   union–find over the decided pairs computes. `cls_spec`: a carrier is in `a`'s class iff it is
   connected to `a` by a chain of tolerance-equal pairs.
3. **Representative** (`rep`): the least-named carrier of the class (names are (body, face),
   invariant under motion). `rep_order`: independent of the order the carriers are listed in.
   `tolEq_iso`, `rep_iso`: independent of the coordinates, for every exact symmetry of ℚ³ that
   keeps axis boxes axis boxes (translations, mirrors, axis swaps and their compositions).
4. **Spread cap** (`spreadOk`): every pair in a class whose region is not empty is within
   `K · LINEAR`; otherwise the verdict is `ambiguous`, naming the class.
5. **Decisions in the representative** (`snapped`, `onIn`): once two surfaces are merged, every
   on/near decision about their entities is made in the representative surface, never across
   the gap; the gap (at most `K · LINEAR`, `gap_carried`) is carried as the affected entities'
   tolerance. `onIn_class`: decisions made this way agree across a class (consistency).
6. **Vertices recomputed exactly** (`vertexAt`) from the snapped carriers round them: either every
   one of them holds the vertex exactly (`vertexAt_on`, so the ledger is closed: a vertex of a face
   is on every carrier tolerance-equal to that face) or the verdict is `ambiguous`.

`snap_result`: the rule always ends in a consistent configuration or in `ambiguous` with its
reason; never anything else.
-/

namespace ForgeExact.Snap

open Ref

/-! ## Rational order facts -/

theorem qmax_comm (a b : Rat) : qmax a b = qmax b a := by unfold qmax; split <;> split <;> grind
theorem qmin_comm (a b : Rat) : qmin a b = qmin b a := by unfold qmin; split <;> split <;> grind

/-! ## Boxes -/

def _root_.ForgeExact.Ref.Box.inter (a b : Box) : Box :=
  ⟨⟨qmax a.lo.x b.lo.x, qmax a.lo.y b.lo.y, qmax a.lo.z b.lo.z⟩,
   ⟨qmin a.hi.x b.hi.x, qmin a.hi.y b.hi.y, qmin a.hi.z b.hi.z⟩⟩

theorem _root_.ForgeExact.Ref.Box.inter_comm (a b : Box) : a.inter b = b.inter a := by
  simp only [Box.inter, qmax_comm a.lo.x, qmax_comm a.lo.y, qmax_comm a.lo.z,
    qmin_comm a.hi.x, qmin_comm a.hi.y, qmin_comm a.hi.z]

/-- The least box holding both. -/
def _root_.ForgeExact.Ref.Box.hull (a b : Box) : Box :=
  ⟨⟨qmin a.lo.x b.lo.x, qmin a.lo.y b.lo.y, qmin a.lo.z b.lo.z⟩,
   ⟨qmax a.hi.x b.hi.x, qmax a.hi.y b.hi.y, qmax a.hi.z b.hi.z⟩⟩

theorem _root_.ForgeExact.Ref.Box.hull_comm (a b : Box) : a.hull b = b.hull a := by
  simp only [Box.hull, qmax_comm a.hi.x, qmax_comm a.hi.y, qmax_comm a.hi.z,
    qmin_comm a.lo.x, qmin_comm a.lo.y, qmin_comm a.lo.z]

def _root_.ForgeExact.Ref.Box.nonempty (b : Box) : Bool := b.lo.x ≤ b.hi.x && b.lo.y ≤ b.hi.y && b.lo.z ≤ b.hi.z

/-- The eight corners. -/
def _root_.ForgeExact.Ref.Box.corners (b : Box) : List V3 :=
  [b.lo.x, b.hi.x].flatMap fun x => [b.lo.y, b.hi.y].flatMap fun y =>
    [b.lo.z, b.hi.z].map fun z => ⟨x, y, z⟩

theorem _root_.ForgeExact.Ref.Box.mem_corners (b : Box) (c : V3) :
    c ∈ b.corners ↔ (c.x = b.lo.x ∨ c.x = b.hi.x) ∧ (c.y = b.lo.y ∨ c.y = b.hi.y) ∧
      (c.z = b.lo.z ∨ c.z = b.hi.z) := by
  obtain ⟨x, y, z⟩ := c
  simp only [Box.corners, List.mem_flatMap, List.mem_map, List.mem_cons, List.mem_nil_iff,
    or_false, V3.mk.injEq]
  constructor
  · rintro ⟨x', hx, y', hy, z', hz, rfl, rfl, rfl⟩; exact ⟨hx, hy, hz⟩
  · rintro ⟨hx, hy, hz⟩; exact ⟨x, hx, y, hy, z, hz, rfl, rfl, rfl⟩

/-! ## Carriers and the relation -/

/-- A face's carrier as the rule sees it: its name (body, face), plane and face box. -/
structure Carrier where
  name : Nat × Nat
  plane : Plane
  box : Box
  deriving DecidableEq, Inhabited

/-- The orthogonal projection onto `S`. -/
def proj (S : Plane) (c : V3) : V3 := c - (S.eval c / S.n.norm2) * S.n

/-- `T` within `d` of `S` over `R`: at the projection onto `S` of each corner of `R`. -/
def near (S T : Plane) (R : Box) (d : Rat) : Bool := R.corners.all fun c => T.within (proj S c) d

/-- Where two carriers' faces' boxes, grown by `l`, overlap. -/
def region (l : Rat) (a b : Carrier) : Box := (a.box.grow l).inter (b.box.grow l)

/-- Both faces' extent: the hull of their boxes, grown by `l`. The planes are compared over this,
not over `region` alone: two faces meeting at an edge (at any angle) have a `region` that is a
slab `2 l` across round that edge, over which each plane is within `l` of the other, so a test
over `region` merges every face of a solid with its neighbours. Over the extent, two planes agree
only if they are within `l` across both faces: parallel to within `l` over the faces' size. -/
def extent (l : Rat) (a b : Carrier) : Box := (a.box.grow l).hull (b.box.grow l)

/-- Tolerance-equal: the faces' region is not empty (they meet, to within `l`) and each plane is
within `l` of the other over both faces' extent. -/
def tolEq (l : Rat) (a b : Carrier) : Bool :=
  (region l a b).nonempty && near a.plane b.plane (extent l a b) l &&
    near b.plane a.plane (extent l a b) l

theorem region_comm (l : Rat) (a b : Carrier) : region l a b = region l b a := Box.inter_comm _ _

theorem extent_comm (l : Rat) (a b : Carrier) : extent l a b = extent l b a := Box.hull_comm _ _

theorem tolEq_symm (l : Rat) (a b : Carrier) : tolEq l a b = tolEq l b a := by
  unfold tolEq; rw [region_comm, extent_comm]
  cases (region l b a).nonempty <;> cases near a.plane b.plane (extent l b a) l <;>
    cases near b.plane a.plane (extent l b a) l <;> rfl

/-! ## Classes: the transitive closure -/

/-- Connected by a chain of tolerance-equal carriers of the list. -/
inductive Conn (l : Rat) (cs : List Carrier) (a : Carrier) : Carrier → Prop
  | refl : Conn l cs a a
  | step {b c : Carrier} : Conn l cs a b → c ∈ cs → tolEq l b c = true → Conn l cs a c

/-- One round: everything tolerance-equal to the set so far, not yet in it. -/
def closeStep (l : Rat) (cs s : List Carrier) : List Carrier :=
  s ++ cs.filter fun b => !s.contains b && s.any fun a => tolEq l a b

def closeN (l : Rat) (cs : List Carrier) : Nat → List Carrier → List Carrier
  | 0, s => s
  | n + 1, s => if (closeStep l cs s).length = s.length then s else closeN l cs n (closeStep l cs s)

/-- The class of `a`. -/
def cls (l : Rat) (cs : List Carrier) (a : Carrier) : List Carrier :=
  closeN l cs (cs.length + 1) [a]

/-- A set closed under the relation within `cs`. -/
def Closed (l : Rat) (cs s : List Carrier) : Prop :=
  ∀ x ∈ s, ∀ c ∈ cs, tolEq l x c = true → c ∈ s

theorem closed_of_fix {l : Rat} {cs s : List Carrier}
    (h : (closeStep l cs s).length = s.length) : Closed l cs s := by
  intro x hx c hc hxc
  apply Classical.byContradiction
  intro hcs
  have hnil : (cs.filter fun b => !s.contains b && s.any fun a => tolEq l a b) = [] := by
    simp only [closeStep, List.length_append] at h
    exact List.length_eq_zero_iff.mp (by omega)
  have : c ∈ cs.filter fun b => !s.contains b && s.any fun a => tolEq l a b := by
    simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.any_eq_true]
    exact ⟨hc, by simp [hcs], x, hx, hxc⟩
  rw [hnil] at this; exact nomatch this

theorem closeStep_sound {l : Rat} {cs : List Carrier} {a : Carrier} {s : List Carrier}
    (hs : ∀ x ∈ s, Conn l cs a x) : ∀ x ∈ closeStep l cs s, Conn l cs a x := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter, Bool.and_eq_true, List.any_eq_true] at hx
  rcases hx with hx | ⟨hc, _, y, hy, hyx⟩
  · exact hs x hx
  · exact Conn.step (hs y hy) hc hyx

theorem closeStep_nodup {l : Rat} {cs s : List Carrier} (hcs : cs.Nodup) (hs : s.Nodup) :
    (closeStep l cs s).Nodup := by
  unfold closeStep
  rw [List.nodup_append]
  refine ⟨hs, hcs.filter _, fun x hx y hy => ?_⟩
  simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
    decide_eq_false_iff_not] at hy
  rintro rfl; exact hy.2.1 hx

theorem closeStep_sub {l : Rat} {cs s : List Carrier} (hs : s ⊆ cs) : closeStep l cs s ⊆ cs := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter] at hx
  rcases hx with hx | ⟨hx, _⟩
  · exact hs hx
  · exact hx

/-- With fuel past the list's length the rounds reach a closed set, holding only connected
carriers. -/
theorem closeN_spec {l : Rat} {cs : List Carrier} {a : Carrier} (hcs : cs.Nodup) :
    ∀ (n : Nat) (s : List Carrier), s.Nodup → s ⊆ cs → cs.length < s.length + n →
      (∀ x ∈ s, Conn l cs a x) → a ∈ s →
      Closed l cs (closeN l cs n s) ∧ (∀ x ∈ closeN l cs n s, Conn l cs a x) ∧ a ∈ closeN l cs n s
  | 0, s, hn, hsub, hlen, _, _ => by
    have := List.Nodup.length_le_of_subset hn hsub; omega
  | n + 1, s, hn, hsub, hlen, hconn, ha => by
    simp only [closeN]
    split
    · exact ⟨closed_of_fix (by assumption), hconn, ha⟩
    · rename_i hne
      have hgrow : s.length + 1 ≤ (closeStep l cs s).length := by
        simp only [closeStep, List.length_append] at hne ⊢; omega
      exact closeN_spec hcs n _ (closeStep_nodup hcs hn) (closeStep_sub hsub) (by omega)
        (closeStep_sound hconn) (List.mem_append_left _ ha)

/-- **The classes are the transitive closure.** -/
theorem cls_spec (l : Rat) {cs : List Carrier} (hcs : cs.Nodup) {a : Carrier} (ha : a ∈ cs)
    (b : Carrier) : b ∈ cls l cs a ↔ Conn l cs a b := by
  obtain ⟨hcl, hsound, hmem⟩ := closeN_spec (l := l) (a := a) hcs (cs.length + 1) [a]
    (List.nodup_cons.mpr ⟨by simp, List.nodup_nil⟩) (fun x hx => by simp at hx; exact hx ▸ ha) (by simp; omega)
    (fun x hx => by simp at hx; exact hx ▸ Conn.refl) (List.mem_singleton_self a)
  refine ⟨hsound b, fun h => ?_⟩
  induction h with
  | refl => exact hmem
  | step _ hc hbc ih => exact hcl _ ih _ hc hbc

theorem conn_mem {l : Rat} {cs : List Carrier} {a b : Carrier} (ha : a ∈ cs) (h : Conn l cs a b) :
    b ∈ cs := by
  induction h with
  | refl => exact ha
  | step _ hc _ _ => exact hc

theorem conn_symm {l : Rat} {cs : List Carrier} {a b : Carrier} (ha : a ∈ cs) (h : Conn l cs a b) :
    Conn l cs b a := by
  induction h with
  | refl => exact Conn.refl
  | @step b c hab hc hbc ih =>
    -- c ~ b, then b ⇝ a
    have hcb : Conn l cs c b := Conn.step Conn.refl (conn_mem ha hab) (by rw [tolEq_symm]; exact hbc)
    exact conn_trans hcb ih
where
  conn_trans {x y z : Carrier} (h1 : Conn l cs x y) (h2 : Conn l cs y z) : Conn l cs x z := by
    induction h2 with
    | refl => exact h1
    | step _ hc hbc ih => exact Conn.step ih hc hbc

theorem conn_trans {l : Rat} {cs : List Carrier} {x y z : Carrier} (h1 : Conn l cs x y)
    (h2 : Conn l cs y z) : Conn l cs x z := conn_symm.conn_trans h1 h2

/-! ## The representative: the least name -/

/-- Lexicographic order on names. -/
def nameLt (p q : Nat × Nat) : Bool := p.1 < q.1 || (p.1 == q.1 && p.2 < q.2)

theorem nameLt_total (p q : Nat × Nat) (h1 : nameLt p q = false) (h2 : nameLt q p = false) :
    p = q := by
  obtain ⟨a, b⟩ := p; obtain ⟨c, d⟩ := q
  simp only [nameLt, Bool.or_eq_false_iff, decide_eq_false_iff_not, Bool.and_eq_false_imp,
    beq_iff_eq] at h1 h2
  have : a = c := by omega
  subst this
  have := h1.2 rfl; have := h2.2 rfl
  simp only [Prod.mk.injEq, true_and]; omega

theorem nameLt_trans {p q r : Nat × Nat} (h1 : nameLt p q = true) (h2 : nameLt q r = true) :
    nameLt p r = true := by
  obtain ⟨a, b⟩ := p; obtain ⟨c, d⟩ := q; obtain ⟨e, f⟩ := r
  simp only [nameLt, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true, beq_iff_eq] at *
  omega

def leastName (a : Carrier) (s : List Carrier) : Carrier :=
  s.foldl (fun m b => if nameLt b.name m.name then b else m) a

theorem nameLt_irrefl (p : Nat × Nat) : nameLt p p = false := by
  obtain ⟨a, b⟩ := p; simp [nameLt]

theorem nameLt_tri (p q : Nat × Nat) (h : nameLt p q = false) : nameLt q p = true ∨ p = q := by
  cases h' : nameLt q p
  · exact Or.inr (nameLt_total p q h h')
  · exact Or.inl rfl

theorem leastName_spec : ∀ (s : List Carrier) (m : Carrier),
    (leastName m s = m ∨ leastName m s ∈ s) ∧ nameLt m.name (leastName m s).name = false ∧
      ∀ b ∈ s, nameLt b.name (leastName m s).name = false
  | [], m => ⟨Or.inl rfl, nameLt_irrefl _, fun _ h => nomatch h⟩
  | b :: s, m => by
    have e : leastName m (b :: s) = leastName (if nameLt b.name m.name then b else m) s := rfl
    rw [e]
    by_cases hb : nameLt b.name m.name = true
    · simp only [hb, ite_true]
      obtain ⟨h1, h2, h3⟩ := leastName_spec s b
      refine ⟨Or.inr (h1.elim (fun h => by rw [h]; exact List.mem_cons_self ..) (List.mem_cons_of_mem _)), ?_, ?_⟩
      · cases hm : nameLt m.name (leastName b s).name
        · rfl
        · rw [nameLt_trans hb hm] at h2; cases h2
      · intro c hc
        rcases List.mem_cons.mp hc with rfl | hc
        · exact h2
        · exact h3 c hc
    · simp only [Bool.not_eq_true] at hb
      simp only [hb, Bool.false_eq_true, ite_false]
      obtain ⟨h1, h2, h3⟩ := leastName_spec s m
      refine ⟨h1.elim Or.inl (fun h => Or.inr (List.mem_cons_of_mem _ h)), h2, ?_⟩
      intro c hc
      rcases List.mem_cons.mp hc with rfl | hc
      · cases hcr : nameLt c.name (leastName m s).name
        · rfl
        · rcases nameLt_tri _ _ h2 with hrm | hrm
          · rw [nameLt_trans hcr hrm] at hb; cases hb
          · rw [← hrm] at hcr; rw [hcr] at hb; cases hb
      · exact h3 c hc

/-- The representative: the least-named carrier of the class. -/
def rep (l : Rat) (cs : List Carrier) (a : Carrier) : Carrier := leastName a (cls l cs a)

theorem rep_mem (l : Rat) {cs : List Carrier} (hcs : cs.Nodup) {a : Carrier} (ha : a ∈ cs) :
    Conn l cs a (rep l cs a) := by
  rcases (leastName_spec (cls l cs a) a).1 with h | h
  · unfold rep; rw [h]; exact Conn.refl
  · exact (cls_spec l hcs ha _).mp h

theorem rep_least (l : Rat) {cs : List Carrier} (hcs : cs.Nodup) {a : Carrier} (ha : a ∈ cs)
    (b : Carrier) (hb : Conn l cs a b) : nameLt b.name (rep l cs a).name = false :=
  (leastName_spec (cls l cs a) a).2.2 b ((cls_spec l hcs ha b).mpr hb)

theorem inj_of_nodup_map {α β : Type} {f : α → β} : ∀ {l : List α}, (l.map f).Nodup →
    ∀ {x y : α}, x ∈ l → y ∈ l → f x = f y → x = y
  | [], _, _, _, hx, _, _ => nomatch hx
  | a :: l, h, x, y, hx, hy, hxy => by
    rw [List.map_cons, List.nodup_cons] at h
    have hnot : ∀ z ∈ l, f z ≠ f a := fun z hz e => h.1 (e ▸ List.mem_map_of_mem hz)
    rcases List.mem_cons.mp hx with hxa | hx' <;> rcases List.mem_cons.mp hy with hya | hy'
    · rw [hxa, hya]
    · subst hxa; exact absurd hxy.symm (hnot y hy')
    · subst hya; exact absurd hxy (hnot x hx')
    · exact inj_of_nodup_map h.2 hx' hy' hxy

/-- The representative is the one carrier of the class that no member's name precedes: it is
fixed by the class and the names alone. -/
theorem rep_unique (l : Rat) {cs : List Carrier} (hcs : cs.Nodup)
    (hnames : (cs.map Carrier.name).Nodup) {a r : Carrier} (ha : a ∈ cs)
    (hr : Conn l cs a r) (hmin : ∀ b, Conn l cs a b → nameLt b.name r.name = false) :
    rep l cs a = r := by
  have h1 := rep_least l hcs ha r hr
  have h2 := hmin _ (rep_mem l hcs ha)
  have hn := nameLt_total _ _ h1 h2
  have hm1 := conn_mem ha (rep_mem l hcs ha)
  have hm2 := conn_mem ha hr
  exact inj_of_nodup_map hnames hm1 hm2 hn.symm

/-- Tolerance-equal pairs and connection depend on the list only through its members. -/
theorem conn_congr {l : Rat} {cs cs' : List Carrier} (h : ∀ x, x ∈ cs ↔ x ∈ cs') {a b : Carrier}
    (hab : Conn l cs a b) : Conn l cs' a b := by
  induction hab with
  | refl => exact Conn.refl
  | step _ hc hbc ih => exact Conn.step ih ((h _).mp hc) hbc

/-- **Independent of input order.** -/
theorem rep_order (l : Rat) {cs cs' : List Carrier} (hp : cs.Perm cs') (hcs : cs.Nodup)
    (hnames : (cs.map Carrier.name).Nodup) {a : Carrier} (ha : a ∈ cs) :
    rep l cs' a = rep l cs a := by
  have hm : ∀ x, x ∈ cs' ↔ x ∈ cs := fun x => hp.mem_iff.symm
  have hcs' := hp.nodup_iff.mp hcs
  have hn' := (hp.map Carrier.name).nodup_iff.mp hnames
  have ha' := hp.mem_iff.mp ha
  apply rep_unique l hcs' hn' ha' (conn_congr (fun x => (hm x).symm) (rep_mem l hcs ha))
  intro b hb
  exact rep_least l hcs ha b (conn_congr hm hb)

/-- Members of one class have one representative. -/
theorem rep_class (l : Rat) {cs : List Carrier} (hcs : cs.Nodup)
    (hnames : (cs.map Carrier.name).Nodup) {a b : Carrier} (ha : a ∈ cs) (hab : Conn l cs a b) :
    rep l cs b = rep l cs a := by
  have hb := conn_mem ha hab
  apply rep_unique l hcs hnames hb (conn_trans (conn_symm ha hab) (rep_mem l hcs ha))
  intro c hc
  exact rep_least l hcs ha c (conn_trans hab hc)

/-! ## Decisions in the representative -/

/-- The carrier `a` is decided in: its representative's plane, in `a`'s own sense. -/
def snapped (l : Rat) (cs : List Carrier) (a : Carrier) : Plane :=
  if a.plane.n.dot (rep l cs a).plane.n < 0 then
    ⟨(-1 : Rat) * (rep l cs a).plane.n, -(rep l cs a).plane.c⟩
  else (rep l cs a).plane

theorem snapped_eval (l : Rat) (cs : List Carrier) (a : Carrier) (x : V3) :
    (snapped l cs a).eval x = (rep l cs a).plane.eval x ∨
      (snapped l cs a).eval x = -(rep l cs a).plane.eval x := by
  unfold snapped; split
  · right
    simp only [Plane.eval, V3.dot, Flat.smul_x, Flat.smul_y, Flat.smul_z]; grind
  · left; rfl

/-- On/near decided once, in the representative: is `x` on `a`'s carrier? -/
def onIn (l : Rat) (cs : List Carrier) (a : Carrier) (x : V3) : Bool :=
  (snapped l cs a).eval x == 0

theorem onIn_rep (l : Rat) (cs : List Carrier) (a : Carrier) (x : V3) :
    onIn l cs a x = ((rep l cs a).plane.eval x == 0) := by
  unfold onIn
  rcases snapped_eval l cs a x with h | h <;> rw [h]
  cases hq : (rep l cs a).plane.eval x == 0
  · have : (rep l cs a).plane.eval x ≠ 0 := by simpa using hq
    have : -(rep l cs a).plane.eval x ≠ 0 := by grind
    simpa using this
  · have : (rep l cs a).plane.eval x = 0 := by simpa using hq
    simp [this]

/-- **Decisions made in the representative agree across a class**: for tolerance-equal (chained)
carriers, whether a point is on them is one answer. -/
theorem onIn_class (l : Rat) {cs : List Carrier} (hcs : cs.Nodup)
    (hnames : (cs.map Carrier.name).Nodup) {a b : Carrier} (ha : a ∈ cs) (hab : Conn l cs a b)
    (x : V3) : onIn l cs a x = onIn l cs b x := by
  rw [onIn_rep, onIn_rep, rep_class l hcs hnames ha hab]

/-! ## The spread cap -/

/-- Every pair of a class whose region is not empty is within `k · l` over their extent. -/
def spreadOk (l k : Rat) (cs : List Carrier) : Bool :=
  cs.all fun a => (cls l cs a).all fun b =>
    !(region l a b).nonempty ||
      (near a.plane b.plane (extent l a b) (k * l) && near b.plane a.plane (extent l a b) (k * l))

/-- **The gap is carried**: past the cap check, each carrier's own plane is within `k · l` of the
representative's over their extent, so the entities on it carry at most that tolerance. -/
theorem gap_carried (l k : Rat) {cs : List Carrier} (hcs : cs.Nodup) (h : spreadOk l k cs = true)
    {a : Carrier} (ha : a ∈ cs) (hR : (region l a (rep l cs a)).nonempty = true) :
    near a.plane (rep l cs a).plane (extent l a (rep l cs a)) (k * l) = true := by
  simp only [spreadOk, List.all_eq_true] at h
  have := h a ha (rep l cs a) ((cls_spec l hcs ha _).mpr (rep_mem l hcs ha))
  simp only [hR, Bool.not_true, Bool.false_or, Bool.and_eq_true] at this
  exact this.1

/-! ## Vertices recomputed exactly -/

/-- A vertex from the snapped carriers round it: the first independent three meet in a point,
and every carrier round it must hold that point exactly. -/
def vertexAt (ps : List Plane) : Option V3 :=
  match (ps.flatMap fun a => ps.flatMap fun b => ps.filterMap fun c => meet3 a b c).head? with
  | none => none
  | some p => if ps.all (fun q => q.eval p == 0) then some p else none

theorem vertexAt_on {ps : List Plane} {p : V3} (h : vertexAt ps = some p) :
    ∀ q ∈ ps, q.eval p = 0 := by
  unfold vertexAt at h
  split at h
  · cases h
  · split at h
    · rename_i hall
      cases h
      intro q hq
      have := List.all_eq_true.mp hall q hq
      simpa using this
    · cases h

/-! ## The whole rule -/

/-- A vertex as the rule sees it: its name and the carriers round it. -/
structure Corner where
  name : Nat × Nat
  round : List Carrier

/-- The rule's outcome: every carrier decided in its representative and every vertex recomputed
exactly, or `ambiguous` with the reason. -/
inductive Outcome
  | consistent (point : Corner → V3)
  | ambiguous (why : String)

def snapAll (l k : Rat) (cs : List Carrier) (vs : List Corner) : Outcome :=
  if !spreadOk l k cs then .ambiguous "a class of tolerance-equal surfaces spreads past the cap"
  else
    match vs.find? (fun v => (vertexAt (v.round.map (snapped l cs))).isNone) with
    | some v => .ambiguous s!"vertex {v.name} is not one point of its snapped surfaces"
    | none => .consistent fun v => (vertexAt (v.round.map (snapped l cs))).getD default

/-- **The rule is well defined**: it ends either `ambiguous`, or in a configuration where every
class is within the cap and every vertex lies exactly on the snapped carrier of every face round
it, so on every carrier tolerance-equal to those (the ledger is closed). -/
theorem snap_result (l k : Rat) (cs : List Carrier) (vs : List Corner)
    (hcs : cs.Nodup) (hnames : (cs.map Carrier.name).Nodup) :
    (∃ why, snapAll l k cs vs = .ambiguous why) ∨
    ∃ point, snapAll l k cs vs = .consistent point ∧ spreadOk l k cs = true ∧
      ∀ v ∈ vs, ∀ a ∈ v.round, a ∈ cs →
        onIn l cs a (point v) = true ∧
        ∀ b, Conn l cs a b → onIn l cs b (point v) = true := by
  unfold snapAll
  by_cases hs : spreadOk l k cs = true
  · simp only [hs, Bool.not_true, Bool.false_eq_true, ite_false]
    split
    · exact Or.inl ⟨_, rfl⟩
    · rename_i hfind
      refine Or.inr ⟨_, rfl, trivial, fun v hv a hav hacs => ?_⟩
      have hsome : (vertexAt (v.round.map (snapped l cs))).isSome := by
        have := List.find?_eq_none.mp hfind v hv
        cases h : vertexAt (v.round.map (snapped l cs)) <;> simp_all
      obtain ⟨p, hp⟩ := Option.isSome_iff_exists.mp hsome
      have hon : onIn l cs a p = true := by
        have := vertexAt_on hp (snapped l cs a) (List.mem_map_of_mem hav)
        simp [onIn, this]
      simp only [hp, Option.getD_some]
      exact ⟨hon, fun b hab => by rw [← onIn_class l hcs hnames hacs hab]; exact hon⟩
  · simp only [Bool.not_eq_true] at hs
    simp only [hs, Bool.not_false, ite_true]
    exact Or.inl ⟨_, rfl⟩

end ForgeExact.Snap
