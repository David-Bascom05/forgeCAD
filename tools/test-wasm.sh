#!/bin/sh
# The wasm suite, failing if any test binary starts but never reports a result: under load Node has
# been seen to print nothing and exit 0, which would pass a binary whose tests never ran.
set -e
log=$(mktemp)
status=0
cargo test --workspace --target wasm32-wasip1 "$@" 2>&1 | tee "$log" || status=$?
awk '/^ +Running /{if(r!="")print "NO RESULT: " r; r=$0} /^test result/{r=""} END{if(r!="")print "NO RESULT: " r}' "$log" > "$log.missing"
if [ -s "$log.missing" ]; then cat "$log.missing"; status=1; fi
rm -f "$log" "$log.missing"
exit $status
