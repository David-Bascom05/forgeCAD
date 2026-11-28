# Hosted collaboration (phase 1): loopback / dev only

Phase 1 of research/hosted/DESIGN.md: one `forge-server` process serves a project folder, and
several clients (the desktop app, browser tabs) edit its parts at once. **It is for loopback and
development only**: no TLS, dev tokens instead of accounts. Never expose it to a network you do
not trust; `--insecure-lan` exists for a LAN demo and says so.

## Run a server

    forge-server PROJECT_DIR --user ada:TOKEN_OF_16+_CHARS [--port 7878] [--origin http://localhost:8080]

- Documents are the `.fg` files of PROJECT_DIR, by their exact relative path. Named versions and
  idle checkpoints are forge-history commits in PROJECT_DIR's git; the live op log is in
  `PROJECT_DIR/.forgecad/server/` (one server per folder: it is locked).
- `--origin` names the page origin a browser tab comes from (the app served by `forgecad web`).
- A `.fg` file changed on disk outside the server is never written over: the server refuses to
  open it, or stops saving it if it is open.

## Desktop app (SERVED project)

The desktop's project folder must be its **own** folder: never the server's PROJECT_DIR (the app
writes the server's text into its copy, and the server would see that as a hand edit). The app
refuses a folder that holds `.forgecad/server/`.

- The project says which remote serves it, and nothing else: `.forgecad/remote.toml` holds
  `remote = "NAME"`.
- The address and token are the user's, in `<config>/forgecad/remotes.toml` (on Linux
  `~/.config/forgecad/remotes.toml`):

      [remote.NAME]
      url = "ws://127.0.0.1:7878/sync"
      token_env = "FORGECAD_TOKEN"      # or token = "...": a FORGECAD_* variable only
      projects = "/home/me/cad"          # the project folders this remote may serve

  An entry is the user's trust: a project naming an unknown remote is refused. A non-loopback
  URL is refused unless the entry says `insecure_lan = true`.
- Typed edits are shared; undo is hosted undo (the inverse of your last op; others' edits stay).
  A hand edit of the file (an editor, `git checkout`) pauses that document's syncing and is kept;
  bringing it in is phase 4.

## Browser

Serve the app with `forgecad web DIR --sync ws://127.0.0.1:7878/sync` (the page's security
policy lets it connect to that server and no other), open it with
`?sync=ws://127.0.0.1:7878/sync&sync_token=TOKEN` (this machine only), and start forge-server
with `--origin` set to the page's origin (`http://localhost:PORT`).
