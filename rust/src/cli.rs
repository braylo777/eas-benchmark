//! Tiny `--key value` argument reader shared by the binaries (no clap dependency).

/// Value after `--key`, if present.
pub fn opt(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Parsed value after `--key`, or `default`. Panics with a clear message on a bad value.
pub fn num<T: std::str::FromStr>(args: &[String], key: &str, default: T) -> T {
    match opt(args, key) {
        None => default,
        Some(v) => v
            .parse()
            .unwrap_or_else(|_| panic!("bad value for {key}: {v:?}")),
    }
}

/// True if the bare flag is present.
pub fn flag(args: &[String], key: &str) -> bool {
    args.iter().any(|a| a == key)
}

/// Expand a leading `~/` with $HOME.
pub fn expand_home(p: &str) -> String {
    match (p.strip_prefix("~/"), std::env::var("HOME")) {
        (Some(rest), Ok(h)) => format!("{h}/{rest}"),
        _ => p.to_string(),
    }
}

/// Filesystem-safe slug for a model tag ("qwen2.5:0.5b" -> "qwen2.5_0.5b").
pub fn slug(model: &str) -> String {
    model
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
