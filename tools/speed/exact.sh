# The exact count's conditions, sourced by gate.sh (callgrind) and phases.sh, so a count is a
# function of the code alone and two builds that compile to the same code count the same, to the
# instruction (measured: base and a switch-off build of the decide-once branch, every line 0):
# - FORGE_KERNEL_HASH_PIN: forge-lang's kernel hash (build.rs) is in every result cache key, and
#   it hashes the kernel's sources, so any kernel edit, a comment or a module behind a feature
#   that is off, moved the keys and with them the bench lines' counts (by -55 to +525). Pinned in
#   a measurement build; FORGECAD_CACHE=off so a pinned build never reads a disk cache.
# - fixrand.so (LD_PRELOAD): getrandom as a fixed stream, so std's HashMap seeds are the same on
#   every run.
# - taskset -c N: one core (GATE_CORE, default 0, so two gates can run side by side), so
#   available_parallelism is 1 and no worker threads interleave their allocations (glibc malloc's
#   paths otherwise moved the counts by up to ~0.4% run to run).
# - env -i: a fixed environment (PATH and the two settings above only), since reading a variable
#   scans the whole environment: the caller's own variables moved every count by hundreds.
# usage: exact_dir=TARGET_DIR; . tools/speed/exact.sh   (after RUSTFLAGS, CARGO_TARGET_DIR are set)
export FORGE_KERNEL_HASH_PIN=0123456789abcdef
export FORGECAD_CACHE=off
exact_so="$exact_dir/fixrand.so"
mkdir -p "$exact_dir"
if [ ! -f "$exact_so" ] || [ "$(dirname "$0")/fixrand.c" -nt "$exact_so" ]; then
  cc -O2 -shared -fPIC -o "$exact_so" "$(dirname "$0")/fixrand.c"
fi
exact_pin=
if command -v taskset > /dev/null; then exact_pin="taskset -c ${GATE_CORE:-0}"; fi
# exact_run CMD..: CMD under the exact conditions
exact_run() {
  env -i PATH=/usr/local/bin:/usr/bin:/bin FORGECAD_CACHE=off LD_PRELOAD="$exact_so" $exact_pin "$@"
}
