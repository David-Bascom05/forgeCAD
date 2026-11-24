#!/bin/sh
# Fetches benchmark data sets into a cache outside the repository, each pinned (manifest.tsv):
# a URL by its SHA-256 (or its MD5 where only that is published: then the SHA-256 is printed,
# to pin), a git repository by its commit, sparse to the listed paths. A mismatch is refused and
# the download removed. Nothing here writes into the checkout.
# usage: tools/bench/fetch.sh [NAME..]     (default: every set in the manifest)
#        FORGECAD_BENCH=DIR picks the cache (default ~/.local/share/forgecad-bench)
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cache=${FORGECAD_BENCH:-$HOME/.local/share/forgecad-bench}
mkdir -p "$cache"
want="$*"
status=0
tab=$(printf '\t')
list=$(mktemp)
trap 'rm -f "$list"' EXIT
grep -v '^#' "${FORGECAD_BENCH_MANIFEST:-$here/manifest.tsv}" > "$list"
while IFS="$tab" read -r name kind src pin files licence note; do
    [ -n "$name" ] || continue
    if [ -n "$want" ] && ! printf ' %s ' "$want" | grep -q " $name "; then
        continue
    fi
    dest="$cache/$name"
    echo "== $name ($licence): $note"
    case "$kind" in
    url)
        mkdir -p "$dest"
        f="$dest/$files"
        if [ ! -f "$f" ]; then
            curl -fsSL --retry 3 -o "$f.part" "$src" || { echo "   download failed"; rm -f "$f.part"; status=1; continue; }
            mv "$f.part" "$f"
        fi
        algo=${pin%%:*}
        hex=${pin#*:}
        case "$algo" in
        sha256) got=$(sha256sum "$f" | cut -d' ' -f1) ;;
        md5) got=$(md5sum "$f" | cut -d' ' -f1) ;;
        *) echo "   unknown pin $pin"; status=1; continue ;;
        esac
        if [ "$got" != "$hex" ]; then
            echo "   REFUSED: $algo $got, pinned $hex; removed"
            rm -f "$f"
            status=1
            continue
        fi
        echo "   ok $f"
        if [ "$algo" = md5 ]; then
            echo "   pin it: sha256:$(sha256sum "$f" | cut -d' ' -f1)"
        fi
        ;;
    git)
        if [ ! -d "$dest/.git" ]; then
            git clone -q --filter=blob:none --no-checkout "$src" "$dest" || { echo "   clone failed"; status=1; continue; }
        fi
        # shellcheck disable=SC2086
        git -C "$dest" sparse-checkout set --no-cone $files
        git -C "$dest" -c advice.detachedHead=false checkout -q "$pin" || { echo "   REFUSED: no commit $pin"; status=1; continue; }
        got=$(git -C "$dest" rev-parse HEAD)
        if [ "$got" != "$pin" ]; then
            echo "   REFUSED: at $got, pinned $pin"
            status=1
            continue
        fi
        echo "   ok $dest @ $pin"
        ;;
    *) echo "   unknown kind $kind"; status=1 ;;
    esac
done < "$list"
exit $status
