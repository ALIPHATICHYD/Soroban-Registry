//! WARNING: This example intentionally demonstrates an anti-pattern.
//! Hardcoded string storage keys can collide silently and corrupt state.
//! See `examples/token_fixed.rs` for the recommended typed-key approach.

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

const STORAGE_KEY_BALANCE: &str = "balance";
const STORAGE_KEY_ALLOWANCE: &str = "balance";  // Intentional collision with STORAGE_KEY_BALANCE to demonstrate the anti-pattern


/// Maximum number of iterations allowed in the mint function.
/// This prevents instruction budget exhaustion on the Stellar network.
/// Soroban contracts are subject to strict CPU instruction limits per transaction.
const MAX_MINT_ITERATIONS: u64 = 1_000;

#[contract]
pub struct TokenWithIssues;

#[contractimpl]
impl TokenWithIssues {
    /// WARNING: anti-pattern for demonstration only.
    /// This writes to a global hardcoded key and does not namespace by account.
    pub fn set_balance(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, STORAGE_KEY_BALANCE), &amount);
    }

    /// WARNING: anti-pattern for demonstration only.
    /// This uses a different logical concept but the same raw key value as `set_balance`.
    pub fn set_allowance(env: Env, owner: Address, spender: Address, amount: i128) {
        owner.require_auth();
        let _ = spender;
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, STORAGE_KEY_ALLOWANCE), &amount);
    }

    pub fn get_balance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .expect("No balance found")  // Issue: expect() in public function
    }
    
/// Mint new tokens up to a bounded iteration limit.
///
/// # Resource Limits
///
/// - Maximum iterations: `MAX_MINT_ITERATIONS` (1,000)
/// - `amount` must be greater than 0
/// - `amount` must not exceed `MAX_MINT_ITERATIONS`
///
/// These limits are intentionally enforced because Soroban contracts execute
/// within a fixed CPU instruction budget per transaction. Allowing an
/// unbounded loop could exhaust that budget, causing the transaction to fail
/// or potentially opening the door to denial-of-service (DoS) scenarios.
///
/// By capping iterations and validating input early, we ensure predictable
/// resource usage and safer execution.
///
/// # Panics
///
/// - Panics if `amount` is 0
/// - Panics if `amount` exceeds `MAX_MINT_ITERATIONS`
    pub fn mint(env: Env, amount: u64) {
        // Parameter validation
        if amount == 0 {
            panic!("amount must be greater than zero");
        }
        if amount > MAX_MINT_ITERATIONS {
            panic!(
                "amount exceeds maximum allowed iterations ({})",
                MAX_MINT_ITERATIONS
            );
        }

        let mut counter: u64 = 0;
        loop {
            // early exit when iteration limit is reached
            if counter >= amount || counter >= MAX_MINT_ITERATIONS {
                break;
            }

            env.storage().persistent().set(
                &Symbol::new(&env, "total_supply"),
                &(amount as i128),
            );
            counter += 1;
        }
    }

    pub fn get_allowance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, STORAGE_KEY_ALLOWANCE))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::TokenWithIssues;
    use soroban_sdk::{testutils::Address as _, Address, Env};


#[test]
fn test_transfer() {
    let env = Env::new();
    
    // Test code can use unwrap - this should NOT trigger
    let val = Some(42).unwrap();
    assert_eq!(val, 42);
}

/// Verifies that minting with a valid amount completes successfully without panicking.
#[test]
fn test_mint_valid_amount() {
    let env = Env::new();
    // amount = 10, well within the MAX_MINT_ITERATIONS limit
    TokenWithIssues::mint(env, 10);
}

/// Verifies that minting with the exact MAX_MINT_ITERATIONS amount completes successfully without panicking.
#[test]
fn test_mint_exact_limit() {
    let env = Env::new();
    TokenWithIssues::mint(env, MAX_MINT_ITERATIONS);
}

/// Mint with amount = 0 must panic (invalid input).
#[test]
#[should_panic(expected = "amount must be greater than zero")]
fn test_mint_zero_amount() {
    let env = Env::new();
    TokenWithIssues::mint(env, 0);
}

#[test]
#[should_panic(expected = "amount exceeds maximum allowed iterations")]
fn test_mint_exceeds_limit() {
    let env = Env::new();
    TokenWithIssues::mint(env, MAX_MINT_ITERATIONS + 1);
}

#[test]
fn hardcoded_keys_collide_and_overwrite_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let spender = Address::generate(&env);

    TokenWithIssues::set_balance(env.clone(), owner.clone(), 100);
    assert_eq!(TokenWithIssues::get_balance(env.clone()), 100);

    // Writing allowance overwrites balance because both map to "balance".
    TokenWithIssues::set_allowance(env.clone(), owner, spender, 7);

    assert_eq!(TokenWithIssues::get_allowance(env.clone()), 7);
    assert_eq!(TokenWithIssues::get_balance(env), 7);
}

#[test]
fn hardcoded_keys_collide_and_overwrite_allowance() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);

    TokenWithIssues::set_allowance(env.clone(), owner.clone(), owner.clone(), 55);
    assert_eq!(TokenWithIssues::get_allowance(env.clone()), 55);

    // Writing balance now overwrites what allowance previously stored.
    TokenWithIssues::set_balance(env.clone(), owner, 12);

    assert_eq!(TokenWithIssues::get_balance(env.clone()), 12);
    assert_eq!(TokenWithIssues::get_allowance(env), 12);
}

}