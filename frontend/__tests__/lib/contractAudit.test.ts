import { analyzeContractSource } from "../../lib/contractAudit";

const vulnerable = `#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};
const STORAGE_KEY_BALANCE: &str = "balance";
const STORAGE_KEY_ALLOWANCE: &str = "balance";
#[contract]
pub struct Token;
#[contractimpl]
impl Token {
  pub fn mint(env: Env, amount: i128) {
    let supply = amount + 1;
    env.storage().persistent().set(&Symbol::new(&env, STORAGE_KEY_BALANCE), &supply);
  }
  pub fn balance(env: Env) -> i128 {
    env.storage().persistent().get::<_, i128>(&Symbol::new(&env, STORAGE_KEY_BALANCE)).expect("missing")
  }
}`;

const safer = `#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
#[derive(Clone)]
#[contracttype]
pub enum DataKey { Balance(Address) }
#[contract]
pub struct Token;
#[contractimpl]
impl Token {
  pub fn set_balance(env: Env, owner: Address, amount: i128) -> Result<(), ()> {
    owner.require_auth();
    env.storage().persistent().set(&DataKey::Balance(owner), &amount);
    env.events().publish(("balance",), amount);
    Ok(())
  }
}`;

describe("contract audit model", () => {
  test("detects explainable vulnerabilities and lowers the score", () => {
    const report = analyzeContractSource(
      vulnerable,
      "2026-04-23T00:00:00.000Z",
    );

    expect(report.score).toBeLessThan(70);
    expect(report.findings.map((finding) => finding.id)).toEqual(
      expect.arrayContaining([
        "missing-auth",
        "duplicate-storage-key",
        "panic-public",
      ]),
    );
    expect(report.recommendations.length).toBeGreaterThan(0);
    expect(
      report.signals.some((signal) => signal.name === "Authorization coverage"),
    ).toBe(true);
  });

  test("rewards typed storage, authorization, events, and recoverable errors", () => {
    const vulnerableReport = analyzeContractSource(
      vulnerable,
      "2026-04-23T00:00:00.000Z",
    );
    const saferReport = analyzeContractSource(
      safer,
      "2026-04-23T00:00:00.000Z",
    );

    expect(saferReport.score).toBeGreaterThan(vulnerableReport.score);
    expect(saferReport.grade).toMatch(/[AB]/);
  });
});
