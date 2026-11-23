export WT=/home/david-bascom/Documents/Github/forgeCAD/.claude/worktrees/agent-a21c1fa16224e6842
export S=/tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov
export CARGO_TARGET_DIR=$WT/target-cov
export RUSTUP_TOOLCHAIN=nightly-2026-09-17
export RUSTFLAGS="-C target-cpu=x86-64-v3 -C instrument-coverage -Z coverage-options=branch"
export LLVM_PROFILE_FILE=$S/prof/stray-%p-%m.profraw
export FORGECAD_CACHE=off
export H=/tmp/claude-1000/forge-heavy.sh
export LB=$HOME/.rustup/toolchains/nightly-2026-09-17-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
