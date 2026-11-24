# Expansion arithmetic: what is proven, what is false in the paper, and the plan

Status: **on hold** (the master's ruling, after sheet metal moved to Grow-Expansion). This note
scopes the remaining proof work for forge-math's exact predicates (`orient2d_adapt`, incircle's
slow paths), so it can be resumed without redoing the analysis.

## What is proven (verify/ieee754, generic over the format)

`IEEE754/Expansion.lean`:
- the invariant `NO k K l` (nonoverlapping, bottom up, stated with `Mult`) and
  `Nonoverlapping l := ∃ k K, NO k K l`; zeros fit anywhere; any sublist keeps it (`NO_sublist`);
- `NO_sign`: the sign of an expansion is its topmost nonzero component's;
- `two_sum_facts`: TwoSum's error is at most each operand, the pair keeps a shared grid, and the
  error is below a power of two the sum is a multiple of;
- `grow_step`, `grow_chain_NO`, `grow_nonoverlapping`: Grow-Expansion keeps nonoverlapping
  (Shewchuk's Theorem 10). It needs round to nearest only, not ties to even.

`IEEE754/ExpansionSum.lean`:
- `round_tie_even`: a tie lands on an even multiple of the quantum;
- `err_large`: a nonzero error `t` of a rounding of `r` gives `|r| ≥ 2^(q+p−1) + |t|`;
- `two_sum_cor8a`, `two_sum_cor8b`, `two_sum_cor9`: Shewchuk's Corollaries 8 and 9 for TwoSum
  (the error against the sum's size; sum and error nonadjacent under ties to even).

These carry sheet metal's exact signs end to end. verify/aeneas/forge-sheet-core's
`disc_exact` and `line_gap_exact` prove the shipped `forge_sheet_core::exact` (Grow-Expansion
only) exact on its domain, with no assumption.

## Shewchuk's Theorem 13 and Corollary 22 are false as stated

Shewchuk (1997) claims:
- Fast-Expansion-Sum keeps *strongly nonoverlapping* (Theorem 13);
- Scale-Expansion keeps it too, under ties to even (Corollary 22).

Strongly nonoverlapping means: nonoverlapping, every adjacent pair both powers of two, and no
component adjacent to two others. Both claims fail. Each counterexample below was found by
`wclose.py` and `fes.py`, and checked by hand. They use exact rationals and `p = 4`, ties to even;
the same failures appear at p = 5, 6 and 8.

- **Fast-Expansion-Sum** (forge-math's `sum_into`):
  - inputs `e = [9/128]` and `f = [−7/256, −1/16, 1664]`, both strongly nonoverlapping;
  - the merged order is `−7/256, −1/16, 9/128, 1664`;
  - `−7/256 − 1/16 = −23/256` ties to `−3/32`, error `1/256`;
  - then `−3/32 + 9/128 = −3/128` exactly, error 0;
  - then `−3/128 + 1664` rounds to `1664`, error `−3/128`;
  - output `[1/256, −3/128, 1664]`: `1/256` and `−3/128` are adjacent (high bit `2⁻⁸`, low bit
    `2⁻⁷`), and `−3/128` is not a power of two.

  The offending pair is separated by a zero error. That is exactly the case the paper's footnote 5
  sets aside ("the proof could be written to consider the case where intervening zeros appear, but
  this would make it even more convoluted. Trust me").
- **Scale-Expansion** (forge-math's `scale_into`), from `scale_cex.py`:
  - input `e = [−3, 128, 512, 2048, −4096]`, which is strongly nonoverlapping (2048 and 4096 are
    an adjacent pair of powers of two), and `b = 13`;
  - output `[1, −40, 128, 2048, −20480]`;
  - the step `8192 + 26624 = 34816` ties to `32768`, error `2048`. Then `32768 − 53248 = −20480`
    exactly;
  - `2048` and `−20480` are adjacent, and `−20480` is not a power of two.

Neither counterexample breaks the *sum* or the *sign*: outputs stay nonoverlapping, and the top
component's sign is the sum's.

## The invariant that holds: W2

**W2**: nonoverlapping, and in every adjacent pair the lower component is a power of two.
Equivalently, each nonzero component is at most half the lowest set bit of the next nonzero one:
`|x| ≤ 2^(ν(y) − 1)`, against plain nonoverlapping's `|x| < 2^ν(y)`.

Empirically W2 is closed under both operations:
- no failure in 1.2 million random cases;
- inputs drawn from W2 by a generator biased toward adjacency and powers of two;
- p = 4, 5, 6, 8;
- both `sum_into` and `scale_into` (`wclose.py`);
- outputs also recirculated as inputs (`fast.py`).
The sign is right in every case. Plain nonoverlapping is not closed: Shewchuk's own example is
`11110000 + 1111 + 0.1111` added to itself. Strongly nonoverlapping is not closed either (above).
**W2 closure is a conjecture**, not a published result.

The predicates' expansions all start as nonadjacent pairs, from TwoSum, TwoDiff or TwoProduct.
Nonadjacent implies W2. So W2 closure would cover every expansion orient2d_adapt and incircle
form. The sign would then follow from `NO_sign`, which needs only nonoverlapping.

## Plan to prove W2 closure

Each step builds and is reviewed by the ieee754 session.

1. **Definitions.** `W2` in grid form, like `NO`: the head is a multiple of `2^k`, at most
   `2^(k'−1)`, and the rest is `W2 k'`. Prove sublist closure (zero elimination), and the sum
   bound below a component (`< 2^ν`, as for NO).
2. **Fast-Expansion-Sum keeps W2.** This is Shewchuk's Lemma 16, generalized to zero runs and
   to W2. The step to prove: when a nonzero error `t` follows the last nonzero error `τ`
   (`2^η ≤ |τ| < 2^(η+1)`), `t` is a multiple of `2^(η+1)`, and of `2^(η+2)` unless `τ` is a
   power of two. Also, the final sum relates to `τ` the same way. The proof by contradiction:
   - the sum `Q` produced with `τ` is a multiple of `2^(η+2)` (`two_sum_cor9`);
   - the steps between `τ` and `t` are exact, so some later input `G` has a low bit at or below
     the offending one;
   - a Lemma-15-type bound on the merged prefix, applied to a *subseries* (prefix plus `G`),
     contradicts `two_sum_cor8a`'s lower bound on `|Q + τ|`.

   Shewchuk's Lemma 15 uses strong nonoverlap (a bit gap below every non-power-of-two). Under W2
   the gap is gone, so the bound must be redone. The finer fact available: the lower of an
   adjacent pair is a single bit, so the sum below `y` is under `2^(ν(y)−1) + 2^(ν(y)−2)·…` only
   in the power-of-two chain case. This is the open part. Expect a case analysis on whether `τ`
   and `G` are powers of two.
3. **Scale-Expansion keeps W2.** This is Shewchuk's Lemmas 20 and 21 (the errors against
   `2^r ulp(b)`) and his nonoverlap argument, which already covers plain nonoverlapping. Then the
   adjacency cases of Corollary 22, redone for W2. TwoProduct's exactness on the grid is
   `two_product_exact`.
4. **Refinement.** The aeneas session's list models are done and proven against the shipped
   Rust, on `claude/repo-review-mbdrlj-aeneas` (`ForgeMath/Expansion.lean`, 5df1151):
   - `sumIntoL`, `scaleL`, `mergeL` (with `mergeL_perm` and `mergeL_sorted`), `estimateL`;
   - `sum_into_spec`, `scale_into_spec`, `estimate_spec`.

   Steps 2 and 3 should be stated on those models: `sumIntoL` is `growChain` over `mergeL` with
   zeros dropped. Then `orient2d_adapt`'s sign theorem composes them.

Size: steps 1–3 are likely 1,500+ lines of Lean, of which step 2's bound is the research risk.

## Rerun

`python3 wclose.py P SEED N CLASS OP` (CLASS ∈ {SNO, W1, W2}, OP ∈ {fes, scale}) tests closure;
`python3 fes.py P SEED N` runs Fast-Expansion-Sum on random strongly nonoverlapping inputs;
`python3 scale_cex.py` prints the Scale-Expansion counterexample in exact rationals. Pure Python
3, no dependencies.
