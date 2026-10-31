# Agent trials: tokens before and after the tools layer

Fresh agents did each task in `tasks.txt` once per arm, through the CLI: "before" with the commit
before the tools layer and its AGENTS.md, "after" with the tools layer. **One run per arm, so this is
one sample of each task, not a distribution.** MCP was not exercised (trial agents cannot attach it).
`run.sh` sets up a directory; `score.sh` counts its calls. The main number is what the docs and the tool
hand the agent, in KB (AGENTS.md plus all tool output). Agent tokens are secondary: each carries a large
system prompt and memory that neither arm changes, and it was not measured apart.

```
                   docs + tool KB    agent tokens     tool uses   forgecad calls  failed
                   before  after     before  after    bef  aft    before  after   bef aft
bracket            17.3    12.7      58.4k   57.0k    2    4      15      28      0   0
flange             16.0    11.6      56.8k   55.6k    3    6      15      33      0   0
clamp              15.5    10.6      62.0k   58.8k    5    6      16      31      0   1
rounded (fails)    15.3     7.3      56.5k   52.0k    4    4      19      20      9   5
```

Rounds 2026-10-31; binaries e0f15f2d (after), 2c52838d (before). An earlier round was thrown away: the
logging wrapper broke on some shells and agents retried.

- Docs plus tool output fell 27-53%, tokens 2-8%. Calls went up, since the grammar now comes by `help`
  (3-9 calls a task). `help` takes several topics at once since this round, which should cut them.
- Bracket, flange and clamp gave the same answers in both arms.
- `rounded` makes `act` fail on purpose (25 mm fillet on a 30 mm side). `try:` fired 5 times in the
  after arm and each suggestion built, but it gave half what was asked (12 of 15), and the agent took it
  as told: 10481.2 mm3, 11 faces. The before agent searched by hand and found 15 by `fullround` instead:
  9785.8 mm3, 7 faces. So `try:` now searches for the largest size that builds. No trial has run since.
- No evidence for typed per-kind calls: of the `act` failures, none was about grammar.
- Both clamp agents noticed `measure wall` says 2.047 where the wall is 2.000.
