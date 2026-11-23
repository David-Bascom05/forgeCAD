#!/bin/sh
# pipe_run.sh EXE DIR OUT.jsonl: every .fg part in DIR through EXE (cliffcheck_fg: one JSON line a
# part), three at a time, one process each, sorted by file into OUT.jsonl
exe=$1; dir=$2; out=$3
: > "$out.tmp"
ls "$dir"/*.fg | xargs -P 3 -I{} sh -c 'r=$(timeout 300 '"$exe"' {} 2>/dev/null | tail -1); [ -n "$r" ] || r="{\"v\":\"crash\",\"sig\":\"\"}"; f=$(basename {}); echo "$r" | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); d[\"file\"]=\"$f\"; print(json.dumps(d))"' >> "$out.tmp"
python3 -c "
import json,sys
rows=sorted((json.loads(l) for l in open('$out.tmp')), key=lambda r:r['file'])
open('$out','w').write(''.join(json.dumps(r)+'\n' for r in rows))"
rm "$out.tmp"
