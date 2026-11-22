# Roadmap: a proven kernel (2026-11-22)

Ordered by dependency. Every merge passes: workspace and app tests, the callgrind speed gate (no line
slower), the reliability scoreboard (0 worse), FUZZ_SEEDS=150, metamorphic checks, native = wasm
bits. Work happens on session branches `claude/repo-review-mbdrlj-*`, is merged into
`claude/repo-review-mbdrlj`, and reaches `main` by pull request.

1. **First merge to main.** The .fc engine deleted, .fg everywhere, today's kernel and app fixes,
   the Lean proofs, the exact boolean reference.
2. **Independent fixes.**
   - Kernel: the probe step, n-ary sliver pruning, touch/thickness (corner pinch, false
     ZeroThickness), the slit edge, the drafted lid (G5).
   - Language: Suppress, Respell and history-id collisions.
   - Measurement and proof: the decision log, the snap-rule comparison.
3. **Decide-once boolean, flat faces.**
   - Each fact decided once, under the proven tolerance rule (research/kernel-audit/README.md §6.1),
     in the representative surface; routines look facts up.
   - Retry rungs retired as their tests pass without them. Acceptance tests are the six rescues in
     research/kernel-audit/decisions.md.
   - Verified by strict mode (no fact with two answers), the coverage proof
     (research/kernel-audit/coverage.md) and the certificate checker.
   - Removes tolerance band 1 and the near-coincident family.
4. **Curved surfaces.** The exact reference, certificate checker and decide-once extended to
   quadrics, then tori and offsets. Certified predicates proven with verify/ieee754 and Aeneas.
5. **Fillets as one definition.** The closing with a ball, from the offset body's crease graph
   (research/fillets/general-method.md), on the proven boolean. Judged by the fillet oracle and the
   census.

In parallel:
- .fg build speed until every part is no slower than .fc (tools/speed/fc-reference.txt);
- the one-pass parser standalone (proven equal to the table parser);
- Aeneas proofs of kernel predicates;
- the certificate checker's proof.
