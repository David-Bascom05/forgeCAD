//! A transcript made into what the person would have typed: the open document's names as it
//! writes them ("hole three" is `hole[3]` when the document has one), numbers before a unit as
//! digits, `M6` for "M six", `YZ plane` for "y z plane".

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 8] = [
    "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn ones(w: &str) -> Option<u64> {
    if w == "oh" {
        return Some(0);
    }
    ONES.iter().position(|o| *o == w).map(|k| k as u64)
}

fn tens(w: &str) -> Option<u64> {
    TENS.iter().position(|o| *o == w).map(|k| 10 * (k as u64 + 2))
}

fn number_word(w: &str) -> bool {
    ones(w).is_some() || tens(w).is_some() || matches!(w, "hundred" | "thousand" | "point")
}

/// A unit word as it's written after a number.
fn unit(w: &str) -> Option<&'static str> {
    Some(match w {
        "mm" | "millimetre" | "millimetres" | "millimeter" | "millimeters" | "mil" | "mils" => "mm",
        "cm" | "centimetre" | "centimetres" | "centimeter" | "centimeters" => "cm",
        "degree" | "degrees" | "deg" | "°" => "degrees",
        "inch" | "inches" => "in",
        _ => return None,
    })
}

/// One word as said: its letters and digits lowercased, what's around them kept to put back.
#[derive(Clone, Debug)]
struct Tok {
    lead: String,
    core: String,
    trail: String,
}

fn split_tok(w: &str) -> Tok {
    let keep = |c: char| c.is_alphanumeric() || "[]-_.:°".contains(c);
    let start = w.find(|c: char| c.is_alphanumeric()).unwrap_or(w.len());
    let end = w
        .char_indices()
        .rev()
        .find(|&(_, c)| c.is_alphanumeric() || c == ']' || c == '°')
        .map_or(start, |(k, c)| k + c.len_utf8())
        .max(start);
    let core = &w[start..end];
    if core.chars().all(keep) {
        Tok {
            lead: w[..start].into(),
            core: core.into(),
            trail: w[end..].into(),
        }
    } else {
        Tok {
            lead: String::new(),
            core: w.into(),
            trail: String::new(),
        }
    }
}

/// The pieces a name or a said word is matched by: runs of letters, runs of digits (a decimal
/// kept whole), lowercased. `DL-108` is `dl 108`; `D.L.` is `dl`.
fn pieces(s: &str) -> Vec<String> {
    let s = s.to_lowercase();
    if s.len() > 2 && s.split('.').all(|p| p.is_empty() || p.chars().count() == 1) && s.contains('.') {
        return vec![s.replace('.', "")];
    }
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut digit = false;
    let cs: Vec<char> = s.chars().collect();
    for (k, &c) in cs.iter().enumerate() {
        let dot_in_number =
            c == '.' && digit && cs.get(k + 1).is_some_and(|n| n.is_ascii_digit()) && !cur.contains('.');
        if c.is_alphanumeric() || dot_in_number {
            let d = c.is_ascii_digit() || dot_in_number;
            if !cur.is_empty() && d != digit {
                out.push(std::mem::take(&mut cur));
            }
            digit = d;
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// A run of number words from `ws[0]`: each reading of it (said as a number, or digit by
/// digit) and the words it took, the longest first.
fn read_number(ws: &[&str]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    // digit by digit: "one oh eight"
    let mut digits = String::new();
    for (k, w) in ws.iter().enumerate() {
        match ones(w).filter(|&d| d < 10) {
            Some(d) => {
                digits.push(char::from(b'0' + d as u8));
                if k > 0 {
                    out.push((digits.clone(), k + 1));
                }
            }
            None => break,
        }
    }
    // as a number: "one hundred and eight", "twelve point five"
    let (mut total, mut cur, mut any, mut k) = (0u64, 0u64, false, 0);
    let mut last: Option<&str> = None;
    while k < ws.len() {
        let w = ws[k];
        let fits = match (last, w) {
            (_, "point") => false,
            (Some(l), "and") => {
                l == "hundred"
                    && ws
                        .get(k + 1)
                        .is_some_and(|n| ones(n).is_some() || tens(n).is_some())
            }
            (None, _) => ones(w).is_some_and(|_| w != "oh") || tens(w).is_some(),
            (Some(l), _) if tens(l).is_some() => ones(w).is_some_and(|d| (1..10).contains(&d) && w != "oh"),
            (Some("hundred" | "thousand" | "and"), _) => {
                ones(w).is_some_and(|_| w != "oh") || tens(w).is_some()
            }
            (Some(_), "hundred") => cur > 0 && cur < 100,
            (Some(_), "thousand") => cur > 0,
            _ => false,
        };
        if !fits {
            break;
        }
        match w {
            "hundred" => cur *= 100,
            "thousand" => {
                total += cur * 1000;
                cur = 0;
            }
            "and" => {}
            _ => cur += ones(w).or(tens(w)).unwrap_or(0),
        }
        any = true;
        last = Some(w);
        k += 1;
    }
    if any && last != Some("and") {
        let whole = (total + cur).to_string();
        let mut took = k;
        let mut value = whole.clone();
        if ws.get(k) == Some(&"point") {
            let frac: String = ws[k + 1..]
                .iter()
                .map_while(|w| ones(w).filter(|&d| d < 10))
                .map(|d| char::from(b'0' + d as u8))
                .collect();
            if !frac.is_empty() {
                took = k + 1 + frac.len();
                value = format!("{whole}.{frac}");
            }
        }
        out.push((value, took));
        if took != k {
            out.push((whole, k));
        }
    } else if ws.first() == Some(&"point") {
        let frac: String = ws[1..]
            .iter()
            .map_while(|w| ones(w).filter(|&d| d < 10))
            .map(|d| char::from(b'0' + d as u8))
            .collect();
        if !frac.is_empty() {
            out.push((format!("0.{frac}"), 1 + frac.len()));
        }
    }
    out.sort_by_key(|o| std::cmp::Reverse(o.1));
    out
}

/// A name, matched in the said pieces from `at`: the pieces it took.
fn match_name(said: &[String], at: usize, name: &[String]) -> Option<usize> {
    let mut i = at;
    for np in name {
        let rest = said.get(i..)?;
        let first = rest.first()?;
        if first == np {
            i += 1;
            continue;
        }
        let words: Vec<&str> = rest.iter().map(String::as_str).collect();
        if np.chars().all(|c| c.is_ascii_digit() || c == '.') {
            // a number said in words, or its digits apart ("1 0 8")
            if let Some((_, took)) = read_number(&words).into_iter().find(|(v, _)| v == np) {
                i += took;
                continue;
            }
            let mut joined = String::new();
            let mut took = 0;
            for w in &words {
                if !w.chars().all(|c| c.is_ascii_digit()) || joined.len() >= np.len() {
                    break;
                }
                joined.push_str(w);
                took += 1;
            }
            if joined == *np && took > 1 {
                i += took;
                continue;
            }
            return None;
        }
        // letters spelled out: "p c b", or a short word said as its letters
        let mut joined = String::new();
        let mut took = 0;
        for w in &words {
            if joined.len() >= np.len() || !np[joined.len()..].starts_with(*w) {
                break;
            }
            joined.push_str(w);
            took += 1;
        }
        if joined == *np && took > 0 && (took > 1 || np.len() <= 3) {
            i += took;
            continue;
        }
        return None;
    }
    Some(i - at)
}

/// The transcript as the person would have typed it, given the document's names.
pub fn tidy(text: &str, names: &[String]) -> String {
    let toks: Vec<Tok> = text.split_whitespace().map(split_tok).collect();
    // each piece and the word it came from
    let mut said: Vec<String> = Vec::new();
    let mut from: Vec<usize> = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        for p in pieces(&t.core) {
            said.push(p);
            from.push(k);
        }
    }
    let mut named: Vec<(Vec<String>, &str)> = names
        .iter()
        .filter(|n| !n.trim().is_empty())
        .map(|n| (pieces(n), n.as_str()))
        .filter(|(p, _)| !p.is_empty())
        .collect();
    // a name wins over one it starts with: hole[12] before hole[1]
    named.sort_by_key(|(p, n)| (std::cmp::Reverse(p.len()), std::cmp::Reverse(n.len())));

    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    let word_bounds = |a: usize, n: usize| -> Option<(usize, usize)> {
        let (w0, w1) = (from[a], from[a + n - 1]);
        // only whole words: a piece starts its word and the last one ends its own
        let starts = a == 0 || from[a - 1] != w0;
        let ends = a + n == from.len() || from[a + n] != w1;
        (starts && ends).then_some((w0, w1))
    };
    let mut last_word = None;
    while i < said.len() {
        let hit = named.iter().find_map(|(np, n)| {
            let took = match_name(&said, i, np)?;
            let (w0, w1) = word_bounds(i, took)?;
            // one piece alone is a name only when it's the whole word as written
            if took == 1 && np.len() == 1 && toks[w0].core != *n && said[i] != n.to_lowercase() {
                return None;
            }
            Some((took, w0, w1, *n))
        });
        let w = from[i];
        if let Some((took, w0, w1, n)) = hit {
            out.push(format!("{}{}{}", toks[w0].lead, n, toks[w1].trail));
            i += took;
            last_word = Some(w1);
            continue;
        }
        if last_word == Some(w) {
            i += 1;
            continue;
        }
        let rest: Vec<&str> = said[i..].iter().map(String::as_str).collect();
        // whole words only from here: the word's own text unless a rule rewrites it
        let whole = |k: usize| i + k <= said.len() && (i + k == said.len() || from[i + k] != from[i + k - 1]);
        // "M six": a metric thread
        if (rest[0] == "m") && toks[w].core.len() == 1 && rest.len() > 1 {
            let n = read_number(&rest[1..]).into_iter().next();
            let n = n.or_else(|| {
                rest[1]
                    .chars()
                    .all(|c| c.is_ascii_digit())
                    .then(|| (rest[1].to_string(), 1))
            });
            if let Some((v, took)) = n
                && whole(1 + took)
                && !v.contains('.')
            {
                let end = from[i + took];
                out.push(format!("{}M{v}{}", toks[w].lead, toks[end].trail));
                i += 1 + took;
                last_word = Some(end);
                continue;
            }
        }
        // a number before a unit, in digits; the unit as it's written
        if number_word(rest[0])
            && let Some((v, took)) = read_number(&rest).into_iter().next()
            && whole(took)
            && let Some(u) = rest.get(took).and_then(|u| unit(u))
            && whole(took + 1)
        {
            let end = from[i + took];
            out.push(format!("{}{v}", toks[w].lead));
            out.push(format!("{u}{}", toks[end].trail));
            i += took + 1;
            last_word = Some(end);
            continue;
        }
        // "12.5mm" or "12.5 millimetres": the number, a space, the unit as written
        if rest[0].chars().all(|c| c.is_ascii_digit() || c == '.')
            && let Some(u) = rest.get(1).and_then(|u| unit(u))
            && whole(2)
        {
            let end = from[i + 1];
            if end == w && (i == 0 || from[i - 1] != w) && toks[w].core.len() == rest[0].len() + rest[1].len()
            {
                out.push(format!("{}{} {u}{}", toks[w].lead, rest[0], toks[w].trail));
                i += 2;
                last_word = Some(w);
                continue;
            }
            if end != w && whole(1) && u != toks[end].core {
                out.push(format!("{}{}{}", toks[w].lead, toks[w].core, toks[w].trail));
                out.push(format!("{u}{}", toks[end].trail));
                i += 2;
                last_word = Some(end);
                continue;
            }
        }
        // "y z plane", "z axis": the axes in capitals
        let axes = rest.iter().take_while(|p| matches!(**p, "x" | "y" | "z")).count();
        if (1..=2).contains(&axes)
            && whole(1)
            && (axes == 1 || whole(2))
            && rest
                .get(axes)
                .is_some_and(|p| matches!(*p, "axis" | "plane" | "axes" | "planes"))
        {
            let end = from[i + axes - 1];
            out.push(format!(
                "{}{}{}",
                toks[w].lead,
                rest[..axes].concat().to_uppercase(),
                toks[end].trail
            ));
            i += axes;
            last_word = Some(end);
            continue;
        }
        let t = &toks[w];
        out.push(format!("{}{}{}", t.lead, t.core, t.trail));
        last_word = Some(w);
        i += 1;
    }
    // words with no pieces (a lone dash) are kept where they were
    if said.is_empty() {
        return text.trim().to_string();
    }
    out.join(" ")
}

/// The words an engine is told to expect: the names, and the said form of each, deduplicated.
pub fn hints(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in names {
        let n = n.trim();
        if n.is_empty() || n.len() > 40 || out.iter().any(|o| o == n) {
            continue;
        }
        out.push(n.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        [
            "bracket",
            "fillet3",
            "extrude1",
            "extrude3",
            "hole[2]",
            "hole[3]",
            "DL-108",
            "sketch2",
            "sketch4",
            "PCB1",
            "bracket B2",
            "base plate",
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn names_as_the_document_writes_them() {
        let n = names();
        let t = |s: &str| tidy(s, &n);
        assert_eq!(
            t("move hole three to the left edge"),
            "move hole[3] to the left edge"
        );
        assert_eq!(
            t("move hole 3 to the left edge."),
            "move hole[3] to the left edge."
        );
        assert_eq!(
            t("hide D L one oh eight and show the base plate"),
            "hide DL-108 and show the base plate"
        );
        assert_eq!(
            t("Hide DL 108, and show the base plate."),
            "Hide DL-108, and show the base plate."
        );
        assert_eq!(t("hide D.L. 108"), "hide DL-108");
        assert_eq!(t("hide dl-108"), "hide DL-108");
        assert_eq!(
            t("Suppress fillet three and rebuild."),
            "Suppress fillet3 and rebuild."
        );
        assert_eq!(
            t("rename extrude 3 to base flange"),
            "rename extrude3 to base flange"
        );
        assert_eq!(
            t("hide P C B one and the standoffs"),
            "hide PCB1 and the standoffs"
        );
        assert_eq!(t("show only bracket B two"), "show only bracket B2");
        assert_eq!(
            t("mirror sketch 2 about the y z plane"),
            "mirror sketch2 about the YZ plane"
        );
        // a name the document hasn't stays as said
        assert_eq!(t("move hole seven up"), "move hole seven up");
        assert_eq!(t("make three holes"), "make three holes");
    }

    #[test]
    fn numbers_before_units() {
        let t = |s: &str| tidy(s, &[]);
        assert_eq!(
            t("set extrude one depth to twelve point five millimetres"),
            "set extrude one depth to 12.5 mm"
        );
        assert_eq!(t("rotate it forty five degrees"), "rotate it 45 degrees");
        assert_eq!(
            t("chamfer the top edges zero point five millimetres"),
            "chamfer the top edges 0.5 mm"
        );
        assert_eq!(t("revolve three hundred sixty degrees"), "revolve 360 degrees");
        assert_eq!(
            t("a bolt circle of one hundred and twenty millimetres."),
            "a bolt circle of 120 mm."
        );
        assert_eq!(t("Make it 2 millimeters thick"), "Make it 2 mm thick");
        assert_eq!(t("Set it to 12.5mm."), "Set it to 12.5 mm.");
        assert_eq!(t("turn it 45deg"), "turn it 45 degrees");
        assert_eq!(t("a 2-0 meter rod"), "a 2-0 meter rod");
        assert_eq!(
            t("make hole three an M six counterbore"),
            "make hole three an M6 counterbore"
        );
        assert_eq!(t("add an M4 tapped hole"), "add an M4 tapped hole");
        assert_eq!(t("one of the holes"), "one of the holes");
        assert_eq!(t("around the z axis"), "around the Z axis");
    }

    #[test]
    fn longer_names_first() {
        let n: Vec<String> = ["hole[1]", "hole[12]"].map(String::from).to_vec();
        assert_eq!(tidy("delete hole twelve", &n), "delete hole[12]");
        assert_eq!(tidy("delete hole one", &n), "delete hole[1]");
        assert_eq!(tidy("", &n), "");
    }
}
