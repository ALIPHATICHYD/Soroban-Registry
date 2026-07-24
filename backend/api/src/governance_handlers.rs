use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "governance_model", rename_all = "snake_case")]
pub enum GovernanceModel {
    TokenWeighted,
    Quadratic,
    Multisig,
    Timelock,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "governance_proposal_status", rename_all = "snake_case")]
pub enum GovernanceProposalStatus {
    Pending,
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "vote_choice", rename_all = "lowercase")]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

impl VoteChoice {
    fn as_str(self) -> &'static str {
        match self {
            Self::For => "for",
            Self::Against => "against",
            Self::Abstain => "abstain",
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateProposalRequest {
    pub contract_id: Uuid,
    pub title: String,
    pub description: String,
    pub governance_model: GovernanceModel,
    pub proposer: Uuid,
    pub voting_starts_at: Option<DateTime<Utc>>,
    pub voting_ends_at: DateTime<Utc>,
    pub execution_delay_hours: Option<i32>,
    pub quorum_required: Option<i32>,
    pub approval_threshold: Option<i32>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CastVoteRequest {
    pub voter: Uuid,
    pub vote_choice: VoteChoice,
    pub delegated_from: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpsertVotingRightsRequest {
    pub publisher_id: Uuid,
    pub voting_power: i64,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListProposalsQuery {
    pub status: Option<String>,
    pub contract_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct GovernanceProposal {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub title: String,
    pub description: String,
    pub governance_model: GovernanceModel,
    pub proposer: Uuid,
    pub status: GovernanceProposalStatus,
    pub voting_starts_at: DateTime<Utc>,
    pub voting_ends_at: DateTime<Utc>,
    pub execution_delay_hours: i32,
    pub quorum_required: i32,
    pub approval_threshold: i32,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct GovernanceVote {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub voter: Uuid,
    pub vote_choice: VoteChoice,
    pub voting_power: i64,
    pub delegated_from: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct GovernanceVotingRight {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub publisher_id: Uuid,
    pub voting_power: i64,
    pub source: String,
    pub synced_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GovernanceVoteTally {
    pub proposal_id: Uuid,
    pub votes_for: i64,
    pub votes_against: i64,
    pub votes_abstain: i64,
    pub total_voting_power: i64,
    pub quorum_required: i32,
    pub approval_threshold: i32,
    pub quorum_met: bool,
    pub approval_met: bool,
    pub status: GovernanceProposalStatus,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ExecuteProposalResponse {
    pub proposal_id: Uuid,
    pub status: GovernanceProposalStatus,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ProposalMeta {
    id: Uuid,
    contract_id: Uuid,
    status: GovernanceProposalStatus,
    voting_starts_at: DateTime<Utc>,
    voting_ends_at: DateTime<Utc>,
    execution_delay_hours: i32,
    quorum_required: i32,
    approval_threshold: i32,
}

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn parse_status_filter(status: Option<&str>) -> ApiResult<Option<GovernanceProposalStatus>> {
    match status {
        Some("pending") => Ok(Some(GovernanceProposalStatus::Pending)),
        Some("active") => Ok(Some(GovernanceProposalStatus::Active)),
        Some("passed") => Ok(Some(GovernanceProposalStatus::Passed)),
        Some("rejected") => Ok(Some(GovernanceProposalStatus::Rejected)),
        Some("executed") => Ok(Some(GovernanceProposalStatus::Executed)),
        Some("cancelled") => Ok(Some(GovernanceProposalStatus::Cancelled)),
        Some(_) => Err(ApiError::bad_request_with(
            "InvalidStatus",
            "status must be one of: pending, active, passed, rejected, executed, cancelled",
        )),
        None => Ok(None),
    }
}

async fn evaluate_proposal_status(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    proposal_id: Uuid,
) -> Result<GovernanceProposalStatus, ApiError> {
    let proposal = sqlx::query_as::<_, ProposalMeta>(
        "SELECT id, contract_id, status, voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required, approval_threshold
         FROM governance_proposals
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(proposal_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| db_err("load governance proposal for status evaluation", e))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "governance proposal not found"))?;

    if proposal.status == GovernanceProposalStatus::Executed
        || proposal.status == GovernanceProposalStatus::Cancelled
    {
        return Ok(proposal.status);
    }

    let now = Utc::now();

    if now < proposal.voting_starts_at {
        let status = sqlx::query_scalar::<_, GovernanceProposalStatus>(
            "UPDATE governance_proposals
             SET status = 'pending'
             WHERE id = $1
             RETURNING status",
        )
        .bind(proposal.id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| db_err("set governance proposal pending", e))?;
        return Ok(status);
    }

    let tally = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT
            COALESCE(SUM(CASE WHEN vote_choice = 'for' THEN voting_power ELSE 0 END), 0)::BIGINT AS votes_for,
            COALESCE(SUM(CASE WHEN vote_choice = 'against' THEN voting_power ELSE 0 END), 0)::BIGINT AS votes_against,
            COALESCE(SUM(CASE WHEN vote_choice = 'abstain' THEN voting_power ELSE 0 END), 0)::BIGINT AS votes_abstain
         FROM governance_votes
         WHERE proposal_id = $1",
    )
    .bind(proposal.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("calculate governance vote tally", e))?;

    let (votes_for, votes_against, votes_abstain) = tally;
    let total_voting_power = votes_for + votes_against + votes_abstain;
    let quorum_met = total_voting_power >= i64::from(proposal.quorum_required);
    let decisive_votes = votes_for + votes_against;
    let approval_met = if decisive_votes == 0 {
        false
    } else {
        (votes_for * 100) >= (decisive_votes * i64::from(proposal.approval_threshold))
    };

    let new_status = if now < proposal.voting_ends_at {
        GovernanceProposalStatus::Active
    } else if quorum_met && approval_met {
        GovernanceProposalStatus::Passed
    } else {
        GovernanceProposalStatus::Rejected
    };

    let final_status = if new_status == GovernanceProposalStatus::Passed
        && now
            >= proposal.voting_ends_at + Duration::hours(i64::from(proposal.execution_delay_hours))
    {
        GovernanceProposalStatus::Executed
    } else {
        new_status
    };

    let status = sqlx::query_scalar::<_, GovernanceProposalStatus>(
        "UPDATE governance_proposals
         SET status = $2,
             executed_at = CASE WHEN $2 = 'executed'::governance_proposal_status THEN COALESCE(executed_at, NOW()) ELSE executed_at END
         WHERE id = $1
         RETURNING status",
    )
    .bind(proposal.id)
    .bind(final_status)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("update governance proposal status", e))?;

    Ok(status)
}

#[utoipa::path(
    post,
    path = "/api/governance/proposals",
    request_body = CreateProposalRequest,
    responses(
        (status = 200, description = "Governance proposal created", body = GovernanceProposal),
        (status = 400, description = "Invalid input")
    ),
    tag = "Governance"
)]
pub async fn create_proposal(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateProposalRequest>,
) -> ApiResult<Json<GovernanceProposal>> {
    if payload.title.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidTitle",
            "title cannot be empty",
        ));
    }
    if payload.description.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidDescription",
            "description cannot be empty",
        ));
    }

    let voting_starts_at = payload.voting_starts_at.unwrap_or_else(Utc::now);
    if payload.voting_ends_at <= voting_starts_at {
        return Err(ApiError::bad_request(
            "InvalidVotingWindow",
            "voting_ends_at must be after voting_starts_at",
        ));
    }

    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
        .bind(payload.contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_err("check contract exists for governance proposal", e))?;

    if exists.is_none() {
        return Err(ApiError::not_found(
            "ContractNotFound",
            "contract not found for governance proposal",
        ));
    }

    let proposer_exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM publishers WHERE id = $1")
            .bind(payload.proposer)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_err("check proposer exists", e))?;

    if proposer_exists.is_none() {
        return Err(ApiError::not_found(
            "PublisherNotFound",
            "proposer publisher not found",
        ));
    }

    let proposal: GovernanceProposal = sqlx::query_as(
        "INSERT INTO governance_proposals (
            contract_id, title, description, governance_model, proposer,
            status, voting_starts_at, voting_ends_at, execution_delay_hours,
            quorum_required, approval_threshold
         )
         VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, $9, $10)
         RETURNING
            id, contract_id, title, description, governance_model, proposer,
            status, voting_starts_at, voting_ends_at, execution_delay_hours,
            quorum_required, approval_threshold, created_at, executed_at",
    )
    .bind(payload.contract_id)
    .bind(payload.title.trim())
    .bind(payload.description.trim())
    .bind(payload.governance_model)
    .bind(payload.proposer)
    .bind(voting_starts_at)
    .bind(payload.voting_ends_at)
    .bind(payload.execution_delay_hours.unwrap_or(0))
    .bind(payload.quorum_required.unwrap_or(50))
    .bind(payload.approval_threshold.unwrap_or(50))
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("create governance proposal", e))?;

    Ok(Json(proposal))
}

#[utoipa::path(
    get,
    path = "/api/governance/proposals",
    params(ListProposalsQuery),
    responses((status = 200, description = "List governance proposals", body = Vec<GovernanceProposal>)),
    tag = "Governance"
)]
pub async fn list_proposals(
    State(state): State<AppState>,
    Query(query): Query<ListProposalsQuery>,
) -> ApiResult<Json<Vec<GovernanceProposal>>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let status_filter = parse_status_filter(query.status.as_deref())?;

    let proposals: Vec<GovernanceProposal> = sqlx::query_as(
        "SELECT
            id, contract_id, title, description, governance_model, proposer,
            status, voting_starts_at, voting_ends_at, execution_delay_hours,
            quorum_required, approval_threshold, created_at, executed_at
         FROM governance_proposals
         WHERE ($1::governance_proposal_status IS NULL OR status = $1)
           AND ($2::UUID IS NULL OR contract_id = $2)
         ORDER BY created_at DESC
         LIMIT $3",
    )
    .bind(status_filter)
    .bind(query.contract_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list governance proposals", e))?;

    Ok(Json(proposals))
}

#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}",
    params(("id" = String, Path, description = "Governance proposal UUID")),
    responses((status = 200, description = "Governance proposal", body = GovernanceProposal)),
    tag = "Governance"
)]
pub async fn get_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<GovernanceProposal>> {
    let proposal_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidProposalId", "proposal id must be a UUID"))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_err("begin governance proposal fetch transaction", e))?;

    let _ = evaluate_proposal_status(&mut tx, proposal_id).await?;

    let proposal: GovernanceProposal = sqlx::query_as(
        "SELECT
            id, contract_id, title, description, governance_model, proposer,
            status, voting_starts_at, voting_ends_at, execution_delay_hours,
            quorum_required, approval_threshold, created_at, executed_at
         FROM governance_proposals
         WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("fetch governance proposal", e))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "governance proposal not found"))?;

    tx.commit()
        .await
        .map_err(|e| db_err("commit governance proposal fetch transaction", e))?;

    Ok(Json(proposal))
}

#[utoipa::path(
    post,
    path = "/api/governance/proposals/{id}/votes",
    params(("id" = String, Path, description = "Governance proposal UUID")),
    request_body = CastVoteRequest,
    responses(
        (status = 200, description = "Vote recorded", body = GovernanceVote),
        (status = 409, description = "Voting is closed")
    ),
    tag = "Governance"
)]
pub async fn cast_vote(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<CastVoteRequest>,
) -> ApiResult<Json<GovernanceVote>> {
    let proposal_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidProposalId", "proposal id must be a UUID"))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_err("begin cast governance vote transaction", e))?;

    let proposal = sqlx::query_as::<_, ProposalMeta>(
        "SELECT id, contract_id, status, voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required, approval_threshold
         FROM governance_proposals
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("load governance proposal for vote", e))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "governance proposal not found"))?;

    let now = Utc::now();
    if now < proposal.voting_starts_at || now > proposal.voting_ends_at {
        return Err(ApiError::conflict(
            "VotingClosed",
            "voting is not active for this proposal",
        ));
    }

    let base_power: i64 = sqlx::query_scalar(
        "SELECT voting_power
         FROM governance_voting_rights
         WHERE contract_id = $1 AND publisher_id = $2",
    )
    .bind(proposal.contract_id)
    .bind(payload.voter)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("load voting rights for voter", e))?
    .unwrap_or(1);

    let delegated_extra: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(voting_power), 0)::BIGINT
         FROM governance_voting_rights vr
         JOIN vote_delegations vd
           ON vd.delegator = vr.publisher_id
          AND vd.delegate = $2
          AND vd.active = TRUE
          AND (vd.contract_id IS NULL OR vd.contract_id = $1)
         WHERE vr.contract_id = $1",
    )
    .bind(proposal.contract_id)
    .bind(payload.voter)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("load delegated voting power", e))?;

    let voting_power = base_power + delegated_extra;

    let vote: GovernanceVote = sqlx::query_as(
        "INSERT INTO governance_votes (
            proposal_id, voter, vote_choice, voting_power, delegated_from
         )
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (proposal_id, voter) DO UPDATE SET
            vote_choice = EXCLUDED.vote_choice,
            voting_power = EXCLUDED.voting_power,
            delegated_from = EXCLUDED.delegated_from,
            created_at = NOW()
         RETURNING id, proposal_id, voter, vote_choice, voting_power, delegated_from, created_at",
    )
    .bind(proposal_id)
    .bind(payload.voter)
    .bind(payload.vote_choice)
    .bind(voting_power)
    .bind(payload.delegated_from)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("insert governance vote", e))?;

    let _ = evaluate_proposal_status(&mut tx, proposal_id).await?;

    tx.commit()
        .await
        .map_err(|e| db_err("commit governance vote transaction", e))?;

    Ok(Json(vote))
}

#[utoipa::path(
    get,
    path = "/api/governance/proposals/{id}/votes",
    params(("id" = String, Path, description = "Governance proposal UUID")),
    responses((status = 200, description = "Vote tally", body = GovernanceVoteTally)),
    tag = "Governance"
)]
pub async fn get_vote_tally(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<GovernanceVoteTally>> {
    let proposal_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidProposalId", "proposal id must be a UUID"))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_err("begin governance tally transaction", e))?;

    let status = evaluate_proposal_status(&mut tx, proposal_id).await?;

    let proposal = sqlx::query_as::<_, ProposalMeta>(
        "SELECT id, contract_id, status, voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required, approval_threshold
         FROM governance_proposals
         WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("fetch governance proposal for tally", e))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "governance proposal not found"))?;

    let (votes_for, votes_against, votes_abstain) =
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT
                COALESCE(SUM(CASE WHEN vote_choice = 'for' THEN voting_power ELSE 0 END), 0)::BIGINT,
                COALESCE(SUM(CASE WHEN vote_choice = 'against' THEN voting_power ELSE 0 END), 0)::BIGINT,
                COALESCE(SUM(CASE WHEN vote_choice = 'abstain' THEN voting_power ELSE 0 END), 0)::BIGINT
             FROM governance_votes
             WHERE proposal_id = $1",
        )
        .bind(proposal_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| db_err("fetch governance vote totals", e))?;

    let total_voting_power = votes_for + votes_against + votes_abstain;
    let quorum_met = total_voting_power >= i64::from(proposal.quorum_required);
    let decisive_votes = votes_for + votes_against;
    let approval_met = if decisive_votes == 0 {
        false
    } else {
        (votes_for * 100) >= (decisive_votes * i64::from(proposal.approval_threshold))
    };

    tx.commit()
        .await
        .map_err(|e| db_err("commit governance tally transaction", e))?;

    Ok(Json(GovernanceVoteTally {
        proposal_id,
        votes_for,
        votes_against,
        votes_abstain,
        total_voting_power,
        quorum_required: proposal.quorum_required,
        approval_threshold: proposal.approval_threshold,
        quorum_met,
        approval_met,
        status,
    }))
}

#[utoipa::path(
    post,
    path = "/api/governance/proposals/{id}/execute",
    params(("id" = String, Path, description = "Governance proposal UUID")),
    responses((status = 200, description = "Proposal executed", body = ExecuteProposalResponse)),
    tag = "Governance"
)]
pub async fn execute_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ExecuteProposalResponse>> {
    let proposal_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidProposalId", "proposal id must be a UUID"))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_err("begin governance execute transaction", e))?;

    let status = evaluate_proposal_status(&mut tx, proposal_id).await?;
    if status != GovernanceProposalStatus::Passed {
        return Err(ApiError::conflict(
            "ProposalNotPassed",
            "proposal must be passed before execution",
        ));
    }

    let proposal = sqlx::query_as::<_, ProposalMeta>(
        "SELECT id, contract_id, status, voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required, approval_threshold
         FROM governance_proposals
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("load governance proposal for execution", e))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "governance proposal not found"))?;

    let earliest_execution =
        proposal.voting_ends_at + Duration::hours(i64::from(proposal.execution_delay_hours));
    if Utc::now() < earliest_execution {
        return Err(ApiError::conflict(
            "ExecutionDelayPending",
            "proposal execution delay period has not elapsed",
        ));
    }

    let executed_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "UPDATE governance_proposals
         SET status = 'executed', executed_at = NOW()
         WHERE id = $1
         RETURNING executed_at",
    )
    .bind(proposal_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("mark governance proposal executed", e))?;

    tx.commit()
        .await
        .map_err(|e| db_err("commit governance execute transaction", e))?;

    Ok(Json(ExecuteProposalResponse {
        proposal_id,
        status: GovernanceProposalStatus::Executed,
        executed_at,
    }))
}

#[utoipa::path(
    get,
    path = "/api/governance/contracts/{id}/voting-rights",
    params(("id" = String, Path, description = "Contract UUID")),
    responses((status = 200, description = "Voting rights for contract", body = Vec<GovernanceVotingRight>)),
    tag = "Governance"
)]
pub async fn list_voting_rights(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<GovernanceVotingRight>>> {
    let contract_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", "contract id must be a UUID"))?;

    let rows = sqlx::query_as::<_, GovernanceVotingRight>(
        "SELECT id, contract_id, publisher_id, voting_power, source, synced_at, created_at, updated_at
         FROM governance_voting_rights
         WHERE contract_id = $1
         ORDER BY voting_power DESC, publisher_id",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list governance voting rights", e))?;

    Ok(Json(rows))
}

#[utoipa::path(
    post,
    path = "/api/governance/contracts/{id}/voting-rights",
    params(("id" = String, Path, description = "Contract UUID")),
    request_body = UpsertVotingRightsRequest,
    responses((status = 200, description = "Voting rights upserted", body = GovernanceVotingRight)),
    tag = "Governance"
)]
pub async fn upsert_voting_rights(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpsertVotingRightsRequest>,
) -> ApiResult<Json<GovernanceVotingRight>> {
    let contract_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", "contract id must be a UUID"))?;

    if payload.voting_power < 0 {
        return Err(ApiError::bad_request(
            "InvalidVotingPower",
            "voting_power must be greater than or equal to 0",
        ));
    }

    let source = payload.source.unwrap_or_else(|| "manual".to_string());

    let row = sqlx::query_as::<_, GovernanceVotingRight>(
        "INSERT INTO governance_voting_rights (
            contract_id, publisher_id, voting_power, source, synced_at
         )
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (contract_id, publisher_id) DO UPDATE SET
            voting_power = EXCLUDED.voting_power,
            source = EXCLUDED.source,
            synced_at = NOW(),
            updated_at = NOW()
         RETURNING id, contract_id, publisher_id, voting_power, source, synced_at, created_at, updated_at",
    )
    .bind(contract_id)
    .bind(payload.publisher_id)
    .bind(payload.voting_power)
    .bind(source)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("upsert governance voting rights", e))?;

    Ok(Json(row))
}
