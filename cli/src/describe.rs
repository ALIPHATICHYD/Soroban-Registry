//! describe.rs — Machine-readable CLI command schema generation & artifact verification (#1145)

use anyhow::{bail, Context, Result};
use clap::{ArgAction, Command, CommandFactory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Schema format version for automation consumers.
pub const SCHEMA_VERSION: &str = "1.0";

/// Machine-readable command schema representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandSchema {
    /// Schema format version (e.g. "1.0")
    pub version: String,
    /// Full command invocation path (e.g. "contract verify-snapshot")
    pub command: String,
    /// Command description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the command is marked deprecated
    pub deprecated: bool,
    /// Map of argument/flag names to argument metadata
    pub arguments: BTreeMap<String, ArgumentSchema>,
    /// List of child subcommand names
    pub subcommands: Vec<String>,
    /// Output format specifications
    pub output: OutputSchema,
    /// Stable exit codes documentation
    pub exit_codes: BTreeMap<String, String>,
    /// Usage examples
    pub examples: Vec<String>,
}

/// Metadata for a single CLI argument or flag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArgumentSchema {
    /// Argument type ("path", "string", "number", "boolean", "enum")
    #[serde(rename = "type")]
    pub arg_type: String,
    /// Whether the argument is required
    pub required: bool,
    /// Whether the argument accepts multiple values or repeat flags
    pub repeatable: bool,
    /// Whether the argument contains sensitive/secret values
    pub secret: bool,
    /// Whether the option is marked deprecated
    pub deprecated: bool,
    /// Enum possible values (when type is "enum")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// Default value (if available and non-secret)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// Help description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Specifications of output formats supported by the command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputSchema {
    /// Supported output format strings (e.g. ["table", "json"])
    pub formats: Vec<String>,
}

/// Generate command schema for a target command path from clap Command specification.
pub fn generate_schema_for_path(root_cmd: &Command, path: &[&str]) -> Result<CommandSchema> {
    let mut current = root_cmd;
    let mut resolved_names = Vec::new();

    for &segment in path {
        if segment.is_empty() {
            continue;
        }
        if let Some(sub) = current.find_subcommand(segment) {
            current = sub;
            resolved_names.push(sub.get_name());
        } else {
            bail!(
                "Unknown subcommand '{}' under '{}'",
                segment,
                if resolved_names.is_empty() {
                    root_cmd.get_name().to_string()
                } else {
                    resolved_names.join(" ")
                }
            );
        }
    }

    let full_command_name = if resolved_names.is_empty() {
        root_cmd.get_name().to_string()
    } else {
        format!("{} {}", root_cmd.get_name(), resolved_names.join(" "))
    };

    Ok(build_schema(current, &full_command_name))
}

/// Build schema object from a single clap Command reference.
fn build_schema(cmd: &Command, full_name: &str) -> CommandSchema {
    let description = cmd
        .get_about()
        .or_else(|| cmd.get_long_about())
        .map(|s| s.to_string());

    let deprecated = cmd
        .get_about()
        .map_or(false, |a| a.to_string().to_lowercase().contains("deprecated"));

    let mut arguments = BTreeMap::new();
    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if id == "help" || id == "version" {
            continue;
        }

        let arg_name = arg
            .get_long()
            .map(|l| l.to_string())
            .unwrap_or_else(|| id.to_string());

        let is_secret = is_secret_argument(id, &arg_name);
        let is_path = is_path_argument(id, &arg_name, arg.get_value_names());

        // Detect boolean flags FIRST: clap generates "true"/"false" as
        // possible values for ArgAction::SetTrue/SetFalse, which would
        // otherwise be misclassified as enum arguments.
        let is_boolean = matches!(
            arg.get_action(),
            ArgAction::SetTrue | ArgAction::SetFalse
        );

        let possible_vals: Vec<String> = if is_boolean {
            Vec::new() // skip clap's auto-generated true/false values
        } else {
            arg
                .get_possible_values()
                .into_iter()
                .filter_map(|pv| {
                    if pv.is_hide_set() {
                        None
                    } else {
                        Some(pv.get_name().to_string())
                    }
                })
                .collect()
        };

        let arg_type = if is_boolean {
            "boolean".to_string()
        } else if !possible_vals.is_empty() {
            "enum".to_string()
        } else if matches!(arg.get_action(), ArgAction::Count) || is_numeric_argument(id, &arg_name)
        {
            "number".to_string()
        } else if is_path {
            "path".to_string()
        } else {
            "string".to_string()
        };

        let required = arg.is_required_set();
        let action = arg.get_action();
        let repeatable = matches!(action, ArgAction::Append | ArgAction::Count)
            || arg
                .get_num_args()
                .map(|r| r.max_values() > 1)
                .unwrap_or(false);

        let enum_values = if possible_vals.is_empty() {
            None
        } else {
            Some(possible_vals)
        };

        let default_value = if is_secret {
            None
        } else {
            arg.get_default_values()
                .first()
                .map(|val| val.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
        };

        let arg_desc = arg.get_help().map(|h| h.to_string());
        let arg_deprecated = arg_desc.as_deref().map_or(false, |h| h.to_lowercase().contains("deprecated"));

        arguments.insert(
            arg_name,
            ArgumentSchema {
                arg_type,
                required,
                repeatable,
                secret: is_secret,
                deprecated: arg_deprecated,
                enum_values,
                default_value,
                description: arg_desc,
            },
        );
    }

    let subcommands: Vec<String> = cmd
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set() && sub.get_name() != "help")
        .map(|sub| sub.get_name().to_string())
        .collect();

    let formats = infer_output_formats(&arguments);

    let mut exit_codes = BTreeMap::new();
    exit_codes.insert("0".to_string(), "success / valid command completion".to_string());
    exit_codes.insert("1".to_string(), "command error / invalid status".to_string());
    exit_codes.insert("2".to_string(), "usage error / invalid arguments".to_string());

    CommandSchema {
        version: SCHEMA_VERSION.to_string(),
        command: full_name.to_string(),
        description,
        deprecated,
        arguments,
        subcommands,
        output: OutputSchema { formats },
        exit_codes,
        examples: Vec::new(),
    }
}

/// Determine whether argument represents sensitive/secret data.
fn is_secret_argument(id: &str, name: &str) -> bool {
    let lower_id = id.to_lowercase();
    let lower_name = name.to_lowercase();

    let secret_keywords = [
        "secret",
        "password",
        "private_key",
        "private-key",
        "api_key",
        "api-key",
        "auth_token",
        "token",
        "signing_seed",
        "seed",
        "bearer",
    ];

    secret_keywords
        .iter()
        .any(|kw| lower_id.contains(kw) || lower_name.contains(kw))
}

/// Determine whether argument represents a file or directory path.
fn is_path_argument(id: &str, name: &str, value_names: Option<&[clap::builder::Str]>) -> bool {
    let lower_id = id.to_lowercase();
    let lower_name = name.to_lowercase();

    let path_keywords = [
        "file",
        "path",
        "wasm",
        "dir",
        "lockfile",
        "manifest",
        "properties",
        "export",
        "output",
        "input",
    ];

    if path_keywords
        .iter()
        .any(|kw| lower_id.contains(kw) || lower_name.contains(kw))
    {
        return true;
    }

    if let Some(names) = value_names {
        for vn in names {
            let lower_vn = vn.as_str().to_lowercase();
            if lower_vn == "path" || lower_vn == "file" || lower_vn == "dir" || lower_vn == "wasm" {
                return true;
            }
        }
    }

    false
}

/// Determine whether argument represents numeric input.
fn is_numeric_argument(id: &str, name: &str) -> bool {
    let lower_id = id.to_lowercase();
    let lower_name = name.to_lowercase();

    let num_keywords = [
        "limit",
        "offset",
        "depth",
        "timeout",
        "age",
        "threshold",
        "port",
        "days",
        "count",
        "iterations",
        "refresh_rate",
        "page_size",
    ];

    num_keywords
        .iter()
        .any(|kw| lower_id.contains(kw) || lower_name.contains(kw))
}

/// Infer output formats supported by command arguments.
fn infer_output_formats(args: &BTreeMap<String, ArgumentSchema>) -> Vec<String> {
    if let Some(fmt_arg) = args.get("format").or_else(|| args.get("report_format")) {
        if let Some(ref vals) = fmt_arg.enum_values {
            return vals.clone();
        }
    }

    let mut formats = vec!["table".to_string()];
    if args.contains_key("json") || args.values().any(|a| a.description.as_deref().unwrap_or("").contains("JSON")) {
        formats.push("json".to_string());
    }

    formats.dedup();
    formats
}

/// Intercept `--describe` flag or command invocation if present in raw arguments.
pub fn process_describe_if_requested(root_cmd: &Command) -> Result<bool> {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--describe") {
        return Ok(false);
    }

    let mut subcommand_path: Vec<&str> = Vec::new();

    for arg in args.iter().skip(1) {
        if arg == "--describe" {
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        subcommand_path.push(arg.as_str());
    }

    let schema = generate_schema_for_path(root_cmd, &subcommand_path)?;
    let json_str = serde_json::to_string_pretty(&schema)
        .context("Failed to serialize command schema to JSON")?;
    println!("{json_str}");
    Ok(true)
}

/// Generate or check CLI schema and completion artifacts.
pub fn generate_or_check_artifacts(check: bool, dir: Option<&Path>) -> Result<()> {
    let default_dir = if Path::new("Cargo.toml").exists()
        && fs::read_to_string("Cargo.toml")
            .map(|c| c.contains("soroban-registry-cli"))
            .unwrap_or(false)
    {
        Path::new("generated")
    } else {
        Path::new("cli/generated")
    };
    let target_dir = dir.unwrap_or(default_dir);
    fs::create_dir_all(target_dir.join("completions"))
        .with_context(|| format!("failed to create directory {}", target_dir.display()))?;

    let root_cmd = crate::Cli::command();
    let root_schema = build_schema(&root_cmd, root_cmd.get_name());
    let schema_json = serde_json::to_string_pretty(&root_schema)?;

    let schema_path = target_dir.join("schema.json");

    if check {
        if !schema_path.exists() {
            bail!("CLI schema file missing: {}", schema_path.display());
        }
        let existing = fs::read_to_string(&schema_path)?;
        let existing_parsed: serde_json::Value = serde_json::from_str(&existing)?;
        let current_parsed: serde_json::Value = serde_json::from_str(&schema_json)?;
        if existing_parsed != current_parsed {
            bail!(
                "CLI schema file is out of date: {}. Run 'soroban-registry generate-artifacts' to regenerate.",
                schema_path.display()
            );
        }
    } else {
        fs::write(&schema_path, schema_json)?;
    }

    crate::completion::generate_all_completions(target_dir.join("completions").as_path(), check)?;

    if !check {
        println!("Successfully generated CLI artifacts in {}", target_dir.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn root_schema_has_version_and_subcommands() {
        let cmd = crate::Cli::command();
        let schema = build_schema(&cmd, cmd.get_name());
        assert_eq!(schema.version, SCHEMA_VERSION);
        assert_eq!(schema.command, "soroban-registry");
        assert!(!schema.subcommands.is_empty());
        assert!(schema.subcommands.contains(&"contract".to_string()));
    }

    #[test]
    fn boolean_arguments_are_typed_boolean_not_enum() {
        let cmd = crate::Cli::command();
        let schema = build_schema(&cmd, cmd.get_name());
        // --json and --describe are boolean flags (ArgAction::SetTrue)
        if let Some(json_arg) = schema.arguments.get("json") {
            assert_eq!(json_arg.arg_type, "boolean", "--json should be typed as boolean");
        }
        if let Some(desc) = schema.arguments.get("describe") {
            assert_eq!(desc.arg_type, "boolean", "--describe should be typed as boolean");
        }
    }

    #[test]
    fn enum_arguments_detected_from_possible_values() {
        let cmd = crate::Cli::command();
        let contract_sub = cmd.find_subcommand("contract").expect("contract subcommand");
        // The 'verify' subcommand should have a --format arg with enum values
        if let Some(verify) = contract_sub.find_subcommand("verify") {
            let schema = build_schema(verify, "soroban-registry contract verify");
            // Check that at least one argument is typed as enum
            let has_enum = schema.arguments.values().any(|a| a.arg_type == "enum");
            assert!(has_enum, "expected at least one enum argument in contract verify");
        }
    }

    #[test]
    fn secret_arguments_flagged() {
        let cmd = crate::Cli::command();
        let contract_sub = cmd.find_subcommand("contract").expect("contract subcommand");
        if let Some(deprecate) = contract_sub.find_subcommand("deprecate") {
            let schema = build_schema(deprecate, "soroban-registry contract deprecate");
            let pk = schema.arguments.get("private-key").expect("private-key arg");
            assert!(pk.secret, "private-key should be flagged as secret");
            assert!(pk.default_value.is_none(), "secret args must not expose default_value");
        }
    }

    #[test]
    fn nested_command_schema_resolves_path() {
        let cmd = crate::Cli::command();
        let schema = generate_schema_for_path(&cmd, &["contract", "inspect-spec"])
            .expect("resolve nested path");
        assert_eq!(schema.command, "soroban-registry contract inspect-spec");
        assert!(schema.arguments.contains_key("wasm-file"));
    }

    #[test]
    fn unknown_subcommand_returns_error() {
        let cmd = crate::Cli::command();
        let result = generate_schema_for_path(&cmd, &["nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn verbose_flag_is_count_type() {
        let cmd = crate::Cli::command();
        let schema = build_schema(&cmd, cmd.get_name());
        let verbose = schema.arguments.get("verbose").expect("verbose arg");
        assert_eq!(verbose.arg_type, "number", "--verbose should be typed as number (count)");
        assert!(verbose.repeatable, "--verbose should be repeatable");
    }

    #[test]
    fn output_formats_inferred_from_format_arg() {
        let cmd = crate::Cli::command();
        if let Some(list) = cmd.find_subcommand("list") {
            let schema = build_schema(list, "soroban-registry list");
            assert!(schema.output.formats.contains(&"json".to_string()));
            assert!(schema.output.formats.contains(&"table".to_string()));
            assert!(schema.output.formats.contains(&"csv".to_string()));
        }
    }

    #[test]
    fn exit_codes_present() {
        let cmd = crate::Cli::command();
        let schema = build_schema(&cmd, cmd.get_name());
        assert!(schema.exit_codes.contains_key("0"));
        assert!(schema.exit_codes.contains_key("1"));
        assert!(schema.exit_codes.contains_key("2"));
    }
}
