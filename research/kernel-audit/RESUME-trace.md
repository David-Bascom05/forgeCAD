# Resume: certified tracing (trace worker)

Branch claude/repo-review-mbdrlj-trace. Design and results are in certified-tracing.md: §11 the
certificate in parameter space, §12 the trace built in the charts with no march, §12.1 empty
traces by the march's octree and the exact counts.

## State
- Feature `certified_trace`, off by default. Turning it on by default waits on every user-facing
  line being at most off at equal threads (latency-gate, wall-clock).
- Exact callgrind counts: 10 of 15 parts are at or below off. crank (+0.31%) and volute (+0.07%)
  are over by codegen: fat-LTO inlining moved by the new code, with the same work. bracket is
  +0.08% (memo and wrapper). See §12.1.

## Parked
- **(2) Crossing sites in the charts** (pipe mitres: two equal-radius cylinders with crossing
  axes). Parked by the master on 2026-11-23: it now buys only coverage, not speed. Such pairs
  certify through the 3D path (Lagrange sites, Krawczyk steps). The chart path returns None
  wherever a leaf is none of its kinds at the least size, which includes every crossing.

## Next
- (3) Aeneas L4 for forge-math interval.rs and the certificate's core. The toolchain is built at
  ~/.cache/forge-aeneas (tools/aeneas/setup.sh from claude/repo-review-mbdrlj-aeneas). Where
  Aeneas cannot take certify.rs, a differential test with a committed digest, marked interim.
- Review fix-sheet's isolate.rs and Interval::atan2 (ecd0dc1b on claude/repo-review-mbdrlj-fix-sheet),
  then move krawczyk::<N> and inverse::<N> next to them.
