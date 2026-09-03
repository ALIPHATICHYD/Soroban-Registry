
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'soroban-registry' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'soroban-registry'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'soroban-registry' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('analytics', 'analytics', [CompletionResultType]::ParameterValue, 'Query contract analytics and statistics')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Get comprehensive registry statistics')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a new contract to the registry')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List contracts in the registry')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show detailed info for a specific contract')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search for contracts in the registry')
            [CompletionResult]::new('compare', 'compare', [CompletionResultType]::ParameterValue, 'Compare multiple contracts')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (#971)')
            [CompletionResult]::new('generate-artifacts', 'generate-artifacts', [CompletionResultType]::ParameterValue, 'Generate or verify CLI command schemas and shell completion scripts (#1145)')
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Check CLI version and update availability')
            [CompletionResult]::new('dashboard', 'dashboard', [CompletionResultType]::ParameterValue, 'Launch an interactive, real-time terminal dashboard')
            [CompletionResult]::new('breaking-changes', 'breaking-changes', [CompletionResultType]::ParameterValue, 'Detect breaking changes between contract versions')
            [CompletionResult]::new('migrate', 'migrate', [CompletionResultType]::ParameterValue, 'Contract state migration assistant')
            [CompletionResult]::new('upgrade-analyze', 'upgrade-analyze', [CompletionResultType]::ParameterValue, 'Analyze upgrades between two contract versions or schema files')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export contract registry data or a contract archive')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import contract data from a file (JSON, CSV, or Archive)')
            [CompletionResult]::new('doc', 'doc', [CompletionResultType]::ParameterValue, 'Generate documentation from a contract WASM')
            [CompletionResult]::new('openapi', 'openapi', [CompletionResultType]::ParameterValue, 'Generate OpenAPI 3.0 spec from contract ABI')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Start an interactive contract deployment workflow')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'Manage contract semantic versions')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Perform batch operations on multiple contracts')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Manage contract upgrades and rollbacks')
            [CompletionResult]::new('wizard', 'wizard', [CompletionResultType]::ParameterValue, 'Launch the interactive setup wizard')
            [CompletionResult]::new('repl', 'repl', [CompletionResultType]::ParameterValue, 'Enter interactive REPL mode')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Show command history')
            [CompletionResult]::new('patch', 'patch', [CompletionResultType]::ParameterValue, 'Security patch management')
            [CompletionResult]::new('incident', 'incident', [CompletionResultType]::ParameterValue, 'Incident response management')
            [CompletionResult]::new('multisig', 'multisig', [CompletionResultType]::ParameterValue, 'Multi-signature contract deployment workflow')
            [CompletionResult]::new('fuzz', 'fuzz', [CompletionResultType]::ParameterValue, 'Fuzz testing for contracts')
            [CompletionResult]::new('perf', 'perf', [CompletionResultType]::ParameterValue, 'Perf contract execution performance')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'Manage your user profile and publishing preferences (#841)')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Run integration tests')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Run a local contract security audit')
            [CompletionResult]::new('sla', 'sla', [CompletionResultType]::ParameterValue, 'SLA compliance monitoring')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Read and edit persisted user configuration values')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Manage authentication sessions and API tokens')
            [CompletionResult]::new('backup', 'backup', [CompletionResultType]::ParameterValue, 'Manage contract backups and disaster recovery')
            [CompletionResult]::new('state', 'state', [CompletionResultType]::ParameterValue, 'Inspect and modify contract state (dev/test mutation only)')
            [CompletionResult]::new('verify-formal', 'verify-formal', [CompletionResultType]::ParameterValue, 'Run formal verification analysis against a deployed or local contract')
            [CompletionResult]::new('scan-deps', 'scan-deps', [CompletionResultType]::ParameterValue, 'Scan a contract''s dependencies for known vulnerabilities')
            [CompletionResult]::new('coverage', 'coverage', [CompletionResultType]::ParameterValue, 'Measure and report code coverage for contract tests')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a contract package with your private key')
            [CompletionResult]::new('verify-package', 'verify-package', [CompletionResultType]::ParameterValue, 'Verify a signed contract package')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a contract in the registry (check status, submit for audit, or show history)')
            [CompletionResult]::new('verify-contract', 'verify-contract', [CompletionResultType]::ParameterValue, 'Verify a contract binary against an Ed25519 signature locally')
            [CompletionResult]::new('keys', 'keys', [CompletionResultType]::ParameterValue, 'Manage signing keys and signatures')
            [CompletionResult]::new('policy', 'policy', [CompletionResultType]::ParameterValue, 'Policy-as-code admission evaluation and reporting (#1148)')
            [CompletionResult]::new('publisher', 'publisher', [CompletionResultType]::ParameterValue, 'Publisher environment diagnostics (#841)')
            [CompletionResult]::new('contract', 'contract', [CompletionResultType]::ParameterValue, 'Contract deployment verification and security scan (#522)')
            [CompletionResult]::new('api-key', 'api-key', [CompletionResultType]::ParameterValue, 'Manage API keys for programmatic access (#842)')
            [CompletionResult]::new('batch-verify', 'batch-verify', [CompletionResultType]::ParameterValue, 'Verify multiple contracts in a bulk batch (#850)')
            [CompletionResult]::new('webhook', 'webhook', [CompletionResultType]::ParameterValue, 'Manage webhooks for contract lifecycle events')
            [CompletionResult]::new('release-notes', 'release-notes', [CompletionResultType]::ParameterValue, 'Auto-generate and manage release notes for contract versions')
            [CompletionResult]::new('cicd', 'cicd', [CompletionResultType]::ParameterValue, 'CI/CD pipeline integration and automation')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Check the status of supported Stellar networks')
            [CompletionResult]::new('batch-register', 'batch-register', [CompletionResultType]::ParameterValue, 'Register multiple contracts from a YAML or JSON manifest file')
            [CompletionResult]::new('batch-audit', 'batch-audit', [CompletionResultType]::ParameterValue, 'Audit multiple contracts in batch for security and best practices')
            [CompletionResult]::new('batch-deploy', 'batch-deploy', [CompletionResultType]::ParameterValue, 'Deploy a contract WASM to multiple networks')
            [CompletionResult]::new('batch-export', 'batch-export', [CompletionResultType]::ParameterValue, 'Export multiple contracts in bulk')
            [CompletionResult]::new('batch-import', 'batch-import', [CompletionResultType]::ParameterValue, 'Import contracts in bulk from a directory')
            [CompletionResult]::new('batch-update', 'batch-update', [CompletionResultType]::ParameterValue, 'Update metadata for multiple contracts in bulk (#849)')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Run advanced analysis on a deployed contract (#530)')
            [CompletionResult]::new('track-deployment', 'track-deployment', [CompletionResultType]::ParameterValue, 'Track contract deployment status until confirmed or timeout (#524)')
            [CompletionResult]::new('plugins', 'plugins', [CompletionResultType]::ParameterValue, 'Plugin management (install, configure, run)')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Manage local cache of registry API responses (#845)')
            [CompletionResult]::new('env', 'env', [CompletionResultType]::ParameterValue, 'Manage environment variable sets for different deployments (#843)')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Manage signed offline registry snapshots (#1146)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;analytics' {
            [CompletionResult]::new('--period', '--period', [CompletionResultType]::ParameterName, 'Time period: 7d, 30d, 90d, or RFC3339 range start..end')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table, json, csv, yaml')
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'Sort mode: value_desc, value_asc, key_asc, key_desc')
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Export output to a file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;stats' {
            [CompletionResult]::new('--timeframe', '--timeframe', [CompletionResultType]::ParameterName, 'Timeframe: 7d, 30d, or all (default: all)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table, json, yaml')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Export to file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;publish' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'On-chain contract ID')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Human-readable contract name')
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'Optional description')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Network (mainnet, testnet, futurenet)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Category')
            [CompletionResult]::new('--tags', '--tags', [CompletionResultType]::ParameterName, 'Comma-separated tags')
            [CompletionResult]::new('--publisher', '--publisher', [CompletionResultType]::ParameterName, 'Publisher Stellar address')
            [CompletionResult]::new('--contract-path', '--contract-path', [CompletionResultType]::ParameterName, 'Path to contract project directory for preflight testing')
            [CompletionResult]::new('--test-command', '--test-command', [CompletionResultType]::ParameterName, 'Custom test command to run before submission')
            [CompletionResult]::new('--coverage-threshold', '--coverage-threshold', [CompletionResultType]::ParameterName, 'Minimum required coverage percentage (0-100)')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Path to policy-as-code YAML or JSON file (#1148)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--require-coverage', '--require-coverage', [CompletionResultType]::ParameterName, 'Require coverage data and fail if unavailable')
            [CompletionResult]::new('--skip-tests', '--skip-tests', [CompletionResultType]::ParameterName, 'Skip pre-submission contract tests')
            [CompletionResult]::new('--explain', '--explain', [CompletionResultType]::ParameterName, 'Display policy evaluation rule details (#1148)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Evaluate policy and validate without submitting to registry (#1148)')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;list' {
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Max number of contracts to list')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Max number of contracts to list')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Number of contracts to skip')
            [CompletionResult]::new('--offset', '--offset', [CompletionResultType]::ParameterName, 'Number of contracts to skip')
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Filter by network (comma-separated: mainnet,testnet,futurenet). The field is `networks`, not `network`: clap derives an arg id from the field name, and the global `--network` (global = true) shares that id, so a subcommand-local `network` field would collide with it and panic trying to downcast the matched value')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated for multiple: DeFi,NFT)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated for multiple: DeFi,NFT)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (table, json, csv, yaml)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json, csv, yaml)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;info' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;search' {
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Filter by network (comma-separated: mainnet,testnet,futurenet). The field is `networks`, not `network`: clap derives an arg id from the field name, and the global `--network` (global = true) shares that id, so clap would populate both from either flag')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated: DeFi,NFT)')
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'Sort by (name, created, updated, relevance)')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum results to return')
            [CompletionResult]::new('--offset', '--offset', [CompletionResultType]::ParameterName, 'Results offset')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--verified-only', '--verified-only', [CompletionResultType]::ParameterName, 'Only show verified contracts')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;compare' {
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Export comparison report to a file (csv or json)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Export format (csv or json). Derived from file extension if not provided')
            [CompletionResult]::new('--diff', '--diff', [CompletionResultType]::ParameterName, 'Diff output format: none, unified, side-by-side')
            [CompletionResult]::new('--fields', '--fields', [CompletionResultType]::ParameterName, 'Limit compared field groups (metadata,verification,deployment,abi,all)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output detailed comparison as JSON')
            [CompletionResult]::new('--exit-code', '--exit-code', [CompletionResultType]::ParameterName, 'Exit with code 1 when differences are found (0 = identical, 2 = error)')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;completion' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;generate-artifacts' {
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Target output directory for generated artifacts')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Verify that existing generated artifacts are up to date (fails CI on drift)')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;version' {
            [CompletionResult]::new('--rollback', '--rollback', [CompletionResultType]::ParameterName, 'Roll back to a previous version (manual install helper)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check upstream for newer versions')
            [CompletionResult]::new('--auto-update', '--auto-update', [CompletionResultType]::ParameterName, 'Print update instructions immediately when newer version exists')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;dashboard' {
            [CompletionResult]::new('--refresh-rate', '--refresh-rate', [CompletionResultType]::ParameterName, 'Minimum interval between UI renders (milliseconds)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by contract category')
            [CompletionResult]::new('--ws-url', '--ws-url', [CompletionResultType]::ParameterName, 'WebSocket URL (or set SOROBAN_REGISTRY_WS_URL)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;breaking-changes' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('preview', 'preview', [CompletionResultType]::ParameterValue, 'Preview migration outcome (dry-run)')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze schema differences between versions')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate migration script template (rust|js)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate migration for data loss risks')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply migration and record history')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a migration by migration ID')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Show migration history')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;migrate;preview' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;analyze' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;generate' {
            [CompletionResult]::new('--language', '--language', [CompletionResultType]::ParameterName, 'language')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'output')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;validate' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;apply' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;rollback' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;history' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;migrate;help' {
            [CompletionResult]::new('preview', 'preview', [CompletionResultType]::ParameterValue, 'Preview migration outcome (dry-run)')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze schema differences between versions')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate migration script template (rust|js)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate migration for data loss risks')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply migration and record history')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a migration by migration ID')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Show migration history')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;migrate;help;preview' {
            break
        }
        'soroban-registry;migrate;help;analyze' {
            break
        }
        'soroban-registry;migrate;help;generate' {
            break
        }
        'soroban-registry;migrate;help;validate' {
            break
        }
        'soroban-registry;migrate;help;apply' {
            break
        }
        'soroban-registry;migrate;help;rollback' {
            break
        }
        'soroban-registry;migrate;help;history' {
            break
        }
        'soroban-registry;migrate;help;help' {
            break
        }
        'soroban-registry;upgrade-analyze' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;export' {
            [CompletionResult]::new('--id', '--id', [CompletionResultType]::ParameterName, 'Contract registry ID (UUID or on-chain address). Omit to export a filtered contract list')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path. Defaults to contracts-export.<format> or contract-export.tar.gz for archive')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path. Defaults to contracts-export.<format> or contract-export.tar.gz for archive')
            [CompletionResult]::new('--contract-dir', '--contract-dir', [CompletionResultType]::ParameterName, 'Path to contract source directory')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Export format: json, csv, markdown, or archive')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Export format: json, csv, markdown, or archive')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'Filter to apply to registry exports, e.g. --filter network=mainnet --filter verified_only=true')
            [CompletionResult]::new('--page-size', '--page-size', [CompletionResultType]::ParameterName, 'Number of contracts to fetch per API page for list exports')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;import' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Format of the file (json | csv | archive). If omitted, inferred from extension')
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Directory to extract into (only for archive format)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--validate', '--validate', [CompletionResultType]::ParameterName, 'Validate the data before importing')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Perform a dry run without actually importing')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;doc' {
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output directory')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;openapi' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format: yaml, json, markdown, html')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: yaml, json, markdown, html')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;deploy' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;versions' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List versions for a contract')
            [CompletionResult]::new('bump', 'bump', [CompletionResultType]::ParameterValue, 'Bump the semantic version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;versions;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;versions;bump' {
            [CompletionResult]::new('--level', '--level', [CompletionResultType]::ParameterName, 'Bump level: major, minor, or patch')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;versions;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List versions for a contract')
            [CompletionResult]::new('bump', 'bump', [CompletionResultType]::ParameterValue, 'Bump the semantic version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;versions;help;list' {
            break
        }
        'soroban-registry;versions;help;bump' {
            break
        }
        'soroban-registry;versions;help;help' {
            break
        }
        'soroban-registry;batch' {
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Optional file containing contract IDs (one per line)')
            [CompletionResult]::new('--value', '--value', [CompletionResultType]::ParameterName, 'Optional operation value (required for tag/categorize)')
            [CompletionResult]::new('--recipients', '--recipients', [CompletionResultType]::ParameterName, 'Recipients file/filter for `batch notify`')
            [CompletionResult]::new('--message-type', '--message-type', [CompletionResultType]::ParameterName, 'Message type for `batch notify`')
            [CompletionResult]::new('--template', '--template', [CompletionResultType]::ParameterName, 'Template file or inline template for `batch notify`')
            [CompletionResult]::new('--schedule', '--schedule', [CompletionResultType]::ParameterName, 'RFC3339 schedule for `batch notify`')
            [CompletionResult]::new('--channels', '--channels', [CompletionResultType]::ParameterName, 'Channels for `batch notify`: email,in-app,webhook')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'Filter expression for `batch migrate`')
            [CompletionResult]::new('--report', '--report', [CompletionResultType]::ParameterName, 'Migration report output path')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--rollback-on-error', '--rollback-on-error', [CompletionResultType]::ParameterName, 'Roll back already-applied operations when any item fails')
            [CompletionResult]::new('--preview', '--preview', [CompletionResultType]::ParameterName, 'Preview notification/migration without sending/writing')
            [CompletionResult]::new('--atomic', '--atomic', [CompletionResultType]::ParameterName, 'Use atomic/fail-safe migration semantics')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output JSON summary')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;upgrade' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze compatibility between two contract versions')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply an upgrade to a deployed contract')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a contract to a previous version')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a migration script template between versions')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;upgrade;analyze' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;upgrade;apply' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;upgrade;rollback' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;upgrade;generate' {
            [CompletionResult]::new('--language', '--language', [CompletionResultType]::ParameterName, 'Language (rust or js)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;upgrade;help' {
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze compatibility between two contract versions')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply an upgrade to a deployed contract')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a contract to a previous version')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a migration script template between versions')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;upgrade;help;analyze' {
            break
        }
        'soroban-registry;upgrade;help;apply' {
            break
        }
        'soroban-registry;upgrade;help;rollback' {
            break
        }
        'soroban-registry;upgrade;help;generate' {
            break
        }
        'soroban-registry;upgrade;help;help' {
            break
        }
        'soroban-registry;wizard' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;repl' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Initial network')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;history' {
            [CompletionResult]::new('--search', '--search', [CompletionResultType]::ParameterName, 'Filter by search term')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of entries to show')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;patch' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new security patch')
            [CompletionResult]::new('notify', 'notify', [CompletionResultType]::ParameterValue, 'Notify subscribers about a patch')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply a patch to a specific contract')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'Manage contract dependencies')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;patch;create' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'hash')
            [CompletionResult]::new('--severity', '--severity', [CompletionResultType]::ParameterName, 'severity')
            [CompletionResult]::new('--rollout', '--rollout', [CompletionResultType]::ParameterName, 'rollout')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;patch;notify' {
            [CompletionResult]::new('--patch-id', '--patch-id', [CompletionResultType]::ParameterName, 'patch-id')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;patch;apply' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--patch-id', '--patch-id', [CompletionResultType]::ParameterName, 'patch-id')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;patch;deps' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List dependencies for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;patch;deps;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;patch;deps;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List dependencies for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;patch;deps;help;list' {
            break
        }
        'soroban-registry;patch;deps;help;help' {
            break
        }
        'soroban-registry;patch;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new security patch')
            [CompletionResult]::new('notify', 'notify', [CompletionResultType]::ParameterValue, 'Notify subscribers about a patch')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply a patch to a specific contract')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'Manage contract dependencies')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;patch;help;create' {
            break
        }
        'soroban-registry;patch;help;notify' {
            break
        }
        'soroban-registry;patch;help;apply' {
            break
        }
        'soroban-registry;patch;help;deps' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List dependencies for a contract')
            break
        }
        'soroban-registry;patch;help;deps;list' {
            break
        }
        'soroban-registry;patch;help;help' {
            break
        }
        'soroban-registry;incident' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('trigger', 'trigger', [CompletionResultType]::ParameterValue, 'Trigger a new incident for a contract')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update the state of an existing incident')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;incident;trigger' {
            [CompletionResult]::new('--severity', '--severity', [CompletionResultType]::ParameterName, 'Incident severity (critical|high|medium|low)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;incident;update' {
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'New state (detected|responding|contained|recovered|post_review)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;incident;help' {
            [CompletionResult]::new('trigger', 'trigger', [CompletionResultType]::ParameterValue, 'Trigger a new incident for a contract')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update the state of an existing incident')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;incident;help;trigger' {
            break
        }
        'soroban-registry;incident;help;update' {
            break
        }
        'soroban-registry;incident;help;help' {
            break
        }
        'soroban-registry;multisig' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('create-policy', 'create-policy', [CompletionResultType]::ParameterValue, 'Create a new multi-sig policy (defines signers and required threshold)')
            [CompletionResult]::new('create-proposal', 'create-proposal', [CompletionResultType]::ParameterValue, 'Create an unsigned deployment proposal')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a deployment proposal (add your approval)')
            [CompletionResult]::new('execute', 'execute', [CompletionResultType]::ParameterValue, 'Execute an approved deployment proposal')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show full info for a proposal (signatures, policy, status)')
            [CompletionResult]::new('list-proposals', 'list-proposals', [CompletionResultType]::ParameterValue, 'List deployment proposals')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;multisig;create-policy' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('--threshold', '--threshold', [CompletionResultType]::ParameterName, 'threshold')
            [CompletionResult]::new('--signers', '--signers', [CompletionResultType]::ParameterName, 'signers')
            [CompletionResult]::new('--expiry-secs', '--expiry-secs', [CompletionResultType]::ParameterName, 'expiry-secs')
            [CompletionResult]::new('--created-by', '--created-by', [CompletionResultType]::ParameterName, 'created-by')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;create-proposal' {
            [CompletionResult]::new('--contract-name', '--contract-name', [CompletionResultType]::ParameterName, 'contract-name')
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--wasm-hash', '--wasm-hash', [CompletionResultType]::ParameterName, 'wasm-hash')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'network')
            [CompletionResult]::new('--policy-id', '--policy-id', [CompletionResultType]::ParameterName, 'policy-id')
            [CompletionResult]::new('--proposer', '--proposer', [CompletionResultType]::ParameterName, 'proposer')
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'description')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;sign' {
            [CompletionResult]::new('--signer', '--signer', [CompletionResultType]::ParameterName, 'signer')
            [CompletionResult]::new('--signature-data', '--signature-data', [CompletionResultType]::ParameterName, 'signature-data')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;execute' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;info' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;list-proposals' {
            [CompletionResult]::new('--status', '--status', [CompletionResultType]::ParameterName, 'status')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;multisig;help' {
            [CompletionResult]::new('create-policy', 'create-policy', [CompletionResultType]::ParameterValue, 'Create a new multi-sig policy (defines signers and required threshold)')
            [CompletionResult]::new('create-proposal', 'create-proposal', [CompletionResultType]::ParameterValue, 'Create an unsigned deployment proposal')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a deployment proposal (add your approval)')
            [CompletionResult]::new('execute', 'execute', [CompletionResultType]::ParameterValue, 'Execute an approved deployment proposal')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show full info for a proposal (signatures, policy, status)')
            [CompletionResult]::new('list-proposals', 'list-proposals', [CompletionResultType]::ParameterValue, 'List deployment proposals')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;multisig;help;create-policy' {
            break
        }
        'soroban-registry;multisig;help;create-proposal' {
            break
        }
        'soroban-registry;multisig;help;sign' {
            break
        }
        'soroban-registry;multisig;help;execute' {
            break
        }
        'soroban-registry;multisig;help;info' {
            break
        }
        'soroban-registry;multisig;help;list-proposals' {
            break
        }
        'soroban-registry;multisig;help;help' {
            break
        }
        'soroban-registry;fuzz' {
            [CompletionResult]::new('--contract-path', '--contract-path', [CompletionResultType]::ParameterName, 'contract-path')
            [CompletionResult]::new('--duration', '--duration', [CompletionResultType]::ParameterName, 'duration')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'timeout')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--max-cases', '--max-cases', [CompletionResultType]::ParameterName, 'max-cases')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'output')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--minimize', '--minimize', [CompletionResultType]::ParameterName, 'minimize')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;perf' {
            [CompletionResult]::new('--method', '--method', [CompletionResultType]::ParameterName, 'Method to profile')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output JSON file')
            [CompletionResult]::new('--flamegraph', '--flamegraph', [CompletionResultType]::ParameterName, 'Generate flame graph')
            [CompletionResult]::new('--compare', '--compare', [CompletionResultType]::ParameterName, 'Compare with baseline profile')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--recommendations', '--recommendations', [CompletionResultType]::ParameterName, 'Show recommendations')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'Display a publisher profile')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Update profile fields')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update a single profile field by key')
            [CompletionResult]::new('list-contracts', 'list-contracts', [CompletionResultType]::ParameterValue, 'List contracts published by a profile')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export full profile data to JSON or CSV')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;profile;view' {
            [CompletionResult]::new('--address', '--address', [CompletionResultType]::ParameterName, 'Stellar address or publisher UUID to look up (defaults to the address in local config)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile;edit' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Display name')
            [CompletionResult]::new('--bio', '--bio', [CompletionResultType]::ParameterName, 'Short biography or description')
            [CompletionResult]::new('--website', '--website', [CompletionResultType]::ParameterName, 'Personal or project website URL')
            [CompletionResult]::new('--email', '--email', [CompletionResultType]::ParameterName, 'Contact email address')
            [CompletionResult]::new('--github', '--github', [CompletionResultType]::ParameterName, 'GitHub profile URL')
            [CompletionResult]::new('--avatar', '--avatar', [CompletionResultType]::ParameterName, 'Avatar image URL')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile;update' {
            [CompletionResult]::new('--field', '--field', [CompletionResultType]::ParameterName, 'Field to update (name | bio | website | email | github | avatar)')
            [CompletionResult]::new('--value', '--value', [CompletionResultType]::ParameterName, 'New value for the field')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile;list-contracts' {
            [CompletionResult]::new('--address', '--address', [CompletionResultType]::ParameterName, 'Stellar address or publisher UUID (defaults to local config)')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of contracts to return')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table | csv')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile;export' {
            [CompletionResult]::new('--address', '--address', [CompletionResultType]::ParameterName, 'Stellar address or publisher UUID (defaults to local config)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Export format: json | csv')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;profile;help' {
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'Display a publisher profile')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Update profile fields')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update a single profile field by key')
            [CompletionResult]::new('list-contracts', 'list-contracts', [CompletionResultType]::ParameterValue, 'List contracts published by a profile')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export full profile data to JSON or CSV')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;profile;help;view' {
            break
        }
        'soroban-registry;profile;help;edit' {
            break
        }
        'soroban-registry;profile;help;update' {
            break
        }
        'soroban-registry;profile;help;list-contracts' {
            break
        }
        'soroban-registry;profile;help;export' {
            break
        }
        'soroban-registry;profile;help;help' {
            break
        }
        'soroban-registry;test' {
            [CompletionResult]::new('--contract-path', '--contract-path', [CompletionResultType]::ParameterName, 'Path to contract directory or file')
            [CompletionResult]::new('--test-command', '--test-command', [CompletionResultType]::ParameterName, 'Custom test command (for auto-detected project tests mode)')
            [CompletionResult]::new('--junit', '--junit', [CompletionResultType]::ParameterName, 'Output JUnit XML report')
            [CompletionResult]::new('--coverage-threshold', '--coverage-threshold', [CompletionResultType]::ParameterName, 'Minimum required coverage percentage (0-100)')
            [CompletionResult]::new('--setup-hook', '--setup-hook', [CompletionResultType]::ParameterName, 'Optional shell command to run before executing tests')
            [CompletionResult]::new('--teardown-hook', '--teardown-hook', [CompletionResultType]::ParameterName, 'Optional shell command to run after executing tests')
            [CompletionResult]::new('--mock-config', '--mock-config', [CompletionResultType]::ParameterName, 'Optional JSON or YAML file describing mock services used in the run')
            [CompletionResult]::new('--report', '--report', [CompletionResultType]::ParameterName, 'Optional JSON report output for the full test session')
            [CompletionResult]::new('--profile-output', '--profile-output', [CompletionResultType]::ParameterName, 'Optional JSON profile output for load-test metadata')
            [CompletionResult]::new('--load-iterations', '--load-iterations', [CompletionResultType]::ParameterName, 'Number of iterations to simulate for load testing')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--coverage', '--coverage', [CompletionResultType]::ParameterName, 'Show coverage report')
            [CompletionResult]::new('--require-coverage', '--require-coverage', [CompletionResultType]::ParameterName, 'Require coverage data and fail if unavailable')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;audit' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: text, json, markdown')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Optional report output file')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Optional report output file')
            [CompletionResult]::new('--fail-on', '--fail-on', [CompletionResultType]::ParameterName, 'Fail the command when findings at or above this severity are present')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;sla' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('record', 'record', [CompletionResultType]::ParameterValue, 'Record hourly SLA metrics for a contract')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show real-time SLA compliance dashboard')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;sla;record' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;sla;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;sla;help' {
            [CompletionResult]::new('record', 'record', [CompletionResultType]::ParameterValue, 'Record hourly SLA metrics for a contract')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show real-time SLA compliance dashboard')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;sla;help;record' {
            break
        }
        'soroban-registry;sla;help;status' {
            break
        }
        'soroban-registry;sla;help;help' {
            break
        }
        'soroban-registry;config' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a user config value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a user config value by key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all persisted user config values')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'Reset user config to defaults')
            [CompletionResult]::new('contract-get', 'contract-get', [CompletionResultType]::ParameterValue, 'Get contract environment configuration')
            [CompletionResult]::new('contract-set', 'contract-set', [CompletionResultType]::ParameterValue, 'Set contract environment configuration')
            [CompletionResult]::new('contract-history', 'contract-history', [CompletionResultType]::ParameterValue, 'Show contract config history')
            [CompletionResult]::new('contract-rollback', 'contract-rollback', [CompletionResultType]::ParameterValue, 'Roll back contract config to a previous version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;config;get' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;set' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;reset' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;contract-get' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--environment', '--environment', [CompletionResultType]::ParameterName, 'environment')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;contract-set' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--environment', '--environment', [CompletionResultType]::ParameterName, 'environment')
            [CompletionResult]::new('--config-data', '--config-data', [CompletionResultType]::ParameterName, 'config-data')
            [CompletionResult]::new('--secrets-data', '--secrets-data', [CompletionResultType]::ParameterName, 'secrets-data')
            [CompletionResult]::new('--created-by', '--created-by', [CompletionResultType]::ParameterName, 'created-by')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;contract-history' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--environment', '--environment', [CompletionResultType]::ParameterName, 'environment')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;contract-rollback' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'contract-id')
            [CompletionResult]::new('--environment', '--environment', [CompletionResultType]::ParameterName, 'environment')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'version')
            [CompletionResult]::new('--created-by', '--created-by', [CompletionResultType]::ParameterName, 'created-by')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;config;help' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a user config value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a user config value by key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all persisted user config values')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'Reset user config to defaults')
            [CompletionResult]::new('contract-get', 'contract-get', [CompletionResultType]::ParameterValue, 'Get contract environment configuration')
            [CompletionResult]::new('contract-set', 'contract-set', [CompletionResultType]::ParameterValue, 'Set contract environment configuration')
            [CompletionResult]::new('contract-history', 'contract-history', [CompletionResultType]::ParameterValue, 'Show contract config history')
            [CompletionResult]::new('contract-rollback', 'contract-rollback', [CompletionResultType]::ParameterValue, 'Roll back contract config to a previous version')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;config;help;get' {
            break
        }
        'soroban-registry;config;help;set' {
            break
        }
        'soroban-registry;config;help;list' {
            break
        }
        'soroban-registry;config;help;reset' {
            break
        }
        'soroban-registry;config;help;contract-get' {
            break
        }
        'soroban-registry;config;help;contract-set' {
            break
        }
        'soroban-registry;config;help;contract-history' {
            break
        }
        'soroban-registry;config;help;contract-rollback' {
            break
        }
        'soroban-registry;config;help;help' {
            break
        }
        'soroban-registry;auth' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with a GitHub account, Stellar wallet, or API key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Sign out and remove stored credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the current authentication state')
            [CompletionResult]::new('token', 'token', [CompletionResultType]::ParameterValue, 'Print the current API token, refreshing it when possible')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;auth;login' {
            [CompletionResult]::new('--method', '--method', [CompletionResultType]::ParameterName, 'Authentication method to use')
            [CompletionResult]::new('--identity', '--identity', [CompletionResultType]::ParameterName, 'Identity to authenticate with')
            [CompletionResult]::new('--secret', '--secret', [CompletionResultType]::ParameterName, 'Secret credential or signing seed')
            [CompletionResult]::new('--scopes', '--scopes', [CompletionResultType]::ParameterName, 'Comma-separated token scopes')
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'Token lifetime, e.g. 1h, 30m, 7d, or seconds')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;auth;logout' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;auth;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;auth;token' {
            [CompletionResult]::new('--scopes', '--scopes', [CompletionResultType]::ParameterName, 'Comma-separated token scopes')
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'Token lifetime, e.g. 1h, 30m, 7d, or seconds')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;auth;help' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with a GitHub account, Stellar wallet, or API key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Sign out and remove stored credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the current authentication state')
            [CompletionResult]::new('token', 'token', [CompletionResultType]::ParameterValue, 'Print the current API token, refreshing it when possible')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;auth;help;login' {
            break
        }
        'soroban-registry;auth;help;logout' {
            break
        }
        'soroban-registry;auth;help;status' {
            break
        }
        'soroban-registry;auth;help;token' {
            break
        }
        'soroban-registry;auth;help;help' {
            break
        }
        'soroban-registry;backup' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new contract backup')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List recent backups for a contract')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a contract from a specific backup date')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify integrity of a specific backup')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show backup statistics for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;backup;create' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--include-state', '--include-state', [CompletionResultType]::ParameterName, 'Include full contract state in backup')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;backup;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;backup;restore' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;backup;verify' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;backup;stats' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;backup;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new contract backup')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List recent backups for a contract')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a contract from a specific backup date')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify integrity of a specific backup')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show backup statistics for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;backup;help;create' {
            break
        }
        'soroban-registry;backup;help;list' {
            break
        }
        'soroban-registry;backup;help;restore' {
            break
        }
        'soroban-registry;backup;help;verify' {
            break
        }
        'soroban-registry;backup;help;stats' {
            break
        }
        'soroban-registry;backup;help;help' {
            break
        }
        'soroban-registry;state' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single state value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a state key/value (testnet and futurenet only)')
            [CompletionResult]::new('dump', 'dump', [CompletionResultType]::ParameterValue, 'Dump full contract state')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Create a state snapshot')
            [CompletionResult]::new('snapshots', 'snapshots', [CompletionResultType]::ParameterValue, 'List saved state snapshots')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Browse state change history')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;state;get' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;set' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;dump' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;snapshot' {
            [CompletionResult]::new('--label', '--label', [CompletionResultType]::ParameterName, 'Optional label for the snapshot')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;snapshots' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of snapshots to return')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;history' {
            [CompletionResult]::new('--key', '--key', [CompletionResultType]::ParameterName, 'Filter by key')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of entries to return')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;state;help' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single state value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a state key/value (testnet and futurenet only)')
            [CompletionResult]::new('dump', 'dump', [CompletionResultType]::ParameterValue, 'Dump full contract state')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Create a state snapshot')
            [CompletionResult]::new('snapshots', 'snapshots', [CompletionResultType]::ParameterValue, 'List saved state snapshots')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Browse state change history')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;state;help;get' {
            break
        }
        'soroban-registry;state;help;set' {
            break
        }
        'soroban-registry;state;help;dump' {
            break
        }
        'soroban-registry;state;help;snapshot' {
            break
        }
        'soroban-registry;state;help;snapshots' {
            break
        }
        'soroban-registry;state;help;history' {
            break
        }
        'soroban-registry;state;help;help' {
            break
        }
        'soroban-registry;verify-formal' {
            [CompletionResult]::new('--properties', '--properties', [CompletionResultType]::ParameterName, 'Path to properties DSL file')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output format (json or text)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--post', '--post', [CompletionResultType]::ParameterName, 'Post results back to registry')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;scan-deps' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract address or registry UUID to scan')
            [CompletionResult]::new('--dependencies', '--dependencies', [CompletionResultType]::ParameterName, 'Comma-separated dependency list to scan')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--fail-on-high', '--fail-on-high', [CompletionResultType]::ParameterName, 'Exit non-zero when a high-severity finding is reported')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;coverage' {
            [CompletionResult]::new('--tests', '--tests', [CompletionResultType]::ParameterName, 'Path to test directory or file')
            [CompletionResult]::new('--threshold', '--threshold', [CompletionResultType]::ParameterName, 'Fail if coverage is below this threshold (0-100)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output directory for HTML reports')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;sign' {
            [CompletionResult]::new('--private-key', '--private-key', [CompletionResultType]::ParameterName, 'Private key (base64-encoded Ed25519)')
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract ID')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Package version')
            [CompletionResult]::new('--expires-at', '--expires-at', [CompletionResultType]::ParameterName, 'Signature expiration (RFC3339 format)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;verify-package' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract ID')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Package version (optional)')
            [CompletionResult]::new('--signature', '--signature', [CompletionResultType]::ParameterName, 'Signature (base64, optional - will lookup from registry if not provided)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;verify' {
            [CompletionResult]::new('--level', '--level', [CompletionResultType]::ParameterName, 'Verification level: basic, intermediate, advanced')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'Path to contract project directory (defaults to current dir)')
            [CompletionResult]::new('--notes', '--notes', [CompletionResultType]::ParameterName, 'Optional notes for submission')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Submit for verification (requires id or local project)')
            [CompletionResult]::new('--submit', '--submit', [CompletionResultType]::ParameterName, 'Submit for verification (requires id or local project)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Check current verification status')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Check current verification status')
            [CompletionResult]::new('--history', '--history', [CompletionResultType]::ParameterName, 'Show verification history')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;verify-contract' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract ID used when signing')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Contract version used when signing')
            [CompletionResult]::new('--signature', '--signature', [CompletionResultType]::ParameterName, 'Ed25519 signature (base64)')
            [CompletionResult]::new('--public-key', '--public-key', [CompletionResultType]::ParameterName, 'Ed25519 public key (base64)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;keys' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a new Ed25519 keypair for signing')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a signature')
            [CompletionResult]::new('custody', 'custody', [CompletionResultType]::ParameterValue, 'Show chain of custody for a contract')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'View transparency log')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;keys;generate' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;keys;revoke' {
            [CompletionResult]::new('--revoked-by', '--revoked-by', [CompletionResultType]::ParameterName, 'Address of the revoker')
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Reason for revocation')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;keys;custody' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;keys;log' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Filter by contract ID')
            [CompletionResult]::new('--entry-type', '--entry-type', [CompletionResultType]::ParameterName, 'Filter by entry type')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum entries to show')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;keys;help' {
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a new Ed25519 keypair for signing')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a signature')
            [CompletionResult]::new('custody', 'custody', [CompletionResultType]::ParameterValue, 'Show chain of custody for a contract')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'View transparency log')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;keys;help;generate' {
            break
        }
        'soroban-registry;keys;help;revoke' {
            break
        }
        'soroban-registry;keys;help;custody' {
            break
        }
        'soroban-registry;keys;help;log' {
            break
        }
        'soroban-registry;keys;help;help' {
            break
        }
        'soroban-registry;policy' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run a policy-as-code admission check against a WASM artifact')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;policy;check' {
            [CompletionResult]::new('--wasm-path', '--wasm-path', [CompletionResultType]::ParameterName, 'Path to the local WASM artifact to evaluate')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Path to the policy YAML/JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--explain', '--explain', [CompletionResultType]::ParameterName, 'Show detailed rule-by-rule evaluation')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Evaluate without submitting the artifact')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;policy;help' {
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run a policy-as-code admission check against a WASM artifact')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;policy;help;check' {
            break
        }
        'soroban-registry;policy;help;help' {
            break
        }
        'soroban-registry;publisher' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose the local publishing environment (config, session, signing key, connectivity)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;publisher;doctor' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;publisher;help' {
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose the local publishing environment (config, session, signing key, connectivity)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;publisher;help;doctor' {
            break
        }
        'soroban-registry;publisher;help;help' {
            break
        }
        'soroban-registry;contract' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List registered contracts, a page at a time')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search the registry, one page at a time or across every page')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Export a signed, offline-verifiable snapshot of a contract (#1116)')
            [CompletionResult]::new('verify-snapshot', 'verify-snapshot', [CompletionResultType]::ParameterValue, 'Verify a previously exported contract snapshot (#1116)')
            [CompletionResult]::new('risk', 'risk', [CompletionResultType]::ParameterValue, 'Assess security and operational risks for a contract (#837)')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Deploy and register a new contract in the registry')
            [CompletionResult]::new('register', 'register', [CompletionResultType]::ParameterValue, 'Register one or more contracts in the registry')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a contract — a local WASM artifact before publishing, or a deployed contract''s authenticity against the on-chain registry')
            [CompletionResult]::new('interfaces', 'interfaces', [CompletionResultType]::ParameterValue, 'Derive and display a contract''s deterministic interface fingerprint (functions, types, events, errors) from a local compiled WASM artifact')
            [CompletionResult]::new('provenance', 'provenance', [CompletionResultType]::ParameterValue, 'Display build-provenance metadata recorded for a contract, read from a local manifest file')
            [CompletionResult]::new('verify-build', 'verify-build', [CompletionResultType]::ParameterValue, 'Attempt to independently reproduce a contract''s published WASM artifact from source, and compare its hash against the expected (registry-recorded) artifact hash')
            [CompletionResult]::new('compatibility', 'compatibility', [CompletionResultType]::ParameterValue, 'Structurally compare two local compiled WASM artifacts and classify ABI changes as compatible, potentially breaking, breaking, or unknown')
            [CompletionResult]::new('details', 'details', [CompletionResultType]::ParameterValue, 'Display detailed information about a contract')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show contract registry statistics and analytics')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export contracts and related registry data for backup or migration')
            [CompletionResult]::new('highlight', 'highlight', [CompletionResultType]::ParameterValue, 'Manage featured (highlighted) contracts (#832)')
            [CompletionResult]::new('interaction', 'interaction', [CompletionResultType]::ParameterValue, 'View a contract''s interactions and call patterns (#835)')
            [CompletionResult]::new('dependency', 'dependency', [CompletionResultType]::ParameterValue, 'Analyze a contract''s dependencies and relationships (#836, #1008)')
            [CompletionResult]::new('dependencies', 'dependencies', [CompletionResultType]::ParameterValue, 'List what a contract depends on (#1147)')
            [CompletionResult]::new('dependents', 'dependents', [CompletionResultType]::ParameterValue, 'List what depends on a contract (#1147)')
            [CompletionResult]::new('dependency-risk', 'dependency-risk', [CompletionResultType]::ParameterValue, 'Report direct and inherited risk across a contract''s dependencies (#1147)')
            [CompletionResult]::new('category', 'category', [CompletionResultType]::ParameterValue, 'List and inspect contract categories')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update contract metadata after registration (#828)')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import contracts into the registry from an external file (#831)')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a deprecated contract to active state (#1091)')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Detect drift between local lockfile and registry state (#1060)')
            [CompletionResult]::new('deprecate', 'deprecate', [CompletionResultType]::ParameterValue, 'Deprecate a contract with publisher-signed authorization (#1091)')
            [CompletionResult]::new('notification', 'notification', [CompletionResultType]::ParameterValue, 'Manage contract event notifications and alerts (#838)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;list' {
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Contracts per page (1-100)')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Contracts per page (1-100)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Contracts to skip; use it with --limit to page through the registry')
            [CompletionResult]::new('--offset', '--offset', [CompletionResultType]::ParameterName, 'Contracts to skip; use it with --limit to page through the registry')
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Filter by network (comma-separated: mainnet,testnet,futurenet). Named `networks` because the global `--network` owns that arg id')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated: DeFi,NFT)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated: DeFi,NFT)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format: table, json or csv')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table, json or csv')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;search' {
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Filter by network (comma-separated: mainnet,testnet,futurenet)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category (comma-separated: DeFi,NFT)')
            [CompletionResult]::new('--tags', '--tags', [CompletionResultType]::ParameterName, 'Filter by tag (comma-separated: defi,amm)')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Results per page (1-100)')
            [CompletionResult]::new('--offset', '--offset', [CompletionResultType]::ParameterName, 'Start at this row offset. Offset pagination only — cannot be combined with --cursor')
            [CompletionResult]::new('--cursor', '--cursor', [CompletionResultType]::ParameterName, 'Resume from a continuation token returned by a previous run. Cursor pagination only — the token is opaque and must not be edited')
            [CompletionResult]::new('--pagination', '--pagination', [CompletionResultType]::ParameterName, 'Pagination mode: cursor (stable, no skips or duplicates) or offset (relevance ordered). Defaults to cursor for --all, offset otherwise')
            [CompletionResult]::new('--max-items', '--max-items', [CompletionResultType]::ParameterName, 'Maximum items to fetch with --all (default 1000)')
            [CompletionResult]::new('--max-pages', '--max-pages', [CompletionResultType]::ParameterName, 'Maximum pages to fetch with --all (default 100)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--verified-only', '--verified-only', [CompletionResultType]::ParameterName, 'Only show verified contracts')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Fetch every page, up to --max-items / --max-pages')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON, including pagination metadata')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;snapshot' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'File to write the signed snapshot to')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'File to write the signed snapshot to')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;verify-snapshot' {
            [CompletionResult]::new('--expect-key', '--expect-key', [CompletionResultType]::ParameterName, 'Registry key fingerprint to pin against. Without it, a valid result proves only that the bundle is self-consistent')
            [CompletionResult]::new('--max-age-days', '--max-age-days', [CompletionResultType]::ParameterName, 'Fail if the snapshot is older than this many days')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--fetch-key', '--fetch-key', [CompletionResultType]::ParameterName, 'Fetch the expected fingerprint from the registry instead of pinning it locally. Requires network access')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;risk' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--threshold', '--threshold', [CompletionResultType]::ParameterName, 'Exit with code 1 if overall risk level meets or exceeds this threshold (low | medium | high | critical)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output the risk report as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;deploy' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Contract name (human-readable)')
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'Contract description')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Contract category (DeFi, Token, Oracle, NFT, Utility, Other)')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--icon', '--icon', [CompletionResultType]::ParameterName, 'Path to contract icon file (PNG, JPG, SVG)')
            [CompletionResult]::new('--publisher', '--publisher', [CompletionResultType]::ParameterName, 'Publisher''s Stellar address (if not set, uses default publisher)')
            [CompletionResult]::new('--tags', '--tags', [CompletionResultType]::ParameterName, 'Comma-separated list of tags for the contract')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--interactive', '--interactive', [CompletionResultType]::ParameterName, 'Enable interactive mode for guided deployment')
            [CompletionResult]::new('--skip-abi', '--skip-abi', [CompletionResultType]::ParameterName, 'Skip ABI extraction and deployment verification')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;register' {
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Path to a YAML or JSON metadata file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--batch', '--batch', [CompletionResultType]::ParameterName, 'Enable repeated prompts for multiple contracts')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;verify' {
            [CompletionResult]::new('--wasm', '--wasm', [CompletionResultType]::ParameterName, 'Path to a local compiled WASM contract to verify before publishing. Runs the same structural checks the backend uses, offline. In local mode, pass the global -v/--verbose flag for detailed diagnostics')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Strict mode: fail if any warnings or errors are found')
            [CompletionResult]::new('--batch', '--batch', [CompletionResultType]::ParameterName, 'Batch mode: verify multiple contracts (comma-separated addresses)')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip cache and always fetch fresh data from registry')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;interfaces' {
            [CompletionResult]::new('--wasm', '--wasm', [CompletionResultType]::ParameterName, 'Path to a local compiled WASM contract to inspect')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;provenance' {
            [CompletionResult]::new('--manifest', '--manifest', [CompletionResultType]::ParameterName, 'Path to a local provenance manifest (JSON) to display')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;verify-build' {
            [CompletionResult]::new('--manifest', '--manifest', [CompletionResultType]::ParameterName, 'Path to a local provenance manifest (JSON) describing the recorded build')
            [CompletionResult]::new('--source-dir', '--source-dir', [CompletionResultType]::ParameterName, 'Directory containing the contract''s source to rebuild')
            [CompletionResult]::new('--expected-hash', '--expected-hash', [CompletionResultType]::ParameterName, 'The registry-recorded WASM artifact hash to compare the rebuild against')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--allow-toolchain-mismatch', '--allow-toolchain-mismatch', [CompletionResultType]::ParameterName, 'Proceed with the rebuild even if the locally installed rustc version doesn''t match the version recorded in provenance')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;compatibility' {
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Path to the earlier/baseline compiled WASM contract')
            [CompletionResult]::new('--to', '--to', [CompletionResultType]::ParameterName, 'Path to the newer/candidate compiled WASM contract')
            [CompletionResult]::new('--from-network-passphrase', '--from-network-passphrase', [CompletionResultType]::ParameterName, 'Network passphrase associated with the `--from` artifact')
            [CompletionResult]::new('--to-network-passphrase', '--to-network-passphrase', [CompletionResultType]::ParameterName, 'Network passphrase associated with the `--to` artifact')
            [CompletionResult]::new('--fail-on', '--fail-on', [CompletionResultType]::ParameterName, 'Minimum severity that triggers a non-zero exit under --strict: breaking | potential | unknown')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Exit non-zero when changes at or above the --fail-on threshold are found (default threshold: potentially_breaking)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;details' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;stats' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Filter stats by network')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter stats by category')
            [CompletionResult]::new('--top-n', '--top-n', [CompletionResultType]::ParameterName, 'Number of popular contracts to display')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table, json, csv, yaml')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Export stats to a file')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Export stats to a file')
            [CompletionResult]::new('--compare', '--compare', [CompletionResultType]::ParameterName, 'Compare against another period, for example 7d or 30d')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;export' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Export format: json, csv, jsonl, sqlite, markdown, or archive')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Export format: json, csv, jsonl, sqlite, markdown, or archive')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Filter by network')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category')
            [CompletionResult]::new('--since', '--since', [CompletionResultType]::ParameterName, 'Export only contracts updated since this date')
            [CompletionResult]::new('--page-size', '--page-size', [CompletionResultType]::ParameterName, 'Number of contracts to fetch per API page')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--compress', '--compress', [CompletionResultType]::ParameterName, 'Write a gzip-compressed export file')
            [CompletionResult]::new('--include-related', '--include-related', [CompletionResultType]::ParameterName, 'Include related data such as versions, dependencies, analytics, and reviews')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;highlight' {
            [CompletionResult]::new('--action', '--action', [CompletionResultType]::ParameterName, 'Action to perform: add | remove | list | check')
            [CompletionResult]::new('--token', '--token', [CompletionResultType]::ParameterName, 'Curator bearer token for mutating actions (add/remove)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;interaction' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Max number of recent interactions to display')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;dependency' {
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Dependency tree depth (0 = direct dependencies only)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: table, json, csv, yaml')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--summary', '--summary', [CompletionResultType]::ParameterName, 'Compact summary mode: show aggregate counts without the full tree')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;dependencies' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Maximum traversal depth (capped server-side)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--transitive', '--transitive', [CompletionResultType]::ParameterName, 'Walk the whole dependency closure, not just direct edges')
            [CompletionResult]::new('--include-telemetry', '--include-telemetry', [CompletionResultType]::ParameterName, 'Include on-chain call edges alongside declared ones')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Print the registry response as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;dependents' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Maximum traversal depth (capped server-side)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--transitive', '--transitive', [CompletionResultType]::ParameterName, 'Walk the whole dependent closure, not just direct edges')
            [CompletionResult]::new('--include-telemetry', '--include-telemetry', [CompletionResultType]::ParameterName, 'Include on-chain call edges alongside declared ones')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Print the registry response as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;dependency-risk' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Maximum traversal depth (capped server-side)')
            [CompletionResult]::new('--fail-on', '--fail-on', [CompletionResultType]::ParameterName, 'Exit 1 when overall risk meets or exceeds this level')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Print the registry response as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;category' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all categories with descriptions and contract counts')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show detailed per-category statistics (counts, recent, trending)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;category;list' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Scope contract counts to a single network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format for stdout: table, json, csv, yaml')
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Also write the category list to a file: csv or json')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;category;stats' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Scope statistics to a single network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format for stdout: table, json, csv, yaml')
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Also write the statistics to a file: csv or json')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;category;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all categories with descriptions and contract counts')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show detailed per-category statistics (counts, recent, trending)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;category;help;list' {
            break
        }
        'soroban-registry;contract;category;help;stats' {
            break
        }
        'soroban-registry;contract;category;help;help' {
            break
        }
        'soroban-registry;contract;update' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Updated contract name')
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'Updated description')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Updated category')
            [CompletionResult]::new('--tags', '--tags', [CompletionResultType]::ParameterName, 'Comma-separated tags')
            [CompletionResult]::new('--icon', '--icon', [CompletionResultType]::ParameterName, 'Path to a new icon image (PNG, JPG, or SVG)')
            [CompletionResult]::new('--homepage', '--homepage', [CompletionResultType]::ParameterName, 'Contract homepage URL (not yet supported by registry API)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview changes without submitting them')
            [CompletionResult]::new('-y', '-y', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;import' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Input format override (json | jsonl | csv | sqlite | archive). Inferred from the file extension when omitted')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Input format override (json | jsonl | csv | sqlite | archive). Inferred from the file extension when omitted')
            [CompletionResult]::new('--on-duplicate', '--on-duplicate', [CompletionResultType]::ParameterName, 'How to handle duplicate contracts: skip | update | fail (default: skip)')
            [CompletionResult]::new('--network-map', '--network-map', [CompletionResultType]::ParameterName, 'Network alias mappings, e.g. --network-map futurenet=testnet May be repeated for multiple aliases')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Write the JSON import-summary report to this file path (prints to stdout when omitted)')
            [CompletionResult]::new('--report-output', '--report-output', [CompletionResultType]::ParameterName, 'Write the JSON import-summary report to this file path (prints to stdout when omitted)')
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Directory for archive extraction (archive format only)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview what would be imported without writing to the registry')
            [CompletionResult]::new('--validate', '--validate', [CompletionResultType]::ParameterName, 'Validate all records before importing; abort on any error')
            [CompletionResult]::new('--atomic', '--atomic', [CompletionResultType]::ParameterName, 'Roll back all successful imports if any record fails')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;rollback' {
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Human-readable reason for rollback')
            [CompletionResult]::new('--private-key', '--private-key', [CompletionResultType]::ParameterName, 'Publisher''s Ed25519 private key (base64-encoded)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('-y', '-y', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;audit' {
            [CompletionResult]::new('--lockfile', '--lockfile', [CompletionResultType]::ParameterName, 'Path to lockfile (default: soroban-registry.lock.json)')
            [CompletionResult]::new('--contracts', '--contracts', [CompletionResultType]::ParameterName, 'Contract IDs for --init (comma-separated)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: text, json')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Auto-sync lockfile to match current registry state')
            [CompletionResult]::new('--init', '--init', [CompletionResultType]::ParameterName, 'Generate an initial lockfile from the given contract IDs')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;deprecate' {
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Human-readable reason for deprecation')
            [CompletionResult]::new('--replacement', '--replacement', [CompletionResultType]::ParameterName, 'Replacement contract ID for downstream migration')
            [CompletionResult]::new('--private-key', '--private-key', [CompletionResultType]::ParameterName, 'Publisher''s Ed25519 private key (base64-encoded)')
            [CompletionResult]::new('--migration-guide', '--migration-guide', [CompletionResultType]::ParameterName, 'URL to a migration guide for consumers')
            [CompletionResult]::new('--grace-period-days', '--grace-period-days', [CompletionResultType]::ParameterName, 'Grace period in days before hard removal (default: 90)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('-y', '-y', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Skip interactive confirmation')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to alerts for a contract address')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from alerts for a contract address')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List active notification rules')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Update an existing notification rule')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test alert for a subscribed contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;notification;subscribe' {
            [CompletionResult]::new('--alerts', '--alerts', [CompletionResultType]::ParameterName, 'Alert types (comma-separated): updates, audits, security, deployments')
            [CompletionResult]::new('--channels', '--channels', [CompletionResultType]::ParameterName, 'Notification channels (comma-separated): email, webhook, cli')
            [CompletionResult]::new('--frequency', '--frequency', [CompletionResultType]::ParameterName, 'Notification frequency: instant, daily, weekly')
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Filter by networks (comma-separated, e.g. mainnet,testnet)')
            [CompletionResult]::new('--categories', '--categories', [CompletionResultType]::ParameterName, 'Filter by categories (comma-separated, e.g. defi,token)')
            [CompletionResult]::new('--target', '--target', [CompletionResultType]::ParameterName, 'Email address or webhook URL for the chosen channel')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification;unsubscribe' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification;configure' {
            [CompletionResult]::new('--alerts', '--alerts', [CompletionResultType]::ParameterName, 'New alert types (comma-separated)')
            [CompletionResult]::new('--channels', '--channels', [CompletionResultType]::ParameterName, 'New channels (comma-separated)')
            [CompletionResult]::new('--frequency', '--frequency', [CompletionResultType]::ParameterName, 'New frequency: instant, daily, weekly')
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'New network filter (comma-separated)')
            [CompletionResult]::new('--categories', '--categories', [CompletionResultType]::ParameterName, 'New category filter (comma-separated)')
            [CompletionResult]::new('--target', '--target', [CompletionResultType]::ParameterName, 'New email address or webhook URL')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification;test' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;contract;notification;help' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to alerts for a contract address')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from alerts for a contract address')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List active notification rules')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Update an existing notification rule')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test alert for a subscribed contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;notification;help;subscribe' {
            break
        }
        'soroban-registry;contract;notification;help;unsubscribe' {
            break
        }
        'soroban-registry;contract;notification;help;list' {
            break
        }
        'soroban-registry;contract;notification;help;configure' {
            break
        }
        'soroban-registry;contract;notification;help;test' {
            break
        }
        'soroban-registry;contract;notification;help;help' {
            break
        }
        'soroban-registry;contract;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List registered contracts, a page at a time')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search the registry, one page at a time or across every page')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Export a signed, offline-verifiable snapshot of a contract (#1116)')
            [CompletionResult]::new('verify-snapshot', 'verify-snapshot', [CompletionResultType]::ParameterValue, 'Verify a previously exported contract snapshot (#1116)')
            [CompletionResult]::new('risk', 'risk', [CompletionResultType]::ParameterValue, 'Assess security and operational risks for a contract (#837)')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Deploy and register a new contract in the registry')
            [CompletionResult]::new('register', 'register', [CompletionResultType]::ParameterValue, 'Register one or more contracts in the registry')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a contract — a local WASM artifact before publishing, or a deployed contract''s authenticity against the on-chain registry')
            [CompletionResult]::new('interfaces', 'interfaces', [CompletionResultType]::ParameterValue, 'Derive and display a contract''s deterministic interface fingerprint (functions, types, events, errors) from a local compiled WASM artifact')
            [CompletionResult]::new('provenance', 'provenance', [CompletionResultType]::ParameterValue, 'Display build-provenance metadata recorded for a contract, read from a local manifest file')
            [CompletionResult]::new('verify-build', 'verify-build', [CompletionResultType]::ParameterValue, 'Attempt to independently reproduce a contract''s published WASM artifact from source, and compare its hash against the expected (registry-recorded) artifact hash')
            [CompletionResult]::new('compatibility', 'compatibility', [CompletionResultType]::ParameterValue, 'Structurally compare two local compiled WASM artifacts and classify ABI changes as compatible, potentially breaking, breaking, or unknown')
            [CompletionResult]::new('details', 'details', [CompletionResultType]::ParameterValue, 'Display detailed information about a contract')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show contract registry statistics and analytics')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export contracts and related registry data for backup or migration')
            [CompletionResult]::new('highlight', 'highlight', [CompletionResultType]::ParameterValue, 'Manage featured (highlighted) contracts (#832)')
            [CompletionResult]::new('interaction', 'interaction', [CompletionResultType]::ParameterValue, 'View a contract''s interactions and call patterns (#835)')
            [CompletionResult]::new('dependency', 'dependency', [CompletionResultType]::ParameterValue, 'Analyze a contract''s dependencies and relationships (#836, #1008)')
            [CompletionResult]::new('dependencies', 'dependencies', [CompletionResultType]::ParameterValue, 'List what a contract depends on (#1147)')
            [CompletionResult]::new('dependents', 'dependents', [CompletionResultType]::ParameterValue, 'List what depends on a contract (#1147)')
            [CompletionResult]::new('dependency-risk', 'dependency-risk', [CompletionResultType]::ParameterValue, 'Report direct and inherited risk across a contract''s dependencies (#1147)')
            [CompletionResult]::new('category', 'category', [CompletionResultType]::ParameterValue, 'List and inspect contract categories')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update contract metadata after registration (#828)')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import contracts into the registry from an external file (#831)')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a deprecated contract to active state (#1091)')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Detect drift between local lockfile and registry state (#1060)')
            [CompletionResult]::new('deprecate', 'deprecate', [CompletionResultType]::ParameterValue, 'Deprecate a contract with publisher-signed authorization (#1091)')
            [CompletionResult]::new('notification', 'notification', [CompletionResultType]::ParameterValue, 'Manage contract event notifications and alerts (#838)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;contract;help;list' {
            break
        }
        'soroban-registry;contract;help;search' {
            break
        }
        'soroban-registry;contract;help;snapshot' {
            break
        }
        'soroban-registry;contract;help;verify-snapshot' {
            break
        }
        'soroban-registry;contract;help;risk' {
            break
        }
        'soroban-registry;contract;help;deploy' {
            break
        }
        'soroban-registry;contract;help;register' {
            break
        }
        'soroban-registry;contract;help;verify' {
            break
        }
        'soroban-registry;contract;help;interfaces' {
            break
        }
        'soroban-registry;contract;help;provenance' {
            break
        }
        'soroban-registry;contract;help;verify-build' {
            break
        }
        'soroban-registry;contract;help;compatibility' {
            break
        }
        'soroban-registry;contract;help;details' {
            break
        }
        'soroban-registry;contract;help;stats' {
            break
        }
        'soroban-registry;contract;help;export' {
            break
        }
        'soroban-registry;contract;help;highlight' {
            break
        }
        'soroban-registry;contract;help;interaction' {
            break
        }
        'soroban-registry;contract;help;dependency' {
            break
        }
        'soroban-registry;contract;help;dependencies' {
            break
        }
        'soroban-registry;contract;help;dependents' {
            break
        }
        'soroban-registry;contract;help;dependency-risk' {
            break
        }
        'soroban-registry;contract;help;category' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all categories with descriptions and contract counts')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show detailed per-category statistics (counts, recent, trending)')
            break
        }
        'soroban-registry;contract;help;category;list' {
            break
        }
        'soroban-registry;contract;help;category;stats' {
            break
        }
        'soroban-registry;contract;help;update' {
            break
        }
        'soroban-registry;contract;help;import' {
            break
        }
        'soroban-registry;contract;help;rollback' {
            break
        }
        'soroban-registry;contract;help;audit' {
            break
        }
        'soroban-registry;contract;help;deprecate' {
            break
        }
        'soroban-registry;contract;help;notification' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to alerts for a contract address')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from alerts for a contract address')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List active notification rules')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Update an existing notification rule')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test alert for a subscribed contract')
            break
        }
        'soroban-registry;contract;help;notification;subscribe' {
            break
        }
        'soroban-registry;contract;help;notification;unsubscribe' {
            break
        }
        'soroban-registry;contract;help;notification;list' {
            break
        }
        'soroban-registry;contract;help;notification;configure' {
            break
        }
        'soroban-registry;contract;help;notification;test' {
            break
        }
        'soroban-registry;contract;help;help' {
            break
        }
        'soroban-registry;api-key' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new API key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List your API keys')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete an API key')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke (disable) an API key without deleting its audit record')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;api-key;create' {
            [CompletionResult]::new('--expires', '--expires', [CompletionResultType]::ParameterName, 'Expiry (ISO date or duration, e.g. 2026-12-31 or 30d)')
            [CompletionResult]::new('--scopes', '--scopes', [CompletionResultType]::ParameterName, 'Comma-separated scopes / permissions')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;api-key;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;api-key;delete' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;api-key;revoke' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;api-key;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new API key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List your API keys')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete an API key')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke (disable) an API key without deleting its audit record')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;api-key;help;create' {
            break
        }
        'soroban-registry;api-key;help;list' {
            break
        }
        'soroban-registry;api-key;help;delete' {
            break
        }
        'soroban-registry;api-key;help;revoke' {
            break
        }
        'soroban-registry;api-key;help;help' {
            break
        }
        'soroban-registry;batch-verify' {
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Path to a contract list file (.txt one-ID-per-line, .json, or .yaml)')
            [CompletionResult]::new('--contracts', '--contracts', [CompletionResultType]::ParameterName, 'Comma-separated IDs — fallback when --file is absent')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Filter by network when discovering from API (mainnet|testnet|futurenet)')
            [CompletionResult]::new('--category', '--category', [CompletionResultType]::ParameterName, 'Filter by category when discovering from API (e.g. defi, nft)')
            [CompletionResult]::new('--age', '--age', [CompletionResultType]::ParameterName, 'Only include contracts created within this many days')
            [CompletionResult]::new('--initiated-by', '--initiated-by', [CompletionResultType]::ParameterName, 'Stellar address or username initiating the batch')
            [CompletionResult]::new('--level', '--level', [CompletionResultType]::ParameterName, 'Verification depth: basic | standard | strict')
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Export report to file; format inferred from extension (.json or .csv)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Save human-readable report to a text file')
            [CompletionResult]::new('--schedule', '--schedule', [CompletionResultType]::ParameterName, 'Save cron schedule and print crontab entry')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Register a new webhook subscription')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered webhooks')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a webhook by ID')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test event to a webhook')
            [CompletionResult]::new('logs', 'logs', [CompletionResultType]::ParameterValue, 'View delivery logs for a webhook')
            [CompletionResult]::new('retry', 'retry', [CompletionResultType]::ParameterValue, 'Manually retry a dead-letter delivery')
            [CompletionResult]::new('verify-sig', 'verify-sig', [CompletionResultType]::ParameterValue, 'Verify a webhook payload signature locally')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;webhook;create' {
            [CompletionResult]::new('--url', '--url', [CompletionResultType]::ParameterName, 'Endpoint URL to receive events (must be HTTPS in production)')
            [CompletionResult]::new('--events', '--events', [CompletionResultType]::ParameterName, 'Comma-separated list of events to subscribe to. Valid: contract.published, contract.verified, contract.failed_verification, version.created')
            [CompletionResult]::new('--secret', '--secret', [CompletionResultType]::ParameterName, 'Optional HMAC-SHA256 secret key (auto-generated if omitted)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;delete' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;test' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;logs' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of log entries to show')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;retry' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;verify-sig' {
            [CompletionResult]::new('--secret', '--secret', [CompletionResultType]::ParameterName, 'HMAC secret key used for signing')
            [CompletionResult]::new('--payload', '--payload', [CompletionResultType]::ParameterName, 'Raw JSON payload body')
            [CompletionResult]::new('--signature', '--signature', [CompletionResultType]::ParameterName, 'Signature header value (e.g. sha256=abc123...)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;webhook;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Register a new webhook subscription')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered webhooks')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a webhook by ID')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test event to a webhook')
            [CompletionResult]::new('logs', 'logs', [CompletionResultType]::ParameterValue, 'View delivery logs for a webhook')
            [CompletionResult]::new('retry', 'retry', [CompletionResultType]::ParameterValue, 'Manually retry a dead-letter delivery')
            [CompletionResult]::new('verify-sig', 'verify-sig', [CompletionResultType]::ParameterValue, 'Verify a webhook payload signature locally')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;webhook;help;create' {
            break
        }
        'soroban-registry;webhook;help;list' {
            break
        }
        'soroban-registry;webhook;help;delete' {
            break
        }
        'soroban-registry;webhook;help;test' {
            break
        }
        'soroban-registry;webhook;help;logs' {
            break
        }
        'soroban-registry;webhook;help;retry' {
            break
        }
        'soroban-registry;webhook;help;verify-sig' {
            break
        }
        'soroban-registry;webhook;help;help' {
            break
        }
        'soroban-registry;release-notes' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Auto-generate release notes from code diff and changelog')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'View generated release notes for a version')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit draft release notes before publishing')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish (finalize) release notes')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all release notes for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;release-notes;generate' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract registry ID (UUID or on-chain ID)')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Version to generate notes for (semver, e.g. 1.2.0)')
            [CompletionResult]::new('--previous-version', '--previous-version', [CompletionResultType]::ParameterName, 'Previous version to diff against (auto-detected if omitted)')
            [CompletionResult]::new('--changelog', '--changelog', [CompletionResultType]::ParameterName, 'Path to CHANGELOG.md file (auto-detected if present in cwd)')
            [CompletionResult]::new('--contract-address', '--contract-address', [CompletionResultType]::ParameterName, 'On-chain contract address to include in notes')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;release-notes;view' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract registry ID')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Version to view')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;release-notes;edit' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract registry ID')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Version to edit')
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Path to a file containing the new release notes text')
            [CompletionResult]::new('--text', '--text', [CompletionResultType]::ParameterName, 'Inline text for the release notes')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;release-notes;publish' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract registry ID')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Version to publish')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--skip-version-update', '--skip-version-update', [CompletionResultType]::ParameterName, 'Skip updating the contract_versions.release_notes column')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;release-notes;list' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'Contract registry ID')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;release-notes;help' {
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Auto-generate release notes from code diff and changelog')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'View generated release notes for a version')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit draft release notes before publishing')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish (finalize) release notes')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all release notes for a contract')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;release-notes;help;generate' {
            break
        }
        'soroban-registry;release-notes;help;view' {
            break
        }
        'soroban-registry;release-notes;help;edit' {
            break
        }
        'soroban-registry;release-notes;help;publish' {
            break
        }
        'soroban-registry;release-notes;help;list' {
            break
        }
        'soroban-registry;release-notes;help;help' {
            break
        }
        'soroban-registry;cicd' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a full CI/CD pipeline (validate, scan, build, publish, verify)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate the current environment for CI/CD integration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;cicd;run' {
            [CompletionResult]::new('--contract-path', '--contract-path', [CompletionResultType]::ParameterName, 'Path to contract directory')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Network to target (testnet|mainnet)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--skip-scan', '--skip-scan', [CompletionResultType]::ParameterName, 'Skip security scans')
            [CompletionResult]::new('--auto-register', '--auto-register', [CompletionResultType]::ParameterName, 'Auto-register contract if not found in registry')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cicd;validate' {
            [CompletionResult]::new('--contract-path', '--contract-path', [CompletionResultType]::ParameterName, 'Path to contract directory')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cicd;help' {
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a full CI/CD pipeline (validate, scan, build, publish, verify)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate the current environment for CI/CD integration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;cicd;help;run' {
            break
        }
        'soroban-registry;cicd;help;validate' {
            break
        }
        'soroban-registry;cicd;help;help' {
            break
        }
        'soroban-registry;network' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show status of all supported Stellar networks')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;network;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;network;help' {
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show status of all supported Stellar networks')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;network;help;status' {
            break
        }
        'soroban-registry;network;help;help' {
            break
        }
        'soroban-registry;batch-register' {
            [CompletionResult]::new('--manifest', '--manifest', [CompletionResultType]::ParameterName, 'Path to the manifest file (.yaml, .yml, or .json)')
            [CompletionResult]::new('--publisher', '--publisher', [CompletionResultType]::ParameterName, 'Publisher Stellar address (overrides `publisher` field in the manifest)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Validate all entries and show what would be registered without submitting')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;batch-audit' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Report format: text, json, markdown')
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Output directory for generated reports')
            [CompletionResult]::new('--fail-on', '--fail-on', [CompletionResultType]::ParameterName, 'Fail on findings at or above this severity')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Audit profile: basic, standard, comprehensive')
            [CompletionResult]::new('--export', '--export', [CompletionResultType]::ParameterName, 'Export audit findings to a file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--high-risk', '--high-risk', [CompletionResultType]::ParameterName, 'Show only high and critical findings')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;batch-deploy' {
            [CompletionResult]::new('--networks', '--networks', [CompletionResultType]::ParameterName, 'Comma-separated target networks (mainnet,testnet,futurenet)')
            [CompletionResult]::new('--signer', '--signer', [CompletionResultType]::ParameterName, 'Signer Stellar address or secret')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--atomic', '--atomic', [CompletionResultType]::ParameterName, 'Stop and report failure if any deployment fails (no on-chain rollback)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;batch-export' {
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'Filter query (e.g. network=testnet or category=defi)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: json, csv, archive')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--organize', '--organize', [CompletionResultType]::ParameterName, 'Organize output by network/category subdirectories')
            [CompletionResult]::new('--compress', '--compress', [CompletionResultType]::ParameterName, 'Compress the output directory into a .tar.gz')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;batch-import' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Force a specific format (json, csv, archive); auto-detected if omitted')
            [CompletionResult]::new('--on-duplicate', '--on-duplicate', [CompletionResultType]::ParameterName, 'How to handle duplicates: skip or fail')
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Output directory for archive imports')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview what would be imported without committing')
            [CompletionResult]::new('--atomic', '--atomic', [CompletionResultType]::ParameterName, 'Abort on first error; report atomically')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;batch-update' {
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Path to a YAML or JSON manifest file describing the updates')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'Filter contracts from the API (e.g. "category=defi" or "network=mainnet")')
            [CompletionResult]::new('--if', '--if', [CompletionResultType]::ParameterName, 'Only update contracts where this field=value condition is true')
            [CompletionResult]::new('--user-id', '--user-id', [CompletionResultType]::ParameterName, 'User ID to attribute the update to')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--preview', '--preview', [CompletionResultType]::ParameterName, 'Show what would change without making any writes')
            [CompletionResult]::new('--rollback-on-error', '--rollback-on-error', [CompletionResultType]::ParameterName, 'On partial failure, rollback all successfully applied contracts')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;analyze' {
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--report-format', '--report-format', [CompletionResultType]::ParameterName, 'Report format: text (default), json, yaml')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Write the report to a file instead of stdout')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Write the report to a file instead of stdout')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;track-deployment' {
            [CompletionResult]::new('--contract-id', '--contract-id', [CompletionResultType]::ParameterName, 'On-chain contract ID')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--tx-hash', '--tx-hash', [CompletionResultType]::ParameterName, 'Optional transaction hash to track (polls transaction endpoints first)')
            [CompletionResult]::new('--wait-timeout', '--wait-timeout', [CompletionResultType]::ParameterName, 'Maximum wait time in seconds before exiting with code 2')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON status')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed plugins and their commands')
            [CompletionResult]::new('marketplace', 'marketplace', [CompletionResultType]::ParameterValue, 'Browse the registry marketplace')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install a plugin from the registry')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'Uninstall an installed plugin')
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a plugin-provided command explicitly')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Enable/disable plugins and set per-plugin configuration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;plugins;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;marketplace' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;install' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Optional version (defaults to marketplace version)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;uninstall' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Optional version (defaults to removing all versions)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;run' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;config' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get the current JSON config for a plugin')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Replace the plugin JSON config (must be a JSON object)')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Disable a plugin (commands won''t be discovered)')
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Enable a plugin (default)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;plugins;config;get' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;config;set' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'JSON object')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;config;disable' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;config;enable' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;plugins;config;help' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get the current JSON config for a plugin')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Replace the plugin JSON config (must be a JSON object)')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Disable a plugin (commands won''t be discovered)')
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Enable a plugin (default)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;plugins;config;help;get' {
            break
        }
        'soroban-registry;plugins;config;help;set' {
            break
        }
        'soroban-registry;plugins;config;help;disable' {
            break
        }
        'soroban-registry;plugins;config;help;enable' {
            break
        }
        'soroban-registry;plugins;config;help;help' {
            break
        }
        'soroban-registry;plugins;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed plugins and their commands')
            [CompletionResult]::new('marketplace', 'marketplace', [CompletionResultType]::ParameterValue, 'Browse the registry marketplace')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install a plugin from the registry')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'Uninstall an installed plugin')
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a plugin-provided command explicitly')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Enable/disable plugins and set per-plugin configuration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;plugins;help;list' {
            break
        }
        'soroban-registry;plugins;help;marketplace' {
            break
        }
        'soroban-registry;plugins;help;install' {
            break
        }
        'soroban-registry;plugins;help;uninstall' {
            break
        }
        'soroban-registry;plugins;help;run' {
            break
        }
        'soroban-registry;plugins;help;config' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get the current JSON config for a plugin')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Replace the plugin JSON config (must be a JSON object)')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Disable a plugin (commands won''t be discovered)')
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Enable a plugin (default)')
            break
        }
        'soroban-registry;plugins;help;config;get' {
            break
        }
        'soroban-registry;plugins;help;config;set' {
            break
        }
        'soroban-registry;plugins;help;config;disable' {
            break
        }
        'soroban-registry;plugins;help;config;enable' {
            break
        }
        'soroban-registry;plugins;help;help' {
            break
        }
        'soroban-registry;cache' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear cached entries from disk')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache statistics and configuration')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Configure cache settings')
            [CompletionResult]::new('optimize', 'optimize', [CompletionResultType]::ParameterValue, 'Remove stale entries and enforce disk size limit')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export cache entries for analysis')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;cache;clear' {
            [CompletionResult]::new('--level', '--level', [CompletionResultType]::ParameterName, 'Cache level to clear: disk (default), memory, all')
            [CompletionResult]::new('--key', '--key', [CompletionResultType]::ParameterName, 'Clear only the entry matching this specific cache key')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cache;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cache;configure' {
            [CompletionResult]::new('--ttl', '--ttl', [CompletionResultType]::ParameterName, 'Default TTL for cached entries in seconds')
            [CompletionResult]::new('--max-size', '--max-size', [CompletionResultType]::ParameterName, 'Maximum disk cache size in bytes (0 = unlimited)')
            [CompletionResult]::new('--compression', '--compression', [CompletionResultType]::ParameterName, 'Enable or disable compression for disk entries: on | off')
            [CompletionResult]::new('--auto-refresh', '--auto-refresh', [CompletionResultType]::ParameterName, 'Enable or disable automatic refresh of stale entries: on | off')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output current (or updated) config as JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cache;optimize' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output results as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cache;export' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: json (default) or csv')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--include-stale', '--include-stale', [CompletionResultType]::ParameterName, 'Include stale (expired) entries in the export')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;cache;help' {
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear cached entries from disk')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache statistics and configuration')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Configure cache settings')
            [CompletionResult]::new('optimize', 'optimize', [CompletionResultType]::ParameterValue, 'Remove stale entries and enforce disk size limit')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export cache entries for analysis')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;cache;help;clear' {
            break
        }
        'soroban-registry;cache;help;status' {
            break
        }
        'soroban-registry;cache;help;configure' {
            break
        }
        'soroban-registry;cache;help;optimize' {
            break
        }
        'soroban-registry;cache;help;export' {
            break
        }
        'soroban-registry;cache;help;help' {
            break
        }
        'soroban-registry;env' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a variable in an environment')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a variable''s value from an environment')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List variables in an environment')
            [CompletionResult]::new('copy', 'copy', [CompletionResultType]::ParameterValue, 'Copy all variables from one environment to another')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a variable from an environment')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export environment variables as a shell-sourceable file')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active environment')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;env;set' {
            [CompletionResult]::new('--env', '--env', [CompletionResultType]::ParameterName, 'Target environment (defaults to the active environment)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--show-value', '--show-value', [CompletionResultType]::ParameterName, 'Print the full value instead of masking it')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;get' {
            [CompletionResult]::new('--env', '--env', [CompletionResultType]::ParameterName, 'Source environment (defaults to the active environment)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;list' {
            [CompletionResult]::new('--env', '--env', [CompletionResultType]::ParameterName, 'Environment to list (defaults to the active environment)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'List variables in every environment')
            [CompletionResult]::new('--merged', '--merged', [CompletionResultType]::ParameterName, 'Merge global config defaults into the output')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output as machine-readable JSON')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;copy' {
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Source environment name')
            [CompletionResult]::new('--to', '--to', [CompletionResultType]::ParameterName, 'Destination environment name')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--overwrite', '--overwrite', [CompletionResultType]::ParameterName, 'Overwrite the destination if it already exists')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;delete' {
            [CompletionResult]::new('--env', '--env', [CompletionResultType]::ParameterName, 'Source environment (defaults to the active environment)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;export' {
            [CompletionResult]::new('--env', '--env', [CompletionResultType]::ParameterName, 'Environment to export (defaults to the active environment)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format: shell (default), json, dotenv')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--merged', '--merged', [CompletionResultType]::ParameterName, 'Merge global config defaults into the export')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;switch' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;env;help' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a variable in an environment')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a variable''s value from an environment')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List variables in an environment')
            [CompletionResult]::new('copy', 'copy', [CompletionResultType]::ParameterValue, 'Copy all variables from one environment to another')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a variable from an environment')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export environment variables as a shell-sourceable file')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active environment')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;env;help;set' {
            break
        }
        'soroban-registry;env;help;get' {
            break
        }
        'soroban-registry;env;help;list' {
            break
        }
        'soroban-registry;env;help;copy' {
            break
        }
        'soroban-registry;env;help;delete' {
            break
        }
        'soroban-registry;env;help;export' {
            break
        }
        'soroban-registry;env;help;switch' {
            break
        }
        'soroban-registry;env;help;help' {
            break
        }
        'soroban-registry;snapshot' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export a signed offline registry snapshot')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a registry snapshot')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a signed registry snapshot locally')
            [CompletionResult]::new('inspect', 'inspect', [CompletionResultType]::ParameterValue, 'Inspect a registry snapshot metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;snapshot;export' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;snapshot;sign' {
            [CompletionResult]::new('--key', '--key', [CompletionResultType]::ParameterName, 'Path to the signing key (Ed25519 PEM or base64)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;snapshot;verify' {
            [CompletionResult]::new('--trust-key', '--trust-key', [CompletionResultType]::ParameterName, 'Path to the trusted public key')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;snapshot;inspect' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Registry API URL')
            [CompletionResult]::new('--network', '--network', [CompletionResultType]::ParameterName, 'Stellar network to use (mainnet | testnet | futurenet)')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Global timeout for network/API operations (seconds)')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Registry configuration profile to use')
            [CompletionResult]::new('--no-cache', '--no-cache', [CompletionResultType]::ParameterName, 'Skip local response cache and always fetch fresh data')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--check-updates', '--check-updates', [CompletionResultType]::ParameterName, 'Check for CLI updates before running the command')
            [CompletionResult]::new('--describe', '--describe', [CompletionResultType]::ParameterName, 'Print machine-readable JSON command schema description (#1145)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'soroban-registry;snapshot;help' {
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export a signed offline registry snapshot')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a registry snapshot')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a signed registry snapshot locally')
            [CompletionResult]::new('inspect', 'inspect', [CompletionResultType]::ParameterValue, 'Inspect a registry snapshot metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;snapshot;help;export' {
            break
        }
        'soroban-registry;snapshot;help;sign' {
            break
        }
        'soroban-registry;snapshot;help;verify' {
            break
        }
        'soroban-registry;snapshot;help;inspect' {
            break
        }
        'soroban-registry;snapshot;help;help' {
            break
        }
        'soroban-registry;help' {
            [CompletionResult]::new('analytics', 'analytics', [CompletionResultType]::ParameterValue, 'Query contract analytics and statistics')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Get comprehensive registry statistics')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish a new contract to the registry')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List contracts in the registry')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show detailed info for a specific contract')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search for contracts in the registry')
            [CompletionResult]::new('compare', 'compare', [CompletionResultType]::ParameterValue, 'Compare multiple contracts')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (#971)')
            [CompletionResult]::new('generate-artifacts', 'generate-artifacts', [CompletionResultType]::ParameterValue, 'Generate or verify CLI command schemas and shell completion scripts (#1145)')
            [CompletionResult]::new('version', 'version', [CompletionResultType]::ParameterValue, 'Check CLI version and update availability')
            [CompletionResult]::new('dashboard', 'dashboard', [CompletionResultType]::ParameterValue, 'Launch an interactive, real-time terminal dashboard')
            [CompletionResult]::new('breaking-changes', 'breaking-changes', [CompletionResultType]::ParameterValue, 'Detect breaking changes between contract versions')
            [CompletionResult]::new('migrate', 'migrate', [CompletionResultType]::ParameterValue, 'Contract state migration assistant')
            [CompletionResult]::new('upgrade-analyze', 'upgrade-analyze', [CompletionResultType]::ParameterValue, 'Analyze upgrades between two contract versions or schema files')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export contract registry data or a contract archive')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import contract data from a file (JSON, CSV, or Archive)')
            [CompletionResult]::new('doc', 'doc', [CompletionResultType]::ParameterValue, 'Generate documentation from a contract WASM')
            [CompletionResult]::new('openapi', 'openapi', [CompletionResultType]::ParameterValue, 'Generate OpenAPI 3.0 spec from contract ABI')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Start an interactive contract deployment workflow')
            [CompletionResult]::new('versions', 'versions', [CompletionResultType]::ParameterValue, 'Manage contract semantic versions')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Perform batch operations on multiple contracts')
            [CompletionResult]::new('upgrade', 'upgrade', [CompletionResultType]::ParameterValue, 'Manage contract upgrades and rollbacks')
            [CompletionResult]::new('wizard', 'wizard', [CompletionResultType]::ParameterValue, 'Launch the interactive setup wizard')
            [CompletionResult]::new('repl', 'repl', [CompletionResultType]::ParameterValue, 'Enter interactive REPL mode')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Show command history')
            [CompletionResult]::new('patch', 'patch', [CompletionResultType]::ParameterValue, 'Security patch management')
            [CompletionResult]::new('incident', 'incident', [CompletionResultType]::ParameterValue, 'Incident response management')
            [CompletionResult]::new('multisig', 'multisig', [CompletionResultType]::ParameterValue, 'Multi-signature contract deployment workflow')
            [CompletionResult]::new('fuzz', 'fuzz', [CompletionResultType]::ParameterValue, 'Fuzz testing for contracts')
            [CompletionResult]::new('perf', 'perf', [CompletionResultType]::ParameterValue, 'Perf contract execution performance')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'Manage your user profile and publishing preferences (#841)')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Run integration tests')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Run a local contract security audit')
            [CompletionResult]::new('sla', 'sla', [CompletionResultType]::ParameterValue, 'SLA compliance monitoring')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Read and edit persisted user configuration values')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Manage authentication sessions and API tokens')
            [CompletionResult]::new('backup', 'backup', [CompletionResultType]::ParameterValue, 'Manage contract backups and disaster recovery')
            [CompletionResult]::new('state', 'state', [CompletionResultType]::ParameterValue, 'Inspect and modify contract state (dev/test mutation only)')
            [CompletionResult]::new('verify-formal', 'verify-formal', [CompletionResultType]::ParameterValue, 'Run formal verification analysis against a deployed or local contract')
            [CompletionResult]::new('scan-deps', 'scan-deps', [CompletionResultType]::ParameterValue, 'Scan a contract''s dependencies for known vulnerabilities')
            [CompletionResult]::new('coverage', 'coverage', [CompletionResultType]::ParameterValue, 'Measure and report code coverage for contract tests')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a contract package with your private key')
            [CompletionResult]::new('verify-package', 'verify-package', [CompletionResultType]::ParameterValue, 'Verify a signed contract package')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a contract in the registry (check status, submit for audit, or show history)')
            [CompletionResult]::new('verify-contract', 'verify-contract', [CompletionResultType]::ParameterValue, 'Verify a contract binary against an Ed25519 signature locally')
            [CompletionResult]::new('keys', 'keys', [CompletionResultType]::ParameterValue, 'Manage signing keys and signatures')
            [CompletionResult]::new('policy', 'policy', [CompletionResultType]::ParameterValue, 'Policy-as-code admission evaluation and reporting (#1148)')
            [CompletionResult]::new('publisher', 'publisher', [CompletionResultType]::ParameterValue, 'Publisher environment diagnostics (#841)')
            [CompletionResult]::new('contract', 'contract', [CompletionResultType]::ParameterValue, 'Contract deployment verification and security scan (#522)')
            [CompletionResult]::new('api-key', 'api-key', [CompletionResultType]::ParameterValue, 'Manage API keys for programmatic access (#842)')
            [CompletionResult]::new('batch-verify', 'batch-verify', [CompletionResultType]::ParameterValue, 'Verify multiple contracts in a bulk batch (#850)')
            [CompletionResult]::new('webhook', 'webhook', [CompletionResultType]::ParameterValue, 'Manage webhooks for contract lifecycle events')
            [CompletionResult]::new('release-notes', 'release-notes', [CompletionResultType]::ParameterValue, 'Auto-generate and manage release notes for contract versions')
            [CompletionResult]::new('cicd', 'cicd', [CompletionResultType]::ParameterValue, 'CI/CD pipeline integration and automation')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Check the status of supported Stellar networks')
            [CompletionResult]::new('batch-register', 'batch-register', [CompletionResultType]::ParameterValue, 'Register multiple contracts from a YAML or JSON manifest file')
            [CompletionResult]::new('batch-audit', 'batch-audit', [CompletionResultType]::ParameterValue, 'Audit multiple contracts in batch for security and best practices')
            [CompletionResult]::new('batch-deploy', 'batch-deploy', [CompletionResultType]::ParameterValue, 'Deploy a contract WASM to multiple networks')
            [CompletionResult]::new('batch-export', 'batch-export', [CompletionResultType]::ParameterValue, 'Export multiple contracts in bulk')
            [CompletionResult]::new('batch-import', 'batch-import', [CompletionResultType]::ParameterValue, 'Import contracts in bulk from a directory')
            [CompletionResult]::new('batch-update', 'batch-update', [CompletionResultType]::ParameterValue, 'Update metadata for multiple contracts in bulk (#849)')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Run advanced analysis on a deployed contract (#530)')
            [CompletionResult]::new('track-deployment', 'track-deployment', [CompletionResultType]::ParameterValue, 'Track contract deployment status until confirmed or timeout (#524)')
            [CompletionResult]::new('plugins', 'plugins', [CompletionResultType]::ParameterValue, 'Plugin management (install, configure, run)')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Manage local cache of registry API responses (#845)')
            [CompletionResult]::new('env', 'env', [CompletionResultType]::ParameterValue, 'Manage environment variable sets for different deployments (#843)')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Manage signed offline registry snapshots (#1146)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'soroban-registry;help;analytics' {
            break
        }
        'soroban-registry;help;stats' {
            break
        }
        'soroban-registry;help;publish' {
            break
        }
        'soroban-registry;help;list' {
            break
        }
        'soroban-registry;help;info' {
            break
        }
        'soroban-registry;help;search' {
            break
        }
        'soroban-registry;help;compare' {
            break
        }
        'soroban-registry;help;completion' {
            break
        }
        'soroban-registry;help;generate-artifacts' {
            break
        }
        'soroban-registry;help;version' {
            break
        }
        'soroban-registry;help;dashboard' {
            break
        }
        'soroban-registry;help;breaking-changes' {
            break
        }
        'soroban-registry;help;migrate' {
            [CompletionResult]::new('preview', 'preview', [CompletionResultType]::ParameterValue, 'Preview migration outcome (dry-run)')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze schema differences between versions')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate migration script template (rust|js)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate migration for data loss risks')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply migration and record history')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a migration by migration ID')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Show migration history')
            break
        }
        'soroban-registry;help;migrate;preview' {
            break
        }
        'soroban-registry;help;migrate;analyze' {
            break
        }
        'soroban-registry;help;migrate;generate' {
            break
        }
        'soroban-registry;help;migrate;validate' {
            break
        }
        'soroban-registry;help;migrate;apply' {
            break
        }
        'soroban-registry;help;migrate;rollback' {
            break
        }
        'soroban-registry;help;migrate;history' {
            break
        }
        'soroban-registry;help;upgrade-analyze' {
            break
        }
        'soroban-registry;help;export' {
            break
        }
        'soroban-registry;help;import' {
            break
        }
        'soroban-registry;help;doc' {
            break
        }
        'soroban-registry;help;openapi' {
            break
        }
        'soroban-registry;help;deploy' {
            break
        }
        'soroban-registry;help;versions' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List versions for a contract')
            [CompletionResult]::new('bump', 'bump', [CompletionResultType]::ParameterValue, 'Bump the semantic version')
            break
        }
        'soroban-registry;help;versions;list' {
            break
        }
        'soroban-registry;help;versions;bump' {
            break
        }
        'soroban-registry;help;batch' {
            break
        }
        'soroban-registry;help;upgrade' {
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze compatibility between two contract versions')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply an upgrade to a deployed contract')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a contract to a previous version')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a migration script template between versions')
            break
        }
        'soroban-registry;help;upgrade;analyze' {
            break
        }
        'soroban-registry;help;upgrade;apply' {
            break
        }
        'soroban-registry;help;upgrade;rollback' {
            break
        }
        'soroban-registry;help;upgrade;generate' {
            break
        }
        'soroban-registry;help;wizard' {
            break
        }
        'soroban-registry;help;repl' {
            break
        }
        'soroban-registry;help;history' {
            break
        }
        'soroban-registry;help;patch' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new security patch')
            [CompletionResult]::new('notify', 'notify', [CompletionResultType]::ParameterValue, 'Notify subscribers about a patch')
            [CompletionResult]::new('apply', 'apply', [CompletionResultType]::ParameterValue, 'Apply a patch to a specific contract')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'Manage contract dependencies')
            break
        }
        'soroban-registry;help;patch;create' {
            break
        }
        'soroban-registry;help;patch;notify' {
            break
        }
        'soroban-registry;help;patch;apply' {
            break
        }
        'soroban-registry;help;patch;deps' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List dependencies for a contract')
            break
        }
        'soroban-registry;help;patch;deps;list' {
            break
        }
        'soroban-registry;help;incident' {
            [CompletionResult]::new('trigger', 'trigger', [CompletionResultType]::ParameterValue, 'Trigger a new incident for a contract')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update the state of an existing incident')
            break
        }
        'soroban-registry;help;incident;trigger' {
            break
        }
        'soroban-registry;help;incident;update' {
            break
        }
        'soroban-registry;help;multisig' {
            [CompletionResult]::new('create-policy', 'create-policy', [CompletionResultType]::ParameterValue, 'Create a new multi-sig policy (defines signers and required threshold)')
            [CompletionResult]::new('create-proposal', 'create-proposal', [CompletionResultType]::ParameterValue, 'Create an unsigned deployment proposal')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a deployment proposal (add your approval)')
            [CompletionResult]::new('execute', 'execute', [CompletionResultType]::ParameterValue, 'Execute an approved deployment proposal')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show full info for a proposal (signatures, policy, status)')
            [CompletionResult]::new('list-proposals', 'list-proposals', [CompletionResultType]::ParameterValue, 'List deployment proposals')
            break
        }
        'soroban-registry;help;multisig;create-policy' {
            break
        }
        'soroban-registry;help;multisig;create-proposal' {
            break
        }
        'soroban-registry;help;multisig;sign' {
            break
        }
        'soroban-registry;help;multisig;execute' {
            break
        }
        'soroban-registry;help;multisig;info' {
            break
        }
        'soroban-registry;help;multisig;list-proposals' {
            break
        }
        'soroban-registry;help;fuzz' {
            break
        }
        'soroban-registry;help;perf' {
            break
        }
        'soroban-registry;help;profile' {
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'Display a publisher profile')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Update profile fields')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update a single profile field by key')
            [CompletionResult]::new('list-contracts', 'list-contracts', [CompletionResultType]::ParameterValue, 'List contracts published by a profile')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export full profile data to JSON or CSV')
            break
        }
        'soroban-registry;help;profile;view' {
            break
        }
        'soroban-registry;help;profile;edit' {
            break
        }
        'soroban-registry;help;profile;update' {
            break
        }
        'soroban-registry;help;profile;list-contracts' {
            break
        }
        'soroban-registry;help;profile;export' {
            break
        }
        'soroban-registry;help;test' {
            break
        }
        'soroban-registry;help;audit' {
            break
        }
        'soroban-registry;help;sla' {
            [CompletionResult]::new('record', 'record', [CompletionResultType]::ParameterValue, 'Record hourly SLA metrics for a contract')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show real-time SLA compliance dashboard')
            break
        }
        'soroban-registry;help;sla;record' {
            break
        }
        'soroban-registry;help;sla;status' {
            break
        }
        'soroban-registry;help;config' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a user config value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a user config value by key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all persisted user config values')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'Reset user config to defaults')
            [CompletionResult]::new('contract-get', 'contract-get', [CompletionResultType]::ParameterValue, 'Get contract environment configuration')
            [CompletionResult]::new('contract-set', 'contract-set', [CompletionResultType]::ParameterValue, 'Set contract environment configuration')
            [CompletionResult]::new('contract-history', 'contract-history', [CompletionResultType]::ParameterValue, 'Show contract config history')
            [CompletionResult]::new('contract-rollback', 'contract-rollback', [CompletionResultType]::ParameterValue, 'Roll back contract config to a previous version')
            break
        }
        'soroban-registry;help;config;get' {
            break
        }
        'soroban-registry;help;config;set' {
            break
        }
        'soroban-registry;help;config;list' {
            break
        }
        'soroban-registry;help;config;reset' {
            break
        }
        'soroban-registry;help;config;contract-get' {
            break
        }
        'soroban-registry;help;config;contract-set' {
            break
        }
        'soroban-registry;help;config;contract-history' {
            break
        }
        'soroban-registry;help;config;contract-rollback' {
            break
        }
        'soroban-registry;help;auth' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with a GitHub account, Stellar wallet, or API key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Sign out and remove stored credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the current authentication state')
            [CompletionResult]::new('token', 'token', [CompletionResultType]::ParameterValue, 'Print the current API token, refreshing it when possible')
            break
        }
        'soroban-registry;help;auth;login' {
            break
        }
        'soroban-registry;help;auth;logout' {
            break
        }
        'soroban-registry;help;auth;status' {
            break
        }
        'soroban-registry;help;auth;token' {
            break
        }
        'soroban-registry;help;backup' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new contract backup')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List recent backups for a contract')
            [CompletionResult]::new('restore', 'restore', [CompletionResultType]::ParameterValue, 'Restore a contract from a specific backup date')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify integrity of a specific backup')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show backup statistics for a contract')
            break
        }
        'soroban-registry;help;backup;create' {
            break
        }
        'soroban-registry;help;backup;list' {
            break
        }
        'soroban-registry;help;backup;restore' {
            break
        }
        'soroban-registry;help;backup;verify' {
            break
        }
        'soroban-registry;help;backup;stats' {
            break
        }
        'soroban-registry;help;state' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single state value by key')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a state key/value (testnet and futurenet only)')
            [CompletionResult]::new('dump', 'dump', [CompletionResultType]::ParameterValue, 'Dump full contract state')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Create a state snapshot')
            [CompletionResult]::new('snapshots', 'snapshots', [CompletionResultType]::ParameterValue, 'List saved state snapshots')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'Browse state change history')
            break
        }
        'soroban-registry;help;state;get' {
            break
        }
        'soroban-registry;help;state;set' {
            break
        }
        'soroban-registry;help;state;dump' {
            break
        }
        'soroban-registry;help;state;snapshot' {
            break
        }
        'soroban-registry;help;state;snapshots' {
            break
        }
        'soroban-registry;help;state;history' {
            break
        }
        'soroban-registry;help;verify-formal' {
            break
        }
        'soroban-registry;help;scan-deps' {
            break
        }
        'soroban-registry;help;coverage' {
            break
        }
        'soroban-registry;help;sign' {
            break
        }
        'soroban-registry;help;verify-package' {
            break
        }
        'soroban-registry;help;verify' {
            break
        }
        'soroban-registry;help;verify-contract' {
            break
        }
        'soroban-registry;help;keys' {
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate a new Ed25519 keypair for signing')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke a signature')
            [CompletionResult]::new('custody', 'custody', [CompletionResultType]::ParameterValue, 'Show chain of custody for a contract')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'View transparency log')
            break
        }
        'soroban-registry;help;keys;generate' {
            break
        }
        'soroban-registry;help;keys;revoke' {
            break
        }
        'soroban-registry;help;keys;custody' {
            break
        }
        'soroban-registry;help;keys;log' {
            break
        }
        'soroban-registry;help;policy' {
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run a policy-as-code admission check against a WASM artifact')
            break
        }
        'soroban-registry;help;policy;check' {
            break
        }
        'soroban-registry;help;publisher' {
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose the local publishing environment (config, session, signing key, connectivity)')
            break
        }
        'soroban-registry;help;publisher;doctor' {
            break
        }
        'soroban-registry;help;contract' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List registered contracts, a page at a time')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search the registry, one page at a time or across every page')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Export a signed, offline-verifiable snapshot of a contract (#1116)')
            [CompletionResult]::new('verify-snapshot', 'verify-snapshot', [CompletionResultType]::ParameterValue, 'Verify a previously exported contract snapshot (#1116)')
            [CompletionResult]::new('risk', 'risk', [CompletionResultType]::ParameterValue, 'Assess security and operational risks for a contract (#837)')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Deploy and register a new contract in the registry')
            [CompletionResult]::new('register', 'register', [CompletionResultType]::ParameterValue, 'Register one or more contracts in the registry')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a contract — a local WASM artifact before publishing, or a deployed contract''s authenticity against the on-chain registry')
            [CompletionResult]::new('interfaces', 'interfaces', [CompletionResultType]::ParameterValue, 'Derive and display a contract''s deterministic interface fingerprint (functions, types, events, errors) from a local compiled WASM artifact')
            [CompletionResult]::new('provenance', 'provenance', [CompletionResultType]::ParameterValue, 'Display build-provenance metadata recorded for a contract, read from a local manifest file')
            [CompletionResult]::new('verify-build', 'verify-build', [CompletionResultType]::ParameterValue, 'Attempt to independently reproduce a contract''s published WASM artifact from source, and compare its hash against the expected (registry-recorded) artifact hash')
            [CompletionResult]::new('compatibility', 'compatibility', [CompletionResultType]::ParameterValue, 'Structurally compare two local compiled WASM artifacts and classify ABI changes as compatible, potentially breaking, breaking, or unknown')
            [CompletionResult]::new('details', 'details', [CompletionResultType]::ParameterValue, 'Display detailed information about a contract')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show contract registry statistics and analytics')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export contracts and related registry data for backup or migration')
            [CompletionResult]::new('highlight', 'highlight', [CompletionResultType]::ParameterValue, 'Manage featured (highlighted) contracts (#832)')
            [CompletionResult]::new('interaction', 'interaction', [CompletionResultType]::ParameterValue, 'View a contract''s interactions and call patterns (#835)')
            [CompletionResult]::new('dependency', 'dependency', [CompletionResultType]::ParameterValue, 'Analyze a contract''s dependencies and relationships (#836, #1008)')
            [CompletionResult]::new('dependencies', 'dependencies', [CompletionResultType]::ParameterValue, 'List what a contract depends on (#1147)')
            [CompletionResult]::new('dependents', 'dependents', [CompletionResultType]::ParameterValue, 'List what depends on a contract (#1147)')
            [CompletionResult]::new('dependency-risk', 'dependency-risk', [CompletionResultType]::ParameterValue, 'Report direct and inherited risk across a contract''s dependencies (#1147)')
            [CompletionResult]::new('category', 'category', [CompletionResultType]::ParameterValue, 'List and inspect contract categories')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update contract metadata after registration (#828)')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import contracts into the registry from an external file (#831)')
            [CompletionResult]::new('rollback', 'rollback', [CompletionResultType]::ParameterValue, 'Rollback a deprecated contract to active state (#1091)')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Detect drift between local lockfile and registry state (#1060)')
            [CompletionResult]::new('deprecate', 'deprecate', [CompletionResultType]::ParameterValue, 'Deprecate a contract with publisher-signed authorization (#1091)')
            [CompletionResult]::new('notification', 'notification', [CompletionResultType]::ParameterValue, 'Manage contract event notifications and alerts (#838)')
            break
        }
        'soroban-registry;help;contract;list' {
            break
        }
        'soroban-registry;help;contract;search' {
            break
        }
        'soroban-registry;help;contract;snapshot' {
            break
        }
        'soroban-registry;help;contract;verify-snapshot' {
            break
        }
        'soroban-registry;help;contract;risk' {
            break
        }
        'soroban-registry;help;contract;deploy' {
            break
        }
        'soroban-registry;help;contract;register' {
            break
        }
        'soroban-registry;help;contract;verify' {
            break
        }
        'soroban-registry;help;contract;interfaces' {
            break
        }
        'soroban-registry;help;contract;provenance' {
            break
        }
        'soroban-registry;help;contract;verify-build' {
            break
        }
        'soroban-registry;help;contract;compatibility' {
            break
        }
        'soroban-registry;help;contract;details' {
            break
        }
        'soroban-registry;help;contract;stats' {
            break
        }
        'soroban-registry;help;contract;export' {
            break
        }
        'soroban-registry;help;contract;highlight' {
            break
        }
        'soroban-registry;help;contract;interaction' {
            break
        }
        'soroban-registry;help;contract;dependency' {
            break
        }
        'soroban-registry;help;contract;dependencies' {
            break
        }
        'soroban-registry;help;contract;dependents' {
            break
        }
        'soroban-registry;help;contract;dependency-risk' {
            break
        }
        'soroban-registry;help;contract;category' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all categories with descriptions and contract counts')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show detailed per-category statistics (counts, recent, trending)')
            break
        }
        'soroban-registry;help;contract;category;list' {
            break
        }
        'soroban-registry;help;contract;category;stats' {
            break
        }
        'soroban-registry;help;contract;update' {
            break
        }
        'soroban-registry;help;contract;import' {
            break
        }
        'soroban-registry;help;contract;rollback' {
            break
        }
        'soroban-registry;help;contract;audit' {
            break
        }
        'soroban-registry;help;contract;deprecate' {
            break
        }
        'soroban-registry;help;contract;notification' {
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe to alerts for a contract address')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Unsubscribe from alerts for a contract address')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List active notification rules')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Update an existing notification rule')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test alert for a subscribed contract')
            break
        }
        'soroban-registry;help;contract;notification;subscribe' {
            break
        }
        'soroban-registry;help;contract;notification;unsubscribe' {
            break
        }
        'soroban-registry;help;contract;notification;list' {
            break
        }
        'soroban-registry;help;contract;notification;configure' {
            break
        }
        'soroban-registry;help;contract;notification;test' {
            break
        }
        'soroban-registry;help;api-key' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new API key')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List your API keys')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete an API key')
            [CompletionResult]::new('revoke', 'revoke', [CompletionResultType]::ParameterValue, 'Revoke (disable) an API key without deleting its audit record')
            break
        }
        'soroban-registry;help;api-key;create' {
            break
        }
        'soroban-registry;help;api-key;list' {
            break
        }
        'soroban-registry;help;api-key;delete' {
            break
        }
        'soroban-registry;help;api-key;revoke' {
            break
        }
        'soroban-registry;help;batch-verify' {
            break
        }
        'soroban-registry;help;webhook' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Register a new webhook subscription')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered webhooks')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a webhook by ID')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Send a test event to a webhook')
            [CompletionResult]::new('logs', 'logs', [CompletionResultType]::ParameterValue, 'View delivery logs for a webhook')
            [CompletionResult]::new('retry', 'retry', [CompletionResultType]::ParameterValue, 'Manually retry a dead-letter delivery')
            [CompletionResult]::new('verify-sig', 'verify-sig', [CompletionResultType]::ParameterValue, 'Verify a webhook payload signature locally')
            break
        }
        'soroban-registry;help;webhook;create' {
            break
        }
        'soroban-registry;help;webhook;list' {
            break
        }
        'soroban-registry;help;webhook;delete' {
            break
        }
        'soroban-registry;help;webhook;test' {
            break
        }
        'soroban-registry;help;webhook;logs' {
            break
        }
        'soroban-registry;help;webhook;retry' {
            break
        }
        'soroban-registry;help;webhook;verify-sig' {
            break
        }
        'soroban-registry;help;release-notes' {
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Auto-generate release notes from code diff and changelog')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'View generated release notes for a version')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit draft release notes before publishing')
            [CompletionResult]::new('publish', 'publish', [CompletionResultType]::ParameterValue, 'Publish (finalize) release notes')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all release notes for a contract')
            break
        }
        'soroban-registry;help;release-notes;generate' {
            break
        }
        'soroban-registry;help;release-notes;view' {
            break
        }
        'soroban-registry;help;release-notes;edit' {
            break
        }
        'soroban-registry;help;release-notes;publish' {
            break
        }
        'soroban-registry;help;release-notes;list' {
            break
        }
        'soroban-registry;help;cicd' {
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a full CI/CD pipeline (validate, scan, build, publish, verify)')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate the current environment for CI/CD integration')
            break
        }
        'soroban-registry;help;cicd;run' {
            break
        }
        'soroban-registry;help;cicd;validate' {
            break
        }
        'soroban-registry;help;network' {
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show status of all supported Stellar networks')
            break
        }
        'soroban-registry;help;network;status' {
            break
        }
        'soroban-registry;help;batch-register' {
            break
        }
        'soroban-registry;help;batch-audit' {
            break
        }
        'soroban-registry;help;batch-deploy' {
            break
        }
        'soroban-registry;help;batch-export' {
            break
        }
        'soroban-registry;help;batch-import' {
            break
        }
        'soroban-registry;help;batch-update' {
            break
        }
        'soroban-registry;help;analyze' {
            break
        }
        'soroban-registry;help;track-deployment' {
            break
        }
        'soroban-registry;help;plugins' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed plugins and their commands')
            [CompletionResult]::new('marketplace', 'marketplace', [CompletionResultType]::ParameterValue, 'Browse the registry marketplace')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install a plugin from the registry')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'Uninstall an installed plugin')
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a plugin-provided command explicitly')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Enable/disable plugins and set per-plugin configuration')
            break
        }
        'soroban-registry;help;plugins;list' {
            break
        }
        'soroban-registry;help;plugins;marketplace' {
            break
        }
        'soroban-registry;help;plugins;install' {
            break
        }
        'soroban-registry;help;plugins;uninstall' {
            break
        }
        'soroban-registry;help;plugins;run' {
            break
        }
        'soroban-registry;help;plugins;config' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get the current JSON config for a plugin')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Replace the plugin JSON config (must be a JSON object)')
            [CompletionResult]::new('disable', 'disable', [CompletionResultType]::ParameterValue, 'Disable a plugin (commands won''t be discovered)')
            [CompletionResult]::new('enable', 'enable', [CompletionResultType]::ParameterValue, 'Enable a plugin (default)')
            break
        }
        'soroban-registry;help;plugins;config;get' {
            break
        }
        'soroban-registry;help;plugins;config;set' {
            break
        }
        'soroban-registry;help;plugins;config;disable' {
            break
        }
        'soroban-registry;help;plugins;config;enable' {
            break
        }
        'soroban-registry;help;cache' {
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear cached entries from disk')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache statistics and configuration')
            [CompletionResult]::new('configure', 'configure', [CompletionResultType]::ParameterValue, 'Configure cache settings')
            [CompletionResult]::new('optimize', 'optimize', [CompletionResultType]::ParameterValue, 'Remove stale entries and enforce disk size limit')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export cache entries for analysis')
            break
        }
        'soroban-registry;help;cache;clear' {
            break
        }
        'soroban-registry;help;cache;status' {
            break
        }
        'soroban-registry;help;cache;configure' {
            break
        }
        'soroban-registry;help;cache;optimize' {
            break
        }
        'soroban-registry;help;cache;export' {
            break
        }
        'soroban-registry;help;env' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a variable in an environment')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a variable''s value from an environment')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List variables in an environment')
            [CompletionResult]::new('copy', 'copy', [CompletionResultType]::ParameterValue, 'Copy all variables from one environment to another')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a variable from an environment')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export environment variables as a shell-sourceable file')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch the active environment')
            break
        }
        'soroban-registry;help;env;set' {
            break
        }
        'soroban-registry;help;env;get' {
            break
        }
        'soroban-registry;help;env;list' {
            break
        }
        'soroban-registry;help;env;copy' {
            break
        }
        'soroban-registry;help;env;delete' {
            break
        }
        'soroban-registry;help;env;export' {
            break
        }
        'soroban-registry;help;env;switch' {
            break
        }
        'soroban-registry;help;snapshot' {
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export a signed offline registry snapshot')
            [CompletionResult]::new('sign', 'sign', [CompletionResultType]::ParameterValue, 'Sign a registry snapshot')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify a signed registry snapshot locally')
            [CompletionResult]::new('inspect', 'inspect', [CompletionResultType]::ParameterValue, 'Inspect a registry snapshot metadata')
            break
        }
        'soroban-registry;help;snapshot;export' {
            break
        }
        'soroban-registry;help;snapshot;sign' {
            break
        }
        'soroban-registry;help;snapshot;verify' {
            break
        }
        'soroban-registry;help;snapshot;inspect' {
            break
        }
        'soroban-registry;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
