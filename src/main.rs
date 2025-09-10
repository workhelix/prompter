use std::env;

use prompter::{init_scaffold, parse_args_from, run_list_stdout, run_render_stdout, run_validate_stdout, AppMode};

fn print_usage(program: &str) {
    eprintln!(
        "Usage:\n  {program} [--separator <STRING>|-s <STRING>] <profile>\n  {program} --list\n  {program} --validate\n  {program} --init\n  {program} --version\n\nEnvironment:\n  Uses $HOME/.config/prompter/config.toml for profiles\n  Uses $HOME/.local/prompter/library for prompt files\n\nNotes:\n  - depends_on entries ending with .md are treated as files relative to library\n  - other entries are treated as profile references (recursive, cycle-checked)\n  - --separator supports escapes: \\n, \\t, \\\" and \\\\."
    );
}

fn parse_args() -> Result<AppMode, String> {
    let args: Vec<String> = env::args().collect();
    match parse_args_from(args) {
        Ok(m) => Ok(m),
        Err(e) => {
            if let Some(prog) = env::args().next() {
                print_usage(&prog);
            }
            Err(e)
        }
    }
}

fn main() {
    let mode = match parse_args() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(2);
        }
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
            if let Err(e) = run_list_stdout() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        AppMode::Validate => match run_validate_stdout() {
            Ok(()) => println!("All profiles valid"),
            Err(errs) => {
                eprintln!("Validation errors:\n{}", errs);
                std::process::exit(1);
            }
        },
        AppMode::Run { profile, separator } => {
            if let Err(e) = run_render_stdout(&profile, separator.as_deref()) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
