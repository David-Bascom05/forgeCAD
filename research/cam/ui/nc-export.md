# NC export: programs as files, saved where you choose (2026-11-24)

David: "we can't JUST output gcode. We need an exporter and to be able to save it somewhere."
This is Fusion's NC Program object, held in the machining document's text.

## The NC program is a statement

Every setting is a keyword argument of the program's own statement. It is saved, versioned,
diffed and undone with the document, and the app and the CLI read the same text.

```text
prog = program(op1, post: "fanuc", number: 1001,
               file: "flange-op1", out: "nc", ext: "nc", comment: "flange, first side", tools: "list")
```

| arg | meaning | default |
|---|---|---|
| first positional | the setups it posts, in order (one name or a list) | — |
| `post:` | dialect (fanuc, haas, linuxcnc, grbl, …; lathes: fanuc-lathe, haas-lathe, linuxcnc-lathe) | the machine's |
| `number:` | program number (O-word) | — |
| `file:` | file name without extension | the program's name |
| `ext:` | extension | the dialect's: linuxcnc → ngc, grbl → gcode, siemens → mpf, heidenhain → h, mach3/4 → tap, else nc |
| `out:` | folder, relative to the document | beside the document |
| `comment:` | a comment line in the header | none |
| `tools: "list"` | a tool list in the header: `T1 flat-10 in bt40-er32, stick-out 32: skim, top` | none |

The code is `interface/forge-cam-lang/src/export.rs` (`files`, `settings`, `tool_list`,
`ext_of`). Both the app and the CLI call it on the same run, so they write the same bytes. The
CLI test checks this: `interface/forge-cli/tests/cam_post.rs`.

The header only adds comment lines, after the first `%` line or at the top. What the control
runs is exactly what was certified.

## The certification rule

- A program with a refused op has no text, and nothing is written. You get a typed refusal
  that gives the reason.
- A program released but not certified in full is refused unless allowed. For example, "except
  G5" means a setup with no `at:`, so the machine positions were not checked.
  - The app's "Allow uncertified" checkbox (NC Programs window) or the CLI's
    `--allow-uncertified` lets it through.
  - The header is then stamped `(NOT CERTIFIED IN FULL: except G5 unchecked: no `at:`)`.
- A certified program is written as posted.

## In the app (Manufacture ▸ NC Programs)

Each program row shows:

- its setups (checkboxes);
- its dialect, number, line count and certification;
- the fields File, Folder and Ext., plus Comment and a "Tool list in header" checkbox. Each edit
  is a SetArg (or a ReplaceFeature to take the argument out) when the field is left;
- its file's state, as a painted dot plus words:
  - **posted**: the file at its path is what it would post now;
  - **stale**: the file differs, so the ops changed since it was posted;
  - **not posted**: no file;
  - **refused**: an op of it is refused.
  The state is read from the file on disk (natively) or in the project (browser), so it holds
  across sessions with nothing extra stored.

Its buttons:

- **Post**: the program's files are written into the document's folder (`out:`). In the browser,
  that means into the project.
- **Export…**: the host's Save As.
  - Natively: the system save dialog for one file (it asks before replacing), or a folder for
    several. Files already in the folder are listed, and they are replaced only on Yes. The
    dialog starts at the folder last exported to (remembered with `plat::kv_set`), else the
    program's `out:`, else the document's folder. The result path is shown in the window and on
    the status line.
  - Browser: the File System Access save picker where the page may use it, else a download. A
    save needs the user's own click, and the export runs a frame after the click, so today it is
    a download in practice. Several files go into one zip (`web/zip.js`).
- **Post Process**: the post dialog at this program (text preview; Post and Export… there too).

At the bottom of the window:

- **Post All**;
- **Re-post Stale**;
- **Export All…**;
- **Setup Sheet & Tools…**: `<doc>-sheet.html` and `<doc>-tools.txt` through the same Save As;
- **New Program**;
- the **Allow uncertified** checkbox.

The ribbon also has **Export NC**, which is Export All….

## CLI

```text
forgecad cam F post [PROGRAM] [-o FILE|DIR/] [--allow-uncertified]
```

- No `-o`: each file goes where its statement says (beside the document, in `out:`).
- `-o FILE`: the one program's file. `-o DIR/`: every file in DIR by its file name.
- Refusals go to stderr with exit 1, and nothing is written. Examples:
  - `refused: program `prog` is certified except G5 (unchecked: no `at:`); export it anyway with --allow-uncertified (its header says so)`
  - `refused: no program `nope` (the document's: prog)`
  - `refused: program `p2` is not posted: …`

## Tests

| where | what |
|---|---|
| forge-cam-lang `export::tests` | naming, folder, header (comment, tool list), refused / uncertified / unknown |
| forge-cli `tests/cam_post.rs` | the CLI writes what the app's function makes; `-o FILE`, `-o DIR/`; refused (exit 1, nothing written) unless allowed, then stamped; turning |
| forge-render `tests/cam_ui.rs` `nc_programs_post_export_and_go_stale` | headless native: fields typed into the statement, Post writes `nc/flange-1.nc`, posted → an op edited → stale → Re-post Stale → posted; Export… asks the host with the same text; sheet + tool list; `nc::write_to` |
| forge-render `nc::tests` | several files into a folder by name; the page's JSON |
| web `tests/cam.cjs` | browser: Post into the project (state current), Export… captured as a download equal to the posted file, the sheet + tools as one zip |

The native headless tests need a GPU adapter. On a machine without one, Mesa's lavapipe
(`mesa-vulkan-drivers`) is enough; without it they print "skipped: no GPU" and pass.

## Not yet: language and post asks

These items need forge-cam-lang or forge-post first. The window then gets a field each.

- **Ops selection** (a program of some ops of a setup, not whole setups). `program(..)` takes
  setups, so this needs an `ops:` list in forge-cam-lang. CAM-CORE owns that, and certification
  has to cover a partial setup's stock.
- **One file per setup** within a program. Today you make one program per setup (New Program,
  tick one setup).
- **Post options**: units (G20/G21), decimals, safe-start block, coolant, line numbers (N-words).
  These are dialect data in forge-post today. Each one changes the certified text, so each must
  become a post argument that the round trip (G6) covers, not a rewrite after posting.

## Send to machine (design only)

A program's destination is one more argument, `send:`, naming a machine connection kept per
machine in the project (`cam/machines/<name>.toml`), not in the document:

```toml
[send]
kind = "folder"            # folder | usb | share | ftp | dnc
path = "/mnt/haas-share/PROGRAMS"
name = "O{number}"         # the control's naming rule; {number} {file} {ext}
overwrite = "ask"          # ask | never | always
```

- **folder / usb / share**: a copy to a mounted path, with the control's naming rule applied
  (Haas wants `O01001.nc`, Fanuc memory cards `O1001`). This is the same write as Post. Native
  only; the browser shows "download instead".
- **ftp**: many controls (Haas NGC, Okuma, Mazak) take FTP. A later step: a small FTP client in
  forge-render, natively only. The password goes in the platform keychain, never in the TOML.
- **dnc (RS-232 drip feed)**: a serial port, XON/XOFF, the control's baud rate. A later step,
  native only. It needs a serial crate, which must be judged against the no-third-party rule for
  kernel/ (this would live in interface/).
- Each send logs the program's hash and certification in the project's `cam/sent.log`, so a
  program on the machine can be traced to the run that certified it.
- A send is refused under the same certification rule as Post. A stale program is refused until
  it is re-posted.
