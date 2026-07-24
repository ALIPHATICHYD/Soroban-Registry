use crate::Cli;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context as RustyContext, Editor, Helper};
use shlex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ShellContext {
    pub api_url: String,
    pub contract_id: Option<String>,
    pub network: String,
    pub vars: HashMap<String, String>,
}

impl ShellContext {
    fn new(api_url: String, network: String) -> Self {
        Self {
            api_url,
            contract_id: None,
            network,
            vars: HashMap::new(),
        }
    }

    fn prompt(&self) -> String {
        let contract = self.contract_id.as_deref().unwrap_or("none");
        format!(
            "{} ({}) [{}] > ",
            "soroban-registry".cyan().bold(),
            self.network.bright_blue(),
            contract.bright_magenta()
        )
    }

    fn continuation_prompt(&self) -> String {
        format!("{} ", "...".bright_black())
    }

    fn get_var(&self, name: &str) -> Option<Cow<'_, str>> {
        match name {
            "api_url" => Some(Cow::Borrowed(self.api_url.as_str())),
            "network" => Some(Cow::Borrowed(self.network.as_str())),
            "contract" => Some(Cow::Borrowed(self.contract_id.as_deref().unwrap_or(""))),
            _ => self.vars.get(name).map(|v| Cow::Borrowed(v.as_str())),
        }
    }
}

pub async fn run(api_url: &str, initial_network: Option<String>) -> Result<()> {
    let history_path = repl_history_path()?;

    let mut rl: Editor<ReplHelper, rustyline::history::DefaultHistory> = Editor::new()?;
    let mut context = ShellContext::new(
        api_url.to_string(),
        initial_network.unwrap_or_else(|| "testnet".to_string()),
    );

    context
        .vars
        .insert("api_url".to_string(), context.api_url.clone());
    context
        .vars
        .insert("network".to_string(), context.network.clone());

    rl.set_helper(Some(ReplHelper::new()?));
    if let Some(helper) = rl.helper_mut() {
        helper.update_vars(&context);
    }
    let _ = rl.load_history(&history_path);
    // rl.set_max_history_size(1000);

    println!("\n{}", "Soroban Registry REPL".bold().cyan());
    println!("Start with 'help' or run any CLI command.");
    println!(
        "Context: network={}, contract=none (history: {})",
        context.network,
        history_path.display().to_string().bright_black()
    );
    println!();

    loop {
        let readline = read_multiline(&mut rl, &context);
        match readline {
            Ok(buffer) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);

                let args = match shlex::split(line) {
                    Some(args) => args,
                    None => {
                        println!("{}", "Error: Invalid quoting in command".red());
                        continue;
                    }
                };

                if args.is_empty() {
                    continue;
                }

                let args = if args[0] == "ls" {
                    let mut next = args.clone();
                    next[0] = "list".to_string();
                    next
                } else {
                    args
                };

                if handle_repl_builtin(&args, &mut context, &mut rl)? {
                    continue;
                }

                let args = expand_vars(&args, &context);

                match args[0].as_str() {
                    "exit" | "quit" => break,
                    _ => {
                        // Try to parse as a normal CLI command
                        let mut cmd_args = vec!["soroban-registry".to_string()];

                        // Inject context if not present in args
                        let has_network = args.iter().any(|a| a == "--network");
                        let has_contract = args.iter().any(|a| a == "--contract-id" || a == "--id");

                        if !has_network {
                            cmd_args.push("--network".to_string());
                            cmd_args.push(context.network.clone());
                        }

                        // Use the sub-command and its arguments
                        let subcmd = args[0].clone();

                        // Context injection for specific commands if context exists
                        let final_args = args.clone();
                        if let Some(ref cid) = context.contract_id {
                            if !has_contract {
                                match subcmd.as_str() {
                                    "info" | "export" | "breaking-changes" | "profile"
                                    | "coverage" | "verify" => {
                                        // These usually take --id or positional.
                                        // If it's a known command that needs ID and it's missing, let's try to add it.
                                        // For simplicity, we just pass what the user typed.
                                    }
                                    _ => {}
                                }
                            }
                        }

                        cmd_args.extend(final_args);

                        if let Err(e) = execute_command(cmd_args, &context).await {
                            println!("{} {}", "Error:".red(), e);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    println!("Bye!");
    Ok(())
}

async fn execute_command(args: Vec<String>, _context: &ShellContext) -> Result<()> {
    match Cli::try_parse_from(args) {
        Ok(cli) => {
            // We call dispatch_command directly to avoid recursion
            // but we need to resolve the network first.
            let cfg_network =
                crate::config::resolve_network(cli.network.clone(), cli.profile.clone())?;
            let mut net_str = cfg_network.to_string();
            if net_str == "auto" {
                net_str = "mainnet".to_string();
            }
            let network: crate::commands::Network = net_str.parse().unwrap();

            crate::dispatch_command(cli, network, cfg_network).await
        }
        Err(e) => {
            if e.to_string().contains("Usage:") {
                println!("{}", e);
                Ok(())
            } else {
                Err(e.into())
            }
        }
    }
}

fn show_shell_help() {
    println!("{}", "\nREPL Commands:".bold());
    println!("  help                   Show this help");
    println!("  exit / quit            Exit the REPL");
    println!("  context                Show current REPL context");
    println!("  use <contract_id>      Set active contract context");
    println!("  use                    Clear active contract context");
    println!("  set network <n>        Change active network");
    println!("  let <k> = <v>          Set a session variable");
    println!("  unset <k>              Remove a session variable");
    println!("  vars                   List session variables");
    println!(
        "\nVariables can be referenced as $name or ${{name}}. Built-ins: $network, $contract, $api_url."
    );
    println!("\nAny other input is treated as a normal CLI command (e.g., 'search gravity' or 'info <id>').");
    println!("Multiline: end a line with \\ or leave brackets/quotes open.\n");
}

fn show_context(context: &ShellContext) {
    println!("\n{}", "Current Context:".bold());
    println!("  API URL:  {}", context.api_url.bright_black());
    println!("  Network:  {}", context.network.bright_blue());
    println!(
        "  Contract: {}",
        context
            .contract_id
            .as_deref()
            .unwrap_or("none")
            .bright_magenta()
    );
    println!();
}

fn repl_history_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let dir = home.join(".soroban-registry");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("repl_history.txt"))
}

fn read_multiline(
    rl: &mut Editor<ReplHelper, rustyline::history::DefaultHistory>,
    context: &ShellContext,
) -> std::result::Result<String, ReadlineError> {
    let mut buffer = String::new();
    loop {
        let prompt = if buffer.trim().is_empty() {
            context.prompt()
        } else {
            context.continuation_prompt()
        };

        let line = rl.readline(&prompt)?;
        let trimmed = line.trim_end();

        // Allow empty single-line to keep the UI responsive
        if buffer.trim().is_empty() && trimmed.is_empty() {
            return Ok(String::new());
        }

        if trimmed.ends_with('\\') {
            buffer.push_str(trimmed.trim_end_matches('\\'));
            buffer.push('\n');
            continue;
        }

        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(trimmed);

        if needs_more_input(&buffer) {
            continue;
        }

        return Ok(buffer);
    }
}

fn needs_more_input(input: &str) -> bool {
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut braces = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            '{' => braces += 1,
            '}' => braces -= 1,
            _ => {}
        }
    }

    in_single || in_double || parens > 0 || brackets > 0 || braces > 0
}

fn handle_repl_builtin(
    args: &[String],
    context: &mut ShellContext,
    rl: &mut Editor<ReplHelper, rustyline::history::DefaultHistory>,
) -> Result<bool> {
    match args[0].as_str() {
        "help" => {
            show_shell_help();
            Ok(true)
        }
        "context" => {
            show_context(context);
            Ok(true)
        }
        "use" => {
            if args.len() > 1 {
                context.contract_id = Some(args[1].clone());
                context.vars.insert("contract".to_string(), args[1].clone());
                println!("Context set to contract: {}", args[1].bright_magenta());
            } else {
                context.contract_id = None;
                context.vars.remove("contract");
                println!("Context cleared (no active contract)");
            }
            if let Some(helper) = rl.helper_mut() {
                helper.update_vars(context);
            }
            Ok(true)
        }
        "set" => {
            if args.len() >= 3 && args[1] == "network" {
                context.network = args[2].clone();
                context
                    .vars
                    .insert("network".to_string(), context.network.clone());
                if let Some(helper) = rl.helper_mut() {
                    helper.update_vars(context);
                }
                println!("Network set to: {}", context.network.bright_blue());
                Ok(true)
            } else {
                println!("Usage: set network <mainnet|testnet|futurenet>");
                Ok(true)
            }
        }
        "let" => {
            let joined = args[1..].join(" ");
            let Some((k, v)) = joined.split_once('=') else {
                println!("Usage: let <name> = <value>");
                return Ok(true);
            };
            let key = k.trim();
            if key.is_empty() {
                println!("Usage: let <name> = <value>");
                return Ok(true);
            }
            let value = v.trim().to_string();
            context.vars.insert(key.to_string(), value);
            if let Some(helper) = rl.helper_mut() {
                helper.update_vars(context);
            }
            Ok(true)
        }
        "unset" => {
            if args.len() != 2 {
                println!("Usage: unset <name>");
                return Ok(true);
            }
            context.vars.remove(&args[1]);
            if let Some(helper) = rl.helper_mut() {
                helper.update_vars(context);
            }
            Ok(true)
        }
        "vars" => {
            println!("\n{}", "Session variables:".bold());
            let mut keys: Vec<_> = context.vars.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let value = context.vars.get(&key).cloned().unwrap_or_default();
                println!("  {} = {}", key.cyan(), value);
            }
            println!();
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn expand_vars(args: &[String], context: &ShellContext) -> Vec<String> {
    args.iter()
        .map(|arg| expand_vars_in_token(arg, context))
        .collect()
}

fn expand_vars_in_token(token: &str, context: &ShellContext) -> String {
    if !token.contains('$') {
        return token.to_string();
    }

    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }

        if matches!(chars.peek(), Some('{')) {
            let _ = chars.next(); // {
            let mut name = String::new();
            while let Some(nc) = chars.next() {
                if nc == '}' {
                    break;
                }
                name.push(nc);
            }
            if let Some(val) = context.get_var(name.trim()) {
                out.push_str(&val);
            } else {
                out.push_str("${");
                out.push_str(&name);
                out.push('}');
            }
            continue;
        }

        let mut name = String::new();
        while let Some(&nc) = chars.peek() {
            if nc.is_ascii_alphanumeric() || nc == '_' {
                name.push(nc);
                let _ = chars.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            out.push('$');
            continue;
        }
        if let Some(val) = context.get_var(name.trim()) {
            out.push_str(&val);
        } else {
            out.push('$');
            out.push_str(&name);
        }
    }

    out
}

#[derive(Debug)]
struct ReplHelper {
    commands: Vec<String>,
    vars: Vec<String>,
}

impl ReplHelper {
    fn new() -> Result<Self> {
        let mut commands: Vec<String> = Vec::new();
        let cmd = crate::Cli::command();
        for sub in cmd.get_subcommands() {
            commands.push(sub.get_name().to_string());
        }
        commands.extend(
            [
                "help", "exit", "quit", "context", "use", "set", "let", "unset", "vars", "ls",
            ]
            .into_iter()
            .map(str::to_string),
        );
        commands.sort();
        commands.dedup();

        Ok(Self {
            commands,
            vars: Vec::new(),
        })
    }

    fn update_vars(&mut self, context: &ShellContext) {
        let mut vars: Vec<String> = context.vars.keys().map(|k| format!("${k}")).collect();
        vars.extend([
            "$network".to_string(),
            "$contract".to_string(),
            "$api_url".to_string(),
        ]);
        vars.sort();
        vars.dedup();
        self.vars = vars;
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &RustyContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[start..pos];

        let mut candidates: Vec<String> = Vec::new();

        // Simple heuristic: if completing first token, suggest commands.
        if start == 0 {
            candidates = self
                .commands
                .iter()
                .filter(|c| c.starts_with(word))
                .cloned()
                .collect();
        } else if word.starts_with('$') {
            candidates = self
                .vars
                .iter()
                .filter(|v| v.starts_with(word))
                .cloned()
                .collect();
        } else {
            // Contextual completion: set network <TAB>
            if let Some(tokens) = shlex::split(&line[..pos]) {
                if tokens.len() >= 2 && tokens[0] == "set" && tokens[1] == "network" {
                    candidates = ["mainnet", "testnet", "futurenet"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .filter(|s| s.starts_with(word))
                        .collect();
                }
            }
        }

        let pairs = candidates
            .into_iter()
            .map(|c| Pair {
                display: c.clone(),
                replacement: c,
            })
            .collect();

        Ok((start, pairs))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &RustyContext<'_>) -> Option<Self::Hint> {
        None
    }
}

impl Highlighter for ReplHelper {}
impl Helper for ReplHelper {}

impl Validator for ReplHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();
        if needs_more_input(input) {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_detects_unclosed_brackets() {
        assert!(needs_more_input("{\"a\": [1, 2"));
        assert!(!needs_more_input("{\"a\": [1, 2]}"));
    }

    #[test]
    fn multiline_detects_unclosed_quotes() {
        assert!(needs_more_input("search \"hello"));
        assert!(!needs_more_input("search \"hello\""));
    }

    #[test]
    fn expands_builtin_vars() {
        let mut ctx = ShellContext::new("http://example".to_string(), "testnet".to_string());
        ctx.contract_id = Some("C123".to_string());
        ctx.vars.insert("x".to_string(), "42".to_string());
        assert_eq!(expand_vars_in_token("$network", &ctx), "testnet");
        assert_eq!(expand_vars_in_token("$contract", &ctx), "C123");
        assert_eq!(expand_vars_in_token("$x", &ctx), "42");
        assert_eq!(expand_vars_in_token("info ${contract}", &ctx), "info C123");
    }
}
