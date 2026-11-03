# Rack subsystem agents: common rules
- The spec is law: ~/projects/forgecad-gate/rack18u/SPEC.md (read ALL of it, especially sections 1, 3, 5, 6, 8, 9, 10, 12).
  Part frames, anchors and names exactly as section 5 gives them. If a SPEC number is wrong or inconsistent, don't
  silently change it: write it in your README under "SPEC issues" and tell the coordinator.
- Forge: build fg (and forgecad) in ~/projects/ForgeCAD/.claude/worktrees/forge-lang:
  `fbuild nice cargo build --release -p forge-lang -p forge-cli` (binaries in its target/release). Read `fg reference`,
  interface/forge-lang/tests/assemblies.rs (assembly syntax, anchors, fits, expects) and the sheet parts in
  ~/projects/forgecad-gate/sheet-metal/complex-parts/ (working sheet metal examples).
- Hardware comes from rack18u/hw/ (built by the hardware agent in parallel). Until a file you need exists, model your
  own parts first and wire hardware in when it lands; don't create your own copies of hardware.
- Every part lint-clean, with `expect` lines from design intent, typed shop violations zero (or each one justified
  in your README), and its DXF (sheet parts) + render written. Your test assembly passes its expects: zero
  unintended interference, fits applied, clearances.
- Forge bugs/gaps: work around only as SPEC section 10 says; otherwise write a minimal repro under
  <sub>/probes/ and report it. Don't edit the ForgeCAD repo.
- Commit in the rack18u local git (configured as David Bascom) often, with only your subsystem's files.
- Heavy commands through `fbuild`; check `free -g`; no rm before 12:00 UTC; kill by PID only.
- Report: parts done, test results, renders, SPEC issues, Forge gaps.
- MEMORY (added after a 9.5 GB python script ran in the terminal's cgroup): run EVERY script or generator that can
  grow (python gen/run scripts, batch fg runs) as `systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0 nice ...`
  or through fbuild, never bare in your shell.
