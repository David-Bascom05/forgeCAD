#!/bin/sh
# All trim cases: reliability (one log per kind). From proto/ after `cargo build --release --bin trim`.
out=../results
fb=$HOME/.local/bin/fbuild
for k in isolated overlap boundary tangent coincident mix overlap-lines; do
  (
    : > "$out/trim-$k.log"
    $fb target/release/trim all $k 2 0 200 >> "$out/trim-$k.log" 2>/dev/null
    $fb target/release/trim all $k 20 0 100 >> "$out/trim-$k.log" 2>/dev/null
    $fb target/release/trim all $k 200 0 20 >> "$out/trim-$k.log" 2>/dev/null
  ) &
  case $k in boundary|mix) wait ;; esac
done
wait
