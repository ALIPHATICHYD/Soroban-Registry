#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env};

#[contracttype]
pub enum DataKey {
    Admin,
    Nonce(Bytes),
    Processed(Bytes),
}

#[contract]
pub struct {{CONTRACT_NAME}};

#[contractimpl]
impl {{CONTRACT_NAME}} {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn lock(env: Env, sender: Address, amount: i128, target_chain: u32, recipient: Bytes) -> Bytes {
        sender.require_auth();
        assert!(amount > 0, "amount must be positive");
        let nonce = env.crypto().sha256(&recipient);
        // `Hash<32>` exposes `to_array` / `to_bytes`; it has no `as_ref`, so the previous
        // form meant this template generated code that did not compile.
        let nonce_bytes = Bytes::from_array(&env, &nonce.to_array());
        let processed: bool = env.storage().instance().get(&DataKey::Processed(nonce_bytes.clone())).unwrap_or(false);
        assert!(!processed, "transfer already processed");
        env.storage().instance().set(&DataKey::Nonce(nonce_bytes.clone()), &(sender, amount, target_chain));
        nonce_bytes
    }

    pub fn release(env: Env, nonce: Bytes, recipient: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        let processed: bool = env.storage().instance().get(&DataKey::Processed(nonce.clone())).unwrap_or(false);
        assert!(!processed, "transfer already processed");
        env.storage().instance().set(&DataKey::Processed(nonce), &true);
        let current: i128 = env.storage().instance().get(&DataKey::Nonce(Bytes::new(&env))).unwrap_or(0);
        let _ = (recipient, amount, current);
    }
}
