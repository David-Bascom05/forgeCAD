//! `help TOPIC`: the part file's grammar a topic at a time, so what every agent reads stays short
//! (AGENTS.md) and the rest costs tokens only when asked for.

const TEXT: &str = include_str!("help.txt");

/// The sections: a `## word word` header's words are the topics it answers.
fn sections() -> impl Iterator<Item = (&'static str, &'static str)> {
    TEXT.split("\n## ").filter_map(|s| {
        let s = s.strip_prefix("## ").unwrap_or(s);
        s.split_once('\n').filter(|_| !s.starts_with('#'))
    })
}

/// The section a topic names; no topic: the list of topics. A feature with no section of its own
/// answers with its syntax.
pub fn help(topic: &str) -> String {
    let topic = topic.trim();
    if topic.is_empty() {
        let names: Vec<&str> = sections().map(|s| s.0.split(' ').next().unwrap_or("")).collect();
        return format!("help TOPIC: {}", names.join(" "));
    }
    // several topics at one go, each section once
    let mut seen: Vec<&str> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for t in topic.split_whitespace() {
        match sections().find(|(names, _)| names.split(' ').any(|n| n == t)) {
            Some((names, body)) => {
                if !seen.contains(&names) {
                    seen.push(names);
                    out.push(body.trim_end().to_string());
                }
            }
            None => match crate::text::kind(t) {
                Some(k) => out.push(k.syntax.to_string()),
                None => out.push(format!("no topic '{t}'; {}", help(""))),
            },
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_feature_and_act_line_has_help() {
        for k in crate::text::KINDS {
            assert!(!help(k.name).starts_with("no topic"), "{}", k.name);
        }
        for t in crate::tools::TOOLS {
            assert!(!help(t.name).starts_with("no topic"), "{}", t.name);
        }
        assert!(help("fillet").contains("setback"));
        assert!(help("").starts_with("help TOPIC: "));
        assert!(help("nope").starts_with("no topic"));
        let two = help("frame loops frame");
        assert!(two.contains("FRAME:") && two.contains("LOOPS") && two.matches("FRAME:").count() == 1);
    }

    #[test]
    fn no_line_word_is_a_feature_kind() {
        for t in crate::tools::TOOLS {
            assert!(
                crate::text::kind(t.name).is_none() || t.name == "param",
                "{}",
                t.name
            );
        }
    }
}
