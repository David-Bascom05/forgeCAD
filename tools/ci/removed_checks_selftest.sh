#!/bin/sh
# The registry check catches each way an entry goes stale (tools/ci/removed_checks.py): run on a
# fixture (tools/ci/removed-checks-fixture), it passes as it is and fails on each change below.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
check="$here/removed_checks.py"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fresh() { rm -rf "$tmp/f"; cp -r "$here/removed-checks-fixture" "$tmp/f"; }
fails() {  # what, expected message
  if out=$(python3 "$check" --registry "$tmp/f/good.toml" 2>&1); then
    echo "self-test FAILED: $1 passed"; exit 1
  fi
  echo "$out" | grep -q "$2" || { echo "self-test FAILED: $1 gave: $out"; exit 1; }
  echo "caught: $1"
}
fresh
python3 "$check" --registry "$tmp/f/good.toml" >/dev/null || { echo "self-test FAILED: the good fixture fails"; exit 1; }
echo "passes: the fixture as it is"
fresh; sed -i 's/x > 0/x >= 0/' "$tmp/f/src/lib.rs"; fails "the code changed" "changed since its review"
fresh; sed -i 's/    \/\/ a comment with a { brace/    \/\/ a comment, reworded { brace/' "$tmp/f/src/lib.rs"; fails "a comment changed" "changed since its review"
fresh; sed -i 's/guarded_never_fails/renamed/' "$tmp/f/lean/Proof.lean"; fails "the lemma is gone" "not declared"
fresh; : > "$tmp/f/lean/Axioms.lean"; fails "the lemma left the axiom report" "axiom report"
fresh; sed -i 's/name = "samples", kind = "mutation"/name = "ignored_mutation", kind = "mutation"/' "$tmp/f/good.toml"; fails "an ignored test" "ignored"
fresh; sed -i 's/name = "samples", kind = "mutation"/name = "not_a_test", kind = "mutation"/' "$tmp/f/good.toml"; fails "a fn that is not a test" "not a test"
fresh; sed -i 's/name = "samples", kind = "mutation"/name = "gone", kind = "mutation"/' "$tmp/f/good.toml"; fails "a missing test" "missing"
fresh; sed -i 's/kind = "mutation"/kind = "unit"/' "$tmp/f/good.toml"; fails "a hand model with no mutation test" "mutation test"
fresh; sed -i 's/owed = \["code = model"\]/owed = []/' "$tmp/f/good.toml"; fails "a hand model owing nothing" "code = model"
fresh; sed -i 's/fn guarded(/fn renamed(/' "$tmp/f/src/lib.rs"; fails "the fn is gone" "not found"
fresh; sed -i 's/^lemmas = .*/lemmas = []/' "$tmp/f/good.toml"; fails "no lemma and no hypothesis" "no lemma"
fresh; sed -i 's/^lemmas = .*/lemmas = []\nhyp = "H.fixture"/; s/^owed = .*/owed = ["code = model", "lemma"]/' "$tmp/f/good.toml"
python3 "$check" --registry "$tmp/f/good.toml" >/dev/null || { echo "self-test FAILED: a hypothesis entry owing its lemma fails"; exit 1; }
echo "passes: an entry on a hypothesis, owing its lemma"
echo "self-test passed"
