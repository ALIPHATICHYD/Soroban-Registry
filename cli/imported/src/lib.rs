#![no_std]
//! Minimal Soroban contract used as the reference contract for the smart-contract CI
//! workflow: it is the crate that proves the toolchain can format, compile to
//! `wasm32-unknown-unknown`, and run contract unit tests.
//!
//! The `soroban_sdk` import below was previously commented out while the code still used
//! `Env`, `Symbol`, `vec!` and the contract macros, so this crate did not compile at all.

use soroban_sdk::{contract, contractimpl, symbol_short, vec, Env, Symbol, Vec};

#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    /// Returns a greeting message
    pub fn hello(env: Env, to: Symbol) -> Vec<Symbol> {
        vec![&env, symbol_short!("Hello"), to]
    }

    /// Returns the contract version
    pub fn version() -> u32 {
        1
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{symbol_short, Env};

    #[test]
    fn test_hello() {
        let env = Env::default();
        // `register_contract` was removed in soroban-sdk 22; `register` takes the
        // constructor arguments as its second parameter.
        let contract_id = env.register(HelloContract, ());
        let client = HelloContractClient::new(&env, &contract_id);

        let result = client.hello(&symbol_short!("World"));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_version() {
        let env = Env::default();
        let contract_id = env.register(HelloContract, ());
        let client = HelloContractClient::new(&env, &contract_id);

        assert_eq!(client.version(), 1);
    }
}
