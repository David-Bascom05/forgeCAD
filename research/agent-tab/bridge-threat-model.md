# The agent bridge: threat model

The browser's agent (audit.md, option C) runs **Claude Code on the person's own machine**, logged
in with their own plan, and the page drives it. Plan auth only ever goes through Claude Code: the
bridge never reads, copies or forwards its credentials, and the page never sees them.

This file is written before any bridge code; the build follows it, and each mitigation below has a
test named beside it.

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
- Case 1: the printed URL carries `?token=T`; the page takes it, removes it from the address
  bar (`history.replaceState`) and keeps it in `sessionStorage` (gone with the tab).
  `Referrer-Policy: no-referrer` on every response, so it never travels in a `Referer`.
- Case 2: the person pastes the token once into the page's Agent settings. It's kept in that
  origin's `localStorage` only if they tick "Remember on this device"; otherwise
  `sessionStorage`. The settings say who can read it: any script served from that origin. On a
  shared origin (a `user.github.io` serving several repositories' pages) that's every one of
  them, so `--allow-origin` names the exact origin, and the docs say to prefer case 1.
- The page loads no third-party scripts. A Content-Security-Policy (`script-src 'self'
  'wasm-unsafe-eval'`, `connect-src 'self'` plus the bridge's origin in case 2) goes in
  `index.html` with this work.
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
- **No bypass.** The bridge never passes `--dangerously-skip-permissions` or
  `bypassPermissions`, whatever the page asks, and it always passes `--permission-mode` itself
  (`default` unless the page chose `acceptEdits` or `plan`), so a person's own
  `defaultMode: bypassPermissions` (the desktop app may run that way) doesn't carry into a
  browser tab. Backstop: if the session's `init` event reports any other mode, the bridge ends
  the process and says why. *Test: the args always carry one of the three modes; a fake `claude`
  whose `init` says `bypassPermissions` is ended with that reason.*
- Arguments go through `Command::args`, never a shell.

### T7. The plan's credentials
- The bridge **never reads** `~/.claude` (or wherever Claude Code keeps its login), never sets
  or forwards an auth variable, and passes the environment through as the desktop does
  (`clean_env`).
- Claude Code could still be *asked* to read its own credentials file (prompt injection, or a
  careless request). Defence in depth: the bridge adds `--disallowedTools` rules denying `Read`,
  `Grep`, `Glob` and `Bash` access to Claude Code's config directory (the exact rule syntax is
  checked against the CLI before it ships), and outside `DIR` the default mode asks anyway, so a
  card appears. *Test: the args carry the deny rules; a fake `claude` that is asked to read
  `~/.claude/.credentials.json` gets the denial.*
- Events going to the page are the CLI's stream-json lines, which carry no credentials.

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

### T10. What reaches the page
- The page draws everything with egui on a canvas, not as HTML, so text from the model or a
  tool can't inject markup or script into the page.
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
  wrong, right), Origin (own, allowed, other, `null`), Host (rebinding), Content-Type, unknown
  fields, the mode allowlist, path checks, loopback-only, a `bypassPermissions` default refused,
  one owner (409), a stream of events, answer and stop reaching the process as the right
  `control_response` and `control_request`, the deny rules for the config directory on the
  command line.
- `web/tests/agent.cjs`, the page with the bridge and a fake `claude`: a turn whose edit shows
  as a diff and undoes, a permission card answered from the page, Stop, the token taken from the
  URL and out of the address bar, and no request leaving for any host but the bridge.
