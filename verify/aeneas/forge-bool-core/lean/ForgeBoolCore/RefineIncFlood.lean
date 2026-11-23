import ForgeBoolCore.RefineIncRooms
import ForgeBoolCore.RefineUF

/-!
# Group C: the inc flood

`inc::flood l s i` unites, for every facet that is not a piece of body `i`, the positions in
`s.rooms` of its two rooms. Its result is a union-find whose roots name each class of those pairs
by its least position (`UInv`, from v1's RefineUF), as v1's `flood_spec` states for the plane
ledger.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- A facet's rooms, as ids. -/
def plusR (l : Ledger) (f : Nat) : Nat := (l.facet_rooms.val.getD f (0#u32, 0#u32)).1.val
def minusR (l : Ledger) (f : Nat) : Nat := (l.facet_rooms.val.getD f (0#u32, 0#u32)).2.val

/-- A room's position in `s.rooms`. -/
def posR (s : Split) (r : Nat) : Nat := (s.room_pos.val.getD r NONE).val

/-- Facet `f` is a piece of body `i`, by the split's table. -/
def pieceB (l : Ledger) (s : Split) (f i : Nat) : Bool := s.piece_of.val.getD (f * l.nb.val + i) false

/-- The pairs body `i`'s flood unites over the first `k` facets. -/
def floodE (l : Ledger) (s : Split) (i : Nat) : Nat → List (Nat × Nat)
  | 0 => []
  | k + 1 => floodE l s i k ++
      (if pieceB l s k i then [] else [(posR s (plusR l k), posR s (minusR l k))])

theorem plusR_mem (l : Ledger) {f : Nat} (hf : f < nfacets l) :
    plusR l f ∈ seenRooms l.facet_rooms.val (nfacets l) := by
  simp only [seenRooms, List.mem_flatMap, List.mem_range]
  exact ⟨f, hf, by simp [plusR]⟩

theorem minusR_mem (l : Ledger) {f : Nat} (hf : f < nfacets l) :
    minusR l f ∈ seenRooms l.facet_rooms.val (nfacets l) := by
  simp only [seenRooms, List.mem_flatMap, List.mem_range]
  exact ⟨f, hf, by simp [minusR]⟩

theorem facet_rooms_get (l : Ledger) (hw : LedgerWF2 l) {f : Nat} (hf : f < nfacets l) :
    plusR l f < l.n_rooms.val ∧ minusR l f < l.n_rooms.val := by
  have hl : f < l.facet_rooms.val.length := by rw [hw.rooms.1]; exact hf
  have hm := hw.rooms.2 _ (List.getElem_mem hl)
  have e : l.facet_rooms.val.getD f (0#u32, 0#u32) = l.facet_rooms.val[f] := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl]; rfl
  simp only [plusR, minusR, e]; exact ⟨hm.1, hm.2.1⟩

/-- A room's position is a position, for a room some facet has. -/
theorem pos_lt (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {r : Nat}
    (hr : r < l.n_rooms.val) (hm : r ∈ seenRooms l.facet_rooms.val (nfacets l)) :
    posR s r < s.rooms.val.length :=
  ((hs.rinv.2.2.2 r hr).2 hm).2

/-- There are no more rooms than room ids. -/
theorem rooms_len_le (l : Ledger) (s : Split) (hs : SplitOK l s) :
    s.rooms.val.length ≤ l.n_rooms.val := by
  have hrv := hs.rinv.1
  have hlt := hs.rinv.2.2.1
  have hnd : (s.rooms.val.map (·.val)).Nodup := by rw [hrv]; exact nodup_dedup _
  have hsub : s.rooms.val.map (·.val) ⊆ List.range l.n_rooms.val := by
    intro x hx; rw [hrv, mem_dedup] at hx; exact List.mem_range.mpr (hlt x hx)
  have := (hnd.subperm hsub).length_le
  simpa using this

theorem floodE_in (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (i : Nat) :
    ∀ k, k ≤ nfacets l → ∀ e ∈ floodE l s i k, e.1 < s.rooms.val.length ∧ e.2 < s.rooms.val.length
  | 0, _, e, he => by simp [floodE] at he
  | k + 1, hk, e, he => by
    unfold floodE at he
    rcases List.mem_append.mp he with h | h
    · exact floodE_in l hw s hs i k (by omega) e h
    · split at h
      · simp at h
      · simp at h; subst h
        have hf := facet_rooms_get l hw (f := k) (by omega)
        exact ⟨pos_lt l hw s hs hf.1 (plusR_mem l (by omega)), pos_lt l hw s hs hf.2 (minusR_mem l (by omega))⟩

theorem flood_loop_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (i : Usize) (hi : i.val < l.nb.val) (uf : union_find.Uf) (k : Usize) (hk : k.val ≤ nfacets l)
    (hu : UInv (floodE l s i.val k.val) s.rooms.val.length uf) :
    flood_loop l.facet_class l.facet_rooms s.piece_of s.room_pos i (UScalar.cast .Usize l.nb) uf k ⦃ r =>
      UInv (floodE l s i.val (nfacets l)) s.rooms.val.length r ⦄ := by
  have hsm := hw.small
  have hrl := rooms_len_le l s hs
  have hrpl := hs.rinv.2.1
  have hfrl := hw.rooms.1
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  unfold flood_loop
  apply loop.spec_decr_nat (fun (q : union_find.Uf × Usize) => nfacets l - q.2.val)
    (fun q => q.2.val ≤ nfacets l ∧ UInv (floodE l s i.val q.2.val) s.rooms.val.length q.1)
  · rintro ⟨u, j⟩ ⟨hj, hu'⟩
    unfold flood_loop.body
    simp only at hj hu' ⊢
    split
    · rename_i h
      have hjl : j.val < nfacets l := by simp [nfacets]; scalar_tac
      have hkey : j.val * l.nb.val + i.val < nfacets l * l.nb.val := by
        have := Nat.mul_le_mul_right l.nb.val (show j.val + 1 ≤ nfacets l by omega)
        rw [Nat.add_mul, Nat.one_mul] at this; omega
      step as ⟨i2, hi2⟩
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = j.val * l.nb.val + i.val := by scalar_tac
      have hi3l : i3.val < s.piece_of.val.length := by rw [hs.piece_len, hi3v]; exact hkey
      step as ⟨b, hb⟩
      have hbv : pieceB l s j.val i.val = b := by
        rw [hb, pieceB, ← hi3v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi3l]; rfl
      split
      · rename_i hb1
        step as ⟨j1, hj1⟩
        have : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        rw [this]; unfold floodE; rw [hbv, hb1]; simpa using hu'
      · rename_i hb1
        have hb1' : b = false := by simpa using hb1
        have hbf : pieceB l s j.val i.val = false := by rw [hbv, hb1']
        clear hbv hb1 hb1' hb
        have hjf : j.val < l.facet_rooms.val.length := by rw [hfrl]; exact hjl
        step as ⟨pm, hpm⟩
        obtain ⟨p, m⟩ := pm
        have hpmg : l.facet_rooms.val.getD j.val (0#u32, 0#u32) = (p, m) := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjf]; simp [hpm]
        clear hpm
        have hfr := facet_rooms_get l hw hjl
        have hpv : plusR l j.val = p.val := by unfold plusR; rw [hpmg]
        have hmv : minusR l j.val = m.val := by unfold minusR; rw [hpmg]
        clear hpmg
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = p.val := by scalar_tac
        step as ⟨i5, hi5⟩
        step as ⟨i6, hi6⟩
        have hi6v : i6.val = posR s (plusR l j.val) := by
          rw [hpv, posR, ← hi4v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
          simp only [Option.getD_some]; rw [← hi5]; scalar_tac
        step as ⟨i7, hi7⟩
        have hi7v : i7.val = m.val := by scalar_tac
        step as ⟨i8, hi8⟩
        step as ⟨i9, hi9⟩
        have hi9v : i9.val = posR s (minusR l j.val) := by
          rw [hmv, posR, ← hi7v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
          simp only [Option.getD_some]; rw [← hi8]; scalar_tac
        have hrange := floodE_in l hw s hs i.val j.val (by omega)
        step with union_spec u hu' hrange (by omega) i6 i9
          (by rw [hi6v]; exact pos_lt l hw s hs hfr.1 (plusR_mem l hjl))
          (by rw [hi9v]; exact pos_lt l hw s hs hfr.2 (minusR_mem l hjl)) as ⟨u1, hu1⟩
        step as ⟨j1, hj1⟩
        have : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        rw [this]; unfold floodE; rw [hbf]; simp only [Bool.false_eq_true, ite_false]
        rw [← hi6v, ← hi9v]; exact hu1
    · rename_i h
      have : j.val = nfacets l := by unfold nfacets at hj ⊢; scalar_tac
      rw [this] at hu'; simp only [WP.spec_ok]; exact hu'
  · exact ⟨hk, hu⟩

/-- **`flood` is the union-find of the crossings**: body `i`'s flood names each class of the pairs
across the facets that are not pieces of `i` by its least position in `rooms`. -/
theorem flood_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (i : Usize) (hi : i.val < l.nb.val) :
    flood l s i ⦃ r => UInv (floodE l s i.val (nfacets l)) s.rooms.val.length r ⦄ := by
  unfold flood
  have hsm := hw.small
  have hrl := rooms_len_le l s hs
  step as ⟨nb, hnb⟩
  have hn : (alloc.vec.Vec.len s.rooms).val ≤ U32.max + 1 := by simp; omega
  step with new_spec _ hn as ⟨u, hu⟩
  rw [hnb]
  apply flood_loop_spec l hw s hs i hi u 0#usize (by simp)
  have : (alloc.vec.Vec.len s.rooms).val = s.rooms.val.length := by simp
  unfold UInv; rw [hu, this]; exact inv_range _

end inc

end forge_bool_core
