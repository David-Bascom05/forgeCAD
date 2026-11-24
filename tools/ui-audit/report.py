#!/usr/bin/env python3
"""The UI audit's report: records.tsv (tests/ui_audit's full crawl) plus the visual review's
verdicts (research/ui-audit/verdicts.tsv) into research/ui-audit/report.md and thumbs.md, the
thumbnails copied beside them.

usage: report.py CRAWL_DIR [CRAWL_DIR_480 ...]  (a narrow crawl's records add its off-screen and
clipping findings to the same commands)"""
import csv, os, shutil, sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "research", "ui-audit")

WEIGHT = {"page-crash": 100, "page-nothing": 60, "crash": 100, "nothing": 60, "error-text": 30, "off-screen": 25, "no-exit": 25,
          "silent-mode": 20, "raw-text": 15, "empty": 15, "overlap": 12, "clipped": 10,
          "no-reason": 8, "tiny-text": 6, "contrast": 6}
SOFT = 3


def read(d, name="records.tsv"):
    with open(os.path.join(d, name), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE))


def key(r):
    env = (r["env"] + ": ") if r["env"] else ""
    return f'{r["fixture"]}/{env}{r["tab"]}/{r["tool"]}'


def findings(r):
    out = []
    for part in filter(None, (p.strip() for p in r["findings"].split(" | "))):
        hard = part.startswith("!")
        rule, _, what = part[1:].partition(": ")
        out.append((rule, hard, what))
    return out


SMALL = {"of", "to", "and", "or", "the", "a", "an", "in", "on", "for", "from", "by", "as", "at", "into"}


def label_findings(r):
    """STANDARD §5 (ux/glyphs): Title Case on the ribbon; "…" one character, only before a dialog."""
    name, out = r["tool"], []
    if any(rule == "label" for rule, _, _ in findings(r)):
        return out
    if "..." in name:
        out.append(("label", False, f'"{name}": three dots for "…"'))
    if r["tab"].startswith("menu ") is False and r["class"] != "DISABLED":
        if name.endswith(("…", "...")) and r["class"] not in ("DIALOG", "PANEL", "FILE PICKER", "HOST"):
            out.append(("label", False, f'"{name}": "…" but no dialog follows'))
    if not r["tab"].startswith("menu "):
        low = [w for w in name.replace("/", " ").split()[1:] if w not in SMALL and w[:1].islower()]
        if low:
            out.append(("label", False, f'"{name}": not Title Case ({", ".join(low)})'))
    return out


P1 = {"crash", "nothing", "page-crash", "page-nothing", "silent-mode"}
P2 = {"raw-text", "no-unit", "no-reason", "clipped", "off-screen", "no-exit", "error-text", "tiny-text", "empty", "overlap"}


def plevel(r, fs):
    rules = {rule for rule, _, _ in fs}
    if r["class"] in ("CRASH", "NOTHING VISIBLE") or rules & P1:
        return "P1"
    if rules & P2:
        return "P2"
    if fs:
        return "P3"
    return ""


def score(r, fs):
    s = {"CRASH": 100, "NOTHING VISIBLE": 0, "ERROR TEXT": 30}.get(r["class"], 0)
    for rule, hard, _ in fs:
        s += WEIGHT.get(rule, 8) if hard else SOFT
    return s


def main():
    dirs = sys.argv[1:]
    if not dirs:
        sys.exit(__doc__)
    recs = read(dirs[0])
    web = {}
    for d in dirs:
        if os.path.exists(os.path.join(d, "records-web.tsv")):
            for r in read(d, "records-web.tsv"):
                web[key(r)] = r
    narrow = defaultdict(list)
    for d in dirs[1:]:
        if not os.path.exists(os.path.join(d, "records.tsv")):
            continue
        for r in read(d):
            for rule, hard, what in findings(r):
                if rule in ("off-screen", "clipped", "overlap"):
                    narrow[key(r)].append((rule + "@narrow", hard, what))
    verdicts = {}
    vpath = os.path.join(OUT, "verdicts.tsv")
    if os.path.exists(vpath):
        for line in open(vpath):
            if line.strip() and not line.startswith("#"):
                k, v, *note = line.rstrip("\n").split("\t")
                verdicts[k] = (v, note[0] if note else "")
    os.makedirs(os.path.join(OUT, "thumbs"), exist_ok=True)
    rows = []
    for r in recs:
        fs = findings(r) + label_findings(r) + narrow.get(key(r), [])
        w = web.get(key(r))
        if w and w["class"] in ("CRASH", "NOTHING VISIBLE") and r["class"] not in ("CRASH", "NOTHING VISIBLE"):
            fs.append(("page-" + ("crash" if w["class"] == "CRASH" else "nothing"), True, w["detail"]))
        s = score(r, fs)
        auto = "redo" if s >= 40 else "fix" if s >= 8 else "good"
        if r["class"] == "DISABLED" and not fs:
            auto = "good"
        v, note = verdicts.get(key(r), (auto, ""))
        thumb = ""
        if r["thumb"]:
            src = os.path.join(dirs[0], r["thumb"])
            if os.path.exists(src):
                name = os.path.basename(src)
                shutil.copy(src, os.path.join(OUT, "thumbs", name))
                thumb = f"thumbs/{name}"
        rows.append(dict(r, p=plevel(r, fs), web=(w["class"] if w else ""), key=key(r), fs=fs, score=s, verdict=v, auto=auto, note=note, thumbp=thumb))
    classes = Counter(r["class"] for r in rows)
    vs = Counter(r["verdict"] for r in rows if r["class"] != "DISABLED")
    worst = sorted((r for r in rows), key=lambda r: -r["score"])[:20]
    L = []
    L.append("# UI audit: every ribbon command, clicked\n")
    L.append("Made by `tools/ui-audit/crawl.sh` (the crawler is `interface/forge-render/tests/ui_audit`). "
             "Each command is clicked headless in a document that suits it, from the app's own registry "
             "(`Shell::ribbon_live`), at 1400x860 and again at 480 px wide. What it did is classed; what it "
             "opened is judged against docs/ux/STANDARD.md (automatic rules, then the visual review in "
             "`verdicts.tsv`). Verdicts: good / fix / redo.\n")
    L.append(f"**{len(rows)} commands** in {len(set(r['fixture'] for r in rows))} documents.\n")
    L.append("| What a click does | Commands |\n|---|---|")
    for c, n in sorted(classes.items(), key=lambda x: -x[1]):
        L.append(f"| {c} | {n} |")
    L.append("")
    L.append("| Verdict (live commands) | Commands |\n|---|---|")
    for c in ("redo", "fix", "good"):
        L.append(f"| {c} | {vs.get(c, 0)} |")
    L.append("")
    rules = Counter(rule for r in rows for rule, hard, _ in r["fs"] if hard)
    L.append("| Rule broken | Commands |\n|---|---|")
    for rule, n in rules.most_common():
        L.append(f"| {rule} | {n} |")
    L.append("\n## The 20 worst\n")
    for i, r in enumerate(worst, 1):
        probs = "; ".join(f"{rule}: {what}" for rule, hard, what in r["fs"] if hard) or r["detail"]
        L.append(f"{i}. **{r['key']}** ({r['p']}, {r['class']}, score {r['score']}, {r['verdict']}): {probs}")
    L.append("\n## Every command\n")
    by = defaultdict(list)
    for r in rows:
        by[(r["fixture"], r["env"], r["tab"])].append(r)
    for (fx, env, tab), rs in by.items():
        L.append(f"### {fx} · {env + ': ' if env else ''}{tab}\n")
        L.append("| Command | What happens | Page | Verdict | P | Problems | Shot |\n|---|---|---|---|---|---|---|")
        for r in rs:
            probs = "<br>".join(f"{'**' if hard else ''}{rule}{'**' if hard else ''}: {what}".replace("|", "\\|")
                                for rule, hard, what in r["fs"])
            if r["note"]:
                probs = (probs + "<br>" if probs else "") + "review: " + r["note"].replace("|", "\\|")
            shot = f"![]({r['thumbp']})" if r["thumbp"] else ""
            det = r["detail"].replace("|", "\\|")
            page = "" if not r["web"] else ("same" if r["web"] == r["class"] else r["web"])
            L.append(f"| {r['group']} › {r['tool']} | {r['class']}: {det} | {page} | {r['verdict']} | {r['p']} | {probs} | {shot} |")
        L.append("")
    open(os.path.join(OUT, "report.md"), "w").write("\n".join(L) + "\n")
    T = ["# UI audit thumbnails\n", "Every popup or panel a command opened, worst first.\n"]
    for r in sorted((r for r in rows if r["thumbp"]), key=lambda r: -r["score"]):
        T.append(f"**{r['key']}** · {r['verdict']} · score {r['score']}\n\n![]({r['thumbp']})\n")
    open(os.path.join(OUT, "thumbs.md"), "w").write("\n".join(T) + "\n")
    # UX-LEAD's single table: research/button-audit/SCORES.md
    S = ["# Button audit scores\n",
         "Every live command, worst first: verdict (good / fix / redo), P-level (STANDARD severity), the "
         "rules it breaks. Full detail and shots: research/ui-audit/report.md.\n",
         "| Command | What a click does | Verdict | P | Score | Problems |", "|---|---|---|---|---|---|"]
    for r in sorted((r for r in rows if r["class"] != "DISABLED" or r["fs"]), key=lambda r: -r["score"]):
        probs = "; ".join(f"{rule}: {what}" for rule, hard, what in r["fs"]).replace("|", "\\|")
        S.append(f"| {r['key']} | {r['class']} | {r['verdict']} | {r['p']} | {r['score']} | {probs} |")
    bdir = os.path.join(OUT, "..", "button-audit")
    os.makedirs(bdir, exist_ok=True)
    open(os.path.join(bdir, "SCORES.md"), "w").write("\n".join(S) + "\n")
    print(f"{len(rows)} commands; classes {dict(classes)}; verdicts {dict(vs)}")
    for i, r in enumerate(worst, 1):
        print(f"{i:2}. {r['key']} [{r['class']}] {r['score']}")


if __name__ == "__main__":
    main()
