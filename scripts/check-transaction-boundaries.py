#!/usr/bin/env python3
"""Enforce transaction boundaries on multi-table write handlers (issue #1164).

Handlers like publish, ownership transfer, and deprecation mutate several
tables that must move together (contract record, audit log, dependency edges).
The `db_transaction::with_transaction` helper makes the atomic unit explicit;
this lint makes it enforced.

Rule: an `async fn` handler in the API crate may perform **at most one** write
against the pool connection (autocommit) inside its own body. A handler that
needs more than one write must do them inside a `with_transaction` closure.

A "write" is:

  * `.execute(...)` called on a pool handle (`state.db`, `self.db`, `pool`,
    `db`, ...) -- not on a transaction handle; or
  * `.fetch_one/.fetch_optional/.fetch_all` on a pool handle whose query is an
    INSERT/UPDATE/DELETE/REPLACE/MERGE (e.g. `INSERT ... RETURNING *`).

Writes performed by *helper functions* (e.g. `write_contract_audit_log(
&state.db, ...)`) are not visible to this grep-level check, which is why the
migrated handlers must also pass their writes through the transaction they are
handed rather than reaching for the pool again.

Usage:

  check-transaction-boundaries.py [--baseline BASELINE_FILE] [--update-baseline]

Without `--update-baseline`, exits 1 if any violation is not listed in the
baseline file (so pre-existing cases don't fail CI, but any new or
re-introduced multi-write handler does). With it, rewrites the baseline to the
current violation set.
"""

import argparse
import os
import re
import sys

API_SRC = os.path.join("backend", "api", "src")

POOL_RECEIVER = re.compile(
    r"""^\s*&?\s*(?:mut\s+\*{1,2}\s*)?(?:state\.db|self\.db|self\.pool|pool|db)\s*$""",
    re.VERBOSE,
)

POOL_EXECUTE = re.compile(r"\.execute\(\s*(?P<recv>[^)]*?)\s*\)", re.DOTALL)

POOL_FETCH = re.compile(
    r"\.fetch_(?:one|optional|all)\(\s*(?P<recv>[^)]*?)\s*\)", re.DOTALL
)

QUERY_START = re.compile(r"sqlx::query(?:_as|_scalar)?\(")

# The first string literal in a query chain (handles plain and r#"..."# raw
# strings; SQL may span multiple lines).
SQL_LITERAL = re.compile(r'(?:r#)?"(.*?)"(?:#)?', re.DOTALL)

WRITE_SQL = re.compile(r"^\s*(INSERT|UPDATE|DELETE|REPLACE|MERGE)\b", re.IGNORECASE)


def is_pool_receiver(recv: str) -> bool:
    # Transaction receivers are named after the transaction (`tx`,
    # `transaction`); never treat those as pool writes.
    if "tx" in recv or "transaction" in recv:
        return False
    return bool(POOL_RECEIVER.match(recv))


def strip_test_modules(source: str) -> str:
    """Remove `#[cfg(test)] mod tests { ... }` blocks.

    Test scaffolding legitimately writes to a scratch database (e.g. the
    `with_transaction` rollback test creates and drops its own table), so it is
    not the target of this lint. Brace-matching keeps nested blocks intact.
    """
    out = []
    i = 0
    while i < len(source):
        match = re.match(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{", source[i:])
        if not match:
            out.append(source[i])
            i += 1
            continue
        start = i + match.end() - 1  # position of the opening brace
        depth = 0
        j = start
        while j < len(source):
            if source[j] == "{":
                depth += 1
            elif source[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        i = j + 1  # skip past the whole test module
    return "".join(out)


def find_functions(source: str):
    """Yield (name, body) for every top-level `async fn` in a source file."""
    for m in re.finditer(
        r"(?m)^\s*(?:pub(?:\(crate\))?\s+)?async\s+fn\s+(\w+)\s*\([^{]*\{",
        source,
    ):
        name = m.group(1)
        start = m.end() - 1  # position of the opening brace
        depth = 0
        i = start
        while i < len(source):
            if source[i] == "{":
                depth += 1
            elif source[i] == "}":
                depth -= 1
                if depth == 0:
                    yield name, source[start : i + 1]
                    break
            i += 1


def query_sql_before(fetch_match, body: str):
    """Return the SQL string of the query chain a fetch belongs to, or None."""
    prefix = body[: fetch_match.start()]
    # Do not cross a statement boundary.
    cut = prefix.rfind(";")
    segment = prefix[cut + 1 :]
    q = QUERY_START.search(segment)
    if not q:
        return None
    literal = SQL_LITERAL.search(segment[q.end() :])
    if not literal:
        return None
    return literal.group(1)


def count_pool_writes(body: str) -> int:
    writes = 0

    for m in POOL_EXECUTE.finditer(body):
        if is_pool_receiver(m.group("recv")):
            writes += 1

    for m in POOL_FETCH.finditer(body):
        if not is_pool_receiver(m.group("recv")):
            continue
        sql = query_sql_before(m, body)
        if sql is not None and WRITE_SQL.match(sql):
            writes += 1

    return writes


def scan():
    violations = []  # (file, function, writes)
    for root, _dirs, files in os.walk(API_SRC):
        for fname in sorted(files):
            if not fname.endswith(".rs"):
                continue
            path = os.path.join(root, fname)
            with open(path, encoding="utf-8") as fh:
                source = strip_test_modules(fh.read())
            for fn_name, body in find_functions(source):
                writes = count_pool_writes(body)
                if writes > 1:
                    rel = os.path.relpath(path, ".")
                    violations.append((rel, fn_name, writes))
    return violations


def load_baseline(path):
    entries = set()
    if path and os.path.exists(path):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line and not line.startswith("#"):
                    # Entries are written as "rel:fn (N writes)"; the count is
                    # informational, only the key is compared.
                    entries.add(line.split(" (")[0])
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--baseline", default=None, help="baseline file of known violations"
    )
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="rewrite the baseline file to the current violation set",
    )
    args = parser.parse_args()

    violations = scan()

    if args.update_baseline:
        assert args.baseline, "--update-baseline requires --baseline"
        with open(args.baseline, "w", encoding="utf-8") as fh:
            fh.write(
                "# Handlers that currently perform more than one pool write outside\n"
                "# with_transaction (issue #1164). Migrate them to\n"
                "# db_transaction::with_transaction and remove the line; CI fails on\n"
                "# any violation NOT listed here.\n"
            )
            for rel, fn, writes in violations:
                fh.write(f"{rel}:{fn} ({writes} writes)\n")
        print(f"Updated baseline with {len(violations)} violation(s).")
        return 0

    baseline = load_baseline(args.baseline)
    new_violations = []
    for rel, fn, writes in violations:
        key = f"{rel}:{fn}"
        if key not in baseline:
            new_violations.append((rel, fn, writes))

    if new_violations:
        print(
            "Transaction boundary violations (more than one pool write outside "
            "with_transaction):"
        )
        for rel, fn, writes in sorted(new_violations):
            print(f"  - {rel}:{fn} ({writes} pool writes)")
        print()
        print(
            "Migrate these handlers to db_transaction::with_transaction "
            "(issue #1164), or add them to the baseline file if they are a "
            "known pre-existing case."
        )
        return 1

    print(
        f"OK: no new transaction-boundary violations "
        f"({len(violations)} known, all baselined)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
