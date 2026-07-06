/// Plugin configuration from $HERDR_PLUGIN_CONFIG_DIR/config.toml.
/// Parsed with a minimal flat-TOML extractor in the spirit of
/// herdr::json_str_field: one `key = value` per line, `#` comments,
/// scanning stops at the first `[table]` header. No arrays, no escapes.
pub const DEFAULT_EXTRACT_HEIGHT: u8 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Extract popup height as a percentage of the screen, 1-100.
    pub extract_height: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            extract_height: DEFAULT_EXTRACT_HEIGHT,
        }
    }
}

impl Config {
    pub fn from_toml(text: &str) -> Self {
        let height = toml_int_field(text, "extract_height")
            .unwrap_or(DEFAULT_EXTRACT_HEIGHT as i64)
            .clamp(1, 100) as u8;
        Config {
            extract_height: height,
        }
    }
}

/// Missing dir, file, or key all fall back to defaults.
pub fn load(env: impl Fn(&str) -> Option<String>) -> Config {
    let text = env("HERDR_PLUGIN_CONFIG_DIR")
        .map(|dir| std::path::Path::new(&dir).join("config.toml"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    Config::from_toml(&text)
}

/// Raw trimmed value for a top-level key, inline comment stripped;
/// quoted values keep their quotes for toml_str_field to remove.
fn toml_value<'a>(toml: &'a str, key: &str) -> Option<&'a str> {
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            return None;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let v = v.trim();
        if let Some(rest) = v.strip_prefix('"') {
            let end = rest.find('"')?;
            return Some(&v[..end + 2]);
        }
        return Some(v.split('#').next().unwrap_or("").trim());
    }
    None
}

pub fn toml_int_field(toml: &str, key: &str) -> Option<i64> {
    toml_value(toml, key)?.parse().ok()
}

pub fn toml_str_field(toml: &str, key: &str) -> Option<String> {
    let v = toml_value(toml, key)?;
    let inner = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v);
    Some(inner.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_field_parses_flat_keys_and_comments() {
        let toml = "# heading\nextract_height = 60 # inline\nother = 2\n";
        assert_eq!(toml_int_field(toml, "extract_height"), Some(60));
        assert_eq!(toml_int_field(toml, "other"), Some(2));
        assert_eq!(toml_int_field(toml, "missing"), None);
    }

    #[test]
    fn scan_stops_at_table_header() {
        let toml = "a = 1\n[table]\nb = 2\n";
        assert_eq!(toml_int_field(toml, "a"), Some(1));
        assert_eq!(toml_int_field(toml, "b"), None);
    }

    #[test]
    fn str_field_strips_quotes() {
        let toml = "name = \"hello # not a comment\"\nbare = word\n";
        assert_eq!(
            toml_str_field(toml, "name").as_deref(),
            Some("hello # not a comment")
        );
        assert_eq!(toml_str_field(toml, "bare").as_deref(), Some("word"));
    }

    #[test]
    fn from_toml_clamps_and_defaults() {
        assert_eq!(Config::from_toml("").extract_height, 40);
        assert_eq!(Config::from_toml("extract_height = 60").extract_height, 60);
        assert_eq!(Config::from_toml("extract_height = 0").extract_height, 1);
        assert_eq!(
            Config::from_toml("extract_height = 500").extract_height,
            100
        );
    }

    #[test]
    fn load_without_config_dir_defaults() {
        assert_eq!(load(|_| None), Config::default());
    }
}
