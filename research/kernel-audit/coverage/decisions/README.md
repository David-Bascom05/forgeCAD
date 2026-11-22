# Band runs through the decision log

`box_decisions.rs` is an example for verify/forge-exact that needs forge-bool's `decisions`
feature, which lives only on branch claude/repo-review-mbdrlj-decisions (used read-only, not merged
here). To run it:

```
git worktree add --detach /tmp/dec origin/claude/repo-review-mbdrlj-decisions
cp -r verify/forge-exact /tmp/dec/verify/
cp research/kernel-audit/coverage/decisions/box_decisions.rs /tmp/dec/verify/forge-exact/examples/
printf '\n[features]\ndecisions = ["forge-bool/decisions"]\n' >> /tmp/dec/verify/forge-exact/Cargo.toml
cd /tmp/dec && BAND_AXES=01 cargo run --release -p forge-exact --features decisions --example box_decisions -- 0.85
```

Its outputs are `results/decisions-*.txt`. Line numbers there are the decisions branch's; coverage.md
maps them to HEAD's.
