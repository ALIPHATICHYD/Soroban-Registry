use async_graphql::dataloader::*;
use shared::models::{Contract, ContractVersion, Organization, Publisher};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct DbLoader {
    pub pool: PgPool,
}

impl Loader<Uuid> for DbLoader {
    type Value = Contract;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let contracts: Vec<Contract> = sqlx::query_as("SELECT * FROM contracts WHERE id = ANY($1)")
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        Ok(contracts.into_iter().map(|c| (c.id, c)).collect())
    }
}

pub struct PublisherLoader {
    pub pool: PgPool,
}

impl Loader<Uuid> for PublisherLoader {
    type Value = Publisher;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let publishers: Vec<Publisher> =
            sqlx::query_as("SELECT * FROM publishers WHERE id = ANY($1)")
                .bind(keys)
                .fetch_all(&self.pool)
                .await
                .map_err(Arc::new)?;

        Ok(publishers.into_iter().map(|p| (p.id, p)).collect())
    }
}

pub struct OrganizationLoader {
    pub pool: PgPool,
}

impl Loader<Uuid> for OrganizationLoader {
    type Value = Organization;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let orgs: Vec<Organization> =
            sqlx::query_as("SELECT * FROM organizations WHERE id = ANY($1)")
                .bind(keys)
                .fetch_all(&self.pool)
                .await
                .map_err(Arc::new)?;

        Ok(orgs.into_iter().map(|o| (o.id, o)).collect())
    }
}

pub struct ContractVersionsLoader {
    pub pool: PgPool,
}

impl Loader<Uuid> for ContractVersionsLoader {
    type Value = Vec<ContractVersion>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let versions: Vec<ContractVersion> = sqlx::query_as(
            "SELECT * FROM contract_versions WHERE contract_id = ANY($1) ORDER BY created_at DESC",
        )
        .bind(keys)
        .fetch_all(&self.pool)
        .await
        .map_err(Arc::new)?;

        let mut map: HashMap<Uuid, Vec<ContractVersion>> = HashMap::new();
        for v in versions {
            map.entry(v.contract_id).or_default().push(v);
        }

        Ok(map)
    }
}

pub struct CategoryLoader {
    pub pool: PgPool,
}

impl Loader<Uuid> for CategoryLoader {
    type Value = crate::category_handlers::CategoryRow;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let categories: Vec<crate::category_handlers::CategoryRow> = sqlx::query_as(
            r#"
            SELECT
                cc.*,
                (SELECT COUNT(*) FROM contracts c WHERE c.category = cc.name)::BIGINT AS usage_count
            FROM contract_categories cc
            WHERE cc.id = ANY($1)
            "#,
        )
        .bind(keys)
        .fetch_all(&self.pool)
        .await
        .map_err(Arc::new)?;

        Ok(categories.into_iter().map(|c| (c.id, c)).collect())
    }
}
