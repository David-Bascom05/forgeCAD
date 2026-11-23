#!/bin/bash
D=/tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov
$D/run_a.sh > $D/logs/run_a.log 2>&1; echo "A done"
$D/run_b.sh > $D/logs/run_b.log 2>&1; echo "B done"
$D/build2.sh > $D/logs/build2.log 2>&1; echo "lake done"
$D/run_c.sh > $D/logs/run_c.log 2>&1; echo "C done"
$D/run_d.sh > $D/logs/run_d.log 2>&1; echo "D done"
echo ALL DONE
