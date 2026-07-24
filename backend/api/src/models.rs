// API-specific models for the review system
// Re-exports from shared with additional API-layer types

pub use shared::{
    ContractRatingStats, CreateReviewRequest, FlagReviewRequest, GetReviewsQuery,
    ModerateReviewRequest, RatingDistribution, ReviewResponse, ReviewSortBy, ReviewStatus,
    ReviewVoteRequest, ReviewVoteResponse,
};

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ContractSourceResponse {
    pub id: uuid::Uuid,
    pub contract_version_id: uuid::Uuid,
    pub source_format: String,
    pub storage_backend: String,
    pub storage_key: String,
    pub source_hash: String,
    pub source_size: i64,
    pub source_base64: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
