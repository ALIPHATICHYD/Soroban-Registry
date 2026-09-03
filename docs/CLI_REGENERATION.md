# CLI Metadata & Artifact Regeneration Workflow (#1145)

The Soroban Registry CLI generates shell completions, machine-readable command schemas (`--describe`), and CLI documentation dynamically from a single command specification source (`clap::Command`).

## 1. Machine-Readable Command Schemas (`--describe`)

Any command or subcommand can be queried for its machine-readable JSON schema by appending the global `--describe` flag:

```bash
# Describe root command
soroban-registry --describe

# Describe nested subcommands
soroban-registry contract verify-snapshot --describe
soroban-registry contract audit --describe
```

### Schema JSON Format (`version: "1.0"`)

```json
{
  "version": "1.0",
  "command": "contract verify-snapshot",
  "description": "Verify an offline contract snapshot file (#1116)",
  "deprecated": false,
  "arguments": {
    "file": {
      "type": "path",
      "required": true,
      "repeatable": false,
      "secret": false,
      "deprecated": false,
      "description": "Path to the snapshot JSON file"
    },
    "expect_key": {
      "type": "string",
      "required": false,
      "repeatable": false,
      "secret": false,
      "deprecated": false,
      "description": "Expected signing key fingerprint for pinning"
    }
  },
  "subcommands": [],
  "output": {
    "formats": ["table", "json"]
  },
  "exit_codes": {
    "0": "success / valid command completion",
    "1": "command error / invalid status",
    "2": "usage error / invalid arguments"
  },
  "examples": []
}
```

- **Secrets Protection**: Arguments containing sensitive data (e.g. `--private-key`, `--secret`, `--api-key`) are flagged with `"secret": true` and default values are omitted.
- **Argument Types**: Identified as `path`, `string`, `number`, `boolean`, or `enum`.

---

## 2. Shell Completion Generation

Generate shell completions for Bash, Zsh, Fish, and PowerShell:

```bash
# Per-shell script output
soroban-registry completion bash
soroban-registry completion zsh
soroban-registry completion fish
soroban-registry completion powershell

# Bulk generation into a directory
soroban-registry generate-artifacts --output-dir cli/generated
```

---

## 3. Regenerating CLI Artifacts

Whenever CLI subcommands, flags, or options are added or modified:

```bash
cargo run --bin soroban-registry -- generate-artifacts
```

This updates:
- `cli/generated/schema.json`
- `cli/generated/completions/soroban-registry.bash`
- `cli/generated/completions/_soroban-registry` (Zsh)
- `cli/generated/completions/soroban-registry.fish`
- `cli/generated/completions/soroban-registry.ps1` (PowerShell)

---

## 4. CI Drift Detection

CI runs artifact verification to ensure generated files remain synchronized with command definitions:

```bash
cargo run --bin soroban-registry -- generate-artifacts --check
```

If any generated artifact is out of date or missing, CI fails with non-zero exit code.
