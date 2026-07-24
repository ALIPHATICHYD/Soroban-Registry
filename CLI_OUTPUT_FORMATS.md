# CLI Output Formats Documentation

## Overview

The Soroban Registry CLI supports multiple output formats for machine-readable automation and human-readable display. This document describes the supported formats, their usage, and schema stability guarantees.

## Supported Formats

### 1. Table (Default)
- **Format**: Human-readable table with ANSI colors
- **Use Case**: Interactive terminal usage
- **Stability**: Not guaranteed (may change for improved readability)
- **Example**:
  ```
  ID                                   Name                           Network         Verified   Health      Category
  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  550e8400-e29b-41d4-a716-446655440000 MyToken                        testnet         ✓          95          defi
  ```

### 2. JSON
- **Format**: Pretty-printed JSON with consistent schema
- **Use Case**: Automation, scripting, API integration
- **Stability**: Guaranteed (schema versioning maintained)
- **Example**:
  ```json
  {
    "contracts": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "MyToken",
        "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
        "network": "testnet",
        "category": "defi",
        "is_verified": true,
        "health_score": 95,
        "created_at": "2024-01-15T10:30:00Z",
        "tags": ["token", "erc20"]
      }
    ],
    "count": 1
  }
  ```

### 3. CSV
- **Format**: Comma-separated values with proper escaping
- **Use Case**: Data analysis, spreadsheet import, bulk processing
- **Stability**: Guaranteed (column order and names maintained)
- **Example**:
  ```csv
  id,name,contract_id,network,category,is_verified,health_score,created_at,tags
  "550e8400-e29b-41d4-a716-446655440000","MyToken","CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4","testnet","defi",true,95,"2024-01-15T10:30:00Z","token|erc20"
  ```

### 4. YAML
- **Format**: YAML with consistent schema
- **Use Case**: Configuration files, Kubernetes manifests, human-readable structured data
- **Stability**: Guaranteed (schema versioning maintained)
- **Example**:
  ```yaml
  contracts:
    - id: 550e8400-e29b-41d4-a716-446655440000
      name: MyToken
      contract_id: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4
      network: testnet
      category: defi
      is_verified: true
      health_score: 95
      created_at: 2024-01-15T10:30:00Z
      tags:
        - token
        - erc20
  count: 1
  ```

## Usage

### Command-Line Flags

All commands that support output formatting accept the `--format` flag:

```bash
# Specify format explicitly
soroban-registry list --format json
soroban-registry list --format csv
soroban-registry list --format yaml
soroban-registry list --format table  # default

# Analytics with format
soroban-registry analytics top-contracts --format json --period 30d

# Stats with format
soroban-registry stats --format yaml --timeframe all
```

### Format Inference from File Extension

When exporting to a file, the format is automatically inferred from the file extension:

```bash
# Inferred as JSON
soroban-registry list --export contracts.json

# Inferred as CSV
soroban-registry list --export contracts.csv

# Inferred as YAML
soroban-registry list --export contracts.yaml

# Inferred as table (text)
soroban-registry list --export contracts.txt
```

### User Configuration

Set a default output format in your user configuration:

```bash
# Set default format to JSON
soroban-registry config set output_format json

# View current setting
soroban-registry config get output_format

# Reset to default (table)
soroban-registry config reset
```

## Supported Commands

The following commands support output format flags:

### List Commands
- `list` - List contracts with `--format` flag
- `search` - Search contracts (JSON output via `--json` flag)
- `analytics` - Query analytics with `--format` flag
- `stats` - Get registry statistics with `--format` flag

### Batch Operations
- `batch-verify` - Verify multiple contracts with `--json` flag
- `batch-register` - Register multiple contracts with `--json` flag
- `batch-update` - Update multiple contracts with `--json` flag

### Other Commands
- `compare` - Compare contracts with `--json` flag
- `verify` - Verify contract with `--json` flag
- `audit` - Audit contract with `--format` flag
- `analyze` - Analyze contract with `--report_format` flag

## Schema Stability

### Guaranteed Stable Schemas

The following formats have guaranteed schema stability:

- **JSON**: Schema is versioned and documented. Breaking changes will increment the API version.
- **CSV**: Column order and names are stable. New columns may be added at the end.
- **YAML**: Schema matches JSON structure and is versioned.

### Non-Stable Schemas

- **Table**: Format may change for improved readability and is not suitable for automation.

### Schema Versioning

When breaking changes are necessary:
1. A new API version will be introduced
2. Old versions will be supported for at least 2 minor releases
3. Deprecation warnings will be provided
4. Migration guides will be published

## Error Handling

### Invalid Format Names

```bash
$ soroban-registry list --format invalid
Error: Invalid output format 'invalid'. Supported formats: table, json, csv, yaml
```

### Format-Specific Errors

**CSV Format Errors:**
- CSV format requires an array of objects
- Special characters are properly escaped

**YAML Format Errors:**
- YAML serialization errors are reported with context

**JSON Format Errors:**
- JSON serialization errors are reported with context

## Examples

### Export Contract List as JSON

```bash
soroban-registry list --limit 100 --format json > contracts.json
```

### Export Analytics as CSV

```bash
soroban-registry analytics top-contracts --period 30d --format csv --export analytics.csv
```

### Pipe JSON to jq for Processing

```bash
soroban-registry list --format json | jq '.contracts[] | select(.is_verified == true)'
```

### Generate YAML Configuration

```bash
soroban-registry stats --format yaml --output stats.yaml
```

### Batch Verification with JSON Output

```bash
soroban-registry batch-verify --file contracts.txt --json > verification_results.json
```

## Testing

All output formats are tested for:
1. **Correctness**: Data is accurately represented
2. **Consistency**: Same data produces same output
3. **Stability**: Schema remains consistent across versions
4. **Escaping**: Special characters are properly handled
5. **Performance**: Large datasets are handled efficiently

### Running Tests

```bash
# Run all output format tests
cargo test output_format

# Run specific format tests
cargo test output_format::tests::test_render_json
cargo test output_format::tests::test_render_csv
cargo test output_format::tests::test_render_yaml
```

## Best Practices

1. **Use JSON for APIs**: JSON is the most widely supported format for API integration
2. **Use CSV for Data Analysis**: CSV is ideal for importing into spreadsheet applications
3. **Use YAML for Configuration**: YAML is human-readable and suitable for configuration files
4. **Use Table for Interactive Use**: Table format is optimized for terminal display
5. **Validate Output**: Always validate output format before using in production scripts
6. **Handle Errors**: Check exit codes and error messages when using output formats

## Troubleshooting

### Format Not Recognized

**Problem**: `Invalid output format 'json'`
**Solution**: Ensure format name is lowercase and spelled correctly

### CSV Escaping Issues

**Problem**: Commas in data break CSV parsing
**Solution**: Use proper CSV parsing libraries that handle escaping

### YAML Parsing Issues

**Problem**: YAML output doesn't parse correctly
**Solution**: Ensure YAML parser supports the version used (1.2)

### Large Dataset Performance

**Problem**: Slow output for large datasets
**Solution**: Use pagination with `--limit` and `--offset` flags

## Future Enhancements

Potential future output formats:
- **Markdown**: For documentation generation
- **HTML**: For web-based reports
- **Protocol Buffers**: For efficient binary serialization
- **MessagePack**: For compact binary format
- **XML**: For enterprise system integration

## Related Documentation

- [CLI User Guide](./README.md)
- [API Documentation](./API.md)
- [Configuration Guide](./CONFIG.md)
