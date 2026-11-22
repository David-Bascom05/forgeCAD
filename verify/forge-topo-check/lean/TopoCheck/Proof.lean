import TopoCheck.Spec

/-!
# `check` accepting a body implies it is valid

`check_sound`: for a body with its back-links (`WF`), if the combinatorial part of `check` finds
no problem, the body is `Valid`. `one_cycle`: with no corner table (each face meets each vertex
at most once), the rotation round each vertex is a single cycle through all its leaving coedges.
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
    b.checkEdges = [] ∧ b.checkLoops = [] ∧ b.checkShellEuler = [] ∧ b.checkFans corners = [] := by
  unfold check at h
  by_cases hb : (b.checkEdges ++ b.checkLoops).any blocks
  · rw [ite_eq_left hb] at h
    rw [h] at hb; simp at hb
  · rw [ite_eq_right hb] at h
    simp only [List.append_eq_nil_iff] at h
    exact ⟨h.1.1.1, h.1.1.2, h.1.2, h.2⟩

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
  obtain ⟨hll, hin⟩ := wf.use_loop e hel c hc
  obtain ⟨i, hi, hci⟩ := (b.mem_loopCoedges).1 hin
  have hlp : (b.co ((b.lo (b.co c).lp).first + i)).lp = (b.co c).lp := by rw [← hci]
  have hp := b.prev_eq hlp
  rw [← hci] at hp
  have hpin : b.prev c ∈ b.loopCoedges (b.co c).lp :=
    (b.mem_loopCoedges).2 ⟨_, pred_idx_lt hi, hp⟩
  refine ⟨i, hi, hci, hp, wf.loop_lp _ hll _ hpin, ?_, wf.loop_use _ hll _ hpin⟩
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

/-! ## Soundness -/

/-- `check` accepting a body with its back-links implies it is a valid closed, oriented,
manifold B-rep. -/
theorem check_sound (corners : Nat → Nat → List (Nat × Nat)) (wf : b.WF)
    (h : b.check corners = []) : b.Valid corners := by
  obtain ⟨he, hl, hs, hf⟩ := b.check_parts h
  refine ⟨?_, (b.loops_ok hl).1, (b.loops_ok hl).2, b.oriented_of wf he,
    fun v c hc => (b.rot_into wf he hl v c hc).1, b.rot_inj wf he hl, b.fans_ok hf, b.euler_ok hs⟩
  intro e hel
  obtain ⟨x, y, hu, hfw⟩ := b.edges_ok he e hel
  exact ⟨x, y, hu, fun h => hfw (by rw [h]), hfw⟩

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

theorem fanBefore_none (frm : List Nat) : b.fanBefore (fun _ _ => []) frm = b.before none := by
  unfold fanBefore
  simp only
  split
  · rfl
  · rw [ite_eq_left (by simp)]

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

/-- With no corner table (each face meets each vertex at most once), the rotation round every
vertex is one cycle through all the coedges leaving it: the faces there form a single disc. -/
theorem one_cycle (wf : b.WF) (h : b.check (fun _ _ => []) = []) :
    ∀ v < b.nv, ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v, ∃ k, iter b.rot k c = d := by
  intro v hv
  have hval := b.check_sound _ wf h
  have reach := hval.oneFan v hv
  rw [b.fanBefore_none] at reach
  exact cycle_of b.rot _ (hval.rotInto v) (hval.rotInj v) reach

/-- With a corner table, the walk `fans` takes round a vertex is one cycle when it permutes the
coedges leaving the vertex. `Body::corners` does not promise that: at a cylinder's seam vertex,
where a closed edge both leaves and arrives, it pairs both leaving coedges with one arriving
(verify/forge-topo-check/tests/diff.rs, `model_agrees`). -/
theorem one_cycle_by (corners : Nat → Nat → List (Nat × Nat)) (wf : b.WF) (h : b.check corners = [])
    (v : Nat) (hv : v < b.nv)
    (hinto : ∀ c ∈ b.leaving v, b.step (b.fanBefore corners (b.leaving v)) c ∈ b.leaving v)
    (hinj : ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v,
      b.step (b.fanBefore corners (b.leaving v)) c = b.step (b.fanBefore corners (b.leaving v)) d → c = d) :
    ∀ c ∈ b.leaving v, ∀ d ∈ b.leaving v,
      ∃ k, iter (b.step (b.fanBefore corners (b.leaving v))) k c = d :=
  cycle_of _ _ hinto hinj ((b.check_sound _ wf h).oneFan v hv)

end TopoCheck.Body
