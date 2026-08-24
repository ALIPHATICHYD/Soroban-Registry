# soroban-registry CLI

Command-line client for the Soroban Registry. The installed binary is named
`soroban-registry`; the Cargo package is `soroban-registry-cli`.

## Layout notes that affect every command below

`cli/` is a **standalone Cargo package**, not a member of a workspace. There is no
top-level `Cargo.toml`, so running `cargo` from the repository root will not find this
crate. Either `cd cli` first, or pass `--manifest-path cli/Cargo.toml`. The commands here
use the `--manifest-path` form so they can be pasted from anywhere in the tree.

The crate depends on three backend crates by path:

- `backend/shared`
- `backend/contract_abi`
- `backend/registry_client` (package `soroban-registry-client`, **lib name
  `registry_client`**)

That matters in two ways. A change to any of those crates can break the CLI even though
nothing under `cli/` was touched, which is why `.github/workflows/cli-ci.yml` triggers on
those paths too. And `cargo fmt --all` follows those path dependencies into the backend,
which carries its own pre-existing formatting drift -- so always scope formatting to this
package with `-p soroban-registry-cli`.

The toolchain is pinned to nightly by the repository-root `rust-toolchain.toml`. Do not
override it with `+stable`.

## Validating a clean checkout

This is the exact sequence CI runs. It should pass on a fresh clone with no database, no
running API, and no environment variables set.

```bash
# Optional: prove it really is a clean build. This deletes cli/target.
git clean -xdf cli

cargo build --manifest-path cli/Cargo.toml --locked --all-targets
cargo test  --manifest-path cli/Cargo.toml --locked
cargo run   --manifest-path cli/Cargo.toml --locked --bin soroban-registry -- --help
cargo fmt   --manifest-path cli/Cargo.toml -p soroban-registry-cli --check
```

Notes on the flags:

- `--all-targets` on the build is deliberate. `cargo build` alone does not compile test
  targets, so it can succeed while `cargo test` fails to compile.
- `--locked` fails rather than silently rewriting `Cargo.lock`. If it errors, the lock file
  genuinely needs updating -- commit that as its own change.
- `--bin soroban-registry` is required because the package also builds a
  `verify_flamegraph_memory` binary, so a bare `cargo run` cannot choose.
- Formatting is `-p soroban-registry-cli`, **not** `--all`. See the layout notes above.

## Running a single test

```bash
cargo test --manifest-path cli/Cargo.toml --locked <name-substring>
cargo test --manifest-path cli/Cargo.toml --locked --test doctor_integration
cargo test --manifest-path cli/Cargo.toml --locked -- --nocapture
```

## Ignored tests

`cargo test` reports several ignored tests. They are ignored for stated reasons, each
recorded in an `#[ignore = "..."]` message next to the test:

- `doctor_integration::test_doctor_network_failure` -- needs a session whose refresh secret
  lives in the OS keyring. Seeding one from a test would write to the developer's login
  keychain and would need a keyring daemon on Linux CI.
- `track_deployment_integration::{test_track_deployment_timeout_exits_code_2,
  test_track_deployment_timeout_json_output}` -- these assert a poll timeout, but
  `track-deployment` falls back to public Horizon and Soroban RPC whose endpoints are
  hardcoded, so on any networked machine the contract resolves and the command exits 0.
  Testing the timeout path requires making those endpoints overridable.
- Several `template` and `docs` tests -- those commands are not wired into the main router.

Run them explicitly with `-- --ignored` if you are working on the underlying feature.

## Guarding the command tree

`cli/src/main.rs` defines every command as a `clap` derive enum. A subcommand can be
deleted from that tree while its module and its `match` arm survive, and the result still
compiles -- the command simply vanishes from `--help`. This has happened repeatedly through
merge conflict resolutions (issue #1156).

`mod command_tree_tests` in `cli/src/main.rs` guards against it:

- `command_tree_is_valid` runs clap's own `debug_assert` over the whole tree.
- `every_subcommand_has_help_text` fails on any command that would render with no
  description.
- `every_top_level_command_appears_in_help` renders the long help and checks each command
  is present.
- `restored_commands_are_reachable` names the specific commands previously lost.

If you add a command, add it to the enum **and** the dispatch arm, then run
`cargo test --manifest-path cli/Cargo.toml --locked command_tree`.

Note that these tests run the clap tree walk on a spawned thread with a larger stack, via
`with_large_stack` in `main.rs`. The tree is deep enough that walking it overflows the
2 MiB stack libtest gives each test thread in a debug build. The shipped binary is
unaffected -- it walks the tree on the main thread, whose default stack is 8 MiB.

## Configuration

Runtime config lives at `~/.soroban-registry/config.toml`; a legacy
`~/.soroban-registry.toml` is migrated automatically. User preferences and the legacy API
key live in `~/.soroban-registry/config.json`. Auth sessions are stored as a session record
file plus secrets in the OS keyring.

## Installing

```bash
cargo install --path cli
```
