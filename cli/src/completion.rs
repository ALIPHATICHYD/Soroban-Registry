//! Shell completion script generation (#971).

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;

/// Supported shells for completion generation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    /// Bash completion script
    Bash,
    /// Zsh completion script
    Zsh,
    /// Fish completion script
    Fish,
    /// Elvish completion script
    Elvish,
    /// PowerShell completion script
    PowerShell,
}

impl From<CompletionShell> for Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::Elvish => Shell::Elvish,
            CompletionShell::PowerShell => Shell::PowerShell,
        }
    }
}

/// Generate a completion script for the given shell.
pub fn generate_script(shell: CompletionShell) {
    let mut cmd = crate::Cli::command();
    generate(
        Shell::from(shell),
        &mut cmd,
        "soroban-registry",
        &mut io::stdout(),
    );
}

pub fn install_hint(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => {
            "Install: soroban-registry completion bash > \"${HOME}/.local/share/bash-completion/completions/soroban-registry\"\n\
             Or: eval \"$(soroban-registry completion bash)\""
        }
        CompletionShell::Zsh => {
            "Install: soroban-registry completion zsh > \"${HOME}/.local/share/zsh/site-functions/_soroban-registry\"\n\
             Then run: compinit"
        }
        CompletionShell::Fish => {
            "Install: soroban-registry completion fish > \"${HOME}/.config/fish/completions/soroban-registry.fish\""
        }
        CompletionShell::Elvish => {
            "Install: soroban-registry completion elvish >> \"${HOME}/.elvish/lib/soroban-registry.elv\""
        }
        CompletionShell::PowerShell => {
            "Install: soroban-registry completion powershell | Out-String | Invoke-Expression"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn generated_bash_contains_root_command() {
        let mut cmd = crate::Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Bash, &mut cmd, "soroban-registry", &mut buf);
        let script = String::from_utf8(buf).expect("utf8");
        assert!(script.contains("soroban-registry"));
        assert!(script.contains("search"));
        assert!(script.contains("compare"));
        assert!(script.contains("completion"));
    }

    #[test]
    fn generated_zsh_contains_contract_subcommands() {
        let mut cmd = crate::Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Zsh, &mut cmd, "soroban-registry", &mut buf);
        let script = String::from_utf8(buf).expect("utf8");
        assert!(script.contains("contract"));
    }
}
