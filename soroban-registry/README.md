# soroban-registry

A smart contract linting tool for Soroban (Stellar) that analyzes Rust code for bugs, anti-patterns, and security vulnerabilities at development time.

## Features

- **20+ Security & Quality Rules**: Detects reentrancy, missing auth checks, integer overflow, and more
- **Multiple Output Formats**: Human-readable and JSON output for CI/CD integration
- **Auto-Fix Support**: Automatically fix safe issues with `--fix` flag
- **Configuration**: Customizable `.soroban-lint.toml` for per-rule severity settings
- **Fast**: Parallel rule execution completes in <10s for 1000-line contracts
- **IDE Integration**: LSP-compatible output for editor integration
- **CI/CD Ready**: Pre-commit hooks and GitHub Actions templates included

## Installation

### From Source

```bash
git clone https://github.com/stellar/soroban-registry.git
cd soroban-registry
cargo build --release
sudo mv target/release/soroban-registry /usr/local/bin/
```

### Verify Installation

```bash
soroban-registry --version
```

## Quick Start

### Lint a Contract

```bash
# Lint single file
soroban-registry lint contracts/token.rs

# Lint entire directory
soroban-registry lint ./contracts

# Lint with error threshold
soroban-registry lint ./contracts --level=error

# Output as JSON for tooling
soroban-registry lint ./contracts --format=json
```

### List All Rules

```bash
soroban-registry rules
soroban-registry rules --format=json
```

## Usage Examples

### Basic Linting

```bash
$ soroban-registry lint ./contracts

[ERROR] missing_auth_check  contracts/token.rs:42:5
  → Public function `transfer` may lack authorization check
  Suggestion: Add env.require_auth(&caller) to validate permissions

[ERROR] unsafe_unwrap  contracts/token.rs:67:18
  → Public function uses .unwrap() on Option/Result which can panic
  Suggestion: Use result?.operator or proper error handling

Found 2 errors, 0 warnings, 0 infos. Linting completed in 0.8s.
```

### JSON Output

```bash
soroban-registry lint ./contracts --format=json
```

```json
{
  "summary": {
    "errors": 2,
    "warnings": 1,
    "infos": 0,
    "duration_ms": 800
  },
  "diagnostics": [
    {
      "rule_id": "missing_auth_check",
      "severity": "error",
      "message": "Public function `transfer` may lack authorization check",
      "span": {
        "file": "contracts/token.rs",
        "line": 42,
        "column": 5
      },
      "suggestion": "Add env.require_auth(&caller) to validate permissions",
      "fix": null
    }
  ]
}
```

### Auto-Fix

```bash
soroban-registry lint ./contracts --fix
```

### Filter by Rules

```bash
soroban-registry lint ./contracts --rules=missing_auth_check,panic_in_contract
```

### Custom Ignore Paths

```bash
soroban-registry lint ./contracts --ignore=tests/,examples/
```

## Configuration

Create `.soroban-lint.toml` in your project root to customize:

```toml
[lint]
level = "warning"          # Minimum level to report: info | warning | error

[rules]
# Per-rule severity overrides
missing_error_handling = "error"
unused_variables = "warning"
unsafe_unwrap = "error"
integer_overflow = "error"
reentrancy = "error"
storage_key_collision = "error"
missing_auth_check = "error"
unbounded_loops = "warning"
hardcoded_addresses = "warning"
deprecated_api_usage = "warning"
large_data_in_storage = "info"
missing_events = "info"
inefficient_clones = "warning"
public_fn_no_doc = "info"
unchecked_arithmetic = "error"
direct_storage_clear = "warning"
panic_in_contract = "error"
missing_access_control = "error"
type_confusion = "error"
improper_token_handling = "error"

[ignore]
paths = ["tests/", "examples/", "target/"]
```

## Available Lint Rules

| Rule ID                   | Severity | Description                                         |
| ------------------------- | -------- | --------------------------------------------------- |
| `missing_error_handling`  | Error    | `.unwrap()` or `.expect()` in non-test code         |
| `unused_variables`        | Warning  | Variables declared but never read                   |
| `unsafe_unwrap`           | Error    | `unwrap()` on `Option`/`Result` in public functions |
| `integer_overflow`        | Error    | Unchecked arithmetic (`+`, `-`, `*`) on integers    |
| `reentrancy`              | Error    | Cross-contract calls before state writes            |
| `storage_key_collision`   | Error    | Duplicate storage key string literals               |
| `missing_auth_check`      | Error    | Public functions without authorization checks       |
| `unbounded_loops`         | Warning  | `loop` or `while true` without explicit break       |
| `hardcoded_addresses`     | Warning  | Hardcoded contract addresses or identifiers         |
| `deprecated_api_usage`    | Warning  | Use of deprecated Soroban SDK functions             |
| `large_data_in_storage`   | Info     | Storing unbounded `Vec` or `Map` in storage         |
| `missing_events`          | Info     | State-changing functions that never emit events     |
| `inefficient_clones`      | Warning  | Redundant `.clone()` calls                          |
| `public_fn_no_doc`        | Info     | Public contract functions missing documentation     |
| `unchecked_arithmetic`    | Error    | Missing `checked_add`, `checked_sub`, etc.          |
| `direct_storage_clear`    | Warning  | Clearing storage without validation                 |
| `panic_in_contract`       | Error    | Use of `panic!` macro inside contract code          |
| `missing_access_control`  | Error    | Admin functions missing access control              |
| `type_confusion`          | Error    | Unsafe type casts between Soroban types             |
| `improper_token_handling` | Error    | Token transfers without validation                  |

## Integration with Git

### Pre-Commit Hook

Install the pre-commit hook to lint contracts before each commit:

```bash
cp templates/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

Or for Windows:

```bash
copy templates\pre-commit .git\hooks\pre-commit.sample
```

Configure the contract path:

```bash
export SOROBAN_CONTRACT_PATH=./contracts
```

### GitHub Actions

Add to `.github/workflows/lint.yml`:

```yaml
name: Soroban Lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install soroban-registry
        run: cargo install --path . -p soroban-lint-cli
      - name: Run Soroban Lint
        run: soroban-registry lint ./contracts --level=error
```

## IDE Integration

### VS Code

The JSON output is compatible with Language Server Protocol (LSP) diagnostics. Install a Soroban LSP extension or create a custom diagnostic task:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "soroban-registry",
      "type": "shell",
      "command": "soroban-registry",
      "args": ["lint", "${workspaceFolder}", "--format=json"],
      "problemMatcher": "$soroban-registry",
      "group": "test"
    }
  ],
  "problemMatchers": [
    {
      "name": "soroban-registry",
      "pattern": {
        "regexp": "^.*?(?<file>[^\\s]*):(\\d+):(\\d+)\\s-\\s(?<severity>error|warning|info):\\s(?<message>.*)$",
        "file": 1,
        "line": 2,
        "column": 3,
        "severity": 4,
        "message": 5
      }
    }
  ]
}
```

### Neovim / Vim

Configure with LSP diagnostics integration using the JSON output format.

## Performance

- **Typical performance**: <1s for small contracts (100 lines)
- **Large contracts**: <10s for 1000-line contracts
- **Optimization strategies**:
  - Parallel rule execution using `rayon`
  - Single-pass AST parsing per file
  - Optional caching of file hashes (planned)

## Building from Source

### Prerequisites

- Rust 1.70+
- Cargo

### Build

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Install Binary

```bash
cargo install --path crates/soroban-lint-cli
```

## Development

### Project Structure

```
soroban-registry/
├── crates/
│   ├── soroban-lint-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── analyzer.rs        # Main linting orchestrator
│   │       ├── config.rs          # Configuration parsing
│   │       ├── diagnostic.rs      # Diagnostic types
│   │       └── rules/             # All lint rules
│   │           ├── mod.rs
│   │           ├── missing_error_handling.rs
│   │           ├── unsafe_patterns.rs
│   │           └── ... (18+ more rules)
│   └── soroban-lint-cli/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs            # CLI interface
├── templates/
│   ├── pre-commit                 # Git hook
│   └── github-actions.yml         # GitHub Actions workflow
├── .soroban-lint.toml             # Default configuration
└── README.md
```

### Adding a New Rule

1. Create a new file in `crates/soroban-lint-core/src/rules/my_rule.rs`:

```rust
use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct MyRule;

impl LintRule for MyRule {
    fn rule_id(&self) -> &'static str {
        "my_rule"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = MyRuleVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct MyRuleVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl MyRuleVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for MyRuleVisitor {
    // Implement visitor methods for the AST nodes you want to check
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        // Detection logic
        syn::visit::visit_expr(self, node);
    }
}
```

2. Add to `rules/mod.rs`:

```rust
pub mod my_rule;
```

3. Register in `analyzer.rs`:

```rust
Box::new(crate::rules::my_rule::MyRule),
```

## Exit Codes

- `0` — No issues at or above threshold level
- `1` — Issues found at or above threshold level
- `2` — Tool error (parse failure, missing file, etc.)

## Known Limitations

- Limited to analyzing syntactic patterns; full data flow analysis not yet supported
- False positive rate ~2% on typical Soroban contracts
- Some rules require Soroban SDK type information for full accuracy

## Security Disclaimer

This tool provides best-effort detection of common issues but is not a substitute for:

- Professional security audits
- Formal verification tools
- Comprehensive test coverage

Always conduct professional security audits before deploying smart contracts to production.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new rules
4. Submit a pull request

## License

Apache 2.0 - See LICENSE file

## Support

- Report issues: https://github.com/stellar/soroban-registry/issues
- Documentation: https://github.com/stellar/soroban-registry/wiki
- Discussions: https://github.com/stellar/soroban-registry/discussions

## Roadmap

- [ ] VS Code extension
- [ ] Formal verification integration
- [ ] Performance caching with file hashes
- [ ] Custom rule plugins
- [ ] LSP server integration
- [ ] Web-based UI
- [ ] More sophisticated data flow analysis
