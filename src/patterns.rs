/// Predefined searches in the spirit of tmux-copycat's defaults. Each
/// pattern is activated from the pattern menu (`p` then the key) or
/// directly with alt+key.
pub struct PatternDef {
    pub key: char,
    pub name: &'static str,
    pub regex: &'static str,
}

pub const PATTERNS: &[PatternDef] = &[
    PatternDef {
        key: 'u',
        name: "url",
        regex: r#"(https?|ftp|git|ssh|file)://[^\s"'<>()\[\]]+"#,
    },
    PatternDef {
        key: 'f',
        // Paths need at least one slash; a :line[:col] suffix from
        // compiler or grep output is included in the match.
        name: "file",
        regex: r"[\w.~-]*(/[\w.#$%&+=@~-]+)+(:\d+(:\d+)?)?",
    },
    PatternDef {
        key: 'g',
        name: "git-sha",
        regex: r"\b[0-9a-f]{7,40}\b",
    },
    PatternDef {
        key: 'i',
        // Loose copycat-level IPv4: octets are not range-checked.
        name: "ip",
        regex: r"\b\d{1,3}(\.\d{1,3}){3}\b",
    },
    PatternDef {
        key: 'd',
        name: "digits",
        regex: r"\b\d+\b",
    },
    PatternDef {
        key: 'q',
        name: "quoted",
        regex: r#""[^"]+"|'[^']+'"#,
    },
];

pub fn by_key(key: char) -> Option<&'static PatternDef> {
    PATTERNS.iter().find(|d| d.key == key)
}

/// One-line menu for the bottom row, e.g. "pattern: [u]rl [f]ile ...".
/// Every pattern name starts with its selection key.
pub fn menu_line() -> String {
    let mut out = "pattern:".to_string();
    for d in PATTERNS {
        out.push_str(&format!(" [{}]{}", d.key, &d.name[1..]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn matches(name: &str, hay: &str) -> Vec<String> {
        let def = PATTERNS.iter().find(|d| d.name == name).unwrap();
        Regex::new(def.regex)
            .unwrap()
            .find_iter(hay)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    #[test]
    fn url_matches_schemes_and_stops_at_delimiters() {
        assert_eq!(
            matches("url", "see https://x.dev/a?b=1 and (git://h/r.git)"),
            vec!["https://x.dev/a?b=1", "git://h/r.git"]
        );
        assert!(matches("url", "no urls here").is_empty());
    }

    #[test]
    fn file_matches_paths_with_line_suffixes() {
        assert_eq!(
            matches(
                "file",
                "error at src/app.rs:12:5, see /var/log/x.log and ~/dl/a.txt"
            ),
            vec!["src/app.rs:12:5", "/var/log/x.log", "~/dl/a.txt"]
        );
        assert!(matches("file", "Cargo.toml alone").is_empty());
    }

    #[test]
    fn git_sha_needs_seven_hex_chars() {
        assert_eq!(matches("git-sha", "commit 0a3773dbe"), vec!["0a3773dbe"]);
        assert!(matches("git-sha", "abc123").is_empty());
    }

    #[test]
    fn ip_matches_dotted_quads() {
        assert_eq!(
            matches("ip", "host 192.168.0.1:8080 up"),
            vec!["192.168.0.1"]
        );
        assert!(matches("ip", "1.2.3").is_empty());
    }

    #[test]
    fn digits_and_quoted() {
        assert_eq!(matches("digits", "took 42 ms"), vec!["42"]);
        assert_eq!(
            matches("quoted", r#"say "hi there" or 'bye'"#),
            vec![r#""hi there""#, "'bye'"]
        );
    }

    #[test]
    fn menu_lists_every_pattern_by_key() {
        let menu = menu_line();
        assert_eq!(
            menu,
            "pattern: [u]rl [f]ile [g]it-sha [i]p [d]igits [q]uoted"
        );
        for d in PATTERNS {
            assert!(by_key(d.key).is_some());
        }
    }
}
