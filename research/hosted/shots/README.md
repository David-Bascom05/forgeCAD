# Hosted sync: review shots (docs/ux/STANDARD.md §10–§11)

1400×860, dark. Taken on hosted/client-web (native: `forge-app --size 1400x860 --shot`, under
Mesa's lavapipe; web: the app's own frame read back, `?offscreen=1400x860`, Chromium with
SwiftShader).

- `native-sync-paused.png`: a served part edited outside the app since it last synced. The
  "Syncing paused" banner, with the reason cut short (the whole reason on hover).
- `web-sync-paused.png`: the browser has no file on disk to compare, so its pause comes from a
  server refusal (a wrong token): "Not synced: the server refused this part: a valid token is
  needed".
- `native-version-note.png`, `web-version-note.png`: the status line after a collaborator
  writes the version "release-a" ("Version release-a written").

Differences: the two pauses have different causes, as above (the browser keeps no file).
