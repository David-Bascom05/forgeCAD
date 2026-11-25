# AGENT-TAB handoff

The brief: the Agent tab should be as good as or better than a terminal for work in the app, and it
should work in the browser. For the browser this is option C:
- case 1: the person's own Claude Code, run by `forgecad web --agent`;
- case 2: a paired localhost bridge, for a page hosted elsewhere.

## Merged or merging

- **Train 7 (main c135b274):** agent/stack up to d4fb5469, with the ETXTBSY spawn fix (c5b8d727 and
  its wasm allow, 6cfa14e4). This is the desktop Agent tab and case 1 (`--agent`).
- **Hotfixes #55, #56, #57 (main):**
  - the .git allowlist for the file API;
  - nothing in .git through a link or another letter case;
  - writes through fresh temporary files.
- **Train 8, item 1: agent/tab-8 @ b31d58a5.** This is train 7's agent work plus:
  - the security review's items (deadlines from a request's first byte, the slot guard, one picture
    limit);
  - pictures (paste and drop, a tray, thumbnails, the view attached);
  - the queue, virtualised long chats, multi-select chips, history and commands polish;
  - BUTTON-AUDITOR's findings (Resume and the switcher have Cancel/Close and close on Esc; markup is
    said in words until the first mark).
  UX-LEAD OK'd it at cb0cf8c0, and the coordinator cleared its security at 7097056f. Since then it has
  only had main merged in.
- **Train 8: agent/paste @ 9a53ec43.** This is tab-8 plus Ctrl/Cmd+V of a picture on the desktop.
  It adds the `arboard` dependency, declared in docs/interface-design.md. UX OK'd it at 5524cd1d.
- A pre-verify on tools/preverify @ 13d78f06 was running on both at wind-down. Its lines go to
  TRAIN-CONDUCTOR.

## Not merged: case 2, on agent/stack @ this commit

agent/stack holds the hosted-page bridge, which is not in tab-8:
- `forgecad web --agent --allow-origin ORIGIN`: CORS for exactly the listed origins, and a pairing
  code shown in the terminal;
- `Agent` with devices bound to their origin, at most 4, revocable;
- a remembered device, opt-in, for a week, forgotten from the page or the bridge;
- the page's pairing UI in app.js, with the CSP's connect-src for 127.0.0.1 and localhost;
- tests/agent_hosted.cjs.

**State:** the coordinator reviewed it twice. The fixes for both reviews are in (dbf003d4 and
3524b2fa), but case 2 was never cleared to ship. It was held back from train 8 on purpose: tab-8 is
agent/stack with the cross-origin parts removed.

**Next steps:**
1. Merge main into agent/stack. Expect the same conflicts tab-8 had, because train 7 squashed
   d4fb5469, which stack holds. Keep stack's side, then re-add whatever main changed beyond d4fb5469
   in those files (`git diff d4fb5469 origin/main -- FILE`).
2. Bring tab-8's later work over to stack by merging agent/tab-8 into it:
   - read_request's clock from the first byte, each line bounded, and Config.head_time;
   - the slot guard and write timeout;
   - the total picture cap (8 MB at the bridge, 7 MB in the tab);
   - BUTTON-AUDITOR's exits and the markup hint.
   web.rs and agent_bridge.rs will conflict where tab-8 removed the cross-origin branch. Keep stack's
   cross-origin code and tab-8's hardening.
3. Run a fresh security review of case 2 alone, at minimum:
   - origin binding on every route;
   - the pairing code's entropy and its tries;
   - the device token's expiry and revocation;
   - CSP;
   - no credentials ever reaching the page (the brief's rule: the bridge's code never reads or
     forwards credentials).
4. Then UX of the pairing flow (`research/agent-tab/shots/agent-web-*`), pre-verify, and a train.

## Risks

- **The duplicated history.** stack, tab-8 and main now hold the same agent work as different
  commits. Merges conflict file by file, and "keep ours" silently drops main's own later edits unless
  they are re-added. Check with the diff in step 1 every time.
- **Case 2 widens the attack surface.** A page on another origin drives a local agent that writes
  files. The origin binding, the pairing and the token are the only gates. Don't ship it without its
  own review.
- **Timing-sensitive tests.** The trickle test waits about 2 s (head_time is 2 s in tests). If CI's
  time budget tightens, make head_time 1 s in tests rather than loosening the assert.
  codex_stop_and_exit_failures_finish_the_turn's failure was ETXTBSY at start, fixed by agent::spawn.
  Any new test that writes a fake agent and runs it gets the same protection only if it starts through
  agent::spawn.
- **Disk.** A full pre-verify needs about 12 GB free. The old
  `/home/user/forgecad/interface/forge-render/target` (36 GB) could not be removed from the session
  (blocked); a person should delete it.
