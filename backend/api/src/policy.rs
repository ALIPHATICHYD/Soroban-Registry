//! Central authorization policy for security-sensitive actions.
//!
//! Authentication proves who made the request. This module resolves that
//! identity to a publisher record and decides what the actor may do.

use axum::extract::FromRequestParts;
use uuid::Uuid;

use crate::{
    auth::AuthClaims,
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct PolicyActor {
    publisher_id: Option<Uuid>,
    stellar_address: String,
    is_admin: bool,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for PolicyActor {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = AuthClaims::from_request_parts(parts, state).await?;
        Self::from_claims(state, claims).await
    }
}

impl PolicyActor {
    async fn from_claims(state: &AppState, claims: AuthClaims) -> ApiResult<Self> {
        let publisher_id = if claims.publisher_id.is_nil() {
            None
        } else {
            let publisher_address: Option<String> =
                sqlx::query_scalar("SELECT stellar_address FROM publishers WHERE id = $1")
                    .bind(claims.publisher_id)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|err| db_internal_error("resolve authenticated publisher", err))?;

            match publisher_address {
                Some(address) if address == claims.sub => Some(claims.publisher_id),
                Some(_) => {
                    return Err(ApiError::unauthorized(
                        "The token publisher ID and Stellar address do not match",
                    ))
                }
                None => None,
            }
        };

        let is_admin = claims_are_admin(&claims);
        Ok(Self {
            publisher_id,
            stellar_address: claims.sub,
            is_admin,
        })
    }

    pub fn publisher_id(&self) -> ApiResult<Uuid> {
        self.publisher_id.ok_or_else(|| {
            ApiError::forbidden_with_error(
                "UnknownPublisher",
                "The authenticated account is not registered as a publisher",
            )
        })
    }

    pub fn stellar_address(&self) -> &str {
        &self.stellar_address
    }

    pub fn require_admin(&self) -> ApiResult<()> {
        if self.is_admin {
            Ok(())
        } else {
            Err(ApiError::forbidden(
                "Administrative privileges are required for this action",
            ))
        }
    }

    pub fn require_contract_owner(&self, owner_id: Uuid) -> ApiResult<()> {
        if self.publisher_id()? == owner_id {
            Ok(())
        } else {
            Err(ApiError::forbidden_with_error(
                "NotContractOwner",
                "Only the contract's current publisher can perform this action",
            ))
        }
    }

    fn require_resource_owner(&self, owner_id: Uuid, resource: &str) -> ApiResult<()> {
        if self.publisher_id()? == owner_id {
            Ok(())
        } else {
            Err(ApiError::forbidden_with_error(
                "NotResourceOwner",
                format!("Only the {resource} owner can perform this action"),
            ))
        }
    }

    pub fn require_publisher_address(&self, address: &str) -> ApiResult<()> {
        self.publisher_id()?;
        self.require_signature_identity(address)
    }

    pub fn require_signature_identity(&self, signer_address: &str) -> ApiResult<()> {
        if self.stellar_address == signer_address {
            Ok(())
        } else {
            Err(ApiError::forbidden_with_error(
                "SignatureIdentityMismatch",
                "The signing address does not match the authenticated account",
            ))
        }
    }

    pub fn require_transfer_action(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        accepting: bool,
    ) -> ApiResult<()> {
        let actor_id = self.publisher_id()?;
        let is_sender = actor_id == sender_id;
        let is_recipient = actor_id == recipient_id;

        if !is_sender && !is_recipient {
            return Err(ApiError::forbidden_with_error(
                "UnauthorizedConfirmation",
                "Only the sender or recipient of this transfer can act on it",
            ));
        }
        if accepting && !is_recipient {
            return Err(ApiError::forbidden_with_error(
                "UnauthorizedConfirmation",
                "Only the recipient of this transfer can accept it",
            ));
        }

        Ok(())
    }
}

pub fn require_admin_claims(claims: &AuthClaims) -> ApiResult<()> {
    if claims_are_admin(claims) {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "Administrative privileges are required for this action",
        ))
    }
}

pub async fn require_contract_owner(
    state: &AppState,
    actor: &PolicyActor,
    contract_id: Uuid,
) -> ApiResult<()> {
    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT publisher_id FROM contracts WHERE id = $1")
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("resolve contract owner", err))?;

    let owner_id =
        owner_id.ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;
    actor.require_contract_owner(owner_id)
}

pub async fn require_webhook_owner(
    state: &AppState,
    actor: &PolicyActor,
    webhook_id: Uuid,
) -> ApiResult<()> {
    let owner_id: Option<Option<Uuid>> =
        sqlx::query_scalar("SELECT user_id FROM webhook_configurations WHERE id = $1")
            .bind(webhook_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("resolve webhook owner", err))?;

    let owner_id = owner_id
        .flatten()
        .ok_or_else(|| ApiError::not_found("webhook", "Webhook not found"))?;
    actor
        .require_resource_owner(owner_id, "webhook")
        .map_err(|_| ApiError::not_found("webhook", "Webhook not found"))
}

pub async fn require_webhook_delivery_owner(
    state: &AppState,
    actor: &PolicyActor,
    delivery_id: Uuid,
) -> ApiResult<()> {
    let owner_id: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT wc.user_id
         FROM notification_delivery_logs ndl
         JOIN webhook_configurations wc ON wc.id = ndl.webhook_id
         WHERE ndl.id = $1",
    )
    .bind(delivery_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("resolve webhook delivery owner", err))?;

    let owner_id = owner_id
        .flatten()
        .ok_or_else(|| ApiError::not_found("delivery", "Delivery log not found"))?;
    actor
        .require_resource_owner(owner_id, "webhook delivery")
        .map_err(|_| ApiError::not_found("delivery", "Delivery log not found"))
}

fn claims_are_admin(claims: &AuthClaims) -> bool {
    claims.admin
        || claims
            .role
            .as_deref()
            .is_some_and(|role| role.eq_ignore_ascii_case("admin"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(publisher_id: Option<Uuid>, address: &str, is_admin: bool) -> PolicyActor {
        PolicyActor {
            publisher_id,
            stellar_address: address.to_string(),
            is_admin,
        }
    }

    #[test]
    fn publisher_policy_requires_a_registered_identity() {
        let publisher_id = Uuid::new_v4();
        assert_eq!(
            actor(Some(publisher_id), "GOWNER", false)
                .publisher_id()
                .unwrap(),
            publisher_id
        );
        assert!(actor(None, "GUNKNOWN", false).publisher_id().is_err());
        assert!(actor(None, "GADMIN", true).publisher_id().is_err());
    }

    #[test]
    fn owner_policy_has_no_admin_bypass() {
        let owner_id = Uuid::new_v4();
        assert!(actor(Some(owner_id), "GOWNER", false)
            .require_contract_owner(owner_id)
            .is_ok());
        assert!(actor(Some(Uuid::new_v4()), "GOTHER", false)
            .require_contract_owner(owner_id)
            .is_err());
        assert!(actor(Some(Uuid::new_v4()), "GADMIN", true)
            .require_contract_owner(owner_id)
            .is_err());
    }

    #[test]
    fn admin_policy_rejects_publishers() {
        assert!(actor(None, "GADMIN", true).require_admin().is_ok());
        assert!(actor(Some(Uuid::new_v4()), "GPUBLISHER", false)
            .require_admin()
            .is_err());
    }

    #[test]
    fn admin_claims_are_case_insensitive_and_explicit() {
        let claims = |role: Option<&str>, admin| AuthClaims {
            sub: "GSUBJECT".to_string(),
            publisher_id: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            scopes: vec![],
            role: role.map(str::to_string),
            admin,
            mfa_verified: false,
            session_id: None,
        };

        assert!(require_admin_claims(&claims(Some("AdMiN"), false)).is_ok());
        assert!(require_admin_claims(&claims(None, true)).is_ok());
        assert!(require_admin_claims(&claims(Some("publisher"), false)).is_err());
    }

    #[test]
    fn publisher_and_signature_addresses_must_match() {
        let owner = actor(Some(Uuid::new_v4()), "GOWNER", false);
        assert!(owner.require_publisher_address("GOWNER").is_ok());
        assert!(owner.require_publisher_address("GOTHER").is_err());
        assert!(owner.require_signature_identity("GOTHER").is_err());
    }

    #[test]
    fn transfer_policy_enforces_participant_roles() {
        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();
        let sender = actor(Some(sender_id), "GSENDER", false);
        let recipient = actor(Some(recipient_id), "GRECIPIENT", false);
        let outsider = actor(Some(Uuid::new_v4()), "GOUTSIDER", false);

        assert!(sender
            .require_transfer_action(sender_id, recipient_id, false)
            .is_ok());
        assert!(sender
            .require_transfer_action(sender_id, recipient_id, true)
            .is_err());
        assert!(recipient
            .require_transfer_action(sender_id, recipient_id, true)
            .is_ok());
        assert!(recipient
            .require_transfer_action(sender_id, recipient_id, false)
            .is_ok());
        assert!(outsider
            .require_transfer_action(sender_id, recipient_id, false)
            .is_err());
    }
}
