# The agent bridge: threat model

The browser's agent (audit.md, option C) runs **Claude Code on the person's own machine**, logged
in with their own plan, and the page drives it. Plan auth only ever goes through Claude Code: the
bridge never reads, copies or forwards its credentials, and the page never sees them.

This file is written before any bridge code; the build follows it, and each mitigation below has a
test named beside it.

**Revised 2026-11-24 after review (David, coordinator).** The agent in the page behaves exactly as
`claude` does in a terminal: the person's own permission settings (allow and deny rules,
`defaultMode`, `bypassPermissions` if *they* set it), their CLAUDE.md, memory, skills and
settings all load as normal. The approval cards are Claude Code's own prompts shown in the page.
The bridge forces no permission mode and adds no deny rules; its own rule is that **its code never
reads or forwards credentials, and the page never receives them**. Added: the page as a target
(escaping, CSP, an injection test) and the page holding the token in memory only.

## What it is

One server, `forgecad web DIR`, which already serves the app and a file API over `DIR`
(docs/web.md), gains an agent API with `--agent`:

- **Case 1, same origin.** `forgecad web DIR --agent`: the page comes from the same server, so the
  agent API is same-origin.
- **Case 2, a page hosted elsewhere** (a static host). `forgecad web DIR --agent --allow-origin
  https://HOST`: the hosted page talks to `http://127.0.0.1:PORT` for the project's files and the
  agent. A project kept only in the browser has no folder for Claude Code to work in, so case 2
  always pairs the page with a folder on the machine. The desktop app can offer the same server
  later (Settings › Agent › "Let the browser use this app's agent"), with the same rules.

The page drives one Claude Code process:

| call | what |
|---|---|
| `POST /api/agent/start` `{"file": REL?}` | starts (or attaches to) the session. `REL` is a project-relative path, checked as the file API checks paths |
| `GET /api/agent/events` | the session's stream-json lines, as a streamed response (`fetch` with a header; not `EventSource`, which can't send one) |
| `POST /api/agent/send` `{"text", "picture"?}` | a message |
| `POST /api/agent/answer` `{"id", "answer": "allow"\|"always"\|"deny"}` | a permission's answer (agent/approvals) |
| `POST /api/agent/mode` `{"mode": "default"\|"acceptEdits"\|"plan"}` | the permission mode |
| `POST /api/agent/stop` | interrupt |

The page parses the lines with the same `agent::claude::Parser` the desktop uses (compiled to
wasm), so the transcript, cards and undo are the desktop's.

## What's worth protecting

1. **What Claude Code can do as the person**: read and write their files, run commands, under
   their permission settings. This is the big one. Whoever drives the bridge drives their machine
   as far as those settings and the approval cards allow.
2. **Their plan**: its usage limits, and its login (the OAuth credentials under `~/.claude`).
3. **What's said**: the conversation, the project's files, tool output.

## Who might attack, and what stops them

### T1. Another website open in the same browser (CSRF, cross-origin reads)
A page at `evil.example` sends `fetch("http://127.0.0.1:PORT/api/agent/send", …)`.
- **Every agent call needs the pairing token**, as `Authorization: Bearer T`. The files API
  doesn't need one on loopback today; the agent API always does. A cross-origin page can't learn
  it: it's never in a response that page can read, and never set as a cookie (no ambient
  credential). *Test: a call without the token, or with a wrong one, is 401, and nothing starts.*
- **Origin must be allowed**: the server's own origin (case 1), or exactly the ones given with
  `--allow-origin` (case 2: scheme, host and port compared whole, no wildcards, no `null`). A
  request with an `Origin` not on the list is 403 *before* the token is looked at, so the token
  can't be guessed through timing either way. *Test: Origin `https://evil.example` → 403;
  `null` → 403.*
- **Writes are JSON** (`Content-Type: application/json` required, else 415), so a cross-origin
  write needs a CORS preflight, which is answered with no `Access-Control-Allow-Origin` unless the
  Origin is on the list. A `no-cors` or form POST can't send JSON or the header. *Test: a
  `text/plain` POST → 415.*
- **GETs change nothing.** `GET /api/agent/events` only reads, needs the token, and a
  cross-origin `no-cors` read is opaque to its page anyway.
- The token is compared in constant time (`web.rs` `same`).

### T2. DNS rebinding
`evil.example` re-resolves to 127.0.0.1 to become "same-origin" with the bridge.
- `forgecad web` on loopback already answers only requests whose `Host` is a loopback name
  (421 otherwise), and the Origin check above sees `evil.example`. *Test: Host `evil.example` →
  421.*

### T3. Someone else on the network
- **The agent API is only served on loopback.** `--agent` with `--bind` other than loopback is
  refused at start; to use it from another machine, use an SSH tunnel. *Test: `--agent --bind
  0.0.0.0` exits with that message.*

### T4. Another user, or another program, on the same machine
Anything local can connect to a loopback port.
- The pairing token (above) stops other users and programs that can't read this person's
  terminal. It's 128 bits from the OS's random source, made fresh at each start and printed
  once in the terminal (and, later, shown in the desktop app's Settings with "Make a new one").
  It's never written to disk by the bridge.
- A program running **as the same user** is out of scope: it can already run `claude` itself.

### T5. The token leaking from the page
- **In memory only, by default.** Case 1: the printed URL carries `?token=T`; the page takes it
  into a variable, removes it from the address bar (`history.replaceState`), and keeps it nowhere
  else, so a reload asks to pair again (open the printed URL again, or paste the token). Case 2:
  the person pastes it into the page's Agent settings, also kept in memory only.
  `Referrer-Policy: no-referrer` on every response, so it never travels in a `Referer`.
- **"Remember on this device"**, ticked by the person, keeps it in `localStorage` with an expiry
  (7 days), as a *device* token the bridge issues for that purpose (`POST /api/agent/remember`,
  answered with a separate token and its expiry). The bridge lists them and can revoke one or
  all (`forgecad web … --forget-devices`, or Settings in the app later). Revoked or expired, the
  page is asked to pair again. The settings say who can read a remembered token: any script
  served from that origin. On a shared origin (a `user.github.io` serving several repositories'
  pages) that's every one of them, so `--allow-origin` names the exact origin, and the docs say to
  prefer case 1. *Test: a remembered token works until revoked, then 401; an expired one is 401;
  the page keeps nothing in storage unless remembered.*
- The page loads no third-party scripts. A strict Content-Security-Policy goes in with this work,
  as a response header from `forgecad web` and a `<meta>` in `index.html` for static hosts:
  `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'` (plus the
  bridge's origin in case 2); `object-src 'none'; base-uri 'none'; frame-ancestors 'none'`. No
  inline script: `index.html`'s inline module moves to a file.
- Revoking: stop the bridge (the token dies with it); "Make a new one" in the app.

### T6. The page choosing what runs
A page that has the token (or a bug in ours) must still not be able to turn the bridge into a
shell.
- **The bridge fixes the whole command line**: the `claude` program found on PATH at start, cwd
  = `DIR`, the app's system prompt, `--permission-prompt-tool stdio`, the forgecad MCP config.
  The page sends only message text, a picture, a permission answer, a mode from the three above,
  stop, and a file path inside `DIR`. Never a program, an argument, an environment variable, a
  cwd, an MCP server or a system prompt. *Test: unknown fields are refused (400); a mode of
  `bypassPermissions` is 400; a `file` of `../x` or `/etc/passwd` is 400.*
- **Permissions are Claude Code's own.** The bridge passes no `--permission-mode` unless the
  person picks one in the page (and then only `default`, `acceptEdits` or `plan`; the page can't
  ask for a bypass), never `--dangerously-skip-permissions`, and adds no allow or deny rules. So
  the person's settings decide, as in a terminal and as in the desktop tab. *Test: by default the
  args carry no `--permission-mode` and no rules; `mode: bypassPermissions` from the page is 400.*
- **Bypass, when the person configured it.** Case 1 (same origin) honours it, like a terminal,
  and the page shows a "Permissions bypassed (your Claude settings)" banner for as long as the
  session reports that mode. Case 2 (a page on another origin) honours a bypass only if the bridge
  was started with `--allow-bypass`. Without that flag, a session whose `init` (or a later status)
  reports `bypassPermissions` is ended and the page told why. With it, the same banner shows.
  *Test: case 2 without the flag, a fake `claude` whose `init` says `bypassPermissions` is ended
  with that reason; with the flag, it runs and the page's state says bypassed; case 1 runs and
  says bypassed.*
- Arguments go through `Command::args`, never a shell.

### T7. The plan's credentials
- The bridge's own code **never reads** `~/.claude` (or wherever Claude Code keeps its login),
  never sets or forwards an auth variable, and passes the environment through as the desktop
  does (`clean_env`). *Test: a grep-style test that forge-cli's bridge source names no
  credential path or auth variable; the child's environment is the parent's less the
  parent-session markers.*
- Claude Code itself loads the person's config, CLAUDE.md, memory and skills normally. It reads
  files outside `DIR` (its own config folder included) only as their permission settings allow:
  a prompt (a card in the page) unless they've allowed it. That's the terminal's behaviour,
  kept on purpose. (A `Read(~/.claude/**)` deny rule was tried against the CLI, 2.1.331, and
  works, but it isn't applied: it would make the page's agent differ from the terminal's.)
- Events going to the page are the CLI's stream-json lines. The bridge forwards them as they
  are and adds nothing of its own.

### T8. Prompt injection through the work itself
A part, an imported STEP's text, a fetched web page can carry instructions to the model. The
bridge doesn't make this worse than the desktop: the same approval cards (agent/approvals) gate
every edit outside what the person allowed and every command. The page shows each one, and an
unanswered card blocks the turn: **nothing is auto-allowed** by the bridge, ever. *Test: a
`can_use_tool` request with no answer stays waiting, and Stop ends it.*

### T9. Using up the plan or the machine
- One session per bridge, owned by the first page that starts it. Another page gets 409 until
  the first stops (or its event stream has been closed for 60 s).
- At most one turn at a time; follow-ups queue as on the desktop (a queue of 20 at most).
- A session with no events reader for 10 minutes is ended (its process killed); a turn has no
  extra time limit beyond Claude Code's own.
- Request bodies are capped (1 MiB; a picture at 8 MiB).

### T10. The page as a target: what reaches it
- The transcript (markdown, tool output, file names, diffs) is drawn by egui on a canvas, never
  as HTML. The page's own DOM (File > Open's list, the pairing box, messages from `app.js`) sets
  only `textContent`, never `innerHTML`, for anything that came from the bridge or the project.
  With the CSP above, an injected `<script>` or an `<img onerror>` could not run even if one
  reached the DOM. *Test (`web/tests/agent.cjs`): a fake `claude` streams text, a tool result
  and a file name holding `<script>window.pwned=1</script>` and `<img src=x
  onerror="window.pwned=2">`; after the turn `window.pwned` is undefined, no CSP-violation or
  page error is logged other than the expected refusals, and the transcript shows the text
  literally.*
- Tool output can hold file contents from `DIR` (that's the point) and, if the person allows
  it, from elsewhere. That goes only to the paired page, over loopback.

### T11. Privacy and telemetry
- The bridge adds no network calls of its own, no analytics, no logging of messages (stderr says
  only start, stop, and refusals with their reason). What Claude Code itself sends is governed
  by the person's Claude settings, as on the desktop.

### T12. The browser's own rules for public pages reaching localhost (case 2)
- An `https` page may fetch `http://127.0.0.1`: browsers treat loopback as potentially
  trustworthy, so it's not blocked as mixed content (to be confirmed per browser in the page
  test; Safari is the one to check).
- Chrome's Private/Local Network Access: a public page reaching loopback sends a preflight with
  `Access-Control-Request-Private-Network: true`, and newer Chrome also asks the person for
  permission. The bridge answers `Access-Control-Allow-Private-Network: true` **only** for an
  allowed Origin. The browser's prompt is an extra layer we keep, not one we route around.

## Not covered

- Malware running as the same user (it doesn't need the bridge).
- A compromised `claude` binary, or a compromised page origin in case 2 (the token's
  readers). Case 1 has no second origin to trust, which is why the docs lead with it.
- Many people sharing one bridge. It's one person's machine, plan and permissions.

## Tests that come with the build

- `forge-cli/tests/agent_bridge.rs`, the server's rules, against a fake `claude`: the token (none,
  wrong, right, remembered, revoked, expired), Origin (own, allowed, other, `null`), Host
  (rebinding), Content-Type, unknown fields, the mode allowlist, path checks, loopback-only, no
  forced mode and no added rules on the command line, bypass honoured in case 1 and refused in
  case 2 without `--allow-bypass`, one owner (409), a stream of events, answer and stop reaching
  the process as the right `control_response` and `control_request`, the child's environment,
  the CSP and Referrer-Policy headers.
- `web/tests/agent.cjs`, the page with the bridge and a fake `claude`: a turn whose edit shows
  as a diff and undoes, a permission card answered from the page, Stop, the token taken from the
  URL and out of the address bar and out of storage, the script and `onerror` injection, and no
  request leaving for any host but the bridge.
