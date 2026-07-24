//! Validation implementations for API request types
//!
//! This module implements the `Validatable` trait for all request types
//! that need validation when received from clients.

use shared::models::{
    AdvancedSearchRequest, BulkStatusUpdateRequest, ChangePublisherRequest, ContractExportRequest,
    ContractImportRequest, CreateContractVersionRequest, CreateInteractionBatchRequest,
    CreateInteractionRequest, CreateMigrationRequest, DependencyDeclaration, PublishRequest,
    Publisher, SaveFavoriteSearchRequest, UpdateContractMetadataRequest,
    UpdateContractStatusRequest, UpdateMigrationStatusRequest, VerifyRequest,
};

use super::extractors::{FieldError, Validatable, ValidationBuilder};
use super::sanitizers::{
    normalize_contract_id, normalize_stellar_address, sanitize_description_optional, sanitize_name,
    sanitize_tags, sanitize_url_optional, trim,
};
use super::validators::{
    validate_category_whitelist, validate_contract_id, validate_json_depth, validate_length,
    validate_name_format, validate_no_xss, validate_semver, validate_source_code_size,
    validate_stellar_address, validate_tags, validate_url_optional, validate_wasm_hash,
};

// ─────────────────────────────────────────────────────────────────────────────
// Constants for validation rules
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum length for contract name
const MAX_NAME_LENGTH: usize = 255;
/// Minimum length for contract name
const MIN_NAME_LENGTH: usize = 1;
/// Maximum length for description
const MAX_DESCRIPTION_LENGTH: usize = 5000;
/// Maximum number of tags allowed
const MAX_TAGS_COUNT: usize = 10;
/// Maximum length for each tag
const MAX_TAG_LENGTH: usize = 50;
/// Maximum source code size (1 MB)
const MAX_SOURCE_CODE_BYTES: usize = 1024 * 1024;
/// Maximum JSON nesting depth
const MAX_JSON_DEPTH: usize = 10;
/// Allowed categories for contracts.
///
/// TODO(issue #414): Replace this static whitelist with a live database lookup
/// against the `contract_categories` table once the category management system
/// is fully integrated.  The seeded default categories below match the rows
/// inserted by migration 051_contract_categories.sql, so existing contract data
/// continues to validate correctly in the meantime.
const ALLOWED_CATEGORIES: &[&str] = &["DEX", "Lending", "Bridge", "Oracle", "Token", "Other"];
/// Maximum length for dependency name
const MAX_DEPENDENCY_NAME_LENGTH: usize = 255;
/// Maximum length for version constraint
const MAX_VERSION_CONSTRAINT_LENGTH: usize = 100;
/// Maximum number of dependencies
const MAX_DEPENDENCIES_COUNT: usize = 50;

// ─────────────────────────────────────────────────────────────────────────────
// PublishRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for PublishRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.wasm_hash = trim(&self.wasm_hash);
        self.name = sanitize_name(&self.name);
        sanitize_description_optional(&mut self.description);
        self.publisher_address = normalize_stellar_address(&self.publisher_address);
        sanitize_url_optional(&mut self.source_url);

        if let Some(ref mut cat) = self.category {
            *cat = trim(cat);
            if cat.is_empty() {
                self.category = None;
            }
        }

        self.tags = sanitize_tags(&self.tags);

        for dep in &mut self.dependencies {
            dep.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("contract_id", || validate_contract_id(&self.contract_id));

        builder.check("wasm_hash", || validate_wasm_hash(&self.wasm_hash));

        builder.check("name", || {
            if self.name.is_empty() {
                return Err("name is required".to_string());
            }
            validate_length(&self.name, MIN_NAME_LENGTH, MAX_NAME_LENGTH)
        });
        builder.check("name", || validate_no_xss(&self.name));
        builder.check("name", || validate_name_format(&self.name));

        if let Some(ref desc) = self.description {
            builder.check("description", || {
                validate_length(desc, 0, MAX_DESCRIPTION_LENGTH)
            });
            builder.check("description", || validate_no_xss(desc));
        }

        builder.check("publisher_address", || {
            validate_stellar_address(&self.publisher_address)
        });

        builder.check("source_url", || validate_url_optional(&self.source_url));

        if let Some(ref cat) = self.category {
            builder.check("category", || {
                validate_category_whitelist(cat, ALLOWED_CATEGORIES)
            });
            builder.check("category", || validate_no_xss(cat));
        }

        builder.check("tags", || {
            validate_tags(&self.tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
        });

        builder.check("dependencies", || {
            if self.dependencies.len() > MAX_DEPENDENCIES_COUNT {
                return Err(format!(
                    "at most {} dependencies are allowed",
                    MAX_DEPENDENCIES_COUNT
                ));
            }
            Ok(())
        });

        for (i, dep) in self.dependencies.iter().enumerate() {
            if let Err(errors) = dep.validate() {
                for err in errors {
                    builder.add_error(format!("dependencies[{}].{}", i, err.field), err.message);
                }
            }
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VerifyRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for VerifyRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.compiler_version = trim(&self.compiler_version);
        self.source_code = super::sanitizers::sanitize_source_code(&self.source_code);
        super::sanitizers::sanitize_json_value(&mut self.build_params);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("contract_id", || validate_contract_id(&self.contract_id));

        builder.check("source_code", || {
            if self.source_code.trim().is_empty() {
                return Err("source_code is required".to_string());
            }
            validate_source_code_size(&self.source_code, MAX_SOURCE_CODE_BYTES)
        });

        builder.check("compiler_version", || {
            if self.compiler_version.is_empty() {
                return Err("compiler_version is required".to_string());
            }
            validate_semver(&self.compiler_version)
        });

        builder.check("build_params", || {
            validate_json_depth(&self.build_params, MAX_JSON_DEPTH)
        });

        builder.build()
    }
}

impl Validatable for ContractExportRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut query) = self.filters.query {
            *query = trim(query);
            if query.is_empty() {
                self.filters.query = None;
            }
        }

        if let Some(ref mut category) = self.filters.category {
            *category = trim(category);
            if category.is_empty() {
                self.filters.category = None;
            }
        }

        if let Some(ref mut categories) = self.filters.categories {
            *categories = categories
                .iter()
                .map(|value| trim(value))
                .filter(|value| !value.is_empty())
                .collect();
            if categories.is_empty() {
                self.filters.categories = None;
            }
        }

        if let Some(ref mut tags) = self.filters.tags {
            *tags = sanitize_tags(tags);
            if tags.is_empty() {
                self.filters.tags = None;
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        if let Some(ref query) = self.filters.query {
            builder.check("filters.query", || validate_length(query, 1, 255));
            builder.check("filters.query", || validate_no_xss(query));
        }

        if let Some(ref category) = self.filters.category {
            builder.check("filters.category", || validate_length(category, 1, 100));
            builder.check("filters.category", || validate_no_xss(category));
        }

        if let Some(ref categories) = self.filters.categories {
            builder.check("filters.categories", || {
                if categories.len() > 20 {
                    return Err("at most 20 categories are allowed".to_string());
                }
                Ok(())
            });
            for (index, category) in categories.iter().enumerate() {
                builder.check(&format!("filters.categories[{index}]"), || {
                    validate_length(category, 1, 100)
                });
            }
        }

        if let Some(ref tags) = self.filters.tags {
            builder.check("filters.tags", || {
                validate_tags(tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
            });
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CreateMigrationRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for CreateMigrationRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.wasm_hash = trim(&self.wasm_hash);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("wasm_hash", || validate_wasm_hash(&self.wasm_hash));
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CreateContractVersionRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for CreateContractVersionRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.version = trim(&self.version);
        self.wasm_hash = trim(&self.wasm_hash);
        sanitize_url_optional(&mut self.source_url);
        if let Some(ref mut c) = self.commit_hash {
            *c = trim(c);
        }
        if let Some(ref mut r) = self.release_notes {
            *r = trim(r);
        }
        if let Some(ref mut s) = self.signature {
            *s = trim(s);
        }
        if let Some(ref mut p) = self.publisher_key {
            *p = trim(p);
        }
        super::sanitizers::sanitize_json_value(&mut self.abi);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("version", || validate_semver(&self.version));
        builder.check("wasm_hash", || validate_wasm_hash(&self.wasm_hash));

        if let Some(ref url) = self.source_url {
            builder.check("source_url", || validate_url_optional(&Some(url.clone())));
        }

        builder.check("abi", || validate_json_depth(&self.abi, MAX_JSON_DEPTH));

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Interaction request validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for CreateInteractionRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut a) = self.account {
            *a = normalize_stellar_address(a);
        }
        if let Some(ref mut m) = self.method {
            *m = trim(m);
        }
        if let Some(ref mut t) = self.transaction_hash {
            *t = trim(t);
        }
        if let Some(ref mut target) = self.target_contract_id {
            *target = trim(target);
            if target.is_empty() {
                self.target_contract_id = None;
            }
        }
        if let Some(ref mut p) = self.parameters {
            super::sanitizers::sanitize_json_value(p);
        }
        if let Some(ref mut r) = self.return_value {
            super::sanitizers::sanitize_json_value(r);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        if let Some(ref a) = self.account {
            builder.check("account", || validate_stellar_address(a));
        }

        if let Some(ref m) = self.method {
            builder.check("method", || validate_length(m, 1, 255));
        }

        if let Some(ref t) = self.transaction_hash {
            builder.check("transaction_hash", || validate_length(t, 64, 64));
        }

        if let Some(ref target) = self.target_contract_id {
            builder.check("target_contract_id", || validate_length(target, 1, 128));
            builder.check("target_contract_id", || validate_no_xss(target));
        }

        if let Some(ref p) = self.parameters {
            builder.check("parameters", || validate_json_depth(p, MAX_JSON_DEPTH));
        }

        if let Some(ref r) = self.return_value {
            builder.check("return_value", || validate_json_depth(r, MAX_JSON_DEPTH));
        }

        builder.build()
    }
}

impl Validatable for CreateInteractionBatchRequest {
    fn sanitize(&mut self) {
        for interaction in &mut self.interactions {
            interaction.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if self.interactions.is_empty() {
            builder.add_error("interactions", "at least one interaction is required");
        }
        for (i, interaction) in self.interactions.iter().enumerate() {
            if let Err(errors) = interaction.validate() {
                for err in errors {
                    builder.add_error(format!("interactions[{}].{}", i, err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpdateMigrationStatusRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for UpdateMigrationStatusRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut log) = self.log_output {
            *log = trim(log);
            if log.is_empty() {
                self.log_output = None;
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpdateContractMetadataRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for UpdateContractMetadataRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_description_optional(&mut self.description);
        if let Some(ref mut cat) = self.category {
            *cat = trim(cat);
            if cat.is_empty() {
                self.category = None;
            }
        }
        if let Some(ref mut tags) = self.tags {
            *tags = sanitize_tags(tags);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        if let Some(ref name) = self.name {
            builder.check("name", || {
                if name.is_empty() {
                    return Err("name cannot be empty".to_string());
                }
                validate_length(name, MIN_NAME_LENGTH, MAX_NAME_LENGTH)
            });
            builder.check("name", || validate_no_xss(name));
            builder.check("name", || validate_name_format(name));
        }

        if let Some(ref desc) = self.description {
            builder.check("description", || {
                validate_length(desc, 0, MAX_DESCRIPTION_LENGTH)
            });
            builder.check("description", || validate_no_xss(desc));
        }

        if let Some(ref cat) = self.category {
            builder.check("category", || {
                validate_category_whitelist(cat, ALLOWED_CATEGORIES)
            });
            builder.check("category", || validate_no_xss(cat));
        }

        if let Some(ref tags) = self.tags {
            builder.check("tags", || {
                validate_tags(tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
            });
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ChangePublisherRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for ChangePublisherRequest {
    fn sanitize(&mut self) {
        self.publisher_address = normalize_stellar_address(&self.publisher_address);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("publisher_address", || {
            validate_stellar_address(&self.publisher_address)
        });
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpdateContractStatusRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for UpdateContractStatusRequest {
    fn sanitize(&mut self) {
        self.status = trim(&self.status).to_uppercase();
        if let Some(ref mut msg) = self.error_message {
            *msg = trim(msg);
            if msg.is_empty() {
                self.error_message = None;
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("status", || {
            if self.status.is_empty() {
                return Err("status is required".to_string());
            }
            Ok(())
        });
        if let Some(ref msg) = self.error_message {
            builder.check("error_message", || validate_no_xss(msg));
        }
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BulkStatusUpdateRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for BulkStatusUpdateRequest {
    fn sanitize(&mut self) {
        for item in &mut self.items {
            item.status = trim(&item.status).to_lowercase();
            if let Some(ref mut msg) = item.error_message {
                *msg = trim(msg);
                if msg.is_empty() {
                    item.error_message = None;
                }
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("items", || {
            if self.items.is_empty() {
                return Err("at least one item is required".to_string());
            }
            Ok(())
        });

        for (index, item) in self.items.iter().enumerate() {
            builder.check(&format!("items[{index}].status"), || {
                match item.status.as_str() {
                    "pending" | "verified" | "failed" => Ok(()),
                    _ => Err("status must be pending, verified, or failed".to_string()),
                }
            });

            if let Some(ref msg) = item.error_message {
                builder.check(&format!("items[{index}].error_message"), || {
                    validate_no_xss(msg)
                });
            }
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Publisher validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for Publisher {
    fn sanitize(&mut self) {
        self.stellar_address = normalize_stellar_address(&self.stellar_address);
        if let Some(ref mut u) = self.username {
            *u = trim(u);
        }
        if let Some(ref mut e) = self.email {
            *e = trim(e);
        }
        if let Some(ref mut g) = self.github_url {
            *g = trim(g);
        }
        if let Some(ref mut w) = self.website {
            *w = trim(w);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("stellar_address", || {
            validate_stellar_address(&self.stellar_address)
        });
        if let Some(ref u) = self.username {
            builder.check("username", || validate_length(u, 1, MAX_NAME_LENGTH));
        }
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DependencyDeclaration validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for DependencyDeclaration {
    fn sanitize(&mut self) {
        self.name = trim(&self.name);
        self.version_constraint = trim(&self.version_constraint);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("name", || {
            if self.name.is_empty() {
                return Err("name is required".to_string());
            }
            validate_length(&self.name, 1, MAX_DEPENDENCY_NAME_LENGTH)
        });

        builder.check("version_constraint", || {
            if self.version_constraint.is_empty() {
                return Err("version_constraint is required".to_string());
            }
            validate_length(&self.version_constraint, 1, MAX_VERSION_CONSTRAINT_LENGTH)
        });

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractImportRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for ContractImportRequest {
    fn sanitize(&mut self) {
        for contract in &mut self.contracts {
            contract.contract_id = normalize_contract_id(&contract.contract_id);
            contract.wasm_hash = trim(&contract.wasm_hash);
            contract.name = sanitize_name(&contract.name);
            sanitize_description_optional(&mut contract.description);
            contract.publisher_address = normalize_stellar_address(&contract.publisher_address);

            if let Some(ref mut cat) = contract.category {
                *cat = trim(cat);
                if cat.is_empty() {
                    contract.category = None;
                }
            }

            if let Some(ref mut tags) = contract.tags {
                *tags = sanitize_tags(tags);
                if tags.is_empty() {
                    contract.tags = None;
                }
            }

            if let Some(ref mut visibility) = contract.visibility {
                *visibility = trim(visibility).to_lowercase();
            }

            if let Some(ref mut versions) = contract.versions {
                for version in versions.iter_mut() {
                    version.version = trim(&version.version);
                    version.wasm_hash = trim(&version.wasm_hash);
                    if let Some(ref mut url) = version.source_url {
                        *url = trim(url);
                    }
                    if let Some(ref mut hash) = version.commit_hash {
                        *hash = trim(hash);
                    }
                    if let Some(ref mut notes) = version.release_notes {
                        *notes = trim(notes);
                    }
                }
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        if self.contracts.is_empty() {
            builder.add_error("contracts", "at least one contract is required");
        }

        if self.contracts.len() > 10_000 {
            builder.add_error("contracts", "maximum 10,000 contracts per batch");
        }

        for (i, contract) in self.contracts.iter().enumerate() {
            let prefix = format!("contracts[{}]", i);

            builder.check(&format!("{}.contract_id", prefix), || {
                validate_contract_id(&contract.contract_id)
            });

            builder.check(&format!("{}.wasm_hash", prefix), || {
                validate_wasm_hash(&contract.wasm_hash)
            });

            builder.check(&format!("{}.name", prefix), || {
                if contract.name.is_empty() {
                    return Err("name is required".to_string());
                }
                validate_length(&contract.name, MIN_NAME_LENGTH, MAX_NAME_LENGTH)
            });
            builder.check(&format!("{}.name", prefix), || {
                validate_no_xss(&contract.name)
            });

            builder.check(&format!("{}.publisher_address", prefix), || {
                validate_stellar_address(&contract.publisher_address)
            });

            if let Some(ref desc) = contract.description {
                builder.check(&format!("{}.description", prefix), || {
                    validate_length(desc, 0, MAX_DESCRIPTION_LENGTH)
                });
                builder.check(&format!("{}.description", prefix), || validate_no_xss(desc));
            }

            if let Some(ref cat) = contract.category {
                builder.check(&format!("{}.category", prefix), || {
                    validate_category_whitelist(cat, ALLOWED_CATEGORIES)
                });
                builder.check(&format!("{}.category", prefix), || validate_no_xss(cat));
            }

            if let Some(ref tags) = contract.tags {
                builder.check(&format!("{}.tags", prefix), || {
                    validate_tags(tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
                });
            }

            if let Some(ref versions) = contract.versions {
                for (vi, version) in versions.iter().enumerate() {
                    let vprefix = format!("{}.versions[{}]", prefix, vi);

                    builder.check(&format!("{}.version", vprefix), || {
                        if version.version.is_empty() {
                            return Err("version is required".to_string());
                        }
                        validate_semver(&version.version)
                    });

                    builder.check(&format!("{}.wasm_hash", vprefix), || {
                        validate_wasm_hash(&version.wasm_hash)
                    });

                    if let Some(ref url) = version.source_url {
                        builder.check(&format!("{}.source_url", vprefix), || {
                            validate_url_optional(&Some(url.clone()))
                        });
                    }
                }
            }
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SaveFavoriteSearchRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for SaveFavoriteSearchRequest {
    fn sanitize(&mut self) {
        self.user_address = normalize_stellar_address(&self.user_address);
        self.name = sanitize_name(&self.name);
        super::sanitizers::sanitize_json_value(&mut self.query_json);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("user_address", || {
            validate_stellar_address(&self.user_address)
        });
        builder.check("name", || validate_length(&self.name, 1, 100));
        builder.check("name", || validate_no_xss(&self.name));
        builder.check("query_json", || {
            validate_json_depth(&self.query_json, MAX_JSON_DEPTH)
        });
        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AdvancedSearchRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for AdvancedSearchRequest {
    fn sanitize(&mut self) {
        // No direct sanitization needed for the query tree currently
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        // Pagination limits
        if let Some(limit) = self.limit {
            if limit <= 0 || limit > 100 {
                builder.add_error("limit", "limit must be between 1 and 100");
            }
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::Network;

    fn valid_contract_id() -> String {
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string()
    }

    fn valid_stellar_address() -> String {
        "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string()
    }

    #[test]
    fn test_publish_request_valid() {
        let req = PublishRequest {
            contract_id: valid_contract_id(),
            wasm_hash: "a".repeat(64),
            name: "My Contract".to_string(),
            slug: None,
            description: Some("A test contract".to_string()),
            network: Network::Testnet,
            category: Some("Token".to_string()),
            tags: vec!["token".to_string(), "defi".to_string()],
            source_url: Some("https://github.com/user/repo".to_string()),
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
            is_cicd: false,
        };

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_publish_request_invalid_contract_id() {
        let req = PublishRequest {
            contract_id: "invalid".to_string(),
            wasm_hash: "a".repeat(64),
            name: "My Contract".to_string(),
            slug: None,
            description: None,
            network: Network::Testnet,
            category: None,
            tags: vec![],
            source_url: None,
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
            is_cicd: false,
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "contract_id"));
    }

    #[test]
    fn test_publish_request_invalid_wasm_hash() {
        let req = PublishRequest {
            contract_id: valid_contract_id(),
            wasm_hash: "invalid".to_string(),
            name: "My Contract".to_string(),
            slug: None,
            description: None,
            network: Network::Testnet,
            category: None,
            tags: vec![],
            source_url: None,
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
            is_cicd: false,
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "wasm_hash"));
    }

    #[test]
    fn test_publish_request_sanitization() {
        let mut req = PublishRequest {
            contract_id: "  cdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  ".to_string(),
            wasm_hash: format!("  {}  ", "a".repeat(64)),
            name: "  <b>My Contract</b>  ".to_string(),
            slug: None,
            description: Some("  <script>alert('xss')</script>Description  ".to_string()),
            network: Network::Testnet,
            category: Some("  DeFi  ".to_string()),
            tags: vec!["  token  ".to_string(), "<b>defi</b>".to_string()],
            source_url: Some("  https://github.com/user/repo  ".to_string()),
            publisher_address: "  gdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  "
                .to_string(),
            dependencies: vec![],
            is_cicd: false,
        };

        req.sanitize();

        assert_eq!(req.contract_id, valid_contract_id());
        assert_eq!(req.wasm_hash, "a".repeat(64));
        assert_eq!(req.name, "My Contract");
        assert_eq!(req.description, Some("alert('xss')Description".to_string()));
        assert_eq!(req.publisher_address, valid_stellar_address());
        assert_eq!(req.category, Some("DeFi".to_string()));
        assert_eq!(req.tags, vec!["token", "defi"]);
        assert_eq!(
            req.source_url,
            Some("https://github.com/user/repo".to_string())
        );
    }
}
