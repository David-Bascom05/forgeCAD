#!/bin/bash
# Build the Lean model and its proofs, then test the kernel's check against it.
set -e
cd "$(dirname "$0")/lean"
export PATH=$HOME/.elan/bin:$PATH
lake build
cd ..
cargo test -p forge-topo-check --test diff -- --include-ignored --nocapture
