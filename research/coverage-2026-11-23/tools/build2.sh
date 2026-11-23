#!/bin/bash
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT/verify/forge-exact
export PATH=$HOME/.elan/bin:$PATH
time lake build forge-exact sweep-ref
ls -la .lake/build/bin
echo BUILD2 DONE
