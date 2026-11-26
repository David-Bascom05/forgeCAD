import ForgeTopo.ShellFaces
open Aeneas Aeneas.Std Result

/-!
# The shipped `topo_edges::edges` and `loops` refine the model's `checkEdges` and `checkLoops`

kernel/forge-topo/src/topo_edges.rs (check.rs's `check_edges` and `check_loops` call it) as Aeneas
translates it. Its one geometric test, `edge_degenerate` (floats), is the instance's: whatever it
answers, the problem it adds is not one the model decides, and the sink's model view (`P`)
leaves it out.
-/

set_option maxRecDepth 8192

namespace forge_topo

open TopoCheck

/-- What an `EdgeTopo` instance reads of model body `b`. -/
structure EdgeReads {T : Type} (EI : topo_edges.EdgeTopo T) (t : T) (b : Body) : Prop where
  base : Reads EI.topoTopoInst t b
  coedge_forward : ∀ c : Usize, c.val < b.coedges.size → EI.coedge_forward t c ⦃ x => x = (b.co c.val).fwd ⦄
  edge_degenerate : ∀ e : Usize, e.val < b.edges.size → EI.edge_degenerate t e ⦃ _ => True ⦄
  /-- Every loop's coedges, and their edges, are in range (those its face no longer lists too). -/
  loop_range : ∀ l < b.loops.size, ∀ c ∈ b.loopCoedges l, c < b.coedges.size ∧ (b.co c).edge < b.edges.size

/-- A sink that appends each edge and loop problem it is handed (the degenerate edge: nothing the
model decides). -/
def AppendsE {S : Type} (SK : topo.Sink S) (P : S → List Problem) : Prop :=
  (∀ o (e n : Usize), SK.edge_uses o e n ⦃ o' => P o' = P o ++ [Problem.edgeUses e.val n.val] ⦄) ∧
  (∀ o (e : Usize), SK.edge_same_way o e ⦃ o' => P o' = P o ++ [Problem.edgeUsesSameWay e.val] ⦄) ∧
  (∀ o (e : Usize), SK.edge_degenerate o e ⦃ o' => P o' = P o ⦄) ∧
  (∀ o (f : Usize), SK.face_no_loops o f ⦃ o' => P o' = P o ++ [Problem.faceNoLoops f.val] ⦄) ∧
  (∀ o (l : Usize), SK.loop_empty o l ⦃ o' => P o' = P o ++ [Problem.loopEmpty l.val] ⦄) ∧
  (∀ o (l c : Usize), SK.loop_broken o l c ⦃ o' => P o' = P o ++ [Problem.loopBroken l.val c.val] ⦄)

variable {T : Type} (EI : topo_edges.EdgeTopo T) (t : T) (b : Body)

/-- The model's problems at edge `e`. -/
def edgeProbs (e : Nat) : List Problem :=
  match (b.ed e).uses with
  | [x, y] => if (b.co x).fwd == (b.co y).fwd then [.edgeUsesSameWay e] else []
  | us => [.edgeUses e us.length]

theorem flatMap_range_succ {α} (n : Nat) (g : Nat → List α) :
    (List.range (n + 1)).flatMap g = (List.range n).flatMap g ++ g n := by
  rw [List.range_succ, List.flatMap_append]; simp

theorem edges_refines {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsE SK P)
    (R : EdgeReads EI t b) (hL : b.checkLinks = []) (out : S) :
    topo_edges.edges EI SK t out ⦃ r => P r = P out ++ b.checkEdges ⦄ := by
  have H := sound_of b hL
  unfold topo_edges.edges
  have rne := R.base.ne
  step with rne as ⟨ne, hne⟩
  unfold topo_edges.edges_loop
  apply loop.spec_decr_nat (fun (y : S × Usize) => ne.val - y.2.val)
    (fun y => y.2.val ≤ ne.val ∧ P y.1 = P out ++ (List.range y.2.val).flatMap (edgeProbs b))
  · rintro ⟨o, e⟩ ⟨hle, ho⟩
    simp only at hle ho ⊢
    unfold topo_edges.edges_loop.body
    split
    · rename_i hlt
      have hlt' : e.val < ne.val := by scalar_tac
      have he : e.val < b.edges.size := by omega
      have hw : (e.wrapping_add 1#usize).val = e.val + 1 := succ_val e ne hlt'
      have hacc : ∀ o', P o' = P o ++ edgeProbs b e.val →
          P o' = P out ++ (List.range (e.val + 1)).flatMap (edgeProbs b) := by
        intro o' h; rw [h, flatMap_range_succ, ho, List.append_assoc]
      step with R.base.edge_alive e he as ⟨al, hal⟩
      rw [hal]; simp only [ite_true]
      apply WP.spec_bind (Pₘ := fun o' => P o' = P o ++ edgeProbs b e.val)
      · step with R.base.edge_n_uses e he as ⟨n, hn⟩
        split
        · rename_i hn2
          have hn2' : n.val ≠ 2 := by
            intro h; have : n = 2#usize := UScalar.eq_of_val_eq h
            rw [this] at hn2; simp at hn2
          have hp : edgeProbs b e.val = [Problem.edgeUses e.val (b.ed e.val).uses.length] := by
            unfold edgeProbs; split
            · rename_i x y hu; rw [hu] at hn; simp at hn; omega
            · rfl
          apply WP.spec_mono (hS.1 o e n)
          intro o' h; rw [h, hp, hn]
        · rename_i hn2
          have hn2' : n.val = 2 := by
            simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hn2; rw [hn2]; rfl
          match hu : (b.ed e.val).uses with
          | [x, y] =>
            have hx : x ∈ (b.ed e.val).uses := by rw [hu]; simp
            have hy : y ∈ (b.ed e.val).uses := by rw [hu]; simp
            have hxn := (H.uses e.val he x hx).1
            have hyn := (H.uses e.val he y hy).1
            step with R.base.edge_use e 0#usize he (by rw [hu]; simp) as ⟨c0, hc0⟩
            have hc0' : c0.val = x := by rw [hc0, hu]; rfl
            step with R.coedge_forward c0 (by rw [hc0']; exact hxn) as ⟨b1, hb1⟩
            step with R.base.edge_use e 1#usize he (by rw [hu]; simp) as ⟨c1, hc1⟩
            have hc1' : c1.val = y := by rw [hc1, hu]; rfl
            step with R.coedge_forward c1 (by rw [hc1']; exact hyn) as ⟨b2, hb2⟩
            have hprobs : edgeProbs b e.val = if (b.co x).fwd == (b.co y).fwd then [.edgeUsesSameWay e.val] else [] := by
              unfold edgeProbs; rw [hu]
            apply WP.spec_bind (Pₘ := fun o' => P o' = P o ++ edgeProbs b e.val)
            · split
              · rename_i hbb
                apply WP.spec_mono (hS.2.1 o e)
                intro o' h; rw [h, hprobs, ← hc0', ← hc1', ← hb1, ← hb2, hbb]; simp
              · rename_i hbb
                simp only [WP.spec_ok]
                rw [hprobs, ← hc0', ← hc1', ← hb1, ← hb2]
                have : (b1 == b2) = false := by simp [hbb]
                rw [this]; simp
            · intro o2 ho2
              step with R.edge_degenerate e he as ⟨dg⟩
              split
              · apply WP.spec_mono (hS.2.2.1 o2 e)
                intro o' h; rw [h, ho2]
              · simp only [WP.spec_ok]; exact ho2
          | [] => rw [hu] at hn; simp at hn; omega
          | [x] => rw [hu] at hn; simp at hn; omega
          | x :: y :: z :: w => rw [hu] at hn; simp at hn; omega
      · intro o' ho'
        step with usize_add_wspec e 1#usize (succ_le_max e ne hlt') as ⟨e1', he1'⟩
        subst he1'
        try simp only [WP.spec_ok]
        rw [hw]
        exact ⟨by omega, hacc o' ho', by omega⟩
    · rename_i hlt
      have : e.val = b.edges.size := by scalar_tac
      simp only [WP.spec_ok]
      rw [ho, this, Body.checkEdges]; rfl
  · exact ⟨by simp, by simp⟩

theorem coedge_start_spec (R : EdgeReads EI t b) (c : Usize) (hc : c.val < b.coedges.size)
    (he : (b.co c.val).edge < b.edges.size) : topo_edges.coedge_start EI t c ⦃ v => v.val = b.start c.val ⦄ := by
  unfold topo_edges.coedge_start
  step with R.base.coedge_edge c hc as ⟨e, hce⟩
  have he' : e.val < b.edges.size := by rw [hce]; exact he
  step with R.base.edge_ends e he' as ⟨a, a2, ha, ha2⟩
  step with R.coedge_forward c hc as ⟨fw, hfw⟩
  unfold Body.start; rw [← hfw, ← hce]
  split <;> simp_all

theorem coedge_end_spec (R : EdgeReads EI t b) (c : Usize) (hc : c.val < b.coedges.size)
    (he : (b.co c.val).edge < b.edges.size) : topo_edges.coedge_end EI t c ⦃ v => v.val = b.stop c.val ⦄ := by
  unfold topo_edges.coedge_end
  step with R.base.coedge_edge c hc as ⟨e, hce⟩
  have he' : e.val < b.edges.size := by rw [hce]; exact he
  step with R.base.edge_ends e he' as ⟨a, a2, ha, ha2⟩
  step with R.coedge_forward c hc as ⟨fw, hfw⟩
  unfold Body.stop; rw [← hfw, ← hce]
  split <;> simp_all

theorem mod_succ (i n : Nat) (h : i < n) : (i + 1) % n = if i + 1 < n then i + 1 else 0 := by
  split
  · exact Nat.mod_eq_of_lt (by assumption)
  · have : i + 1 = n := by omega
    rw [this, Nat.mod_self]

/-- The model's problems at loop `l` (its first clause aside). -/
def loopProbs (l : Nat) : List Problem :=
  let lp := b.lo l
  if lp.len == 0 then [.loopEmpty l]
  else (List.range lp.len).filterMap fun i =>
    if b.stop (lp.first + i) != b.start (lp.first + (i + 1) % lp.len)
    then some (.loopBroken l (lp.first + i)) else none

theorem broken_spec {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsE SK P)
    (R : EdgeReads EI t b) (l first n : Usize) (hl : l.val < b.loops.size)
    (hfirst : first.val = (b.lo l.val).first) (hn : n.val = (b.lo l.val).len) (out : S) :
    topo_edges.loops_loop1_loop0 EI SK t out l first n 0#usize ⦃ r =>
      P r = P out ++ (List.range n.val).filterMap fun i =>
        if b.stop (first.val + i) != b.start (first.val + (i + 1) % n.val)
        then some (Problem.loopBroken l.val (first.val + i)) else none ⦄ := by
  have hin : ∀ i < n.val, first.val + i < b.coedges.size ∧ (b.co (first.val + i)).edge < b.edges.size := by
    intro i hi
    exact R.loop_range l.val hl _ (by rw [mem_loopCoedges]; omega)
  unfold topo_edges.loops_loop1_loop0
  apply loop.spec_decr_nat (fun (y : S × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ P y.1 = P out ++ (List.range y.2.val).filterMap fun i =>
        if b.stop (first.val + i) != b.start (first.val + (i + 1) % n.val)
        then some (Problem.loopBroken l.val (first.val + i)) else none)
  · rintro ⟨o, i⟩ ⟨hle, ho⟩
    simp only at hle ho ⊢
    unfold topo_edges.loops_loop1_loop0.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hw : (i.wrapping_add 1#usize).val = i.val + 1 := succ_val i n hlt'
      have hci := hin i.val hlt'
      have hadd : ∀ k : Usize, first.val + k.val < b.coedges.size → (first.wrapping_add k).val = first.val + k.val := by
        intro k hk; rw [Usize.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
        obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
        have := m.hBounds; simp only [UScalar.size]; omega
      step with usize_add_wspec i 1#usize (succ_le_max i n hlt') as ⟨i1', hi1'⟩
      subst hi1'
      have hj : ∀ j : Usize, j.val = (i.val + 1) % n.val →
          (first.wrapping_add j).val = first.val + (i.val + 1) % n.val ∧
            first.val + (i.val + 1) % n.val < b.coedges.size ∧
            (b.co (first.val + (i.val + 1) % n.val)).edge < b.edges.size := by
        intro j hjv
        have := hin ((i.val + 1) % n.val) (Nat.mod_lt _ (by omega))
        exact ⟨by rw [hadd j (by rw [hjv]; exact this.1), hjv], this⟩
      apply WP.spec_bind (Pₘ := fun j : Usize => j.val = (i.val + 1) % n.val)
      · split
        · rename_i h1
          simp only [WP.spec_ok]
          rw [hw, mod_succ i.val n.val hlt', if_pos (by rw [← hw]; scalar_tac)]
        · rename_i h1
          simp only [WP.spec_ok]
          rw [mod_succ i.val n.val hlt', if_neg (by rw [← hw]; scalar_tac)]; rfl
      · intro j hjv
        have ⟨hjw, hjn, hje⟩ := hj j hjv
        have hiw := hadd i hci.1
        obtain ⟨m, hm⟩ := WP.spec_exists R.base.nc
        step with usize_add_wspec first i (le_max_of_lt_val m _ (by have := m.hBounds; omega)) as ⟨i2', hi2'⟩
        subst hi2'
        step with coedge_end_spec EI t b R (first.wrapping_add i) (by rw [hiw]; exact hci.1)
          (by rw [hiw]; exact hci.2) as ⟨v1, hv1⟩
        step with usize_add_wspec first j (le_max_of_lt_val m _ (by rw [hjv]; omega)) as ⟨i4', hi4'⟩
        subst hi4'
        step with coedge_start_spec EI t b R (first.wrapping_add j) (by rw [hjw]; exact hjn)
          (by rw [hjw]; exact hje) as ⟨v2, hv2⟩
        have hacc : ∀ o', P o' = P o ++ (if b.stop (first.val + i.val) != b.start (first.val + (i.val + 1) % n.val)
            then some (Problem.loopBroken l.val (first.val + i.val)) else none).toList →
            P o' = P out ++ (List.range (i.val + 1)).filterMap fun i =>
              if b.stop (first.val + i) != b.start (first.val + (i + 1) % n.val)
              then some (Problem.loopBroken l.val (first.val + i)) else none := by
          intro o' h; rw [h, filterMap_range_succ, ho, List.append_assoc]
        rw [hiw] at hv1; rw [hjw] at hv2
        split
        · rename_i hne
          have hne' : b.stop (first.val + i.val) ≠ b.start (first.val + (i.val + 1) % n.val) := by
            rw [← hv1, ← hv2]; intro h; have := UScalar.eq_of_val_eq h; rw [this] at hne; simp at hne
          step with hS.2.2.2.2.2 o l (first.wrapping_add i) as ⟨o1, ho1⟩
          try simp only [WP.spec_ok]
          rw [hw]
          refine ⟨by omega, hacc o1 ?_, by omega⟩
          rw [ho1, hiw]; simp [hne']
        · rename_i hne
          have hne' : b.stop (first.val + i.val) = b.start (first.val + (i.val + 1) % n.val) := by
            simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne; rw [← hv1, ← hv2, hne]
          simp only [Std.bind_ok, WP.spec_ok]
          rw [hw]
          refine ⟨by omega, hacc o ?_, by omega⟩
          simp [hne']
    · rename_i hlt
      have : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [ho, this]
  · exact ⟨by simp, by simp⟩

theorem loops_refines {S : Type} (SK : topo.Sink S) (P : S → List Problem) (hS : AppendsE SK P)
    (R : EdgeReads EI t b) (hL : b.checkLinks = []) (out : S) :
    topo_edges.loops EI SK t out ⦃ r => P r = P out ++ b.checkLoops ⦄ := by
  have W := b.links_ok hL
  unfold topo_edges.loops
  have rnf := R.base.nf
  have rnl := R.base.nl
  step with rnf as ⟨nf, hnf⟩
  -- the faces with no loop
  apply WP.spec_bind (Pₘ := fun o1 => P o1 = P out ++ (List.range b.faces.size).filterMap fun f =>
      if (b.faceLoops f).isEmpty then some (Problem.faceNoLoops f) else none)
  · unfold topo_edges.loops_loop0
    apply loop.spec_decr_nat (fun (y : S × Usize) => nf.val - y.2.val)
      (fun y => y.2.val ≤ nf.val ∧ P y.1 = P out ++ (List.range y.2.val).filterMap fun f =>
        if (b.faceLoops f).isEmpty then some (Problem.faceNoLoops f) else none)
    · rintro ⟨o, f⟩ ⟨hle, ho⟩
      simp only at hle ho ⊢
      unfold topo_edges.loops_loop0.body
      split
      · rename_i hlt
        have hlt' : f.val < nf.val := by scalar_tac
        have hf : f.val < b.faces.size := by omega
        have hw : (f.wrapping_add 1#usize).val = f.val + 1 := succ_val f nf hlt'
        step with R.base.face_alive f hf as ⟨al, hal⟩
        rw [hal]; simp only [ite_true]
        step with R.base.face_n_loops f hf as ⟨n, hn⟩
        apply WP.spec_bind (Pₘ := fun o' => P o' = P o ++ (if (b.faceLoops f.val).isEmpty
            then some (Problem.faceNoLoops f.val) else none).toList)
        · split
          · rename_i h0
            have he : (b.faceLoops f.val).isEmpty = true := by
              rw [List.isEmpty_iff, ← List.length_eq_zero_iff, ← hn, h0]; rfl
            apply WP.spec_mono (hS.2.2.2.1 o f); intro o' h; rw [h, he]; simp
          · rename_i h0
            have he : (b.faceLoops f.val).isEmpty = false := by
              rw [Bool.eq_false_iff]; intro h
              rw [List.isEmpty_iff, ← List.length_eq_zero_iff, ← hn] at h
              exact h0 (UScalar.eq_of_val_eq h)
            simp only [WP.spec_ok]; rw [he]; simp
        · intro o' ho'
          step with usize_add_wspec f 1#usize (succ_le_max f nf hlt') as ⟨f1', hf1'⟩
          subst hf1'
          try simp only [WP.spec_ok]
          rw [hw]
          refine ⟨by omega, ?_, by omega⟩
          rw [ho', filterMap_range_succ, ho, List.append_assoc]
      · rename_i hlt
        have : f.val = b.faces.size := by scalar_tac
        simp only [WP.spec_ok]
        rw [ho, this]
    · exact ⟨by simp, by simp⟩
  -- the loops
  intro o1 ho1
  step with rnl as ⟨nl, hnl⟩
  unfold topo_edges.loops_loop1
  apply WP.spec_mono (P₀ := fun o => P o = P o1 ++ (List.range b.loops.size).flatMap (loopProbs b))
  · apply loop.spec_decr_nat (fun (y : S × Usize) => nl.val - y.2.val)
      (fun y => y.2.val ≤ nl.val ∧ P y.1 = P o1 ++ (List.range y.2.val).flatMap (loopProbs b))
    · rintro ⟨o, l⟩ ⟨hle, ho⟩
      simp only at hle ho ⊢
      unfold topo_edges.loops_loop1.body
      split
      · rename_i hlt
        have hlt' : l.val < nl.val := by scalar_tac
        have hl : l.val < b.loops.size := by omega
        have hw : (l.wrapping_add 1#usize).val = l.val + 1 := succ_val l nl hlt'
        have hacc : ∀ o', P o' = P o ++ loopProbs b l.val →
            P o' = P o1 ++ (List.range (l.val + 1)).flatMap (loopProbs b) := by
          intro o' h; rw [h, flatMap_range_succ, ho, List.append_assoc]
        step with R.base.loop_face l hl as ⟨g, hg⟩
        have hgf : g.val < b.faces.size := by rw [hg]; exact (W.listed_le l.val hl).1
        step with R.base.face_alive g hgf as ⟨al, hal⟩
        rw [hal]; simp only [ite_true]
        step with R.base.loop_first l hl as ⟨first, hfirst⟩
        step with R.base.loop_len l hl as ⟨n, hn⟩
        split
        · rename_i h0
          have h0' : (b.lo l.val).len = 0 := by rw [← hn, h0]; rfl
          step with hS.2.2.2.2.1 o l as ⟨o2, ho2⟩
          step with usize_add_wspec l 1#usize (succ_le_max l nl hlt') as ⟨l1', hl1'⟩
          subst hl1'
          try simp only [WP.spec_ok]
          rw [hw]
          refine ⟨by omega, hacc o2 ?_, by omega⟩
          rw [ho2]; unfold loopProbs; simp [h0']
        · rename_i h0
          have h0' : (b.lo l.val).len ≠ 0 := by
            intro h; apply h0; apply UScalar.eq_of_val_eq; rw [hn, h]; rfl
          step with broken_spec EI t b SK P hS R l first n hl hfirst hn o as ⟨o2, ho2⟩
          step with usize_add_wspec l 1#usize (succ_le_max l nl hlt') as ⟨l1', hl1'⟩
          subst hl1'
          try simp only [WP.spec_ok]
          rw [hw]
          refine ⟨by omega, hacc o2 ?_, by omega⟩
          rw [ho2]; unfold loopProbs
          have : ((b.lo l.val).len == 0) = false := by simp [h0']
          simp only [this, Bool.false_eq_true, ite_false]
          rw [hfirst, hn]
      · rename_i hlt
        have : l.val = b.loops.size := by scalar_tac
        simp only [WP.spec_ok]
        rw [ho, this]
    · exact ⟨by simp, by simp⟩
  · intro r hr
    rw [hr, ho1, Body.checkLoops, List.append_assoc]; rfl

end forge_topo
