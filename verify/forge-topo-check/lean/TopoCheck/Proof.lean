import TopoCheck.Spec

/-!
# `check` accepting a body implies it is valid

`check_sound`: if the combinatorial part of `check` finds no problem, the body is `Valid`, for
any corner table. `one_cycle`: with none, the rotation round each vertex by loop order is a
single cycle through all its leaving coedges.
-/

namespace TopoCheck.Body
variable (b : Body)

/-! ## Index arithmetic in a loop -/

theorem pred_idx {i n : Nat} (h : i < n) : (i + n - 1) % n = if i = 0 then n - 1 else i - 1 := by
  split
  · subst i; rw [Nat.zero_add]; exact Nat.mod_eq_of_lt (by omega)
  · have : i + n - 1 = (i - 1) + n := by omega
    rw [this, Nat.add_mod_right]; exact Nat.mod_eq_of_lt (by omega)

theorem pred_idx_lt {i n : Nat} (h : i < n) : (i + n - 1) % n < n := Nat.mod_lt _ (by omega)

theorem succ_pred_idx {i n : Nat} (h : i < n) : ((i + n - 1) % n + 1) % n = i := by
  rw [pred_idx h]
  split
  · subst i; rw [show n - 1 + 1 = n by omega, Nat.mod_self]
  · rw [show i - 1 + 1 = i by omega]; exact Nat.mod_eq_of_lt h

theorem pred_idx_inj {i j n : Nat} (hi : i < n) (hj : j < n)
    (h : (i + n - 1) % n = (j + n - 1) % n) : i = j := by
  rw [pred_idx hi, pred_idx hj] at h
  split at h <;> split at h <;> omega

theorem mem_loopCoedges {l c : Nat} :
    c ∈ b.loopCoedges l ↔ ∃ i < (b.lo l).len, c = (b.lo l).first + i := by
  unfold loopCoedges
  simp only [List.mem_map, List.mem_range]
  constructor
  · rintro ⟨i, hi, rfl⟩; exact ⟨i, hi, rfl⟩
  · rintro ⟨i, hi, rfl⟩; exact ⟨i, hi, rfl⟩

/-- `prev` of the `i`th coedge of loop `l` is its `(i + n - 1) % n`th. -/
theorem prev_eq {l i : Nat} (hl : (b.co ((b.lo l).first + i)).lp = l) :
    b.prev ((b.lo l).first + i) = (b.lo l).first + (i + (b.lo l).len - 1) % (b.lo l).len := by
  unfold prev; rw [hl]; simp only [Nat.add_sub_cancel_left]

/-! ## Reading the checks -/

theorem check_parts {corners} (h : b.check corners = []) :
    b.checkLinks = [] ∧ b.checkEdges = [] ∧ b.checkLoops = [] ∧ b.checkShellEuler = [] ∧
      b.checkFans corners = [] ∧ b.checkShellFaces = [] ∧ b.checkStrays = [] := by
  unfold check at h
  by_cases hl : b.checkLinks.isEmpty
  · have hl' : b.checkLinks = [] := List.isEmpty_iff.1 hl
    simp only [hl, Bool.not_true, Bool.false_eq_true, ↓reduceIte] at h
    simp only [List.append_eq_nil_iff] at h
    obtain ⟨⟨hm, hsf⟩, hst⟩ := h
    by_cases hb : (b.checkEdges ++ b.checkLoops).any blocks
    · rw [ite_eq_left hb] at hm
      rw [hm] at hb; simp at hb
    · rw [ite_eq_right hb] at hm
      simp only [List.append_eq_nil_iff] at hm
      exact ⟨hl', hm.1.1.1, hm.1.1.2, hm.1.2, hm.2, hsf, hst⟩
  · simp only [hl, Bool.not_false, ↓reduceIte] at h
    rw [h] at hl; simp at hl

theorem edges_ok (h : b.checkEdges = []) :
    ∀ e < b.edges.size, ∃ x y, (b.ed e).uses = [x, y] ∧ (b.co x).fwd ≠ (b.co y).fwd := by
  intro e he
  unfold checkEdges at h
  rw [List.flatMap_eq_nil_iff] at h
  have := h e (List.mem_range.2 he)
  revert this
  split
  · rename_i x y hxy
    intro hn
    refine ⟨x, y, hxy, ?_⟩
    intro heq
    rw [ite_eq_left (by simp [heq])] at hn
    exact List.cons_ne_nil _ _ hn
  · intro hn; exact absurd hn (List.cons_ne_nil _ _)

theorem loops_ok (h : b.checkLoops = []) :
    (∀ f < b.faces.size, b.faceLoops f ≠ []) ∧
    ∀ l < b.loops.size, 0 < (b.lo l).len ∧
      ∀ i < (b.lo l).len, b.stop ((b.lo l).first + i) = b.start ((b.lo l).first + (i + 1) % (b.lo l).len) := by
  unfold checkLoops at h
  rw [List.append_eq_nil_iff] at h
  obtain ⟨hf, hl⟩ := h
  constructor
  · intro f hfl
    rw [List.filterMap_eq_nil_iff] at hf
    have := hf f (List.mem_range.2 hfl)
    intro hne
    simp [hne] at this
  · intro l hll
    rw [List.flatMap_eq_nil_iff] at hl
    have := hl l (List.mem_range.2 hll)
    simp only at this
    by_cases h0 : (b.lo l).len = 0
    · simp [h0] at this
    · rw [ite_eq_right (by simpa using h0)] at this
      refine ⟨Nat.pos_of_ne_zero h0, fun i hi => ?_⟩
      rw [List.filterMap_eq_nil_iff] at this
      have := this i (List.mem_range.2 hi)
      by_cases hs : b.stop ((b.lo l).first + i) = b.start ((b.lo l).first + (i + 1) % (b.lo l).len)
      · exact hs
      · simp [hs] at this

theorem euler_ok (h : b.checkShellEuler = []) :
    ∀ s < b.shells.size, b.shellValue (b.shells.getD s []) % 2 = 0 ∧
      b.shellValue (b.shells.getD s []) ≤ 2 := by
  intro s hs
  unfold checkShellEuler at h
  rw [List.filterMap_eq_nil_iff] at h
  have := h s (List.mem_range.2 hs)
  dsimp only at this
  generalize b.shellValue (b.shells.getD s []) = x at this ⊢
  by_cases hc : (x % 2 != 0 || decide (x > 2)) = true
  · rw [ite_eq_left hc] at this; exact absurd this (by simp)
  · simp at hc; exact ⟨by omega, by omega⟩

/-! ## Partner across an edge -/

theorem partner_of_two {e : Nat} {x y : Nat} (hu : (b.ed e).uses = [x, y]) (hxy : x ≠ y)
    (hx : (b.co x).edge = e) (hy : (b.co y).edge = e) :
    b.partner x = some y ∧ b.partner y = some x := by
  unfold partner
  rw [hx, hy, hu]
  have h1 : (y != x) = true := by simp [Ne.symm hxy]
  have h2 : (x != y) = true := by simp [hxy]
  exact ⟨by simp [List.find?, h1], by simp [List.find?, h2]⟩

theorem oriented_of (wf : b.WF) (he : b.checkEdges = []) :
    ∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, ∃ d, b.partner c = some d ∧
      d ∈ (b.ed e).uses ∧ d ≠ c ∧ b.start d = b.stop c ∧ b.stop d = b.start c ∧ b.partner d = some c := by
  intro e hel c hc
  obtain ⟨x, y, hu, hf⟩ := b.edges_ok he e hel
  have hxy : x ≠ y := fun h => hf (by rw [h])
  have hx := wf.use_edge e hel x (by simp [hu])
  have hy := wf.use_edge e hel y (by simp [hu])
  obtain ⟨px, py⟩ := b.partner_of_two hu hxy hx hy
  have ends : ∀ p q, (b.co p).edge = e → (b.co q).edge = e → (b.co p).fwd ≠ (b.co q).fwd →
      b.start p = b.stop q ∧ b.stop p = b.start q := by
    intro p q hp hq hpq
    unfold start stop; rw [hp, hq]
    cases h1 : (b.co p).fwd <;> cases h2 : (b.co q).fwd <;> simp_all
  rw [hu] at hc
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl
  · exact ⟨y, px, by simp [hu], Ne.symm hxy, (ends y c hy hx (Ne.symm hf)).1,
      (ends y c hy hx (Ne.symm hf)).2, py⟩
  · exact ⟨x, py, by simp [hu], hxy, (ends x c hx hy hf).1, (ends x c hx hy hf).2, px⟩

/-! ## The rotation round a vertex -/

theorem mem_leaving {v c : Nat} :
    c ∈ b.leaving v ↔ ∃ e < b.edges.size, c ∈ (b.ed e).uses ∧ b.start c = v := by
  unfold leaving
  simp only [List.mem_flatMap, List.mem_range, List.mem_filter, beq_iff_eq]

/-- A used coedge is the `i`th of its loop, and `prev` of it is its predecessor there, which ends
where it starts and is used by its own edge. -/
theorem prev_spec (wf : b.WF) (hl : b.checkLoops = []) {e c : Nat} (hel : e < b.edges.size)
    (hc : c ∈ (b.ed e).uses) :
    ∃ i < (b.lo (b.co c).lp).len, c = (b.lo (b.co c).lp).first + i ∧
      b.prev c = (b.lo (b.co c).lp).first + (i + (b.lo (b.co c).lp).len - 1) % (b.lo (b.co c).lp).len ∧
      (b.co (b.prev c)).lp = (b.co c).lp ∧
      b.stop (b.prev c) = b.start c ∧
      (b.co (b.prev c)).edge < b.edges.size ∧ b.prev c ∈ (b.ed (b.co (b.prev c)).edge).uses := by
  obtain ⟨hll, hin, hlst⟩ := wf.use_loop e hel c hc
  obtain ⟨i, hi, hci⟩ := (b.mem_loopCoedges).1 hin
  have hlp : (b.co ((b.lo (b.co c).lp).first + i)).lp = (b.co c).lp := by rw [← hci]
  have hp := b.prev_eq hlp
  rw [← hci] at hp
  have hpin : b.prev c ∈ b.loopCoedges (b.co c).lp :=
    (b.mem_loopCoedges).2 ⟨_, pred_idx_lt hi, hp⟩
  refine ⟨i, hi, hci, hp, wf.loop_lp _ hll hlst _ hpin, ?_, wf.loop_use _ hll hlst _ hpin⟩
  have := ((b.loops_ok hl).2 _ hll).2 _ (pred_idx_lt hi)
  rw [succ_pred_idx hi, ← hci, ← hp] at this
  exact this

theorem rot_into (wf : b.WF) (he : b.checkEdges = []) (hl : b.checkLoops = []) :
    ∀ v, ∀ c ∈ b.leaving v, b.rot c ∈ b.leaving v ∧ b.partner (b.prev c) = some (b.rot c) := by
  intro v c hc
  obtain ⟨e, hel, hce, hs⟩ := (b.mem_leaving).1 hc
  obtain ⟨_, _, _, _, _, hstop, hpe, hpu⟩ := b.prev_spec wf hl hel hce
  obtain ⟨d, hd, hdu, _, hsd, _, _⟩ := b.oriented_of wf he _ hpe _ hpu
  have hrot : b.rot c = d := by unfold rot step; rw [hd]; rfl
  rw [hrot]
  exact ⟨(b.mem_leaving).2 ⟨_, hpe, hdu, by rw [hsd, hstop, hs]⟩, hd⟩

theorem rot_inj (wf : b.WF) (he : b.checkEdges = []) (hl : b.checkLoops = []) :
    ∀ v, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, b.rot c = b.rot d → c = d := by
  intro v c hc d hd hcd
  obtain ⟨_, hpc⟩ := b.rot_into wf he hl v c hc
  obtain ⟨_, hpd⟩ := b.rot_into wf he hl v d hd
  obtain ⟨e, hel, hce, _⟩ := (b.mem_leaving).1 hc
  obtain ⟨e', hel', hde, _⟩ := (b.mem_leaving).1 hd
  obtain ⟨i, hi, hci, hpi, hlpc, _, hpe, hpu⟩ := b.prev_spec wf hl hel hce
  obtain ⟨j, hj, hdj, hpj, hlpd, _, _, _⟩ := b.prev_spec wf hl hel' hde
  -- partner is an involution, so the two predecessors are the same coedge
  obtain ⟨_, _, _, _, _, _, hpe2, hpu2⟩ := b.prev_spec wf hl hel' hde
  obtain ⟨r, hr, _, _, _, _, hrr⟩ := b.oriented_of wf he _ hpe _ hpu
  obtain ⟨r2, hr2, _, _, _, _, hrr2⟩ := b.oriented_of wf he _ hpe2 _ hpu2
  have e1 : r = b.rot c := Option.some_inj.mp (hr.symm.trans hpc)
  have e2 : r2 = b.rot d := Option.some_inj.mp (hr2.symm.trans hpd)
  have hpp : b.prev c = b.prev d := by
    have : some (b.prev c) = some (b.prev d) := by rw [← hrr, ← hrr2, e1, e2, hcd]
    exact Option.some_inj.mp this
  -- in one loop, and the predecessor index is one to one
  have hsame : (b.co c).lp = (b.co d).lp := by rw [← hlpc, ← hlpd, hpp]
  rw [hpi, hpj, hsame] at hpp
  have := Nat.add_left_cancel hpp
  rw [hsame] at hi
  have hij := pred_idx_inj hi hj this
  rw [hci, hdj, hsame, hij]

/-! ## The fan walk -/

theorem walk_spec (bef : Nat → Nat) :
    ∀ fuel c seen, (b.walk bef fuel c seen).2 = seen ++ (b.walk bef fuel c seen).1 ∧
      ∀ x ∈ (b.walk bef fuel c seen).1, ∃ k, iter (b.step bef) k c = x := by
  intro fuel
  induction fuel with
  | zero => intro c seen; simp [walk]
  | succ n ih =>
    intro c seen
    unfold walk
    by_cases hs : seen.contains c
    · rw [ite_eq_left hs]; simp
    · rw [ite_eq_right hs]
      split
      · rename_i m hm
        obtain ⟨h1, h2⟩ := ih m (seen ++ [c])
        refine ⟨by simp [h1], ?_⟩
        intro x hx
        simp only [List.mem_cons] at hx
        rcases hx with rfl | hx
        · exact ⟨0, rfl⟩
        · obtain ⟨k, hk⟩ := h2 x hx
          refine ⟨k + 1, ?_⟩
          have : b.step bef c = m := by unfold step; rw [hm]; rfl
          simp only [iter, this, hk]
      · refine ⟨rfl, ?_⟩
        intro x hx
        simp only [List.mem_singleton] at hx
        exact ⟨0, hx.symm⟩

theorem walk_head (bef : Nat → Nat) (fuel c : Nat) (seen : List Nat) (hs : ¬ seen.contains c = true) :
    c ∈ (b.walk bef (fuel + 1) c seen).1 := by
  unfold walk
  rw [ite_eq_right hs]
  split <;> simp

theorem fansGo_nil (bef : Nat → Nat) (fuel : Nat) :
    ∀ l seen, b.fansGo bef fuel l seen = [] → ∀ c ∈ l, c ∈ seen := by
  intro l
  induction l with
  | nil => intro _ _ c hc; simp at hc
  | cons c0 rest ih =>
    intro seen h c hc
    unfold fansGo at h
    by_cases hs : seen.contains c0
    · rw [ite_eq_left hs] at h
      simp only [List.mem_cons] at hc
      rcases hc with rfl | hc
      · simpa using hs
      · exact ih seen h c hc
    · rw [ite_eq_right hs] at h
      exact absurd h (List.cons_ne_nil _ _)

/-- At most one fan: every coedge of `frm` is reached from the first by the walk. -/
theorem one_fan (bef : Nat → Nat) (fuel : Nat) (frm : List Nat)
    (h : (b.fansGo bef (fuel + 1) frm []).length ≤ 1) :
    ∀ c ∈ frm, ∃ k, iter (b.step bef) k (frm.headD 0) = c := by
  cases frm with
  | nil => intro c hc; simp at hc
  | cons c0 rest =>
    intro c hc
    unfold fansGo at h
    rw [ite_eq_right (by simp)] at h
    simp only [List.length_cons] at h
    have hnil : b.fansGo bef (fuel + 1) rest (b.walk bef (fuel + 1) c0 []).2 = [] :=
      List.eq_nil_of_length_eq_zero (by omega)
    have sub := b.fansGo_nil bef (fuel + 1) rest _ hnil
    obtain ⟨hseen, hreach⟩ := b.walk_spec bef (fuel + 1) c0 []
    simp only [List.mem_cons] at hc
    rcases hc with rfl | hc
    · exact ⟨0, rfl⟩
    · have := sub c hc
      rw [hseen, List.nil_append] at this
      exact hreach c this

theorem fans_eq (corners : Nat → Nat → List (Nat × Nat)) (frm : List Nat) :
    b.fans corners frm = b.fansGo (b.fanBefore corners frm) (b.fuel frm) frm [] := by
  unfold fans fanBefore fansBy
  simp only
  split
  · rfl
  · split
    · rfl
    · split <;> rfl

theorem fans_ok {corners} (h : b.checkFans corners = []) :
    ∀ v < b.nv, ∀ c ∈ b.leaving v,
      ∃ k, iter (b.step (b.fanBefore corners (b.leaving v))) k ((b.leaving v).headD 0) = c := by
  intro v hv
  unfold checkFans at h
  rw [List.filterMap_eq_nil_iff] at h
  have := h v (List.mem_range.2 hv)
  simp only at this
  have hle : (b.fans corners (b.leaving v)).length ≤ 1 := by
    by_cases hn : (b.fans corners (b.leaving v)).length > 1
    · simp [hn] at this
    · omega
  rw [b.fans_eq] at hle
  exact b.one_fan _ _ _ (by unfold fuel at hle; exact hle)

/-! ## One cycle round each vertex -/

theorem iter_add (f : Nat → Nat) : ∀ m n x, iter f (m + n) x = iter f n (iter f m x) := by
  intro m
  induction m with
  | zero => intro n x; simp [iter]
  | succ m ih =>
    intro n x
    rw [show m + 1 + n = (m + n) + 1 by omega]
    simp only [iter]
    exact ih n (f x)

theorem iter_succ' (f : Nat → Nat) (k x : Nat) : iter f (k + 1) x = f (iter f k x) := by
  rw [iter_add]; rfl

theorem iter_mem (f : Nat → Nat) (L : List Nat) (hf : ∀ c ∈ L, f c ∈ L) :
    ∀ k x, x ∈ L → iter f k x ∈ L := by
  intro k
  induction k with
  | zero => intro x hx; exact hx
  | succ k ih => intro x hx; exact ih _ (hf x hx)

/-- Pigeonhole: `L.length + 1` values in `L` repeat. -/
theorem pigeon (L : List Nat) (f : Nat → Nat) (hf : ∀ t ≤ L.length, f t ∈ L) :
    ∃ i j, i < j ∧ j ≤ L.length ∧ f i = f j := by
  apply Classical.byContradiction
  intro hno
  have hnd : ∀ n, n ≤ L.length → ((List.range (n + 1)).map f).Nodup := by
    intro n
    induction n with
    | zero => intro _; simp
    | succ n ih =>
      intro hn
      rw [List.range_succ, List.map_append, List.nodup_append]
      refine ⟨ih (by omega), by simp, ?_⟩
      intro a ha c hc hac
      simp only [List.mem_map, List.mem_range] at ha
      simp only [List.map_cons, List.map_nil, List.mem_singleton] at hc
      obtain ⟨i, hi, rfl⟩ := ha
      exact hno ⟨i, n + 1, by omega, hn, by rw [hac, hc]⟩
  have := (hnd L.length (Nat.le_refl _)).length_le_of_subset (by
    intro x hx
    simp only [List.mem_map, List.mem_range] at hx
    obtain ⟨t, ht, rfl⟩ := hx
    exact hf t (by omega))
  simp at this
  omega

/-- A one-to-one map of `L` into itself returns every point of `L` to itself. -/
theorem period (f : Nat → Nat) (L : List Nat) (hf : ∀ c ∈ L, f c ∈ L)
    (hinj : ∀ c ∈ L, ∀ d ∈ L, f c = f d → c = d) (x : Nat) (hx : x ∈ L) :
    ∃ m, 0 < m ∧ iter f m x = x := by
  obtain ⟨i, j, hij, _, he⟩ := pigeon L (fun t => iter f t x) (fun t _ => iter_mem f L hf t x hx)
  have cancel : ∀ i j, i < j → iter f i x = iter f j x → iter f (j - i) x = x := by
    intro i
    induction i with
    | zero => intro j _ h; simp only [Nat.sub_zero]; exact h.symm
    | succ i ih =>
      intro j hj h
      obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
      rw [iter_succ', iter_succ'] at h
      have := hinj _ (iter_mem f L hf i x hx) _ (iter_mem f L hf j' x hx) h
      rw [show j' + 1 - (i + 1) = j' - i by omega]
      exact ih j' (by omega) this
  exact ⟨j - i, by omega, cancel i j hij he⟩

theorem iter_mul (f : Nat → Nat) (m x : Nat) (hm : iter f m x = x) : ∀ q, iter f (m * q) x = x := by
  intro q
  induction q with
  | zero => simp [iter]
  | succ q ih => rw [Nat.mul_succ, iter_add, ih, hm]

/-- A one-to-one map of `L` into itself that reaches all of `L` from its head is one cycle. -/
theorem cycle_of (f : Nat → Nat) (L : List Nat) (hf : ∀ c ∈ L, f c ∈ L)
    (hinj : ∀ c ∈ L, ∀ d ∈ L, f c = f d → c = d)
    (reach : ∀ c ∈ L, ∃ k, iter f k (L.headD 0) = c) :
    ∀ c ∈ L, ∀ d ∈ L, ∃ k, iter f k c = d := by
  intro c hc d hd
  have hx : L.headD 0 ∈ L := by
    cases L with
    | nil => simp at hc
    | cons a _ => simp
  obtain ⟨a, ha⟩ := reach c hc
  obtain ⟨e, he⟩ := reach d hd
  obtain ⟨m, hm, hper⟩ := period f L hf hinj _ hx
  subst ha he
  generalize L.headD 0 = x at hper
  by_cases hae : a ≤ e
  · exact ⟨e - a, by rw [← iter_add, show a + (e - a) = e by omega]⟩
  · have hle : a ≤ m * a := Nat.le_mul_of_pos_left a hm
    refine ⟨m * a + e - a, ?_⟩
    rw [← iter_add, show a + (m * a + e - a) = m * a + e by omega, iter_add, iter_mul _ _ _ hper]

/-! ## The links, the strays, the shells -/

theorem links_ok (h : b.checkLinks = []) : b.WF := by
  unfold checkLinks at h
  simp only [List.append_eq_nil_iff, List.filterMap_eq_nil_iff, List.mem_range] at h
  obtain ⟨⟨hf, he⟩, hl⟩ := h
  have hf' : ∀ f < b.faces.size, b.faceOk f = true := by
    intro f hfl; have := hf f hfl; revert this; split <;> simp_all
  have he' : ∀ e < b.edges.size, b.edgeOk e = true := by
    intro e hel; have := he e hel; revert this; split <;> simp_all
  have hl' : ∀ l < b.loops.size, (b.loopSkip l || b.loopOk l) = true := by
    intro l hll; have := hl l hll; revert this; split <;> simp_all
  have E : ∀ e < b.edges.size, (b.ed e).start < b.nv ∧ (b.ed e).stop < b.nv ∧ ∀ c ∈ (b.ed e).uses,
      c < b.coedges.size ∧ (b.co c).edge = e ∧ (b.co c).lp < b.loops.size ∧
        b.listed (b.co c).lp = 1 ∧ c ∈ b.loopCoedges (b.co c).lp := by
    intro e hel; have := he' e hel; simp only [edgeOk, Bool.and_eq_true, decide_eq_true_eq,
      List.all_eq_true, beq_iff_eq, List.contains_iff_mem, and_assoc] at this
    exact this
  have L : ∀ l < b.loops.size, b.listed l = 1 → ∀ c ∈ b.loopCoedges l,
      (b.co c).lp = l ∧ (b.co c).edge < b.edges.size ∧ c ∈ (b.ed (b.co c).edge).uses := by
    intro l hll h1 c hc
    have := hl' l hll
    simp only [loopSkip, loopOk, h1, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq,
      beq_iff_eq, List.all_eq_true, List.contains_iff_mem] at this
    simp only [and_assoc] at this
    rcases this with ⟨_, h0⟩ | ⟨_, _, hall⟩
    · omega
    · obtain ⟨_, hlp, hed, hmem⟩ := hall c hc; exact ⟨hlp, hed, hmem⟩
  refine ⟨fun e hel => ⟨(E e hel).1, (E e hel).2.1⟩, fun e hel c hc => ((E e hel).2.2 c hc).2.1,
    fun e hel c hc => ⟨((E e hel).2.2 c hc).2.2.1, ((E e hel).2.2 c hc).2.2.2.2,
      ((E e hel).2.2 c hc).2.2.2.1⟩,
    fun l hll h1 c hc => (L l hll h1 c hc).1, fun l hll h1 c hc => (L l hll h1 c hc).2, ?_, ?_⟩
  · intro f hfl l hl
    have := hf' f hfl
    simp only [faceOk, List.all_eq_true, Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at this
    exact this l hl
  · intro l hll
    have := hl' l hll
    simp only [loopSkip, loopOk, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq,
      beq_iff_eq] at this
    rcases this with ⟨h1, h0⟩ | ⟨⟨h1, h2⟩, _⟩ <;> omega

theorem strays_ok (h : b.checkStrays = []) :
    ∀ v < b.nv, ∃ e < b.edges.size, (b.ed e).start = v ∨ (b.ed e).stop = v := by
  intro v hv
  unfold checkStrays at h
  rw [List.filterMap_eq_nil_iff] at h
  have := h v (List.mem_range.2 hv)
  by_cases ha : b.edges.any (fun e => e.start == v || e.stop == v) = true
  · obtain ⟨i, hi, hp⟩ := Array.any_eq_true.1 ha
    refine ⟨i, hi, ?_⟩
    have hed : b.ed i = b.edges[i] := by
      simp [ed, Array.getD_eq_getD_getElem?, hi]
    rw [hed]
    simpa using hp
  · simp [ha] at this

theorem flood_sound (h : Nat) : ∀ fuel seen stack, (∀ x ∈ seen, b.Reach h x) →
    (∀ x ∈ stack, b.Reach h x) → ∀ x ∈ b.flood fuel seen stack, b.Reach h x := by
  intro fuel
  induction fuel with
  | zero => intro seen stack hs _ x hx; exact hs x hx
  | succ n ih =>
    intro seen stack hs hst x hx
    cases stack with
    | nil => exact hs x hx
    | cons f st =>
      simp only [flood] at hx
      have hf := hst f (by simp)
      have hnew : ∀ y ∈ ((b.nbrs f).filter (· ∉ seen)).eraseDups, b.Reach h y := by
        intro y hy
        exact Reach.step hf (List.mem_filter.1 (List.mem_eraseDups.1 hy)).1
      refine ih _ _ ?_ ?_ x hx
      · intro y hy
        rcases List.mem_append.1 hy with hy | hy
        · exact hs y hy
        · exact hnew y hy
      · intro y hy
        rcases List.mem_append.1 hy with hy | hy
        · exact hnew y hy
        · exact hst y (by simp [hy])

theorem shells_ok (h : b.checkShellFaces = []) :
    (∀ s < b.shells.size, b.shellList s ≠ [] ∧ ∀ f ∈ b.shellList s, f < b.faces.size ∧ b.faceShell f = s) ∧
    (∀ f < b.faces.size, b.faceShell f < b.shells.size ∧ (b.shellList (b.faceShell f)).count f = 1) ∧
    (∀ e < b.edges.size, ∀ c ∈ (b.ed e).uses, ∀ d ∈ (b.ed e).uses,
      b.faceShell (b.faceOf c) = b.faceShell (b.faceOf d)) ∧
    (∀ s < b.shells.size, ∀ f ∈ b.shellList s, b.Reach ((b.shellList s).headD 0) f) := by
  unfold checkShellFaces at h
  simp only [List.append_eq_nil_iff, List.filterMap_eq_nil_iff, List.mem_range] at h
  obtain ⟨hb, hc⟩ := h
  have nb : ∀ s ≤ b.shells.size, b.bad s = false := by
    intro s hs; have := hb s (by omega); revert this; split <;> simp_all
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro s hs
    have h0 := nb s (Nat.le_of_lt hs)
    unfold bad at h0
    simp only [Bool.or_eq_false_iff] at h0
    have h1 := h0.1.1
    have hss : decide (s < b.shells.size) = true := by simpa using hs
    rw [hss, Bool.true_and, Bool.or_eq_false_iff] at h1
    refine ⟨fun hn => by rw [hn] at h1; simp at h1, fun f hf => ?_⟩
    have := List.any_eq_false.1 h1.2 f hf
    simpa using this
  · intro f hf
    have h0 := nb (b.atS f) (Nat.min_le_right _ _)
    unfold bad at h0
    simp only [Bool.or_eq_false_iff] at h0
    have h2 := List.any_eq_false.1 h0.1.2 f (List.mem_range.2 hf)
    have hl : b.listedIn f = 1 := by
      apply Classical.byContradiction; intro hne; apply h2; simp [hne]
    unfold listedIn at hl
    split at hl
    · exact ⟨by assumption, hl⟩
    · simp at hl
  · intro e he c hc d hd
    cases hu : (b.ed e).uses with
    | nil => rw [hu] at hc; simp at hc
    | cons c0 rest =>
      have h0 := nb (b.atS (b.faceOf c0)) (Nat.min_le_right _ _)
      unfold bad at h0
      simp only [Bool.or_eq_false_iff] at h0
      have h3 := List.any_eq_false.1 h0.2 e (List.mem_range.2 he)
      simp only [hu] at h3
      have key : ∀ x ∈ c0 :: rest, b.faceShell (b.faceOf x) = b.faceShell (b.faceOf c0) := by
        intro x hx; apply Classical.byContradiction; intro hne; apply h3
        simp only [Bool.and_eq_true, List.any_eq_true, bne_iff_ne, beq_self_eq_true, and_true]
        exact ⟨x, hx, hne⟩
      rw [hu] at hc hd
      rw [key c hc, key d hd]
  · intro s hs f hf
    have hs' := nb s (Nat.le_of_lt hs)
    have := hc s hs
    rw [hs'] at this
    simp only [Bool.not_false, Bool.true_and] at this
    by_cases hall : (b.shellList s).all (fun f => decide (f ∈ b.reached s)) = true
    · have hr := List.all_eq_true.1 hall f hf
      simp only [decide_eq_true_eq] at hr
      unfold reached at hr
      exact b.flood_sound _ _ _ _ (by simp [Reach.refl]) (by simp [Reach.refl]) f hr
    · simp [hall] at this

/-! ## The fan walk permutes, by either rule -/

theorem nodup_map_inj {f : Nat → Nat} : ∀ {l : List Nat}, (l.map f).Nodup →
    ∀ x ∈ l, ∀ y ∈ l, f x = f y → x = y
  | [], _, x, hx, _, _, _ => by simp at hx
  | a :: t, h, x, hx, y, hy, hxy => by
    rw [List.map_cons, List.nodup_cons] at h
    simp only [List.mem_cons] at hx hy
    rcases hx with rfl | hx <;> rcases hy with rfl | hy
    · rfl
    · exact absurd (hxy ▸ List.mem_map_of_mem hy) h.1
    · exact absurd (hxy.symm ▸ List.mem_map_of_mem hx) h.1
    · exact nodup_map_inj h.2 x hx y hy hxy

theorem head_start {v c : Nat} (hc : c ∈ b.leaving v) : b.start ((b.leaving v).headD 0) = v := by
  cases hL : b.leaving v with
  | nil => rw [hL] at hc; simp at hc
  | cons a _ =>
    have ha : a ∈ b.leaving v := by rw [hL]; simp
    obtain ⟨_, _, _, hs⟩ := (b.mem_leaving).1 ha
    simpa using hs

theorem guarded (wf : b.WF) (he : b.checkEdges = []) (bef : Nat → Nat) (v : Nat)
    (hok : b.closesOk bef (b.leaving v) = true) :
    (∀ c ∈ b.leaving v, b.step bef c ∈ b.leaving v) ∧
    (∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, b.step bef c = b.step bef d → c = d) := by
  simp only [closesOk, Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq, beq_iff_eq,
    List.contains_iff_mem] at hok
  obtain ⟨hall, hnd⟩ := hok
  have one : ∀ c ∈ b.leaving v, ∃ d, b.partner (bef c) = some d ∧ d ∈ b.leaving v ∧
      b.partner d = some (bef c) := by
    intro c hc
    obtain ⟨⟨⟨_, hstop⟩, hlt⟩, hmem⟩ := hall c hc
    obtain ⟨d, hd, hdu, _, hsd, _, hdd⟩ := b.oriented_of wf he _ hlt _ hmem
    refine ⟨d, hd, (b.mem_leaving).2 ⟨_, hlt, hdu, ?_⟩, hdd⟩
    rw [hsd, hstop, b.head_start hc]
  have st : ∀ c ∈ b.leaving v, ∀ d, b.partner (bef c) = some d → b.step bef c = d := by
    intro c _ d hd; unfold step; rw [hd]; rfl
  refine ⟨fun c hc => ?_, fun c hc d hd hcd => ?_⟩
  · obtain ⟨d, hd, hdl, _⟩ := one c hc; rw [st c hc d hd]; exact hdl
  · obtain ⟨x, hx, _, hxx⟩ := one c hc
    obtain ⟨y, hy, _, hyy⟩ := one d hd
    rw [st c hc x hx, st d hd y hy] at hcd
    subst hcd
    have : bef c = bef d := Option.some_inj.mp (hxx.symm.trans hyy)
    exact nodup_map_inj hnd c hc d hd this

theorem fan_perm (wf : b.WF) (he : b.checkEdges = []) (hl : b.checkLoops = [])
    (corners : Nat → Nat → List (Nat × Nat)) (v : Nat) :
    (∀ c ∈ b.leaving v, b.step (b.fanBefore corners (b.leaving v)) c ∈ b.leaving v) ∧
    (∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, b.step (b.fanBefore corners (b.leaving v)) c =
      b.step (b.fanBefore corners (b.leaving v)) d → c = d) := by
  have byLoop : (∀ c ∈ b.leaving v, b.step (b.before none) c ∈ b.leaving v) ∧
      (∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, b.step (b.before none) c = b.step (b.before none) d →
        c = d) := by
    have hrot : b.step (b.before none) = b.rot := rfl
    rw [hrot]
    exact ⟨fun c hc => (b.rot_into wf he hl v c hc).1, b.rot_inj wf he hl v⟩
  unfold fanBefore
  simp only
  split
  · exact byLoop
  · split
    · exact byLoop
    · split
      · rename_i hok; exact b.guarded wf he _ v hok
      · exact byLoop

/-! ## Soundness -/

/-- `check` accepting a body implies it is a valid closed, oriented, manifold B-rep, whatever
corner table it read. -/
theorem check_sound (corners : Nat → Nat → List (Nat × Nat)) (h : b.check corners = []) :
    b.Valid corners := by
  obtain ⟨hk, he, hl, hs, hf, hsf, hst⟩ := b.check_parts h
  have wf := b.links_ok hk
  obtain ⟨s1, s2, s3, s4⟩ := b.shells_ok hsf
  refine ⟨wf, ?_, (b.loops_ok hl).1, (b.loops_ok hl).2, b.oriented_of wf he,
    fun v => (b.fan_perm wf he hl corners v).1, fun v => (b.fan_perm wf he hl corners v).2, ?_,
    b.strays_ok hst, s1, s2, s3, s4, b.euler_ok hs⟩
  · intro e hel
    obtain ⟨x, y, hu, hfw⟩ := b.edges_ok he e hel
    exact ⟨x, y, hu, fun h => hfw (by rw [h]), hfw⟩
  · intro v hv
    exact cycle_of _ _ (b.fan_perm wf he hl corners v).1 (b.fan_perm wf he hl corners v).2
      (b.fans_ok hf v hv)

theorem fanBefore_none (frm : List Nat) : b.fanBefore (fun _ _ => []) frm = b.before none := by
  unfold fanBefore
  simp only
  split
  · rfl
  · rw [ite_eq_left (by simp)]

/-- With no corner table (each face meets each vertex at most once), the rotation round every
vertex by loop order is one cycle through all the coedges leaving it. -/
theorem one_cycle (h : b.check (fun _ _ => []) = []) :
    ∀ v < b.nv, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, ∃ k, iter b.rot k c = d := by
  intro v hv c hc d hd
  have := (b.check_sound _ h).oneFan v hv c hc d hd
  rwa [b.fanBefore_none] at this

end TopoCheck.Body
