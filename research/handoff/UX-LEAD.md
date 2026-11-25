# UX-LEAD handoff

Written at wind-down. The standard is `docs/ux/STANDARD.md`, and its gate is §11: a UI
branch rides a train only with a fresh shot set at its exact sha and a UX OK sent to
TRAIN-CONDUCTOR by that sha. Severities: P1 is a crash, a dead or unreachable control, or a
lie; P2 is missing words, units or reasons; P3 is polish.

**Who OKs UI after me:** the coordinator decides. BUTTON-AUDITOR, the natural successor (they
audited the whole shell against the standard), is inactive. Awaiting an OK when I stopped:
HOSTED-BUILD's hosted/client-web a2809600 and hosted/client-native 69759c78, and
IMPORT-SPEED's import/faces.

## My branches

- **ux/standard-focus**: in train 8 (main 10d87778). It adds the §9 rule on dialog focus.
- **ux/part-box**: in train 8. A part's box follows its shape, and a sheet's flat pattern
  frames as flat.
- **ux/undrawn-faces** @ 0f37fb14: pre-verify GREEN on main 10d87778 (tool b9318989).
  It rides with IMPORT-SPEED's fix/undrawn-face f7253c4d (train 10). A part drawn
  without faces the mesher failed on says so in Issues (counted, the reason on hover,
  Show), and the border of each left-out face is outlined in amber in the view
  (render `Mark::warn`, `WARN_BIT` = 1<<29; placements keep 29 bits).
- **ux/narrow-browser** (1cc6792a): **withdrawn**. The browser's default width followed
  the window (0.375 of it, down to 180 px), so at 480 px the view or CAM column got ~300 px
  instead of 209. At 180 px the Agent tab's header has no room for its mode chip
  (`agent:mode`), and `tests/agent_approvals.rs` `polish_pictures` fails at 480 px (P1:
  unreachable). To revive it, keep the narrow default at 240 px or more, or let the Agent
  header wrap, then re-run `agent_approvals` at 480. The need stands: at 480 px a 209 px CAM
  column side-cuts its rows.

## Open polish (P2)

- **Analyze Interference's expect list** still reads "FAIL line 3: interferences 1 (limit
  0)". It should use the Issues window's words ("expected no interferences, found 1",
  LANG-PROOFS' claude/expect-words), with the raw line on hover. LANG-PROOFS' handoff has
  the way (ExpectOut needs cmp and tol).
- **Interference volume** reads "500.0000 mm3"; it should be "500 mm³" (ux::show with
  Qty::Volume).
- **Web worker states** (WEB-WORKER, hosted/web-worker). Asked for, not yet seen:
  - a pick waiting on the worker past ~300 ms shows ux::loading with "Waiting for the build
    to take the pick", and Esc drops it;
  - "Exporting <file>" with the spinner, not a trailing "…";
  - the Project Geometry refusal in words a user can act on ("… can't project this edge in
    the browser yet: its curve comes from a blend or an intersection. The desktop app
    can."), with `?worker=0` on hover only.

## Open polish (P3)

- **Render Studio framing** (RENDER-STUDIO): on entering the Studio the part fills ~20% of
  the view, although env_frame_view asks for 0.7 (§2). Fit All on entry was tried and
  reverted, because it broke tests/environments.rs. See research/handoff/RENDER-STUDIO.md.
- **CAM machine view** (CAM-SIM, cam/sim-machine): with the machine shown, the view frames
  the head, and the part is a few percent of it (§2). The position readout runs Y X Z;
  X Y Z is expected.
- **Bend simulation**: during a handling step the camera sits on the press's upper tooling,
  and the part being handled isn't clearly in frame (§2). The bed strip's station labels
  overlap ("s3s4") when stations are close.
- **Workholding vise warning** (WORKHOLDING), an optional rewording without the inner
  backticks: "vise("generic-vise-150") is now the workholding library's standin-vise-150:
  the same jaws and opening, with its body down to the table. It's a stand-in: measure your
  vise."
- **Undrawn faces, Show**: Show frames a left-out face by its box radius, which leaves a thin
  face small in the view. It could frame at 60–75% (§2) with frame::frame on the face's box.
- **CAM 480 px shot**: re-shoot `research/cam/ui/cam-480-view.png` if the browser's width at
  480 px ever changes.

## Decisions made in review (for the standard's next edit)

- A working state shows the shared spinner (ux::loading) and a word ("checking",
  "Tracing · N samples", "Exporting <file>"). "…" stays reserved for commands that open a
  dialog.
- A view setting the user chose (occlusion, background picture) never resets itself on an
  edit.
- What the view leaves out says so: an Issues warning plus a visible mark in the view (the
  amber outline of undrawn faces).
- A disabled control's reason is the reason alone, with no "Off:" prefix.
- Status lines shorten a path's folders first, never its file name, then cut the line from
  its end. The whole line goes on hover.
- Scroll bars are visible in both themes, with the handle at least 3:1 against its track.
- One name per thing: a tool is called by the same designation in the panel, captions,
  chips and hovers; catalogue keys go on hover.
- Narrow windows (under ~320 px of view): the properties column and the view take turns
  (a View tab and a chip back), rather than overlapping.
