//! Health check and diagnostics module.

use std::path::Path;

/// Run doctor command to check health and configuration.
///
/// Returns exit code: 0 if healthy, 1 if issues found.
pub fn run_doctor() -> i32 {
    println!("🏥 prompter health check");
    println!("========================");
    println!();

    let mut has_errors = false;
    let mut has_warnings = false;

    // Check configuration
    println!("Configuration:");
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let config_path = Path::new(&home).join(".config/prompter/config.toml");

    if config_path.exists() {
        println!("  ✅ Config file: {}", config_path.display());

        // Try to parse it
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                if toml::from_str::<toml::Value>(&content).is_ok() {
                    println!("  ✅ Config is valid TOML");
                } else {
                    println!("  ❌ Config is invalid TOML");
                    has_errors = true;
                }
            }
            Err(e) => {
                println!("  ❌ Failed to read config: {e}");
                has_errors = true;
            }
        }
    } else {
        println!("  ❌ Config file not found: {}", config_path.display());
        println!("  ℹ️  Run 'prompter init' to create default configuration");
        has_errors = true;
    }

    // Check library directory
    let library_path = Path::new(&home).join(".local/prompter/library");

    if library_path.exists() {
        println!("  ✅ Library directory: {}", library_path.display());
    } else {
        println!(
            "  ❌ Library directory not found: {}",
            library_path.display()
        );
        println!("  ℹ️  Run 'prompter init' to create default library");
        has_errors = true;
    }

    println!();

    // Check for updates
    println!("Updates:");
    match check_for_updates() {
        Ok(Some(latest)) => {
            let current = env!("CARGO_PKG_VERSION");
            println!("  ⚠️  Update available: v{latest} (current: v{current})");
            println!("  💡 Run 'prompter update' to install the latest version");
            has_warnings = true;
        }
        Ok(None) => {
            println!(
                "  ✅ Running latest version (v{})",
                env!("CARGO_PKG_VERSION")
            );
        }
        Err(e) => {
            println!("  ⚠️  Failed to check for updates: {e}");
            has_warnings = true;
        }
    }

    println!();

    // Summary
    if has_errors {
        println!(
            "❌ {} found",
            if has_warnings {
                format!(
                    "{} error{}, {} warning{}",
                    if has_errors { "1" } else { "0" },
                    if has_errors { "" } else { "s" },
                    if has_warnings { "1" } else { "0" },
                    if has_warnings { "" } else { "s" }
                )
            } else {
                "1 error".to_string()
            }
        );
        1
    } else if has_warnings {
        println!("⚠️  1 warning found");
        0 // Warnings don't cause failure
    } else {
        println!("✨ Everything looks healthy!");
        0
    }
}

fn check_for_updates() -> Result<Option<String>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("prompter-doctor")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.github.com/repos/workhelix/prompter/releases/latest";
    let response: serde_json::Value = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let tag_name = response["tag_name"]
        .as_str()
        .ok_or_else(|| "No tag_name in response".to_string())?;

    let latest = tag_name
        .trim_start_matches("prompter-v")
        .trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");

    if latest == current {
        Ok(None)
    } else {
        Ok(Some(latest.to_string()))
    }
}
