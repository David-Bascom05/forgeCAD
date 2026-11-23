#!/bin/sh
# This session's sample: NIST whole, then round by round one shard each of CADFS, ABC and AutoMate,
# every 45th shard (CADFS from 18, ABC from 9, AutoMate from 0), and Fusion every 9th from 4 (it has 86),
# so each round spans the corpora.
# Another session takes other offsets. Resumable: a finished shard is skipped, a partial one resumed.
# CORPORA picks some of them (two queues side by side keep the cores busy through a shard's slow tail).
cd "$(dirname "$0")"
python3 -I scale.py run nist 0
i=0
while [ $i -lt ${ROUNDS:-20} ]; do
  for c in ${CORPORA:-cadfs abc automate}; do
    case $c in cadfs) k=18 s=45 ;; abc) k=9 s=45 ;; automate) k=0 s=45 ;; fusion) k=4 s=9 ;; esac
    python3 -I scale.py run $c $((k + s * i)) --jobs ${JOBS:-4}
  done
  i=$((i + 1))
done
