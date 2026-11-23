#!/bin/sh
# This session's sample: NIST whole, then round by round one shard each of CADFS, ABC and AutoMate,
# every 45th shard (CADFS from 18, ABC from 9, AutoMate from 0), so each round spans the corpora.
# Another session takes other offsets. Resumable: a finished shard is skipped, a partial one resumed.
cd "$(dirname "$0")"
python3 -I scale.py run nist 0
i=0
while [ $i -lt ${ROUNDS:-20} ]; do
  python3 -I scale.py run cadfs $((18 + 45 * i))
  python3 -I scale.py run abc $((9 + 45 * i))
  python3 -I scale.py run automate $((45 * i))
  i=$((i + 1))
done
