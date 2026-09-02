//! Typed endpoint bindings, grouped by domain.
//!
//! Each module adds inherent methods to [`crate::RegistryClient`], so every call
//! shares one client, one auth configuration, one retry policy, and one error
//! taxonomy. Request and response types come from `shared` wherever the backend
//! defines them; where an endpoint's types live in the API crate they are
//! mirrored here with the same wire shape.

pub mod contracts;
pub mod deprecation;
pub mod ownership;
pub mod snapshots;
pub mod verification;
pub mod vulnerabilities;
pub mod webhooks;
