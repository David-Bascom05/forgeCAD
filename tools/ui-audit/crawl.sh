#!/bin/sh
# The full UI audit: every ribbon command in every document kind, clicked headless at 1400x860
# and again at 480 px wide, then research/ui-audit/report.md. Needs a Vulkan adapter (lavapipe:
# apt-get install mesa-vulkan-drivers). UI_AUDIT_ONLY=plate,cam narrows the documents.
set -e
here=$(cd "$(dirname "$0")" && pwd)
out=${UI_AUDIT_OUT:-${TMPDIR:-/tmp}/forge-ui-audit}
rm -rf "$out" "$out-480"
cd "$here/../../interface/forge-render"
UI_AUDIT=full UI_AUDIT_OUT="$out" cargo test --release --test ui_audit full_crawl -- --nocapture
UI_AUDIT=full UI_AUDIT_OUT="$out-480" UI_AUDIT_SIZE=480x860 cargo test --release --test ui_audit full_crawl -- --nocapture
python3 "$here/report.py" "$out" "$out-480"
