#!/bin/sh
# Option A again with its final code, in place of its earlier lines in each log (B's code did not change since).
# With SHEET_FIXED_CROSS=1 set it runs A with the cross_sections fix copy instead, into trim-a-fixed-KIND.txt.
out=../results
fb=$HOME/.local/bin/fbuild
for k in isolated overlap boundary tangent coincident mix overlap-lines nudged; do
  (
    grep -v '^a ' "$out/trim-$k.log" > "$out/.tmp-$k"
    for spec in "2 200" "20 100" "200 20"; do
      set -- $spec
      c=$2; [ $k = nudged ] && [ $1 != 200 ] && c=100; [ $k = nudged ] && [ $1 = 200 ] && c=10
      $fb target/release/trim verify a $k $1 0 $c >> "$out/.tmp-$k" 2>/dev/null
    done
    cat "$out/.tmp-$k" > "$out/trim-$k.log"
  ) &
  case $k in boundary|mix|nudged) wait ;; esac
done
wait
