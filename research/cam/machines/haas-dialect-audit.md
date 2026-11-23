# forge-post `haas` dialect vs Haas's public documentation (2026-11-23)

Checked `kernel/forge-post/data/haas.toml` against the Haas Factory Outlet (Productivity Inc) Mill
Programming training manual (rev 04-2014, classic control; hosted by UCSB, URL in
research/cam/machines/haas/README.md) and the Gerotech (HFO) NGC installation training
(2024). Haas's own NGC operator's manual is on haascnc.com, which refuses automated reads:
anything only it could settle stays OPEN.

| # | dialect | source says | verdict |
|---|---|---|---|
| 1 | `dwell = "G04 P{ms}"`, {ms} an integer | "If the P has no fraction part, the delay is in milliseconds; otherwise ... seconds" (HFO, G04) | **agrees**: forge-post prints {ms} from an i64, no decimal point |
| 2 | `coolant_mist = "M07"` | neither M-code list has M07: classic: M08/M09 flood, M88/M89 through-tool, M83/M84 auto air jet; NGC: M73/M74 tool air blast, M83/M84 auto air gun, MQL listed as an option with no codes shown | **mismatch (evidence, not proof)**: no source supports M07 on a Haas mill. Fix on branch `post/haas-audit`: drop `coolant_mist` so posting mist is refused with a typed error instead of emitting an unsupported code. Haas air blast (M73/M74) has no Coolant variant in forge-post yet |
| 3 | `coolant_through = "M88"`, off `"M89"` | M88 / M89 "Thru the Tool Coolant On/Off (option)" (HFO) | agrees (it's an option: the machine records don't declare `through`) |
| 4 | `retract_home = ["G91 G28 Z0.", "G90"]` | "G28 G91 Z0" returns Z to machine zero, the tool index position (HFO); with G234 or G254 active "DO NOT G28 G91 ... Use G53!" (Gerotech NGC) | agrees for 3-axis. **For a 5-axis Haas post: retract with G53 Z0**, not G28 |
| 5 | `cycle_tap = "G98 G84 ... F{f}"` | G84 starts the spindle itself and needs an S value; no M29 shown; G74 is the left-hand tap (HFO) | agrees (forge-post emits S before the cycle). LH tapping not posted |
| 6 | `program_open = ["%", "O{num:04} ({title})"]` | classic examples use O0010 (4 digits) | agrees for classic; **OPEN for NGC** (5-digit O numbers are widely reported, not confirmed from a source here) |
| 7 | `safe_start` has `G187 P2` | G187 is "Haas specific ... with machines that have the high speed machining option"; E (corner rounding) 0.0001-0.25; setting 191 = default smoothness, normally medium (HFO); G187 + settings 85/191 under the HSM option (Gerotech NGC) | **OPEN, risk**: emitted unconditionally, but documented as an HSM-option code; the P1/P2/P3 to rough/medium/finish mapping isn't confirmed by these sources. Proposal: make it a per-machine option (or drop it and leave smoothness to setting 191) |
| 8 | `G98` with every cycle; G80 cancel | G98 initial point return, G80 cancels (HFO) | agrees |
| 9 | (absent) | Haas lathes, G234 TCPC / G254 DWO / G255 (UMC), M19, M97/M98 | **missing**: no haas-lathe dialect (lathe records use `fanuc-lathe` and say so), no 5-axis codes. Needs Haas's lathe programming manual (an HFO lathe manual copy was not reachable: productivity.com links 404) |

Also noted, not dialect: a Haas M06 moves Z to machine zero by itself (HFO, M06). Posts don't
need to retract before M06, but the simulator must model it (FORMAT-GAPS #8).
