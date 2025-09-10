use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub(crate) profiles: HashMap<String, Vec<String>>, // profile -> deps
}

#[derive(Debug)]
pub enum AppMode {
    Run { profile: String, separator: Option<String> },
    List,
    Validate,
    Init,
    Version,
}

pub fn parse_args_from(mut args: Vec<String>) -> Result<AppMode, String> {
    // args[0] is program
    if args.is_empty() {
        return Err("No args".into());
    }
    args.remove(0);

    let mut separator: Option<String> = None;
    let mut mode: Option<AppMode> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => {
                if mode.is_some() { return Err("Cannot combine --list with other modes".into()); }
                mode = Some(AppMode::List);
                i += 1;
            }
            "--validate" => {
                if mode.is_some() { return Err("Cannot combine --validate with other modes".into()); }
                mode = Some(AppMode::Validate);
                i += 1;
            }
            "--init" => {
                if mode.is_some() { return Err("Cannot combine --init with other modes".into()); }
                mode = Some(AppMode::Init);
                i += 1;
            }
            "--version" => {
                if mode.is_some() { return Err("Cannot combine --version with other modes".into()); }
                mode = Some(AppMode::Version);
                i += 1;
            }
            "--separator" | "-s" => {
                if i + 1 >= args.len() { return Err("--separator requires a value".into()); }
                separator = Some(unescape(&args[i + 1]));
                i += 2;
            }
            s if s.starts_with('-') => {
                return Err(format!("Unknown flag: {}", s));
            }
            _ => {
                if mode.is_some() { return Err("Unexpected positional argument".into()); }
                let profile = args[i].clone();
                mode = Some(AppMode::Run { profile, separator: separator.clone() });
                i += 1;
                if i < args.len() { return Err("Too many positional arguments".into()); }
            }
        }
    }

    mode.ok_or_else(|| "No action specified".into())
}

#[allow(clippy::while_let_on_iterator)]
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn home_dir() -> Result<PathBuf, String> {
    env::var("HOME").map(PathBuf::from).map_err(|_| "$HOME not set".into())
}

fn config_path() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".config/prompter/config.toml"))
}

fn library_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".local/prompter/library"))
}

fn read_config() -> Result<String, String> {
    let path = config_path()?;
    fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))
}

pub fn parse_config_toml(input: &str) -> Result<Config, String> {
    let mut profiles: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;

    let mut collecting = false;
    let mut buffer = String::new();

    for raw_line in input.lines() {
        let line = strip_comments(raw_line).trim().to_string();
        if line.is_empty() { continue; }

        if collecting {
            buffer.push(' ');
            buffer.push_str(&line);
            if contains_closing_bracket_outside_quotes(&buffer) {
                let items = parse_array_items(&buffer).map_err(|e| format!(
                    "Invalid depends_on array for [{}]: {}",
                    current.clone().unwrap_or_default(), e))?;
                let name = current.clone().ok_or_else(|| "depends_on outside of a profile section".to_string())?;
                profiles.insert(name, items);
                collecting = false;
                buffer.clear();
            }
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim().to_string();
            if name.is_empty() { return Err("Empty section name []".into()); }
            current = Some(name);
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim();
            if key != "depends_on" { continue; }
            if !value.starts_with('[') { return Err("depends_on must be an array".into()); }
            buffer.clear();
            buffer.push_str(value);
            if contains_closing_bracket_outside_quotes(&buffer) {
                let items = parse_array_items(&buffer).map_err(|e| format!(
                    "Invalid depends_on array for [{}]: {}",
                    current.clone().unwrap_or_default(), e))?;
                let name = current.clone().ok_or_else(|| "depends_on outside of a profile section".to_string())?;
                profiles.insert(name, items);
                buffer.clear();
            } else {
                collecting = true;
            }
        }
    }

    Ok(Config { profiles })
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_str = false;
    for c in s.chars() {
        if c == '"' {
            out.push(c);
            in_str = !in_str;
            continue;
        }
        if !in_str && c == '#' { break; }
        out.push(c);
    }
    out
}

fn contains_closing_bracket_outside_quotes(s: &str) -> bool {
    let mut in_str = false;
    for c in s.chars() {
        if c == '"' { in_str = !in_str; }
        if !in_str && c == ']' { return true; }
    }
    false
}

fn parse_array_items(s: &str) -> Result<Vec<String>, String> {
    let mut items = Vec::new();
    let mut in_str = false;
    let mut buf = String::new();
    let mut escaped = false;
    let mut started = false;

    for c in s.chars() {
        if !started {
            if c == '[' { started = true; }
            continue;
        }
        if c == ']' && !in_str { break; }
        if in_str {
            if escaped { buf.push(c); escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' {
                in_str = false;
                items.push(buf.clone());
                buf.clear();
                continue;
            }
            buf.push(c);
        } else if c == '"' { in_str = true; continue; }
    }

    if in_str { return Err("Unterminated string in array".into()); }
    Ok(items)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    UnknownProfile(String),
    Cycle(Vec<String>),
    MissingFile(PathBuf, String), // (path, referenced_by)
}

pub fn resolve_profile(
    name: &str,
    cfg: &Config,
    lib: &Path,
    seen_files: &mut HashSet<PathBuf>,
    stack: &mut Vec<String>,
    out: &mut Vec<PathBuf>,
) -> Result<(), ResolveError> {
    if stack.contains(&name.to_string()) {
        let mut cycle = stack.clone();
        cycle.push(name.to_string());
        return Err(ResolveError::Cycle(cycle));
    }
    let deps = cfg.profiles.get(name).ok_or_else(|| ResolveError::UnknownProfile(name.to_string()))?;
    stack.push(name.to_string());
    for dep in deps {
        if dep.ends_with(".md") {
            let path = lib.join(dep);
            if !path.exists() { return Err(ResolveError::MissingFile(path, name.to_string())); }
            if seen_files.insert(path.clone()) { out.push(path); }
        } else {
            resolve_profile(dep, cfg, lib, seen_files, stack, out)?;
        }
    }
    stack.pop();
    Ok(())
}

pub fn list_profiles(cfg: &Config, mut w: impl Write) -> io::Result<()> {
    let mut names: Vec<_> = cfg.profiles.keys().cloned().collect();
    names.sort();
    for n in names { writeln!(&mut w, "{}", n)?; }
    Ok(())
}

pub fn validate(cfg: &Config, lib: &Path) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    for (profile, deps) in &cfg.profiles {
        for dep in deps {
            if dep.ends_with(".md") {
                let path = lib.join(dep);
                if !path.exists() {
                    errors.push(format!("Missing file: {} (referenced by [{}])", path.display(), profile));
                }
            } else if !cfg.profiles.contains_key(dep) {
                errors.push(format!("Unknown profile: {} (referenced by [{}])", dep, profile));
            }
        }
    }

    for name in cfg.profiles.keys() {
        let mut seen_files = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        if let Err(ResolveError::Cycle(cycle)) = resolve_profile(name, cfg, lib, &mut seen_files, &mut stack, &mut out) {
            let chain = cycle.join(" -> ");
            errors.push(format!("Cycle detected: {}", chain));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

pub fn init_scaffold() -> Result<(), String> {
    let cfg_path = config_path()?;
    let cfg_dir = cfg_path.parent().ok_or_else(|| "Invalid config path".to_string())?;
    fs::create_dir_all(cfg_dir).map_err(|e| format!("Failed to create {}: {}", cfg_dir.display(), e))?;

    let lib = library_dir()?;
    fs::create_dir_all(&lib).map_err(|e| format!("Failed to create {}: {}", lib.display(), e))?;

    if !cfg_path.exists() {
        let default_cfg = r#"# Prompter configuration
# Profiles map to sets of markdown files and/or other profiles.
# Files are relative to $HOME/.local/prompter/library

[python.api]
depends_on = ["a/b/c.md", "f/g/h.md"]

[general.testing]
depends_on = ["python.api", "a/b/d.md"]
"#;
        fs::write(&cfg_path, default_cfg).map_err(|e| format!("Failed to write {}: {}", cfg_path.display(), e))?;
    }

    let paths_and_contents: Vec<(PathBuf, &str)> = vec![
        (lib.join("a/b/c.md"), "# a/b/c.md\nExample snippet for python.api.\n"),
        (lib.join("a/b.md"), "# a/b.md\nFolder-level notes.\n"),
        (lib.join("a/b/d.md"), "# a/b/d.md\nGeneral testing snippet.\n"),
        (lib.join("f/g/h.md"), "# f/g/h.md\nShared helper snippet.\n"),
    ];

    for (path, contents) in paths_and_contents {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?; }
        if !path.exists() {
            fs::write(&path, contents).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        }
    }

    println!("Initialized config at {}", cfg_path.display());
    println!("Library root at {}", lib.display());
    Ok(())
}

pub fn run_list_stdout() -> Result<(), String> {
    let cfg_text = read_config()?;
    let cfg = parse_config_toml(&cfg_text)?;
    list_profiles(&cfg, io::stdout()).map_err(|e| e.to_string())
}

pub fn run_validate_stdout() -> Result<(), String> {
    let cfg_text = read_config()?;
    let cfg = parse_config_toml(&cfg_text)?;
    let lib = library_dir()?;
    validate(&cfg, &lib)
}

pub fn render_to_writer(
    cfg: &Config,
    lib: &Path,
    mut w: impl Write,
    profile: &str,
    separator: Option<&str>,
) -> Result<(), String> {
    let mut seen_files = HashSet::new();
    let mut stack = Vec::new();
    let mut files = Vec::new();
    resolve_profile(profile, cfg, lib, &mut seen_files, &mut stack, &mut files)
        .map_err(|e| match e {
            ResolveError::UnknownProfile(p) => format!("Unknown profile: {}", p),
            ResolveError::Cycle(c) => format!("Cycle detected: {}", c.join(" -> ")),
            ResolveError::MissingFile(path, prof) => format!("Missing file: {} (referenced by [{}])", path.display(), prof),
        })?;

    let mut first = true;
    let sep = separator.unwrap_or("");
    for path in files {
        if !first && !sep.is_empty() && let Err(e) = w.write_all(sep.as_bytes()) {
            return Err(format!("Write error: {}", e));
        }
        first = false;
        match fs::read(&path) {
            Ok(bytes) => w.write_all(&bytes).map_err(|e| format!("Write error: {}", e))?,
            Err(e) => return Err(format!("Failed to read {}: {}", path.display(), e)),
        }
    }
    Ok(())
}

pub fn run_render_stdout(profile: &str, separator: Option<&str>) -> Result<(), String> {
    let cfg_text = read_config()?;
    let cfg = parse_config_toml(&cfg_text)?;
    let lib = library_dir()?;
    let stdout = io::stdout();
    let handle = stdout.lock();
    render_to_writer(&cfg, &lib, handle, profile, separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_tmp(prefix: &str) -> PathBuf {
        let mut p = env::temp_dir();
        let unique = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        p.push(unique);
        p
    }

    #[test]
    fn test_unescape() {
        assert_eq!(unescape("a\\nb\\t\\\"\\\\c"), "a\nb\t\"\\c");
        assert_eq!(unescape("line1\\rline2"), "line1\rline2");
        assert_eq!(unescape("noesc"), "noesc");
    }

    #[test]
    fn test_strip_comments_and_brackets_detection() {
        let s = r#"ab#cd"#;
        assert_eq!(strip_comments(s), "ab");
        let s = r#""ab#cd" # trailing"#;
        assert_eq!(strip_comments(s), "\"ab#cd\" ");
        assert!(contains_closing_bracket_outside_quotes("[\"not]here\"]]"));
        assert!(!contains_closing_bracket_outside_quotes("[\"no]close\""));
    }

    #[test]
    fn test_parse_array_items_escape_and_error() {
        let s = r#"["a\"b", "c"]"#;
        let items = parse_array_items(s).unwrap();
        assert_eq!(items, vec!["a\"b", "c"]);
        let err = parse_array_items("[\"unterminated").unwrap_err();
        assert!(err.contains("Unterminated"));
    }

    #[test]
    fn test_parse_config_errors() {
        let err = parse_config_toml("[]\n").unwrap_err();
        assert!(err.contains("Empty section name"));
        let err = parse_config_toml("[p]\ndepends_on = \"x\"\n").unwrap_err();
        assert!(err.contains("must be an array"));
        let err = parse_config_toml("depends_on = [\"a.md\"]\n").unwrap_err();
        assert!(err.contains("outside of a profile section"));
    }

    #[test]
    fn test_validate_success_and_unknowns() {
        let cfg = Config { profiles: HashMap::from([
            ("p1".into(), vec!["a.md".into()]),
            ("p2".into(), vec!["p1".into(), "b.md".into()]),
        ])};
        let lib = mk_tmp("prompter_validate_ok");
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("a.md"), b"A").unwrap();
        fs::write(lib.join("b.md"), b"B").unwrap();
        assert!(validate(&cfg, &lib).is_ok());
        let cfg2 = Config { profiles: HashMap::from([
            ("root".into(), vec!["nope".into()]),
        ])};
        let err = validate(&cfg2, &lib).unwrap_err();
        assert!(err.contains("Unknown profile"));
    }

    #[test]
    fn test_resolve_errors_and_dedup() {
        let cfg = Config { profiles: HashMap::from([
            ("root".into(), vec!["missing.md".into()]),
        ])};
        let lib = mk_tmp("prompter_resolve_errs");
        fs::create_dir_all(&lib).unwrap();
        let mut seen = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        let err = resolve_profile("root", &cfg, &lib, &mut seen, &mut stack, &mut out).unwrap_err();
        match err { ResolveError::MissingFile(_, p) => assert_eq!(p, "root"), _ => panic!("expected missing file") }

        let cfg2 = Config { profiles: HashMap::from([
            ("A".into(), vec!["a/b.md".into()]),
            ("B".into(), vec!["A".into(), "a/b.md".into()]),
        ])};
        fs::create_dir_all(lib.join("a")).unwrap();
        fs::write(lib.join("a/b.md"), b"X").unwrap();
        let mut seen = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        resolve_profile("B", &cfg2, &lib, &mut seen, &mut stack, &mut out).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn test_parse_args_errors() {
        // unknown flag
        let args = vec!["prompter".into(), "--bogus".into()];
        let err = parse_args_from(args).unwrap_err();
        assert!(err.contains("Unknown flag"));
        // missing separator value
        let args = vec!["prompter".into(), "--separator".into()];
        let err = parse_args_from(args).unwrap_err();
        assert!(err.contains("requires a value"));
        // unexpected positional after mode
        let args = vec!["prompter".into(), "--list".into(), "foo".into()];
        let err = parse_args_from(args).unwrap_err();
        assert!(err.contains("Unexpected positional"));
        // too many positionals
        let args = vec!["prompter".into(), "foo".into(), "bar".into()];
        let err = parse_args_from(args).unwrap_err();
        assert!(err.contains("Too many positional"));
        // no action specified
        let args = vec!["prompter".into()];
        let err = parse_args_from(args).unwrap_err();
        assert!(err.contains("No action"));
    }

    #[test]
    fn test_list_profiles_order() {
        let cfg = Config { profiles: HashMap::from([
            ("b".into(), vec![]),
            ("a".into(), vec![]),
        ])};
        let mut out = Vec::new();
        super::list_profiles(&cfg, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "a\nb\n");
    }

    #[test]
    fn test_validate_cycle_detected() {
        let cfg = Config { profiles: HashMap::from([
            ("A".into(), vec!["B".into()]),
            ("B".into(), vec!["A".into()]),
        ])};
        let lib = mk_tmp("prompter_cycle");
        fs::create_dir_all(&lib).unwrap();
        let err = validate(&cfg, &lib).unwrap_err();
        assert!(err.contains("Cycle detected"));
    }

    #[test]
    fn test_parse_config_multiline_long() {
        let cfg = r#"
[profile.x]
depends_on = [
  "a/b.md",
  "c/d.md",
  "e/f.md",
]
"#;
        let parsed = parse_config_toml(cfg).unwrap();
        assert_eq!(parsed.profiles.get("profile.x").unwrap().len(), 3);
    }

    #[test]
    fn test_render_to_writer_basic() {
        // library and files
        let lib = mk_tmp("prompter_render_to_writer");
        fs::create_dir_all(lib.join("a")).unwrap();
        fs::create_dir_all(lib.join("f")).unwrap();
        fs::write(lib.join("a/x.md"), b"AX\n").unwrap();
        fs::write(lib.join("f/y.md"), b"FY\n").unwrap();
        // config with nested profile and duplicate file reference
        let cfg = Config { profiles: HashMap::from([
            ("child".into(), vec!["a/x.md".into()]),
            ("root".into(), vec!["child".into(), "f/y.md".into(), "a/x.md".into()]),
        ])};
        let mut out = Vec::new();
        super::render_to_writer(&cfg, &lib, &mut out, "root", Some("\n--\n")).unwrap();
        assert_eq!(out, b"AX\n\n--\nFY\n");
    }

    #[test]
    fn test_array_items_escaped_backslash() {
        let s = r#"["a\\"]"#; // a single backslash in content
        let items = parse_array_items(s).unwrap();
        assert_eq!(items, vec!["a\\"]);
    }

    #[test]
    fn test_parse_args_from() {
        let args = vec!["prompter".into(), "--separator".into(), "\\n--\\n".into(), "profile".into()];
        match parse_args_from(args).unwrap() {
            AppMode::Run { profile, separator } => {
                assert_eq!(profile, "profile");
                assert_eq!(separator, Some("\n--\n".into()));
            }
            _ => panic!("expected run"),
        }
        let args = vec!["prompter".into(), "--list".into()];
        assert!(matches!(parse_args_from(args).unwrap(), AppMode::List));
        let args = vec!["prompter".into(), "--validate".into()];
        assert!(matches!(parse_args_from(args).unwrap(), AppMode::Validate));
        let args = vec!["prompter".into(), "--init".into()];
        assert!(matches!(parse_args_from(args).unwrap(), AppMode::Init));
        let args = vec!["prompter".into(), "--version".into()];
        assert!(matches!(parse_args_from(args).unwrap(), AppMode::Version));
    }

    struct FailAfterN {
        writes_done: usize,
        fail_on: usize,
    }

    impl Write for FailAfterN {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes_done += 1;
            if self.writes_done == self.fail_on {
                Err(io::Error::new(io::ErrorKind::Other, "synthetic write failure"))
            } else {
                Ok(buf.len())
            }
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }

    #[test]
    fn test_render_to_writer_write_error_on_separator() {
        let lib = mk_tmp("prompter_write_err_sep");
        fs::create_dir_all(lib.join("a")).unwrap();
        fs::write(lib.join("a/x.md"), b"AX").unwrap();
        fs::write(lib.join("a/y.md"), b"AY").unwrap();
        let cfg = Config { profiles: HashMap::from([
            ("p".into(), vec!["a/x.md".into(), "a/y.md".into()]),
        ])};
        let mut w = FailAfterN { writes_done: 0, fail_on: 2 }; // first file ok, fail on separator
        let err = super::render_to_writer(&cfg, &lib, &mut w, "p", Some("--")).unwrap_err();
        assert!(err.contains("Write error"), "err={}", err);
    }

    #[test]
    fn test_render_to_writer_write_error_on_file() {
        let lib = mk_tmp("prompter_write_err_file");
        fs::create_dir_all(lib.join("a")).unwrap();
        fs::write(lib.join("a/x.md"), b"AX").unwrap();
        let cfg = Config { profiles: HashMap::from([
            ("p".into(), vec!["a/x.md".into()]),
        ])};
        let mut w = FailAfterN { writes_done: 0, fail_on: 1 }; // fail on first write (file content)
        let err = super::render_to_writer(&cfg, &lib, &mut w, "p", Some("--")).unwrap_err();
        assert!(err.contains("Write error"), "err={}", err);
    }

    #[test]
    fn test_run_list_and_validate_with_home_injection() {
        let home = mk_tmp("prompter_home_unit_ok");
        let cfg_dir = home.join(".config/prompter");
        let lib_dir = home.join(".local/prompter/library");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::create_dir_all(lib_dir.join("a")).unwrap();
        fs::create_dir_all(lib_dir.join("f")).unwrap();
        fs::write(lib_dir.join("a/x.md"), b"AX\n").unwrap();
        fs::write(lib_dir.join("f/y.md"), b"FY\n").unwrap();
        let cfg = r#"
[child]
depends_on = ["a/x.md"]

[root]
depends_on = ["child", "f/y.md"]
"#;
        fs::write(cfg_dir.join("config.toml"), cfg).unwrap();
        let prev_home = env::var("HOME").ok();
        unsafe { env::set_var("HOME", &home); }
        assert!(super::run_validate_stdout().is_ok());
        assert!(super::run_list_stdout().is_ok());
        if let Some(prev) = prev_home { unsafe { env::set_var("HOME", prev); } } else { unsafe { env::remove_var("HOME"); } }
    }

    #[test]
    fn test_run_validate_with_home_injection_failure() {
        let home = mk_tmp("prompter_home_unit_bad");
        let cfg_dir = home.join(".config/prompter");
        let lib_dir = home.join(".local/prompter/library");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::create_dir_all(&lib_dir).unwrap();
        let cfg = r#"
[root]
depends_on = ["missing.md", "unknown_profile"]
"#;
        fs::write(cfg_dir.join("config.toml"), cfg).unwrap();
        let prev_home = env::var("HOME").ok();
        unsafe { env::set_var("HOME", &home); }
        let err = super::run_validate_stdout().unwrap_err();
        assert!(err.contains("Missing file") && err.contains("Unknown profile"), "err={}", err);
        if let Some(prev) = prev_home { unsafe { env::set_var("HOME", prev); } } else { unsafe { env::remove_var("HOME"); } }
    }
}
