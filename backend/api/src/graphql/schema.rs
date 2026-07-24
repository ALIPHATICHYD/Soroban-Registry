use async_graphql::{
    dataloader::DataLoader, Context, EmptyMutation, EmptySubscription, Object, Result, Schema,
};
use uuid::Uuid;

use crate::{
    graphql::{
        loaders::{
            CategoryLoader, ContractVersionsLoader, DbLoader, OrganizationLoader, PublisherLoader,
        },
        types::{CategoryType, ContractType, OrganizationType, PaginatedContracts, PublisherType},
    },
    state::AppState,
};

pub struct Query;

#[Object]
impl Query {
    /// List contracts with cursor-style pagination.
    /// `page` is 1-based; defaults to 1. `limit` defaults to 50, max 100.
    async fn contracts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i64>,
        page: Option<i64>,
    ) -> Result<PaginatedContracts> {
        let state = ctx.data::<AppState>()?;
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let page = page.unwrap_or(1).max(1);
        let offset = (page - 1) * limit;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
            .fetch_one(&state.db)
            .await?;

        let rows: Vec<shared::models::Contract> =
            sqlx::query_as("SELECT * FROM contracts ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&state.db)
                .await?;

        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };

        Ok(PaginatedContracts {
            items: rows.into_iter().map(ContractType::from).collect(),
            total,
            page,
            page_size: limit,
            total_pages,
        })
    }

    /// Look up a single contract by its registry UUID.
    async fn contract(&self, ctx: &Context<'_>, id: Uuid) -> Result<Option<ContractType>> {
        let state = ctx.data::<AppState>()?;
        let row: Option<shared::models::Contract> =
            sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?;
        Ok(row.map(ContractType::from))
    }

    /// Look up a single publisher by UUID.
    async fn publisher(&self, ctx: &Context<'_>, id: Uuid) -> Result<Option<PublisherType>> {
        let state = ctx.data::<AppState>()?;
        let row: Option<shared::models::Publisher> =
            sqlx::query_as("SELECT * FROM publishers WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?;
        Ok(row.map(PublisherType::from))
    }

    /// List all contract categories.
    async fn categories(&self, ctx: &Context<'_>) -> Result<Vec<CategoryType>> {
        let state = ctx.data::<AppState>()?;
        let rows: Vec<crate::category_handlers::CategoryRow> = sqlx::query_as(
            r#"
            SELECT
                cc.*,
                (SELECT COUNT(*) FROM contracts c WHERE c.category = cc.name)::BIGINT AS usage_count
            FROM contract_categories cc
            ORDER BY cc.is_default DESC, cc.name ASC
            "#,
        )
        .fetch_all(&state.db)
        .await?;
        Ok(rows.into_iter().map(CategoryType::from).collect())
    }

    /// List all publishers.
    async fn publishers(&self, ctx: &Context<'_>) -> Result<Vec<PublisherType>> {
        let state = ctx.data::<AppState>()?;
        let rows: Vec<shared::models::Publisher> =
            sqlx::query_as("SELECT * FROM publishers ORDER BY created_at DESC")
                .fetch_all(&state.db)
                .await?;
        Ok(rows.into_iter().map(PublisherType::from).collect())
    }

    /// List organizations.
    async fn organizations(&self, ctx: &Context<'_>) -> Result<Vec<OrganizationType>> {
        let state = ctx.data::<AppState>()?;
        let rows: Vec<shared::models::Organization> = sqlx::query_as(
            "SELECT * FROM organizations WHERE is_private = false ORDER BY created_at DESC",
        )
        .fetch_all(&state.db)
        .await?;
        Ok(rows.into_iter().map(OrganizationType::from).collect())
    }
}

pub type RegistrySchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Build the GraphQL schema, injecting the AppState and DataLoaders.
pub fn build_schema(state: AppState) -> RegistrySchema {
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(state.clone())
        .data(DataLoader::new(
            DbLoader {
                pool: state.db.clone(),
            },
            tokio::spawn,
        ))
        .data(DataLoader::new(
            PublisherLoader {
                pool: state.db.clone(),
            },
            tokio::spawn,
        ))
        .data(DataLoader::new(
            OrganizationLoader {
                pool: state.db.clone(),
            },
            tokio::spawn,
        ))
        .data(DataLoader::new(
            ContractVersionsLoader {
                pool: state.db.clone(),
            },
            tokio::spawn,
        ))
        .data(DataLoader::new(
            CategoryLoader {
                pool: state.db.clone(),
            },
            tokio::spawn,
        ))
        .finish()
}
