TAKEN = "2026-11-23"

CONV_VMC = ("joint 0 = mid-travel for X and Y and the top of travel for Z; the control's machine zero "
            "is at a travel end (which: UNKNOWN), so a real G54 differs by a constant per axis")
CONV_LATHE = ("where each stroke sits relative to the spindle centreline and chuck face is UNKNOWN: X's top "
              "end is put at the max turning radius and Z's at the max turning length + 50 mm (estimates)")
ENV = ("bodies are single boxes/cylinders from published overall sizes; head, column, saddle, bed and "
       "turret sizes are estimates, the spindle housing radius kept large; the enclosure is not modelled")

def note(*parts):
    return "; ".join(p for p in parts if p) + "."

def mm_min(m_min):
    return float(m_min) * 1000.0

def rpm_to_deg_min(rpm):
    return float(rpm) * 360.0

G = 9806.65  # mm/s^2

# A required field the maker does not publish takes the value that makes its check vacuous (never a
# false alarm) and keeps t_min a lower bound; each such field carries this comment.
VAC_RAPID = 100000.0        # mm/min: above any machine here, so t_min stays a lower bound
VAC_ROT = 360000.0          # deg/min (1000 rpm): above any rotary here
UNK_RAPID = "UNKNOWN (unpublished): a high value keeps t_min a lower bound"
UNK_FEED = "max cutting feed UNKNOWN (unpublished): = rapid, no feed clamp"
UNK_CHANGE = "UNKNOWN (unpublished): 0.001 s (must be > 0), so t_min stays a lower bound"
UNK_KG = "UNKNOWN (unpublished): no weight check"
VACUOUS = ("an unpublished required field takes the value that makes its check vacuous (rapid high, "
           "max feed = rapid, change time 0.001 s, tool weight 99 kg), named UNKNOWN beside it")

def mark(m, joint, field, comment):
    for j in m["joints"]:
        if j["name"] == joint:
            j["c_" + field] = comment
            return
    raise KeyError(joint)

def over(m, path, status, basis):
    m.setdefault("cite_over", {})[path] = dict(status=status, basis=basis)

NOSE_EST = "spindle nose to table not published: placed as the note says"
def nose_estimate(m, basis=NOSE_EST):
    over(m, "spindle.S1.nose", "estimate", basis)
    over(m, "mount.spindle.at", "estimate", basis)
