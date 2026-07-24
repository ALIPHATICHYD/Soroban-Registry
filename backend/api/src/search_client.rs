use anyhow::Result;
use chrono::{DateTime, Utc};
use elasticsearch::{
    http::transport::Transport,
    indices::{IndicesCreateParts, IndicesDeleteParts, IndicesExistsParts},
    params::Refresh,
    Elasticsearch, IndexParts, SearchParts,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::models::{Contract, Network};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractDocument {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub network: Network,
    pub is_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct SearchClient {
    client: Elasticsearch,
}

impl SearchClient {
    pub fn new(url: &str) -> Result<Self> {
        let transport = Transport::single_node(url)?;
        Ok(Self {
            client: Elasticsearch::new(transport),
        })
    }

    pub async fn ensure_index(&self) -> Result<()> {
        let index_name = "contracts";
        let exists = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[index_name]))
            .send()
            .await?
            .status_code()
            .is_success();

        if !exists {
            let body = json!({
                "settings": {
                    "number_of_shards": 1,
                    "number_of_replicas": 0,
                    "analysis": {
                        "analyzer": {
                            "autocomplete": {
                                "tokenizer": "autocomplete",
                                "filter": ["lowercase"]
                            },
                            "autocomplete_search": {
                                "tokenizer": "lowercase"
                            }
                        },
                        "tokenizer": {
                            "autocomplete": {
                                "type": "edge_ngram",
                                "min_gram": 2,
                                "max_gram": 20,
                                "token_chars": ["letter", "digit"]
                            }
                        }
                    }
                },
                "mappings": {
                    "properties": {
                        "id": { "type": "keyword" },
                        "contract_id": { "type": "keyword" },
                        "name": {
                            "type": "text",
                            "boost": 3.0,
                            "fields": {
                                "autocomplete": {
                                    "type": "text",
                                    "analyzer": "autocomplete",
                                    "search_analyzer": "autocomplete_search"
                                }
                            }
                        },
                        "description": { "type": "text", "boost": 1.5 },
                        "category": { "type": "keyword" },
                        "author": { "type": "keyword" },
                        "tags": { "type": "keyword", "boost": 2.0 },
                        "network": { "type": "keyword" },
                        "is_verified": { "type": "boolean" },
                        "created_at": { "type": "date" }
                    }
                }
            });

            self.client
                .indices()
                .create(IndicesCreateParts::Index(index_name))
                .body(body)
                .send()
                .await?;
        }
        Ok(())
    }

    pub async fn index_contract(&self, contract: &Contract, author: Option<String>) -> Result<()> {
        let doc = ContractDocument {
            id: contract.id,
            contract_id: contract.contract_id.clone(),
            name: contract.name.clone(),
            description: contract.description.clone(),
            category: contract.category.clone(),
            author,
            tags: contract.tags.iter().map(|t| t.name.clone()).collect(),
            network: contract.network.clone(),
            is_verified: contract.is_verified,
            created_at: contract.created_at,
        };

        self.client
            .index(IndexParts::IndexId(
                "contracts",
                contract.id.to_string().as_str(),
            ))
            .body(doc)
            .refresh(Refresh::True)
            .send()
            .await?;

        Ok(())
    }

    pub async fn search_contracts(
        &self,
        query: &str,
        categories: Option<Vec<String>>,
        networks: Option<Vec<Network>>,
    ) -> Result<Value> {
        let must_queries = vec![json!({
            "multi_match": {
                "query": query,
                "fields": ["name^3", "description^1.5", "tags^2"],
                "fuzziness": "AUTO"
            }
        })];

        let mut filter_queries = Vec::new();

        if let Some(cats) = categories {
            if !cats.is_empty() {
                filter_queries.push(json!({
                    "terms": { "category": cats }
                }));
            }
        }

        if let Some(nets) = networks {
            if !nets.is_empty() {
                filter_queries.push(json!({
                    "terms": { "network": nets }
                }));
            }
        }

        let body = json!({
            "query": {
                "bool": {
                    "must": must_queries,
                    "filter": filter_queries
                }
            },
            "aggs": {
                "categories": { "terms": { "field": "category" } },
                "networks": { "terms": { "field": "network" } },
                "authors": { "terms": { "field": "author" } }
            }
        });

        let response = self
            .client
            .search(SearchParts::Index(&["contracts"]))
            .body(body)
            .send()
            .await?;

        let response_body = response.json::<Value>().await?;
        Ok(response_body)
    }

    pub async fn autocomplete(&self, query: &str) -> Result<Vec<String>> {
        let body = json!({
            "query": {
                "match": {
                    "name.autocomplete": {
                        "query": query,
                        "operator": "and"
                    }
                }
            },
            "_source": ["name"],
            "size": 5
        });

        let response = self
            .client
            .search(SearchParts::Index(&["contracts"]))
            .body(body)
            .send()
            .await?;

        let response_body = response.json::<Value>().await?;
        let mut suggestions = Vec::new();

        if let Some(hits) = response_body["hits"]["hits"].as_array() {
            for hit in hits {
                if let Some(name) = hit["_source"]["name"].as_str() {
                    suggestions.push(name.to_string());
                }
            }
        }

        Ok(suggestions)
    }
}
