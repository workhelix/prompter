use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Config {
    // profile name -> list of dependencies (files like "a/b/c.md" or profile names)
    profiles: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
enum AppMode {
    Run { profile: String, separator: Option<String> },
    List,
    Validate,
    Init,
    Version,
}

fn print_usage(program: &str) {
    eprintln!("Usage:\n  {program} [--separator <STRING>|-s <STRING>] <profile>\n  {program} --list\n  {program} --validate\n  {program} --init\n  {program} --version\n\nEnvironment:\n  Uses $HOME/.config/prompter/config.toml for profiles\n  Uses $HOME/.local/prompter/library for prompt files\n\nNotes:\n  - depends_on entries ending with .md are treated as files relative to library\n  - other entries are treated as profile references (recursive, cycle-checked)\n  - --separator supports escapes: \\n, \\t, \\\" and \\\\.");
}

fn parse_args_from(mut args: Vec<String>) -> Result<AppMode, String> {
    let program = args.remove(0);

    // Simple manual parsing
    let mut separator: Option<String> = None;
    let mut mode = None;
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
                separator = Some(unescape(&args[i+1]));
                i += 2;
            }
            s if s.starts_with('-') => {
                print_usage(&program);
                return Err(format!("Unknown flag: {}", s));
            }
            // positional profile
            _ => {
                if mode.is_some() {
                    print_usage(&program);
                    return Err("Unexpected positional argument".into());
                }
                let profile = args[i].clone();
                mode = Some(AppMode::Run { profile, separator: separator.clone() });
                i += 1;
                if i < args.len() {
                    print_usage(&program);
                    return Err("Too many positional arguments".into());
                }
            }
        }
    }

    mode.ok_or_else(|| {
        print_usage(&program);
        "No action specified".into()
    })
}

fn unescape(s: &str) -> String {
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
                Some(other) => { out.push('\\'); out.push(other); },
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

fn parse_config_toml(input: &str) -> Result<Config, String> {
    let mut profiles: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;

    // State for multi-line depends_on arrays
    let mut collecting = false;
    let mut buffer = String::new();

    for raw_line in input.lines() {
        let line = strip_comments(raw_line).trim().to_string();
        if line.is_empty() { continue; }

        if collecting {
            buffer.push(' ');
            buffer.push_str(&line);
            if contains_closing_bracket_outside_quotes(&buffer) {
                // finalize
                let items = parse_array_items(&buffer).map_err(|e| format!("Invalid depends_on array for [{}]: {}", current.clone().unwrap_or_default(), e))?;
                let name = current.clone().ok_or_else(|| "depends_on outside of a profile section".to_string())?;
                profiles.insert(name, items);
                collecting = false;
                buffer.clear();
            }
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len()-1].trim().to_string();
            if name.is_empty() { return Err("Empty section name []".into()); }
            current = Some(name);
            continue;
        }

        // key = value lines (we only support depends_on)
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos+1..].trim();
            if key != "depends_on" { continue; }
            if !value.starts_with('[') {
                return Err("depends_on must be an array".into());
            }
            buffer.clear();
            buffer.push_str(value);
            if contains_closing_bracket_outside_quotes(&buffer) {
                let items = parse_array_items(&buffer).map_err(|e| format!("Invalid depends_on array for [{}]: {}", current.clone().unwrap_or_default(), e))?;
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
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            out.push(c);
            in_str = !in_str;
            continue;
        }
        if !in_str && c == '#' {
            break; // comment starts
        }
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
    // Expect s begins with '[' and contains ']'. Extract quoted strings.
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
        if c == ']' && !in_str {
            break;
        }
        if in_str {
            if escaped { buf.push(c); escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' { in_str = false; items.push(buf.clone()); buf.clear(); continue; }
            buf.push(c);
        } else {
            if c == '"' { in_str = true; continue; }
        }
    }

    if in_str { return Err("Unterminated string in array".into()); }

    Ok(items)
}

#[derive(Debug, PartialEq, Eq)]
enum ResolveError {
    UnknownProfile(String),
    Cycle(Vec<String>),
    MissingFile(PathBuf, String), // (path, referenced_by)
    Io(String), // reserved for future direct IO mapping
}

fn resolve_profile(
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
            if !path.exists() {
                return Err(ResolveError::MissingFile(path, name.to_string()));
            }
            if seen_files.insert(path.clone()) {
                out.push(path);
            }
        } else {
            resolve_profile(dep, cfg, lib, seen_files, stack, out)?;
        }
    }
    stack.pop();
    Ok(())
}

fn list_profiles(cfg: &Config) {
    let mut names: Vec<_> = cfg.profiles.keys().cloned().collect();
    names.sort();
    for n in names { println!("{}", n); }
}

fn validate(cfg: &Config, lib: &Path) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    // Unknown references and missing files
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

    // Cycle detection
    for name in cfg.profiles.keys() {
        let mut seen_files = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        if let Err(ResolveError::Cycle(cycle)) = resolve_profile(name, cfg, lib, &mut seen_files, &mut stack, &mut out) {
            let chain = cycle.join(" -> ");
            errors.push(format!("Cycle detected: {}", chain));
        } else {
            // ignore other errors during cycle check; they are reported above
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

fn init_scaffold() -> Result<(), String> {
    // Ensure config dir exists
    let cfg_path = config_path()?;
    let cfg_dir = cfg_path.parent().ok_or_else(|| "Invalid config path".to_string())?;
    fs::create_dir_all(cfg_dir).map_err(|e| format!("Failed to create {}: {}", cfg_dir.display(), e))?;

    // Ensure library dir exists
    let lib = library_dir()?;
    fs::create_dir_all(&lib).map_err(|e| format!("Failed to create {}: {}", lib.display(), e))?;

    // Default config if missing
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

    // Sample library files if missing
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

fn parse_args() -> Result<AppMode, String> { parse_args_from(env::args().collect()) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn mk_tmp(prefix: &str) -> PathBuf {
        let mut p = env::temp_dir();
        let unique = format!("{}_{}_{}", prefix, std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        p.push(unique);
        p
    }

    #[test]
    fn test_unescape() {
        assert_eq!(unescape("a\\nb\\t\\\"\\\\c"), "a\nb\t\"\\c");
        assert_eq!(unescape("noesc"), "noesc");
    }

    #[test]
    fn test_parse_config_simple_and_multiline() {
        let cfg = r#"
[python.api]
# inline depends array
depends_on = ["a/b/c.md", "f/g/h.md"]

[general.testing]
# multi-line depends array
depends_on = [
  "python.api",
  "a/b/d.md", # trailing comment
]
"#;
        let parsed = parse_config_toml(cfg).expect("parse ok");
        assert_eq!(parsed.profiles.get("python.api").unwrap(), &vec!["a/b/c.md".to_string(), "f/g/h.md".to_string()]);
        assert_eq!(parsed.profiles.get("general.testing").unwrap(), &vec!["python.api".to_string(), "a/b/d.md".to_string()]);
    }

    #[test]
    fn test_resolve_profile_simple_and_dedup() {
        let cfg = Config { profiles: HashMap::from([
            ("A".into(), vec!["a/b.md".into()]),
            ("B".into(), vec!["A".into(), "f/g.md".into(), "a/b.md".into()]),
        ])};
        let lib = mk_tmp("prompter_lib");
        fs::create_dir_all(lib.join("a")).unwrap();
        fs::create_dir_all(lib.join("f")).unwrap();
        fs::write(lib.join("a/b.md"), b"ab").unwrap();
        fs::write(lib.join("f/g.md"), b"fg").unwrap();
        let mut seen = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        resolve_profile("B", &cfg, &lib, &mut seen, &mut stack, &mut out).unwrap();
        assert_eq!(out, vec![lib.join("a/b.md"), lib.join("f/g.md")]);
    }

    #[test]
    fn test_resolve_cycle() {
        let cfg = Config { profiles: HashMap::from([
            ("A".into(), vec!["B".into()]),
            ("B".into(), vec!["A".into()]),
        ])};
        let lib = mk_tmp("prompter_lib_cycle");
        fs::create_dir_all(&lib).unwrap();
        let mut seen = HashSet::new();
        let mut stack = Vec::new();
        let mut out = Vec::new();
        let err = resolve_profile("A", &cfg, &lib, &mut seen, &mut stack, &mut out).unwrap_err();
        match err { ResolveError::Cycle(chain) => assert!(chain.windows(2).any(|w| w==["A","B"] || w==["B","A"])), _ => panic!("expected cycle") }
    }

    #[test]
    fn test_validate_reports_missing_and_unknown() {
        let cfg = Config { profiles: HashMap::from([
            ("root".into(), vec!["unknown_profile".into(), "x/y.md".into()]),
        ])};
        let lib = mk_tmp("prompter_lib_validate");
        fs::create_dir_all(&lib).unwrap();
        let err = validate(&cfg, &lib).unwrap_err();
        assert!(err.contains("Unknown profile: unknown_profile"));
        assert!(err.contains("Missing file:"));
    }

    #[test]
    fn test_parse_args_from() {
        // run with separator
        let args = vec!["prompter".into(), "--separator".into(), "\\n--\\n".into(), "profile".into()];
        match parse_args_from(args).unwrap() {
            AppMode::Run { profile, separator } => {
                assert_eq!(profile, "profile");
                assert_eq!(separator, Some("\n--\n".into()));
            }
            _ => panic!("expected run"),
        }
        // list
        let args = vec!["prompter".into(), "--list".into()];
        matches!(parse_args_from(args).unwrap(), AppMode::List);
        // validate
        let args = vec!["prompter".into(), "--validate".into()];
        matches!(parse_args_from(args).unwrap(), AppMode::Validate);
        // init
        let args = vec!["prompter".into(), "--init".into()];
        matches!(parse_args_from(args).unwrap(), AppMode::Init);
        // version
        let args = vec!["prompter".into(), "--version".into()];
        matches!(parse_args_from(args).unwrap(), AppMode::Version);
    }
}

fn main() {
    let mode = match parse_args() {
        Ok(m) => m,
        Err(e) => { eprintln!("{}", e); std::process::exit(2); }
    };

    match mode {
        AppMode::Version => {
            println!("prompter {}", env!("CARGO_PKG_VERSION"));
        }
        AppMode::Init => {
            if let Err(e) = init_scaffold() {
                eprintln!("Init failed: {}", e);
                std::process::exit(1);
            }
        }
        AppMode::List => {
            let cfg_text = match read_config() {
                Ok(s) => s,
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
            };
            let cfg = match parse_config_toml(&cfg_text) {
                Ok(c) => c,
                Err(e) => { eprintln!("Config parse error: {}", e); std::process::exit(1); }
            };
            list_profiles(&cfg);
        }
        AppMode::Validate => {
            let cfg_text = match read_config() {
                Ok(s) => s,
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
            };
            let cfg = match parse_config_toml(&cfg_text) {
                Ok(c) => c,
                Err(e) => { eprintln!("Config parse error: {}", e); std::process::exit(1); }
            };
            let lib = match library_dir() {
                Ok(p) => p,
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
            };
            match validate(&cfg, &lib) {
                Ok(()) => println!("All profiles valid"),
                Err(errs) => { eprintln!("Validation errors:\n{}", errs); std::process::exit(1); }
            }
        }
        AppMode::Run { profile, separator } => {
            let cfg_text = match read_config() {
                Ok(s) => s,
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
            };
            let cfg = match parse_config_toml(&cfg_text) {
                Ok(c) => c,
                Err(e) => { eprintln!("Config parse error: {}", e); std::process::exit(1); }
            };
            let lib = match library_dir() {
                Ok(p) => p,
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
            };

            let mut seen_files = HashSet::new();
            let mut stack = Vec::new();
            let mut files = Vec::new();
            match resolve_profile(&profile, &cfg, &lib, &mut seen_files, &mut stack, &mut files) {
                Ok(()) => {
                    let mut first = true;
                    let sep = separator.unwrap_or_default();
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    for path in files {
                        if !first && !sep.is_empty() {
                            if let Err(e) = handle.write_all(sep.as_bytes()) { eprintln!("Write error: {}", e); std::process::exit(1); }
                        }
                        first = false;
                        match fs::read(&path) {
                            Ok(bytes) => {
                                if let Err(e) = handle.write_all(&bytes) { eprintln!("Write error: {}", e); std::process::exit(1); }
                            }
                            Err(e) => {
                                eprintln!("Failed to read {}: {}", path.display(), e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                Err(ResolveError::UnknownProfile(p)) => {
                    eprintln!("Unknown profile: {}", p);
                    std::process::exit(1);
                }
                Err(ResolveError::Cycle(cycle)) => {
                    eprintln!("Cycle detected: {}", cycle.join(" -> "));
                    std::process::exit(1);
                }
                Err(ResolveError::MissingFile(path, prof)) => {
                    eprintln!("Missing file: {} (referenced by [{}])", path.display(), prof);
                    std::process::exit(1);
                }
                Err(ResolveError::Io(e)) => {
                    eprintln!("I/O error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
