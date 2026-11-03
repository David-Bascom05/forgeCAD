#!/bin/sh
# fg run PART --json, summarised: ok, time, errors, warnings, expects
F=$HOME/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg
for f in "$@"; do
  $F run "$f" --json | python3 -c "
import json,sys;d=json.load(sys.stdin);print('$f', 'ok' if d['ok'] else 'FAIL', d['timings_ms'].get('total'))
for e in d['errors']: print('  E',e.get('kind'),e.get('text'))
for e in d['warnings']: print('  W',e.get('kind'),e.get('severity'),e.get('text') or '')
for e in d['expects']: print('  X',e)"
done
