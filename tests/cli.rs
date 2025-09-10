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

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_prompter")
}

#[test]
fn test_init_list_validate_run() {
    let home = tmp_home("prompter_it_home");
    fs::create_dir_all(&home).unwrap();

    // --init
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("--init")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // --list
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("--list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let list = String::from_utf8_lossy(&out.stdout);
    assert!(list.contains("python.api"));
    assert!(list.contains("general.testing"));

    // --validate
    let out = Command::new(bin_path())
        .env("HOME", &home)
        .arg("--validate")
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

    // expected = a/b/c.md + f/g/h.md contents
    let lib = home.join(".local/prompter/library");
    let mut expected = Vec::new();
    expected.extend(read_all(&lib.join("a/b/c.md")));
    expected.extend(read_all(&lib.join("f/g/h.md")));

    assert_eq!(out.stdout, expected);
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
        .arg("--validate")
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
    let expected = b"AX\n\n--\nFY\n".to_vec();
    assert_eq!(out.stdout, expected);
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
        .arg("--validate")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Cycle detected"), "stderr: {}", err);
}

#[test]
fn test_version_flag() {
    let out = Command::new(bin_path()).arg("--version").output().unwrap();
    assert!(out.status.success());
    let got = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let expected = format!("prompter {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(got, expected);
}
