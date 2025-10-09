//! Integration tests for the prompter CLI.
//!
//! These tests verify the complete CLI functionality by running the actual
//! binary with various arguments and checking the results.

use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

fn tmp_home(prefix: &str) -> PathBuf {
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

fn read_all(path: &std::path::Path) -> Vec<u8> {
    let mut f = fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

const fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_prompter")
}

#[test]
fn test_init_list_validate_run() {
    let home = tmp_home("prompter_it_home");
    fs::create_dir_all(&home).unwrap();

    // init
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("init")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // list
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let list = String::from_utf8_lossy(&out.stdout);
    assert!(list.contains("python.api"));
    assert!(list.contains("general.testing"));

    // validate
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("validate")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // run profile
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("python.api")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Check output contains expected content (pre-prompt + prefix + file contents + post-prompt)
    let output_str = String::from_utf8_lossy(&out.stdout);
    // Should start with default pre-prompt
    assert!(output_str.starts_with("You are an LLM coding agent."));
    // Should contain system prefix
    assert!(output_str.contains("Today is "));
    assert!(output_str.contains(", and you are running on a "));
    assert!(output_str.contains(" system.\n\n"));
    // Should contain the library file contents
    let lib = home.join(".local/prompter/library");
    let c_bytes = read_all(&lib.join("a/b/c.md"));
    let h_bytes = read_all(&lib.join("f/g/h.md"));
    let c_content = String::from_utf8_lossy(&c_bytes);
    let h_content = String::from_utf8_lossy(&h_bytes);
    assert!(output_str.contains(&*c_content));
    assert!(output_str.contains(&*h_content));
    // Should end with default post-prompt
    assert!(output_str.ends_with(
        "Now, read the @AGENTS.md and @CLAUDE.md files in this directory, if they exist."
    ));
}

#[test]
fn test_missing_file_and_unknown_profile_fail() {
    let home = tmp_home("prompter_it_missing");
    let cfg_path = home.join(".config/prompter");
    let lib_path = home.join(".local/prompter/library");
    fs::create_dir_all(&cfg_path).unwrap();
    fs::create_dir_all(&lib_path).unwrap();

    let cfg = r#"
[root]
depends_on = ["does.not.exist.md", "unknown_profile"]
"#;
    fs::write(cfg_path.join("config.toml"), cfg).unwrap();

    // validate should fail
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("validate")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Missing file:"));
    assert!(err.contains("Unknown profile:"));

    // running profile should also fail
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("root")
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn test_recursive_resolution_and_separator() {
    let home = tmp_home("prompter_it_recursive");
    let cfg_path = home.join(".config/prompter");
    let lib_path = home.join(".local/prompter/library");
    fs::create_dir_all(&cfg_path).unwrap();
    fs::create_dir_all(lib_path.join("a")).unwrap();
    fs::create_dir_all(lib_path.join("f")).unwrap();

    // files
    fs::write(home.join(".local/prompter/library/a/x.md"), b"AX\n").unwrap();
    fs::write(home.join(".local/prompter/library/f/y.md"), b"FY\n").unwrap();

    // config with recursive profile dep
    let cfg = r#"
[child]
depends_on = ["a/x.md"]

[root]
depends_on = ["child", "f/y.md", "a/x.md"]
"#;
    fs::write(cfg_path.join("config.toml"), cfg).unwrap();

    // run with separator that will be unescaped
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .args(["--separator", "\\n--\\n", "root"]) // CLI will unescape to "\n--\n"
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Dedup behavior: only first occurrence of a file is included
    let output_str = String::from_utf8_lossy(&out.stdout);
    // Should start with default pre-prompt
    assert!(output_str.starts_with("You are an LLM coding agent."));
    // Should contain system prefix
    assert!(output_str.contains("Today is "));
    assert!(output_str.contains(", and you are running on a "));
    assert!(output_str.contains(" system.\n\n"));
    // Should contain file content with separator, in expected order
    assert!(output_str.contains("AX\n"));
    assert!(output_str.contains("\n--\n"));
    assert!(output_str.contains("FY\n"));
    // Should end with the default post-prompt
    assert!(output_str.ends_with(
        "Now, read the @AGENTS.md and @CLAUDE.md files in this directory, if they exist."
    ));
}

#[test]
fn test_cycle_detection_in_validate() {
    let home = tmp_home("prompter_it_cycle");
    let cfg_path = home.join(".config/prompter");
    let lib_path = home.join(".local/prompter/library");
    fs::create_dir_all(&cfg_path).unwrap();
    fs::create_dir_all(&lib_path).unwrap();

    let cfg = r#"
[A]
depends_on = ["B"]
[B]
depends_on = ["A"]
"#;
    fs::write(cfg_path.join("config.toml"), cfg).unwrap();

    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("validate")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Cycle detected"), "stderr: {err}");
}

#[test]
fn test_version_flag() {
    let out = Command::new(bin_path()).arg("version").output().unwrap();
    assert!(out.status.success());
    let got = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let expected = format!("prompter {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(got, expected);
}

#[test]
fn test_completions_bash() {
    let out = Command::new(bin_path())
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("prompter"));
    assert!(stdout.contains("_prompter"));
}

#[test]
fn test_completions_zsh() {
    let out = Command::new(bin_path())
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("prompter"));
}

#[test]
fn test_completions_fish() {
    let out = Command::new(bin_path())
        .args(["completions", "fish"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("prompter"));
}

#[test]
fn test_doctor_command() {
    let out = Command::new(bin_path()).arg("doctor").output().unwrap();
    // Doctor may succeed or fail depending on local setup
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("health check"));
}

#[test]
fn test_help_flag() {
    let out = Command::new(bin_path()).arg("--help").output().unwrap();
    // Help output may go to stdout or stderr depending on exit code
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(combined.contains("prompter") || combined.contains("Usage:"));
}

#[test]
fn test_run_with_separator() {
    let home = tmp_home("prompter_it_sep");
    fs::create_dir_all(&home).unwrap();

    // Init first
    Command::new(bin_path())
        .env("HOME", &home)
        .arg("init")
        .output()
        .unwrap();

    // Run with custom separator
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .args(["--separator", "\\n---\\n", "python.api"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\n---\n"));
}

#[test]
fn test_run_with_custom_pre_prompt() {
    let home = tmp_home("prompter_it_pre");
    fs::create_dir_all(&home).unwrap();

    Command::new(bin_path())
        .env("HOME", &home)
        .arg("init")
        .output()
        .unwrap();

    let out = Command::new(bin_path())
        .env("HOME", &home)
        .args(["--pre-prompt", "Custom prefix", "python.api"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("Custom prefix"));
}

#[test]
fn test_run_with_custom_post_prompt() {
    let home = tmp_home("prompter_it_post");
    fs::create_dir_all(&home).unwrap();

    Command::new(bin_path())
        .env("HOME", &home)
        .arg("init")
        .output()
        .unwrap();

    let out = Command::new(bin_path())
        .env("HOME", &home)
        .args(["--post-prompt", "Custom suffix", "python.api"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.ends_with("Custom suffix"));
}
