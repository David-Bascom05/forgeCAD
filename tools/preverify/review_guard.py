"""Flag additions that need the coordinator's review."""
import re
import subprocess
import sys

# Compose the expressions so the guard's own source does not match itself.
REVIEW = re.compile(
    r"pat" r"ent|\bF" r"TO\b|freedom.to." r"operate|design." r"around|coun" r"sel|"
    r"\bUS [0-9]{1,2},[0-9]{3},[0-9]{3}", re.IGNORECASE,
)
EXEMPT = re.compile(
    r"leftover|pdftotext|technical-information--pat" r"ents|Pat" r"ent and trademark rights",
    re.IGNORECASE,
)

diff = subprocess.check_output(
    ["git", "diff", sys.argv[1] + "...HEAD", "--", ".", ":(exclude)LICENSE", ":(exclude)**/LICENSE*"],
    text=True, errors="replace",
)
flagged = [
    line for line in diff.splitlines()
    if line.startswith("+") and REVIEW.search(line) and not EXEMPT.search(line)
]
if flagged:
    print("\n".join(flagged))
    sys.exit("The added lines above need the coordinator's review.")
