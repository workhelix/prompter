//! Shell completion generation module.

use clap::CommandFactory;
use clap_complete::Shell;
use std::io;

use crate::Cli;

/// Generate shell completion scripts.
///
/// Outputs both instructions and the completion script to stdout.
pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    // Print instructions
    println!("# Shell completion for {bin_name}");
    println!("#");
    println!("# To enable completions, add this to your shell config:");
    println!("#");

    match shell {
        Shell::Bash => {
            println!("# For bash (~/.bashrc):");
            println!("#   source <({bin_name} completions bash)");
        }
        Shell::Zsh => {
            println!("# For zsh (~/.zshrc):");
            println!("#   {bin_name} completions zsh > ~/.zsh/completions/_{bin_name}");
            println!("#   # Ensure fpath includes ~/.zsh/completions");
        }
        Shell::Fish => {
            println!("# For fish (~/.config/fish/config.fish):");
            println!("#   {bin_name} completions fish | source");
        }
        _ => {
            println!("# For {shell}:");
            println!("#   {bin_name} completions {shell} > /path/to/completions/_{bin_name}");
        }
    }

    println!();

    // Generate completions
    clap_complete::generate(shell, &mut cmd, bin_name, &mut io::stdout());
}
