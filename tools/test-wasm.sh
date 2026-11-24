#!/bin/sh
# The workspace's tests as wasm32-wasip1 under Node's WASI, the test binaries run in parallel.
# Builds every test binary first (cargo test --no-run, same flags), then runs them JOBS at a time
# (default: the CPU count), largest binary first so the long ones do not start last. Each runs as
# cargo would run it: from its package's directory, with CARGO_MANIFEST_DIR and CARGO_PKG_NAME set.
# Fails if any binary fails, or starts but never reports a result: under load Node has been seen
# to print nothing and exit 0, which would pass a binary whose tests never ran.
# Usage: sh tools/test-wasm.sh [cargo test package flags, e.g. --exclude forge-certify]
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
jobs=${JOBS:-$(nproc 2>/dev/null || echo 4)}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# the whole workspace unless packages are named (-p overrides it)
case " $* " in *" -p "*|*" --package "*) ws= ;; *) ws=--workspace ;; esac
cargo test $ws --target wasm32-wasip1 --no-run --message-format=json-render-diagnostics "$@" \
  > "$work/build.json"

# one line per test binary: size <TAB> wasm path <TAB> package dir <TAB> package name <TAB> target
python3 -I - "$work/build.json" > "$work/list" <<'EOF'
import json, os, sys
seen = set()
for line in open(sys.argv[1]):
    try: m = json.loads(line)
    except ValueError: continue
    if m.get("reason") != "compiler-artifact" or not m.get("profile", {}).get("test"): continue
    exe = m.get("executable")
    if not exe or exe in seen: continue
    seen.add(exe)
    pid = m["package_id"]
    if " " in pid: name = pid.split(" ")[0]                    # "name 0.1.0 (path+file://...)"
    else:
        tail = pid.split("#")[-1]                              # "path+file://.../dir#name@0.1.0"
        name = tail.split("@")[0] if "@" in tail else pid.split("#")[0].rstrip("/").split("/")[-1]
    print(f'{os.path.getsize(exe)}\t{exe}\t{os.path.dirname(m["manifest_path"])}\t{name}\t{m["target"]["name"]}')
EOF
sort -t "$(printf '\t')" -k1,1nr "$work/list" -o "$work/list"
echo "running $(wc -l < "$work/list") test binaries, $jobs at a time"

# run one binary: its output goes to its own log, then the log is printed whole
cat > "$work/one.sh" <<'EOF'
#!/bin/sh
IFS='	' read -r size exe dir pkg target <<LINE
$1
LINE
log="$2/$(echo "$exe" | tr '/' '_').log"
status=0
(cd "$dir" && CARGO_MANIFEST_DIR="$dir" CARGO_PKG_NAME="$pkg" CARGO_CRATE_NAME="$target" \
  "$3/tools/wasi-run.mjs" "$exe") > "$log" 2>&1 || status=$?
show() { printf '     Running %s (%s, %s)\n' "$target" "$pkg" "$exe"; cat "$log"; }
if command -v flock >/dev/null; then (flock 9; show) 9>"$2/lock"; else show; fi
if ! grep -q '^test result' "$log"; then echo "NO RESULT: $pkg $target ($exe)" >> "$2/failed"
elif [ $status -ne 0 ]; then echo "FAILED: $pkg $target ($exe), exit $status" >> "$2/failed"; fi
exit 0
EOF
tr '\n' '\0' < "$work/list" | xargs -0 -P "$jobs" -I{} sh "$work/one.sh" {} "$work" "$root"

if [ -s "$work/failed" ]; then
  echo; echo "wasm test failures:"; cat "$work/failed"; exit 1
fi
echo; echo "all wasm test binaries passed"
