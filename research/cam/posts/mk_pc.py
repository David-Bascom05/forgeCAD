import json, os
P = "/tmp/claude-0/-home-user-forgecad/f84f38ce-577c-5af8-87d9-367113b8c34b/scratchpad/posts"
OUT = "/home/user/forgecad/kernel/forge-post/data"

def srcmap(n):
    d = json.load(open(f"{P}/{n}.json"))
    return {s["id"]: s for s in d["sources"]}, d["items"]

def esc(x):
    return x.replace("\\", "\\\\").replace('"', '\\"')

def write(n, header, fields, cites, refused):
    sm, items = srcmap(n)
    out = header.rstrip() + "\n[dialect]\n"
    for k, v in fields:
        if isinstance(v, bool):
            out += f"{k} = {'true' if v else 'false'}\n"
        elif isinstance(v, (int, float)):
            out += f"{k} = {v}\n"
        elif isinstance(v, list):
            out += f"{k} = [" + ", ".join(str(x) if isinstance(x, (int, float)) else f'"{esc(x)}"' for x in v) + "]\n"
        else:
            out += f'{k} = "{esc(v)}"\n'
    out += "cite_required = true\n"
    for key, item, note in cites:
        it = items[item]
        s = sm[it["source"].split()[0].strip(",;")]
        out += (f'\n[[dialect.cite]]\nkey = "{key}"\nsource = "{s["url"]}"\ntaken = "{s.get("taken", "2026-11-23")}"\n'
                f'where = "{esc((s.get("title") or "") + ", " + (s.get("edition") or "") + "; " + (it.get("where") or ""))}"\nnote = "{esc(note)}"\n')
    for key, note in refused:
        out += f'\n[[dialect.cite]]\nkey = "{key}"\nstatus = "refused"\nnote = "{esc(note)}"\n'
    open(f"{OUT}/{n}.toml", "w").write(out)

EXP_PECK = "posted as moves: the control's peck retract/clearance is a parameter (or fixed differently) and forge-post's cycle model would not match it"

# ---------------------------------------------------------------------------------------------
write("pathpilot", """# Tormach PathPilot (mill): restated from Tormach's openly published G-code and machine-code pages
# (tormach.com) and the PathPilot post processor guidelines (knowledgebase.tormach.com); every code
# is cited below. Values were read through a page summariser: verify against the pages.
""", [
 ("name","pathpilot"),("extension","nc"),("decimals",4),("feed_decimals",1),("point","always"),
 ("program_open",["({title})"]),("program_close",["M30"]),("comment","({text})"),("comment_strip","()"),
 ("line_numbers",False),("safe_start",["G17 G21 G40 G49 G80 G90 G94"]),
 ("tool_change",["T{t} M06 G43 H{h}"]),("spindle_cw","S{rpm} M03"),("spindle_ccw","S{rpm} M04"),("spindle_stop","M05"),
 ("wcs","G{n}"),("wcs_first",54),("wcs_count",6),("arc","ijk"),
 ("coolant_flood","M08"),("coolant_mist","M07"),("coolant_off","M09"),("dwell","G04 P{s}"),
 ("cycle_drill","G98 G81 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_dwell","G98 G82 X{x} Y{y} Z{z} R{r} P{s} F{f}"),
 ("cycle_peck","expand"),("cycle_chipbreak","expand"),("cycle_tap","refuse"),
 ("cycle_ream","G98 G85 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_next","X{x} Y{y}"),("cycle_cancel","G80"),
 ("source","Tormach PathPilot G-code reference pages and post guidelines (cited per code)"),("taken","2026-11-23"),
 ("note","no % marks (the guidelines say so); G43 H on the tool-change line; arcs in the default incremental centre mode (G91.1)"),
],[
 ("program_open","program_open","no start code; no % at the start or end"),
 ("program_close","program_close","M30 end and rewind"),
 ("comment","comment","( ) comments"),
 ("line_numbers","line_numbers","N optional: not written"),
 ("safe_start","safe_start","G17 G21 G40 G49 G80 G90 G94 all supported"),
 ("tool_change","tool_change","T M06 with G43 H on the tool-change line"),
 ("spindle_cw","spindle","S M03"),("spindle_ccw","spindle","S M04"),("spindle_stop","spindle","M05"),
 ("wcs","work_offset","G54..G59"),("arc","arc_format","G02/G03 I J incremental from the start (G91.1 default)"),
 ("coolant_flood","coolant","M08 flood"),("coolant_mist","coolant","M07 mist"),("coolant_off","coolant","M09"),
 ("dwell","dwell","G04 P seconds"),("cycle_drill","drill_G81","G81"),("cycle_dwell","dwell_drill_G82","G82 P seconds"),
 ("cycle_peck","peck_G83",EXP_PECK),("cycle_chipbreak","chipbreak_G73","G73 retracts a fixed 0.010 in: "+EXP_PECK),
 ("cycle_ream","bore_G85","G85 feed in, feed out"),("cycle_next","drill_G81","cycles modal: further holes as X Y"),
 ("cycle_cancel","cycle_cancel","G80"),("point","decimal_point","number with optional point: written with one"),
 ("decimals","max_digits","17 significant figures kept; 4 decimals written"),
],[("cycle_tap","G84 dwells 0.5 s per 1000 rpm at the bottom when P is left out, and whether P0 is accepted is not stated: refused"),
  ("retract_home","G28 goes to a user-stored position (G28.1), not a fixed home: refused (end with a rapid to clearance)")])

# ---------------------------------------------------------------------------------------------
write("mach3", """# Mach3 Mill: restated from "Using Mach3Mill" Rev 1.84-A2 (machsupport.com); every code is cited
# below. Mach3 is configured per machine: coolant outputs and the dwell unit are settings, so those
# are refused rather than guessed.
""", [
 ("name","mach3"),("extension","tap"),("decimals",4),("feed_decimals",1),("point","always"),
 ("program_open",["({title})"]),("program_close",["M30"]),("comment","({text})"),("comment_strip","()"),
 ("line_numbers",False),("safe_start",["G17 G21 G40 G49 G80 G90 G94", "G91.1"]),
 ("tool_change",["T{t} M6"]),("length_comp","G43 H{h}"),("spindle_cw","S{rpm} M3"),("spindle_ccw","S{rpm} M4"),("spindle_stop","M5"),
 ("wcs","G{n}"),("wcs_first",54),("wcs_count",6),("arc","ijk"),("retract_home",["G91 G28 Z0.","G90"]),
 ("coolant_off","M9"),
 ("cycle_drill","G98 G81 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_dwell","expand"),
 ("cycle_peck","expand"),("cycle_chipbreak","expand"),("cycle_tap","G98 G84 X{x} Y{y} Z{z} R{r} F{f}"),
 ("cycle_ream","G98 G85 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_next","X{x} Y{y}"),("cycle_cancel","G80"),
 ("read_g_ignore",[91.1]),
 ("source","Using Mach3Mill Rev 1.84-A2 (cited per code)"),("taken","2026-11-23"),
 ("note","G91.1 states the incremental arc-centre mode (a configuration otherwise); flood/mist coolant and dwell are refused (machine-configured)"),
],[
 ("program_open","program_open","no start code; a comment names the program"),
 ("program_close","program_close","M30"),("comment","comment","( ) on one line, not nested"),
 ("line_numbers","line_numbers","N optional: not written"),
 ("safe_start","safe_start","G17 G21 G40 G49 G80 G90 G94; G91.1 incremental IJ (10.7.26)"),
 ("tool_change","tool_change","T M6 (M6Start macro, Cycle Start, M6End)"),("length_comp","tool_length_comp","G43 H"),
 ("spindle_cw","spindle","S M3"),("spindle_ccw","spindle","S M4"),("spindle_stop","spindle","M5"),
 ("wcs","work_offset","G54..G59"),("arc","arc_format","I J incremental under G91.1"),
 ("retract_home","retract_home","G28 with Z only, from G91 Z0: straight up to Z home"),
 ("coolant_off","coolant","M9 all off"),("cycle_drill","drill_G81","G81"),
 ("cycle_dwell","dwell_drill_G82","posted as moves: the dwell between them is refused (units are a setting)"),
 ("cycle_peck","peck_G83",EXP_PECK),("cycle_chipbreak","chipbreak_G73","G73 pull-back is a Settings DRO: "+EXP_PECK),
 ("cycle_tap","rigid_tap_G84","G84 F units/min matched to the pitch (F = pitch x rpm)"),
 ("cycle_ream","bore_G85","G85"),("cycle_next","drill_G81","cycles modal: further holes as X Y"),("cycle_cancel","cycle_cancel","G80"),
 ("point","decimal_point","written with a point"),("decimals","max_digits","4 decimals written"),
 ("read_g_ignore","safe_start","G91.1 (arc mode) carries no motion"),
],[("coolant_flood","M7/M8 flood vs mist: the manual contradicts itself and the outputs are user-assigned"),
  ("coolant_mist","M7/M8 flood vs mist: the manual contradicts itself and the outputs are user-assigned"),
  ("dwell","G4 P is seconds or milliseconds by a Config>Logic setting the program cannot see")])

# ---------------------------------------------------------------------------------------------
write("mach4", """# Mach4 Mill: restated from the "Mach4 Mill G-code Programming Guide" v1.1 build 3775
# (machsupport.com); every code is cited below. The guide does not state the comment syntax, so the
# post writes no comments.
""", [
 ("name","mach4"),("extension","tap"),("decimals",4),("feed_decimals",1),("point","always"),
 ("program_open",[]),("program_close",["M30","%"]),
 ("line_numbers",False),("safe_start",["G00 G90 G17 G40 G49 G80", "G21 G94", "G91.1"]),
 ("tool_change",["T{t} M06"]),("length_comp","G43 H{h}"),("spindle_cw","S{rpm} M03"),("spindle_ccw","S{rpm} M04"),("spindle_stop","M05"),
 ("wcs","G{n}"),("wcs_first",54),("wcs_count",6),("arc","ijk"),("retract_home",["G53 G00 Z0"]),
 ("coolant_flood","M08"),("coolant_mist","M07"),("coolant_off","M09"),("dwell","G04 P{ms}"),
 ("cycle_drill","G98 G81 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_dwell","expand"),
 ("cycle_peck","expand"),("cycle_chipbreak","expand"),("cycle_tap","G98 G84 X{x} Y{y} Z{z} R{r} F{f}"),
 ("cycle_ream","G98 G85 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_next","X{x} Y{y}"),("cycle_cancel","G80"),
 ("read_g_ignore",[91.1]),
 ("source","Mach4 Mill G-code Programming Guide v1.1 (cited per code)"),("taken","2026-11-23"),
 ("note","comments are not written (the guide gives no comment syntax); a trailing % guarantees the last end-of-block"),
],[
 ("program_open","program_open","no start code required: none written"),
 ("program_close","program_close","M30, then % so the last block has its EOB"),
 ("line_numbers","line_numbers","N optional: not written"),
 ("safe_start","safe_start","the guide's safe-start block, G21 G94, G91.1 incremental arc centres"),
 ("tool_change","tool_change","T M06"),("length_comp","tool_length_comp","G43 H"),
 ("spindle_cw","spindle","S M03"),("spindle_ccw","spindle","S M04"),("spindle_stop","spindle","M05"),
 ("wcs","work_offset","G54..G59"),("arc","arc_format","I J incremental under G91.1"),
 ("retract_home","retract_home","G53 Z0: the guide's safer way home"),
 ("coolant_flood","coolant","M08"),("coolant_mist","coolant","M07"),("coolant_off","coolant","M09"),
 ("dwell","dwell","G04 P without a point = milliseconds"),("cycle_drill","drill_G81","G81"),
 ("cycle_dwell","dwell_drill_G82","posted as moves with G04 P ms: the guide's cycle P units contradict its example"),
 ("cycle_peck","peck_G83","G83 retract clearance is parameter 5115: "+EXP_PECK),
 ("cycle_chipbreak","chipbreak_G73","G73 retract is parameter 5114: "+EXP_PECK),
 ("cycle_tap","rigid_tap_G84","G84 F = rpm x pitch under G94"),("cycle_ream","bore_G85","G85"),
 ("cycle_next","drill_G81","cycles modal: further holes as X Y"),("cycle_cancel","cycle_cancel","G80"),
 ("point","decimal_point","a point is not required but recommended: always written"),
 ("decimals","feed","4 decimals written"),("read_g_ignore","arc_format","G91.1 carries no motion"),
],[])

# ---------------------------------------------------------------------------------------------
write("centroid-cnc12", """# Centroid CNC12 (mill): restated from the CNC12 v4.12 mill operator's manual (centroidcnc.com);
# every code is cited below. Parentheses are expressions on this control, not comments: comments
# are ; to the end of the line. G0 moves Z first going up (a dog-leg): simulate with dogleg = true.
""", [
 ("name","centroid-cnc12"),("extension","nc"),("decimals",4),("feed_decimals",1),("point","always"),
 ("program_open",["; {title}"]),("program_close",["M30"]),("comment","; {text}"),("comment_eol",";"),("comment_strip",";()[]"),
 ("line_numbers",False),("safe_start",["G17 G21 G40 G49 G80 G90 G94"]),
 ("tool_change",["T{t} M6"]),("length_comp","G43 H{h}"),("spindle_cw","S{rpm} M3"),("spindle_ccw","S{rpm} M4"),("spindle_stop","M5"),
 ("wcs","G{n}"),("wcs_first",54),("wcs_count",6),("arc","ijk"),("retract_home",["G91 G28 Z0.","G90"]),
 ("coolant_flood","M8"),("coolant_mist","M7"),("coolant_off","M9"),("dwell","G4 P{s}"),
 ("cycle_drill","G98 G81 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_dwell","G98 G82 X{x} Y{y} Z{z} R{r} P{s} F{f}"),
 ("cycle_peck","expand"),("cycle_chipbreak","expand"),("cycle_tap","G98 G84 X{x} Y{y} Z{z} R{r} F{f}"),
 ("cycle_ream","G98 G85 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_next","X{x} Y{y}"),("cycle_cancel","G80"),
 ("source","Centroid CNC12 v4.12 mill operator's manual (cited per code)"),("taken","2026-11-23"),
 ("note","M30 does nothing unless customised (Intercon posts it); M2 would restart the program, so it is never written"),
],[
 ("program_open","program_open","no start code; a ; comment names the program"),
 ("program_close","program_close","M30 (customised end; M2 restarts the program)"),
 ("comment","comment","; to the end of the line; ( ) [ ] are expressions"),("comment_eol","comment","; to the end of the line"),
 ("line_numbers","line_numbers","N optional: not written"),
 ("safe_start","safe_start","G17 G21 G40 G49 G80 G90 G94"),
 ("tool_change","tool_change","T M6 (M6 always moves Z home first)"),("length_comp","tool_length_comp","G43 H"),
 ("spindle_cw","spindle","S M3"),("spindle_ccw","spindle","S M4"),("spindle_stop","spindle","M5"),
 ("wcs","work_offset","G54..G59"),("arc","arc_format","I J K incremental from the start (default)"),
 ("retract_home","retract_home","G28 G91 Z0: Z straight home"),
 ("coolant_flood","coolant","M8"),("coolant_mist","coolant","M7"),("coolant_off","coolant","M9"),
 ("dwell","dwell","G4 P seconds"),("cycle_drill","drill_G81","G81"),("cycle_dwell","dwell_drill_G82","G82 P seconds"),
 ("cycle_peck","peck_G83","G83 clearance set by G10 P83: "+EXP_PECK),("cycle_chipbreak","chipbreak_G73","G73 retract set by G10 P73: "+EXP_PECK),
 ("cycle_tap","rigid_tap_G84","floating G84: F (units/min) with S sets the lead"),("cycle_ream","bore_G85","G85"),
 ("cycle_next","cycle_cancel","cycles need X and/or Y on each hole"),("cycle_cancel","cycle_cancel","G80"),
 ("point","decimal_point","always a point: integers may be reinterpreted by parameters 245/246"),("decimals","feed","4 decimals written"),
],[])

# ---------------------------------------------------------------------------------------------
write("masso", """# MASSO G3 (mill): restated from MASSO's documentation (docs.masso.com.au, updated 5 Oct 2026);
# every code is cited below. MASSO skips a whole line containing a code it does not support, so only
# listed codes are written: no G43/G49 (length offsets are applied at the tool change), no G84/G85.
""", [
 ("name","masso"),("extension","nc"),("decimals",4),("feed_decimals",1),("point","always"),
 ("program_open",["({title})"]),("program_close",["M30"]),("comment","({text})"),("comment_strip","()"),
 ("line_numbers",False),("safe_start",["G17 G21 G40 G80 G90 G94"]),
 ("tool_change",["T{t} M06"]),("spindle_cw","S{rpm} M03"),("spindle_ccw","S{rpm} M04"),("spindle_stop","M05"),
 ("wcs","G{n}"),("wcs_first",54),("wcs_count",6),("arc","ijk"),("retract_home",["G53 G00 Z0"]),
 ("coolant_flood","M08"),("coolant_mist","M07"),("coolant_off","M09"),("dwell","G04 P{ms}"),
 ("cycle_drill","G98 G81 X{x} Y{y} Z{z} R{r} F{f}"),("cycle_dwell","G98 G82 X{x} Y{y} Z{z} R{r} P{ms} F{f}"),
 ("cycle_peck","expand"),("cycle_chipbreak","expand"),("cycle_tap","refuse"),("cycle_ream","expand"),
 ("cycle_next","X{x} Y{y}"),("cycle_cancel","G80"),
 ("source","MASSO documentation, docs.masso.com.au (cited per code)"),("taken","2026-11-23"),
 ("note","the program must stop the spindle (M05) before M06; T before M06 on the line"),
],[
 ("program_open","comment","a ( ) comment names the program"),("program_close","program_close","M30"),
 ("comment","comment","( ) not nested"),("line_numbers","line_numbers","N optional: not written"),
 ("safe_start","safe_start","G17 G21 G40 G80 G90 G94 (G49 is not supported)"),
 ("tool_change","tool_change","T before M06; the control applies the length offset itself"),
 ("spindle_cw","spindle","S M03"),("spindle_ccw","spindle","S M04"),("spindle_stop","spindle","M05"),
 ("wcs","work_offset","G54..G59"),("arc","arc_format","I J K always incremental from the start"),
 ("retract_home","retract_home","G53 G00 Z0: Z to the top of travel"),
 ("coolant_flood","coolant","M08"),("coolant_mist","coolant","M07"),("coolant_off","coolant","M09"),
 ("dwell","dwell","G04 P milliseconds"),("cycle_drill","drill_G81","G81 (R required)"),("cycle_dwell","dwell_drill_G82","G82 P milliseconds"),
 ("cycle_peck","peck_G83",EXP_PECK),("cycle_chipbreak","chipbreak_G73","G73 retracts a fixed 1.0 mm: "+EXP_PECK),
 ("cycle_ream","rapid_linear","G85 is not supported: posted as G00/G01 moves"),
 ("cycle_next","drill_G81","cycles modal: further holes as X Y"),("cycle_cancel","cycle_cancel","G80"),
 ("point","feed","written with a point"),("decimals","feed","4 decimals written"),
],[("cycle_tap","G84 is not in MASSO's supported list")])
print("ok")
