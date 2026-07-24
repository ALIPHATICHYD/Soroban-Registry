import {
  compileContractSource,
  createDebugTrace,
  createVersionSnapshot,
  diffSnapshots,
  runContractTests,
} from "../../lib/contractIde";

const source = `#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};
#[contract]
pub struct Token;
#[contractimpl]
impl Token {
  pub fn set_balance(env: Env, owner: Address, amount: i128) {
    owner.require_auth();
    env.storage().persistent().set(&owner, &amount);
  }
}`;

describe("contract IDE helpers", () => {
  test("compiles a valid browser workspace source into a WASM artifact summary", () => {
    const result = compileContractSource(source);

    expect(result.ok).toBe(true);
    expect(result.artifact?.name).toBe("contract.wasm");
    expect(result.artifact?.exportedFunctions).toEqual(["set_balance"]);
  });

  test("runs integrated smoke tests and reports authorization coverage", () => {
    const tests = runContractTests(source);

    expect(tests.find((test) => test.name === "wasm compilation")?.status).toBe(
      "passed",
    );
    expect(
      tests.find((test) => test.name === "authorization path")?.status,
    ).toBe("passed");
  });

  test("creates debugging traces for contract execution paths", () => {
    const trace = createDebugTrace(source);

    expect(trace.map((step) => step.label)).toContain(
      "Execute authorization gate",
    );
    expect(
      trace.every((step) => ["ok", "warning", "error"].includes(step.status)),
    ).toBe(true);
  });

  test("creates version history diffs", () => {
    const first = createVersionSnapshot(
      source,
      "first",
      "2026-04-23T00:00:00.000Z",
    );
    const second = createVersionSnapshot(
      `${source}\n// changed`,
      "second",
      "2026-04-23T00:00:01.000Z",
    );

    expect(diffSnapshots(first, second)).toContainEqual({
      line: source.split("\n").length + 1,
      after: "// changed",
      type: "added",
    });
  });
});
