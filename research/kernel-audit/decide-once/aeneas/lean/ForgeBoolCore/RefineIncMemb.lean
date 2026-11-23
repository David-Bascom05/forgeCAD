import ForgeBoolCore.RefineIncFlood
import ForgeBoolCore.HeadBridgeI

/-!
# Group C: the inc membership

`inc::membership l s` returns, for each body `i`, a row over `s.rooms`: room `t`'s entry is the
witness of its flood class's root. Here:
* `membership_loop0_spec`: the `inside` table, `r * nb + b` set iff `(r, b)` is a witness;
* the rows, from `flood_spec` (RefineIncFlood);
* the bridge to the Lean `memb` through `head_leastI` (HeadBridgeI).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- A witness's room and body, as numbers. -/
def wR (l : Ledger) (j : Nat) : Nat := ((l.inside.val.getD j default).1 : U32).val
def wB (l : Ledger) (j : Nat) : Nat := ((l.inside.val.getD j default).2 : U32).val

theorem pieces_mem' {β : Type} [Inhabited β] {L : List β} {j : Nat} (hj : j < L.length) : L.getD j default ∈ L := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]; exact List.getElem_mem hj

/-- The `inside` table after the first `k` witnesses. -/
def IInv (l : Ledger) (ins : List Bool) (k : Nat) : Prop :=
  k ≤ l.inside.val.length ∧ ins.length = l.n_rooms.val * l.nb.val ∧
  ∀ r, r < l.n_rooms.val → ∀ b, b < l.nb.val →
    (ins.getD (r * l.nb.val + b) false = true ↔ ∃ j, j < k ∧ wR l j = r ∧ wB l j = b)

theorem membership_loop0_spec (l : Ledger) (hw : LedgerWF2 l)
    (ins : alloc.vec.Vec Bool) (k : Usize) (h : IInv l ins.val k.val) :
    membership_loop0 l.inside (UScalar.cast .Usize l.nb) (UScalar.cast .Usize l.n_rooms) ins k ⦃ r =>
      IInv l r.val l.inside.val.length ⦄ := by
  have hsm := hw.small
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  have hnrv : (UScalar.cast UScalarTy.Usize l.n_rooms : Usize).val = l.n_rooms.val := by scalar_tac
  unfold membership_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => l.inside.val.length - q.2.val)
    (fun q => IInv l q.1.val q.2.val)
  · rintro ⟨p, j⟩ ⟨hj, hlen, hP⟩
    unfold membership_loop0.body
    simp only at hj hlen hP ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.inside.val.length := by scalar_tac
      step as ⟨r, b, hrb⟩
      have hg : l.inside.val.getD j.val default = (r, b) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hrb]
      clear hrb
      have hrv : wR l j.val = (r : U32).val := by unfold wR; rw [hg]
      have hbv : wB l j.val = (b : U32).val := by unfold wB; rw [hg]
      have hin := hw.inside _ (pieces_mem' hjl)
      rw [hg] at hin
      simp only at hin
      have hnr := hsm.2.2.2.2.2
      clear hg
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = (b : U32).val := by rw [hi1]; scalar_tac
      split
      · rename_i hb1
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = (r : U32).val := by rw [hi2]; scalar_tac
        split
        · rename_i hr1
          step as ⟨i3, hi3⟩
          have hi3v : i3.val = (r : U32).val := by rw [hi3]; scalar_tac
          have hkey : (r : U32).val * l.nb.val + (b : U32).val < l.n_rooms.val * l.nb.val := by
            have := Nat.mul_le_mul_right l.nb.val (show (r : U32).val + 1 ≤ l.n_rooms.val by omega)
            rw [Nat.add_mul, Nat.one_mul] at this; omega
          have hk1 : (r : U32).val * l.nb.val ≤ Usize.max := by
            have := Nat.mul_le_mul_right l.nb.val (show (r : U32).val ≤ l.n_rooms.val by omega)
            omega
          step as ⟨i4, hi4⟩
          have hi4v : i4.val = (r : U32).val * l.nb.val := by rw [hi4, hi3v, hnbv]
          step as ⟨i5, hi5⟩
          have hi5v : i5.val = (b : U32).val := by rw [hi5]; scalar_tac
          step as ⟨i6, hi6⟩
          have hi6v : i6.val = (r : U32).val * l.nb.val + (b : U32).val := by rw [hi6, hi4v, hi5v]
          have hi6l : i6.val < p.val.length := by rw [hlen, hi6v]; exact hkey
          step as ⟨x0, _, hbk⟩
          obtain ⟨x1, bk⟩ := x0
          simp only at hbk ⊢
          rw [hbk]
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨⟨by omega, by simp [hlen], ?_⟩, by omega⟩
          intro c hc b' hb'
          rw [hj1v]
          simp only [alloc.vec.Vec.set_val_eq]
          by_cases hk : c * l.nb.val + b' = i6.val
          · rw [hi6v] at hk
            obtain ⟨rfl, rfl⟩ := key_eq hb' hin.2 hk
            rw [hi6v, List.getD_eq_getElem?_getD, List.getElem?_set_self (by rw [← hlen] at hkey; exact hkey)]
            simp only [Option.getD_some, true_iff]
            exact ⟨j.val, by omega, hrv, hbv⟩
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hk), ← List.getD_eq_getElem?_getD,
              hP c hc b' hb']
            constructor
            · rintro ⟨j', hj', h1, h2⟩; exact ⟨j', by omega, h1, h2⟩
            · rintro ⟨j', hj', h1, h2⟩
              refine ⟨j', ?_, h1, h2⟩
              rcases Nat.lt_succ_iff_lt_or_eq.mp hj' with h | h
              · exact h
              · exfalso; apply hk; rw [hi6v, ← h1, ← h2, h, hrv, hbv]
        · rename_i hr1; exfalso; apply hr1; have := hin.1; scalar_tac
      · rename_i hb1; exfalso; apply hb1; have := hin.2; scalar_tac
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.inside.val.length := by scalar_tac
      rw [← this]; exact ⟨hj, hlen, hP⟩
  · exact h

/-! ### The bridge to the Lean `memb` -/

/-- The rooms as the Lean lists them. -/
def roomsF (s : Split) : List FCell := s.rooms.val.map fun r => FCell.room r.val

theorem roomsF_nodup (l : Ledger) (s : Split) (hs : SplitOK l s) : (roomsF s).Nodup := by
  unfold roomsF; rw [hs.rooms]; exact nodup_dedup _

theorem roomsF_get (s : Split) {p : Nat} (hp : p < s.rooms.val.length) :
    (roomsF s)[p]'(by simp [roomsF, hp]) = FCell.room (s.rooms.val[p]).val := by
  simp [roomsF]

/-- A room's position is where `roomsF` lists it. -/
theorem idxOf_room (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {x : Nat}
    (hx : x < l.n_rooms.val) (hm : x ∈ seenRooms l.facet_rooms.val (nfacets l)) :
    (roomsF s).idxOf (FCell.room x) = posR s x := by
  have hp := pos_lt l hw s hs hx hm
  have he := ((hs.rinv.2.2.2 x hx).2 hm).1
  have hv : (s.rooms.val[posR s x]'hp).val = x := by
    change (s.rooms.val.getD (posR s x) 0#u32).val = x at he
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp] at he; exact he
  have hg : (roomsF s)[posR s x]'(by simp [roomsF, hp]) = FCell.room x := by
    rw [roomsF_get s hp, hv]
  conv => lhs; rw [← hg]
  exact List.Nodup.idxOf_getElem (roomsF_nodup l s hs) _ _

/-- The Lean's crossings of body `i` over the first `k` facets. -/
def crossK (l : Ledger) (i k : Nat) : List (FCell × FCell) :=
  (((List.range k).map FCell.facet).filter fun φ => !(toLF l).isPieceOf i φ).map
    fun φ => ((toLF l).plus φ, (toLF l).minus φ)

theorem crossK_full (l : Ledger) (i : Nat) : crossK l i (nfacets l) = (toLF l).crossings i := rfl

theorem floodE_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {i : Nat}
    (hi : i < l.nb.val) : ∀ k, k ≤ nfacets l → posPairsI (roomsF s) (crossK l i k) = floodE l s i k
  | 0, _ => by simp [crossK, posPairsI, floodE]
  | k + 1, hk => by
    have ih := floodE_eq l hw s hs hi k (by omega)
    unfold floodE
    rw [← ih]
    have hpb : (toLF l).isPieceOf i (FCell.facet k) = pieceB l s k i := by
      rw [pieceB, hs.piece_of k (by omega) i hi]
    have hfr := facet_rooms_get l hw (f := k) (by omega)
    simp only [crossK, List.range_succ, List.map_append, List.filter_append, posPairsI, List.map_map]
    congr 1
    simp only [List.map_cons, List.map_nil, List.filter_cons, List.filter_nil, hpb]
    cases pieceB l s k i
    · simp only [Bool.not_false, Bool.false_eq_true, ite_true, ite_false, List.map_cons, List.map_nil]
      have e1 : (toLF l).plus (FCell.facet k) = FCell.room (plusR l k) := rfl
      have e2 : (toLF l).minus (FCell.facet k) = FCell.room (minusR l k) := rfl
      simp only [Function.comp_apply, e1, e2]
      rw [idxOf_room l hw s hs hfr.1 (plusR_mem l (by omega)),
        idxOf_room l hw s hs hfr.2 (minusR_mem l (by omega))]
    · simp

theorem cross_in (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (i : Nat) :
    ∀ p ∈ (toLF l).crossings i, p.1 ∈ roomsF s ∧ p.2 ∈ roomsF s := by
  intro p hp
  simp only [IncLedger.crossings, List.mem_map, List.mem_filter] at hp
  obtain ⟨φ, ⟨hφ, _⟩, rfl⟩ := hp
  simp only [toLF, List.mem_map, List.mem_range] at hφ
  obtain ⟨f, hf, rfl⟩ := hφ
  have hfr := facet_rooms_get l hw hf
  have hm : ∀ x, x < l.n_rooms.val → x ∈ seenRooms l.facet_rooms.val (nfacets l) →
      FCell.room x ∈ roomsF s := by
    intro x hx hxm
    unfold roomsF; rw [hs.rooms, IncLedger.rooms]
    rw [mem_dedup]
    simp only [toLF, List.mem_flatMap, List.mem_map, List.mem_range, List.mem_cons, List.mem_nil_iff, or_false]
    simp only [seenRooms, List.mem_flatMap, List.mem_range, List.mem_cons, List.mem_nil_iff, or_false] at hxm
    obtain ⟨g, hg, hx'⟩ := hxm
    exact ⟨FCell.facet g, ⟨g, hg, rfl⟩, by rcases hx' with h | h <;> simp [h, facetNum]⟩
  exact ⟨hm _ hfr.1 (plusR_mem l hf), hm _ hfr.2 (minusR_mem l hf)⟩

/-- The witness table answers `inside.contains`. -/
theorem inside_contains (l : Ledger) {ins : List Bool} (h : IInv l ins l.inside.val.length)
    {x i : Nat} (hx : x < l.n_rooms.val) (hi : i < l.nb.val) :
    (toLF l).inside.contains (FCell.room x, i) = ins.getD (x * l.nb.val + i) false := by
  rw [Bool.eq_iff_iff, h.2.2 x hx i hi, List.contains_iff_mem]
  simp only [toLF, List.mem_map, Prod.mk.injEq, FCell.room.injEq]
  constructor
  · rintro ⟨q, hq, h1, h2⟩
    obtain ⟨j, hj, rfl⟩ := List.mem_iff_getElem.mp hq
    have hg : l.inside.val.getD j default = l.inside.val[j] := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]; rfl
    exact ⟨j, hj, by unfold wR; rw [hg]; exact h1, by unfold wB; rw [hg]; exact h2⟩
  · rintro ⟨j, hj, h1, h2⟩
    exact ⟨_, pieces_mem' hj, h1, h2⟩

/-- **The root's witness is the Lean's `memb`.** -/
theorem memb_bridge (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {i : Nat}
    (hi : i < l.nb.val) {R : Nat → Nat} (hl : Least (floodE l s i (nfacets l)) s.rooms.val.length R)
    {ins : List Bool} (hins : IInv l ins l.inside.val.length) {t : Nat} (ht : t < s.rooms.val.length) :
    ins.getD ((s.rooms.val[R t]'(hl t ht).1).val * l.nb.val + i) false =
      (toLF l).memb i (FCell.room (s.rooms.val[t]).val) := by
  have hlen : (roomsF s).length = s.rooms.val.length := by simp [roomsF]
  have hl' : Least (posPairsI (roomsF s) ((toLF l).crossings i)) (roomsF s).length R := by
    rw [← crossK_full, floodE_eq l hw s hs hi _ (Nat.le_refl _), hlen]; exact hl
  have hh := head_leastI (roomsF_nodup l s hs) (cross_in l hw s hs i) hl' (t := t) (by rw [hlen]; exact ht)
  unfold IncLedger.memb
  rw [show FCell.room (s.rooms.val[t]).val = (roomsF s)[t]'(by rw [hlen]; exact ht) from (roomsF_get s ht).symm]
  have hrf : (toLF l).rooms = roomsF s := hs.rooms.symm
  rw [hrf, hh]
  simp only
  rw [roomsF_get s (hl t ht).1]
  have hrl : (s.rooms.val[R t]'(hl t ht).1).val < l.n_rooms.val := by
    have := hs.rinv.2.2.1
    have hmem : (s.rooms.val[R t]'(hl t ht).1).val ∈ s.rooms.val.map (·.val) :=
      List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
    rw [hs.rinv.1, mem_dedup] at hmem
    exact this _ hmem
  rw [inside_contains l hins hrl hi]

/-! ### The rows -/

/-- The Lean's membership of the room at position `x`, for body `i`. -/
def membRow (l : Ledger) (s : Split) (i x : Nat) : Bool :=
  (toLF l).memb i (FCell.room (s.rooms.val.getD x 0#u32).val)

theorem rooms_lt (l : Ledger) (s : Split) (hs : SplitOK l s) {h : Nat} (hh : h < s.rooms.val.length) :
    (s.rooms.val[h]).val < l.n_rooms.val := by
  have hmem : (s.rooms.val[h]).val ∈ s.rooms.val.map (·.val) :=
    List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
  rw [hs.rinv.1, mem_dedup] at hmem
  exact hs.rinv.2.2.1 _ hmem

theorem membership_loop1_loop0_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (ins : alloc.vec.Vec Bool) (hins : IInv l ins.val l.inside.val.length)
    (i : Usize) (hi : i.val < l.nb.val) (uf : union_find.Uf)
    (hu : UInv (floodE l s i.val (nfacets l)) s.rooms.val.length uf)
    (row : alloc.vec.Vec Bool) (t : Usize) (ht : t.val ≤ s.rooms.val.length)
    (hrow : row.val = (List.range t.val).map (membRow l s i.val)) :
    membership_loop1_loop0 s (UScalar.cast .Usize l.nb) ins i uf row t ⦃ r =>
      r.1 = s.piece_of ∧ r.2.1 = s.rooms ∧ r.2.2.1 = s.room_pos ∧
      r.2.2.2.val = (List.range s.rooms.val.length).map (membRow l s i.val) ⦄ := by
  have hsm := hw.small
  have hnr := hsm.2.2.2.2.2
  have hrl := rooms_len_le l s hs
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  unfold membership_loop1_loop0
  apply loop.spec_decr_nat
    (fun (q : union_find.Uf × alloc.vec.Vec Bool × Usize) => s.rooms.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ s.rooms.val.length ∧
      UInv (floodE l s i.val (nfacets l)) s.rooms.val.length q.1 ∧
      q.2.1.val = (List.range q.2.2.val).map (membRow l s i.val))
  · rintro ⟨u, rw, x⟩ ⟨hx, hu', hrw⟩
    unfold membership_loop1_loop0.body
    simp only at hx hu' hrw ⊢
    split
    · rename_i hlt
      have hxl : x.val < s.rooms.val.length := by scalar_tac
      have hul := uinv_len hu'
      obtain ⟨hg, hpl, hle⟩ := hu'
      step with find_spec u x (by omega) hg as ⟨h, u1, hh, hl1, hg1, hr1⟩
      have hR := hle x.val hxl
      have hhl : h.val < s.rooms.val.length := by rw [hh]; exact hR.1
      step as ⟨i2, hi2⟩
      have hi2l := rooms_lt l s hs hhl
      have hi2v : i2.val = (s.rooms.val[h.val]).val := by rw [hi2]
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = (s.rooms.val[h.val]).val := by rw [hi3, ← hi2v]; scalar_tac
      have hk1 : i3.val * l.nb.val ≤ Usize.max := by
        have := Nat.mul_le_mul_right l.nb.val (show i3.val ≤ l.n_rooms.val by omega)
        omega
      step as ⟨i4, hi4⟩
      have hi4v : i4.val = i3.val * l.nb.val := by rw [hi4, hnbv]
      have hkey : i3.val * l.nb.val + i.val < l.n_rooms.val * l.nb.val := by
        have := Nat.mul_le_mul_right l.nb.val (show i3.val + 1 ≤ l.n_rooms.val by omega)
        rw [Nat.add_mul, Nat.one_mul] at this; omega
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = i3.val * l.nb.val + i.val := by scalar_tac
      have hinl := hins.2.1
      step as ⟨b, hb⟩
      have hbv : b = membRow l s i.val x.val := by
        have hm := memb_bridge l hw s hs hi hle hins hxl
        have e : (s.rooms.val[ufRoot (vals u.parent) x.val]'(hle x.val hxl).1).val * l.nb.val + i.val = i5.val := by
          rw [hi5v, hi3v]; simp only [hh]
        unfold membRow
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hxl, Option.getD_some, ← hm, e, hb,
          List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
      have hrwl : rw.val.length < Usize.max := by
        rw [hrw, List.length_map, List.length_range]; scalar_tac
      step as ⟨rw1, hrw1⟩
      step as ⟨x1, hx1⟩
      have hx1v : x1.val = x.val + 1 := by scalar_tac
      refine ⟨by omega, ⟨hg1, by rw [vals_length, hl1, ← vals_length]; exact hpl, ?_⟩, ?_, by omega⟩
      · rw [funext hr1]; exact hle
      · rw [hrw1, hx1v, List.range_succ, List.map_append, ← hrw, hbv]; simp
    · rename_i hge
      simp only [WP.spec_ok]
      have : x.val = s.rooms.val.length := by scalar_tac
      refine ⟨by simp, by simp, by simp, ?_⟩
      rw [hrw, this]
  · exact ⟨ht, hu, hrow⟩

/-- The Lean's membership table, body by body, over the rooms by position. -/
def membTab (l : Ledger) (s : Split) (k : Nat) : List (List Bool) :=
  (List.range k).map fun i => (List.range s.rooms.val.length).map (membRow l s i)

theorem membership_loop1_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (ins : alloc.vec.Vec Bool) (hins : IInv l ins.val l.inside.val.length)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (i : Usize) (hi : i.val ≤ l.nb.val)
    (hm : memb.val.map (·.val) = membTab l s i.val) :
    membership_loop1 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets l.vert_edges l.vert_sectors s (UScalar.cast .Usize l.nb) ins memb i ⦃ r =>
      r.val.map (·.val) = membTab l s l.nb.val ⦄ := by
  have hsm := hw.small
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  unfold membership_loop1
  apply loop.spec_decr_nat
    (fun (q : Split × alloc.vec.Vec (alloc.vec.Vec Bool) × Usize) => l.nb.val - q.2.2.val)
    (fun q => q.1 = s ∧ q.2.2.val ≤ l.nb.val ∧ q.2.1.val.map (·.val) = membTab l s q.2.2.val)
  · rintro ⟨s', m, j⟩ ⟨rfl, hj, hm'⟩
    unfold membership_loop1.body
    simp only at hj hm' ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.nb.val := by scalar_tac
      step with flood_spec l hw s' hs j hjl as ⟨uf, huf⟩
      step with membership_loop1_loop0_spec l hw s' hs ins hins j hjl uf huf (alloc.vec.Vec.new Bool) 0#usize
        (by simp) (by simp) as ⟨v6, v7, v8, row, h6, h7, h8, hrow⟩
      have hml : m.val.length < Usize.max := by
        have : m.val.length = j.val := by
          have := congrArg List.length hm'; simpa [membTab] using this
        scalar_tac
      step as ⟨m1, hm1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by scalar_tac
      refine ⟨by rw [h6, h7, h8], by omega, ?_, by omega⟩
      rw [hm1, hj1v, List.map_append, hm', membTab, membTab, List.range_succ, List.map_append]
      simp [hrow]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.nb.val := by scalar_tac
      rw [← this]; exact hm'
  · exact ⟨rfl, hi, hm⟩

/-- **`membership` is the Lean's `memb`**, body by body, over the rooms by position. -/
theorem membership_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) :
    membership l s ⦃ r => r.val.map (·.val) = membTab l s l.nb.val ⦄ := by
  unfold membership
  have hsm := hw.small
  have hnr := hsm.2.2.2.2.2
  step as ⟨nb, hnb⟩
  step as ⟨nr, hnr'⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  have hnrv : nr.val = l.n_rooms.val := by rw [hnr']; scalar_tac
  step as ⟨i, hi⟩
  step with alloc.vec.from_elem_spec core.clone.CloneBool false i (by rfl) as ⟨ins, hp1, hp2⟩
  have h0 : IInv l ins.val (0#usize).val := by
    refine ⟨by simp, by rw [hp1, List.length_replicate, hi, hnrv, hnbv], ?_⟩
    intro c hc b hb
    rw [hp1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
    constructor
    · intro h; exfalso; revert h; split <;> simp
    · rintro ⟨j, hj, _⟩; simp at hj
  rw [hnb, hnr']
  step with membership_loop0_spec l hw ins 0#usize h0 as ⟨ins1, hins1⟩
  exact membership_loop1_spec l hw s hs ins1 hins1 _ 0#usize (by simp) (by simp [membTab])

end inc

end forge_bool_core
