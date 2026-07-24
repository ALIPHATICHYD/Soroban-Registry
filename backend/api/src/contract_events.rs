use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use shared::{Contract, ContractVersion};
use tokio::sync::broadcast;
use tokio::time::{interval, MissedTickBehavior};
use uuid::Uuid;

use crate::auth::AuthClaims;
use crate::state::{AppState, ContractEventVisibility, RealtimeEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEventContract {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub network: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub wasm_hash: String,
    pub is_verified: bool,
}

impl From<&Contract> for ContractEventContract {
    fn from(contract: &Contract) -> Self {
        Self {
            id: contract.id,
            contract_id: contract.contract_id.clone(),
            name: contract.name.clone(),
            description: contract.description.clone(),
            publisher_id: contract.publisher_id,
            network: contract.network.to_string(),
            category: contract.category.clone(),
            tags: contract.tags.iter().map(|t| t.name.clone()).collect(),
            wasm_hash: contract.wasm_hash.clone(),
            is_verified: contract.is_verified,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEventEnvelope {
    pub event_type: String,
    pub visibility: ContractEventVisibility,
    pub contract: ContractEventContract,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ContractEventEnvelope {
    pub fn version_created(contract: &Contract, version: &ContractVersion) -> RealtimeEvent {
        RealtimeEvent::VersionCreated {
            contract_id: contract.contract_id.clone(),
            version: version.version.clone(),
            network: contract.network.clone(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn deployed(contract: &Contract, publisher_address: Option<String>) -> RealtimeEvent {
        RealtimeEvent::ContractDeployed {
            contract_id: contract.contract_id.clone(),
            contract_name: contract.name.clone(),
            publisher: publisher_address.unwrap_or_default(),
            version: "".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            network: contract.network.clone(),
        }
    }

    pub fn metadata_updated(
        contract: &Contract,
        changes: serde_json::Value,
        visibility: ContractEventVisibility,
    ) -> RealtimeEvent {
        RealtimeEvent::MetadataUpdated {
            contract_id: contract.contract_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            changes,
            visibility,
        }
    }

    pub fn status_updated(
        contract: &Contract,
        status: String,
        is_verified: bool,
        details: Option<serde_json::Value>,
        visibility: ContractEventVisibility,
    ) -> RealtimeEvent {
        RealtimeEvent::StatusUpdated {
            contract_id: contract.contract_id.clone(),
            status,
            timestamp: Utc::now().to_rfc3339(),
            is_verified,
            details,
            visibility,
        }
    }
}

const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 30_000;
const DEFAULT_RECONNECT_AFTER_MS: u64 = 5_000;
const DEFAULT_EVENT_BUFFER: usize = 4_096;

#[derive(Debug)]
pub struct ContractEventHub {
    tx: broadcast::Sender<RealtimeEvent>,
    next_connection_id: AtomicU64,
    heartbeat_interval: Duration,
    reconnect_after: Duration,
}

impl ContractEventHub {
    pub fn from_env() -> Self {
        let buffer = std::env::var("CONTRACT_EVENT_BUFFER_SIZE")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_EVENT_BUFFER);
        let heartbeat_interval = std::env::var("WS_HEARTBEAT_INTERVAL_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_MS);
        let reconnect_after = std::env::var("WS_RECONNECT_AFTER_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RECONNECT_AFTER_MS);
        let (tx, _) = broadcast::channel(buffer);

        Self {
            tx,
            next_connection_id: AtomicU64::new(1),
            heartbeat_interval: Duration::from_millis(heartbeat_interval),
            reconnect_after: Duration::from_millis(reconnect_after),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: RealtimeEvent) {
        let _ = self.tx.send(event);
    }

    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn heartbeat_interval_ms(&self) -> u64 {
        self.heartbeat_interval.as_millis() as u64
    }

    pub fn reconnect_after_ms(&self) -> u64 {
        self.reconnect_after.as_millis() as u64
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ContractWsQuery {
    #[serde(default)]
    pub contract_id: Vec<String>,
    #[serde(default)]
    pub category: Vec<String>,
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub include_private: bool,
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SubscriptionFilter {
    contract_ids: HashSet<String>,
    categories: HashSet<String>,
    networks: HashSet<String>,
    include_private: bool,
}

impl SubscriptionFilter {
    fn from_query(query: &ContractWsQuery) -> Self {
        Self {
            contract_ids: normalize_values(&query.contract_id),
            categories: normalize_values(&query.category),
            networks: normalize_values(&query.network),
            include_private: query.include_private,
        }
    }

    fn apply_subscribe(&mut self, cmd: SubscribeCommand, is_authenticated: bool) -> Option<String> {
        if let Some(contract_ids) = cmd.contract_ids {
            self.contract_ids = normalize_values(&contract_ids);
        }
        if let Some(categories) = cmd.categories {
            self.categories = normalize_values(&categories);
        }
        if let Some(networks) = cmd.networks {
            self.networks = normalize_values(&networks);
        }
        if let Some(include_private) = cmd.include_private {
            self.include_private = include_private && is_authenticated;
            if include_private && !is_authenticated {
                return Some(
                    "private updates require a valid bearer token via Authorization or access_token"
                        .to_string(),
                );
            }
        }
        None
    }

    #[allow(dead_code)]
    fn matches(&self, event: &RealtimeEvent) -> bool {
        let (contract_id, network, visibility) = match event {
            RealtimeEvent::ContractDeployed {
                contract_id,
                network,
                ..
            } => (contract_id.as_str(), Some(network.to_string()), None),
            RealtimeEvent::ContractUpdated { contract_id, .. } => {
                (contract_id.as_str(), None, None)
            }
            RealtimeEvent::CicdPipeline { contract_id, .. } => (contract_id.as_str(), None, None),
            RealtimeEvent::VersionCreated {
                contract_id,
                network,
                ..
            } => (contract_id.as_str(), Some(network.to_string()), None),
            RealtimeEvent::MetadataUpdated {
                contract_id,
                visibility,
                ..
            } => (contract_id.as_str(), None, Some(visibility)),
            RealtimeEvent::StatusUpdated {
                contract_id,
                visibility,
                ..
            } => (contract_id.as_str(), None, Some(visibility)),
        };

        if !self.contract_ids.is_empty() {
            let contract_id = contract_id.to_ascii_lowercase();
            if !self.contract_ids.contains(&contract_id) {
                return false;
            }
        }

        if !self.networks.is_empty() {
            match network {
                Some(network_name) => {
                    if !self.networks.contains(&network_name.to_ascii_lowercase()) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        if !self.include_private && matches!(visibility, Some(ContractEventVisibility::Private)) {
            return false;
        }

        true
    }
}

fn normalize_values(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ClientCommand {
    Subscribe(SubscribeCommand),
    UnsubscribeAll,
    Ping,
}

#[derive(Debug, Deserialize)]
struct SubscribeCommand {
    contract_ids: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    networks: Option<Vec<String>>,
    include_private: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Connected {
        connection_id: u64,
        authenticated: bool,
        heartbeat_interval_ms: u64,
        reconnect_after_ms: u64,
        subscriptions: SubscriptionSnapshot,
    },
    Subscribed {
        subscriptions: SubscriptionSnapshot,
        warning: Option<String>,
    },
    #[allow(dead_code)]
    Event { event: RealtimeEvent },
    Heartbeat {
        timestamp: chrono::DateTime<chrono::Utc>,
        reconnect_after_ms: u64,
    },
    Error {
        message: String,
        reconnect_after_ms: u64,
    },
    #[allow(dead_code)]
    ResyncRequired {
        dropped_events: u64,
        reconnect_after_ms: u64,
    },
}

#[derive(Debug, Serialize)]
struct SubscriptionSnapshot {
    contract_ids: Vec<String>,
    categories: Vec<String>,
    networks: Vec<String>,
    include_private: bool,
}

impl From<&SubscriptionFilter> for SubscriptionSnapshot {
    fn from(filter: &SubscriptionFilter) -> Self {
        let mut contract_ids = filter.contract_ids.iter().cloned().collect::<Vec<_>>();
        let mut categories = filter.categories.iter().cloned().collect::<Vec<_>>();
        let mut networks = filter.networks.iter().cloned().collect::<Vec<_>>();
        contract_ids.sort();
        categories.sort();
        networks.sort();

        Self {
            contract_ids,
            categories,
            networks,
            include_private: filter.include_private,
        }
    }
}

pub async fn contracts_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ContractWsQuery>,
) -> Response {
    let claims = authenticate_connection(&state, &headers, query.access_token.as_deref());
    let is_authenticated = claims.is_some();
    let mut filter = SubscriptionFilter::from_query(&query);
    let auth_warning = if filter.include_private && !is_authenticated {
        filter.include_private = false;
        Some("private updates require authentication".to_string())
    } else {
        None
    };

    ws.on_upgrade(move |socket| handle_socket(state, socket, filter, claims, auth_warning))
}

fn authenticate_connection(
    state: &AppState,
    headers: &HeaderMap,
    access_token: Option<&str>,
) -> Option<AuthClaims> {
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let token = bearer.or(access_token
        .map(str::trim)
        .filter(|value| !value.is_empty()))?;
    let auth = state.auth_mgr.read().ok()?;
    auth.validate_jwt(token).ok()
}

async fn handle_socket(
    state: AppState,
    socket: WebSocket,
    mut filter: SubscriptionFilter,
    claims: Option<AuthClaims>,
    auth_warning: Option<String>,
) {
    // Use u64 counter for connection ID
    static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);
    let connection_id = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.contract_events.subscribe();

    // Fixed intervals for heartbeat and reconnect
    let heartbeat_ms: u64 = 30_000; // 30 seconds
    let reconnect_ms: u64 = 5_000; // 5 seconds

    let connected = ServerMessage::Connected {
        connection_id,
        authenticated: claims.is_some(),
        heartbeat_interval_ms: heartbeat_ms,
        reconnect_after_ms: reconnect_ms,
        subscriptions: SubscriptionSnapshot::from(&filter),
    };
    if send_json(&mut sender, &connected).await.is_err() {
        return;
    }

    if let Some(warning) = auth_warning {
        let msg = ServerMessage::Error {
            message: warning,
            reconnect_after_ms: reconnect_ms,
        };
        if send_json(&mut sender, &msg).await.is_err() {
            return;
        }
    }

    let mut heartbeat = interval(Duration::from_millis(heartbeat_ms));
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            message = receiver.next() => {
                match message {
                    Some(Ok(message)) => {
                        if handle_client_message(&mut sender, message, &mut filter, claims.is_some(), reconnect_ms).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) | None => break,
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        if filter.matches(&event) {
                            let msg = ServerMessage::Event { event: event.clone() };
                            if send_json(&mut sender, &msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let msg = serde_json::json!({
                            "type": "resync_required",
                            "dropped_events": skipped,
                            "reconnect_after_ms": reconnect_ms,
                        });
                        if let Ok(msg_str) = serde_json::to_string(&msg) {
                            if sender.send(Message::Text(msg_str)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = heartbeat.tick() => {
                let msg = ServerMessage::Heartbeat {
                    timestamp: Utc::now(),
                    reconnect_after_ms: reconnect_ms,
                };
                if send_json(&mut sender, &msg).await.is_err() {
                    break;
                }
                if sender.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn handle_client_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: Message,
    filter: &mut SubscriptionFilter,
    is_authenticated: bool,
    reconnect_after_ms: u64,
) -> Result<(), ()> {
    match message {
        Message::Text(text) => {
            let cmd: ClientCommand = serde_json::from_str(text.as_str()).map_err(|_| ())?;
            match cmd {
                ClientCommand::Subscribe(cmd) => {
                    let warning = filter.apply_subscribe(cmd, is_authenticated);
                    let msg = ServerMessage::Subscribed {
                        subscriptions: SubscriptionSnapshot::from(&*filter),
                        warning,
                    };
                    send_json(sender, &msg).await.map_err(|_| ())?;
                }
                ClientCommand::UnsubscribeAll => {
                    *filter = SubscriptionFilter::default();
                    let msg = ServerMessage::Subscribed {
                        subscriptions: SubscriptionSnapshot::from(&*filter),
                        warning: None,
                    };
                    send_json(sender, &msg).await.map_err(|_| ())?;
                }
                ClientCommand::Ping => {
                    let msg = ServerMessage::Heartbeat {
                        timestamp: Utc::now(),
                        reconnect_after_ms,
                    };
                    send_json(sender, &msg).await.map_err(|_| ())?;
                }
            }
        }
        Message::Close(_) => return Err(()),
        Message::Ping(payload) => {
            sender.send(Message::Pong(payload)).await.map_err(|_| ())?;
        }
        Message::Pong(_) | Message::Binary(_) => {}
    }

    Ok(())
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), axum::Error> {
    let payload =
        serde_json::to_string(message).expect("server websocket messages should always serialize");
    sender.send(Message::Text(payload)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Network;

    fn sample_contract() -> Contract {
        Contract {
            id: Uuid::nil(),
            contract_id: "CABC123".to_string(),
            wasm_hash: "wasm-hash".to_string(),
            name: "Sample".to_string(),
            slug: "sample".to_string(),
            description: Some("sample contract".to_string()),
            publisher_id: Uuid::nil(),
            network: Network::Testnet,
            is_verified: false,
            verification_status: shared::VerificationStatus::Pending,
            category: Some("DeFi".to_string()),
            tags: vec![shared::Tag {
                id: Uuid::nil(),
                name: "amm".to_string(),
                color: "#888888".to_string(),
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_at: None,
            deployed_at: None,
            verified_by: None,
            verification_notes: None,
            health_score: 0,
            is_maintenance: false,
            logical_id: None,
            network_configs: None,
            last_accessed_at: None,
            relevance_score: None,
            organization_id: None,
            visibility: shared::VisibilityType::Public,
            current_version: None,
            usage_count: 0,
        }
    }

    #[test]
    fn filters_match_network() {
        let event = ContractEventEnvelope::deployed(&sample_contract(), None);
        let filter = SubscriptionFilter {
            contract_ids: HashSet::new(),
            categories: HashSet::new(),
            networks: normalize_values(&["testnet".to_string()]),
            include_private: false,
        };

        assert!(filter.matches(&event));
    }

    #[test]
    fn filters_reject_private_when_not_enabled() {
        let event = ContractEventEnvelope::metadata_updated(
            &sample_contract(),
            serde_json::json!({"name": {"before": "A", "after": "B"}}),
            ContractEventVisibility::Private,
        );
        let filter = SubscriptionFilter::default();

        assert!(!filter.matches(&event));
    }

    #[test]
    fn unauthenticated_private_subscribe_is_downgraded() {
        let mut filter = SubscriptionFilter::default();
        let warning = filter.apply_subscribe(
            SubscribeCommand {
                contract_ids: None,
                categories: None,
                networks: None,
                include_private: Some(true),
            },
            false,
        );

        assert_eq!(
            warning.as_deref(),
            Some("private updates require a valid bearer token via Authorization or access_token")
        );
        assert!(!filter.include_private);
    }
}
