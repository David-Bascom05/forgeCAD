#!/bin/sh
# Sets up one trial directory: usage run.sh OUT FORGECAD AGENTS.md.
# The agent sees AGENTS.md and ./forgecad, a wrapper logging each call to calls.log with its output;
# score.sh counts them. The prompt given to a fresh agent, once per task in tasks.txt:
#   Work only in OUT. Read AGENTS.md there and use ./forgecad, nothing else of the repository.
#   TASK. End with the numbers asked for and nothing more.
set -e
mkdir -p "$1"
cp "$3" "$1/AGENTS.md"
cp "$2" "$1/forgecad.bin"
cat > "$1/forgecad" <<'WRAP'
#!/bin/sh
here=$(dirname "$0")
out=$("$here/forgecad.bin" "$@" 2>&1 </dev/stdin); code=$?
printf '$ forgecad %s\n[exit %s]\n%s\n' "$*" "$code" "$out" >> "$here/calls.log"
printf '%s\n' "$out"
exit $code
WRAP
chmod +x "$1/forgecad"
