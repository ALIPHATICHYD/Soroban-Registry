# Requirements Document

## Introduction

The Centralized Audit Logging Subsystem provides a unified, tamper-evident record of all mutating operations across the Soroban Registry package management system. This subsystem consolidates security-sensitive operations into a queryable audit trail to support security investigations, operational audits, compliance verification, and dispute resolution. The system builds upon the existing `audit_logs` table and `audit.rs` module, extending them with comprehensive API endpoints, automated retention policies, and enhanced querying capabilities.

## Glossary

- **Audit_Subsystem**: The centralized audit logging system that records, stores, and manages audit events for all mutating operations
- **Audit_Event**: A structured record of a single security-sensitive operation including actor, action, target resource, outcome, and contextual metadata
- **Mutating_Operation**: Any operation that modifies persistent state, including package publish, delete, ownership transfer, deprecation, and signature updates
- **Actor**: The authenticated user, system process, or anonymous entity that initiated an operation
- **Chain_Hash**: A SHA-256 cryptographic hash linking each audit record to its predecessor, providing tamper-evidence
- **Admin_API**: The administrative REST API endpoints for querying and managing audit logs
- **Retention_Policy**: The automated policy that removes audit records older than the configured retention period (default: 1 year)
- **Audit_Logger**: The Rust module responsible for writing audit events to the database
- **Audit_Interceptor**: Middleware or hook that captures mutating operations and delegates to the Audit_Logger
- **Target_Resource**: The entity affected by the operation (e.g., package, user, organization, signature)
- **Action_Type**: The specific operation performed (e.g., PACKAGE_PUBLISH, OWNERSHIP_TRANSFER, DEPRECATION_SET)
- **Metadata**: Contextual JSONB data associated with an audit event, including previous state, version tags, request details
- **Tamper_Evident_Log**: An append-only log where each entry is cryptographically linked to prevent undetected modification

## Requirements

### Requirement 1: Database Schema and Indexes

**User Story:** As a security engineer, I want audit data stored in a well-indexed relational table, so that I can efficiently query historical operations for investigations.

#### Acceptance Criteria

1. THE Audit_Subsystem SHALL store audit events in the `audit_logs` table with columns: id (BIGSERIAL PRIMARY KEY), actor_id (TEXT), actor_email (TEXT), actor_ip (TEXT), request_id (TEXT), operation (TEXT NOT NULL), resource_type (TEXT NOT NULL), resource_id (TEXT NOT NULL), metadata (JSONB NOT NULL DEFAULT '{}'), status (TEXT NOT NULL), error_message (TEXT), chain_hash (TEXT NOT NULL), created_at (TIMESTAMPTZ NOT NULL DEFAULT NOW())
2. THE Audit_Subsystem SHALL maintain index `idx_audit_logs_actor_id` on the actor_id column
3. THE Audit_Subsystem SHALL maintain index `idx_audit_logs_operation` on the operation column
4. THE Audit_Subsystem SHALL maintain index `idx_audit_logs_resource` on (resource_type, resource_id) columns
5. THE Audit_Subsystem SHALL maintain index `idx_audit_logs_created_at` on created_at DESC
6. THE Audit_Subsystem SHALL enforce append-only semantics by preventing UPDATE and DELETE operations on audit_logs via database trigger
7. THE Audit_Subsystem SHALL enforce status values to be either 'success' or 'failure' via CHECK constraint

### Requirement 2: Audit Event Capture for Package Operations

**User Story:** As a registry administrator, I want all package lifecycle operations logged, so that I can trace who published, modified, or deleted packages.

#### Acceptance Criteria

1. WHEN a package is published, THE Audit_Interceptor SHALL record an audit event with operation='contract.publish', resource_type='package', and metadata containing package name, version, and publisher ID
2. WHEN a package is deleted, THE Audit_Interceptor SHALL record an audit event with operation='contract.delete', resource_type='package', and metadata containing package name, version, and deletion reason
3. WHEN a package is deprecated, THE Audit_Interceptor SHALL record an audit event with operation='contract.deprecate', resource_type='package', and metadata containing deprecation reason and alternative package recommendations
4. WHEN a package is undeprecated, THE Audit_Interceptor SHALL record an audit event with operation='contract.undeprecate', resource_type='package', and metadata containing undeprecation justification
5. WHEN a package verification status changes, THE Audit_Interceptor SHALL record an audit event with operation='contract.verify', resource_type='package', and metadata containing previous and new verification status

### Requirement 3: Audit Event Capture for Ownership Operations

**User Story:** As a registry administrator, I want all ownership changes logged, so that I can verify legitimate transfers and detect unauthorized access.

#### Acceptance Criteria

1. WHEN an ownership transfer is initiated, THE Audit_Interceptor SHALL record an audit event with operation='ownership_transfer.create', resource_type='package', and metadata containing current owner ID, proposed new owner ID, and transfer token
2. WHEN an ownership transfer is confirmed, THE Audit_Interceptor SHALL record an audit event with operation='ownership_transfer.confirm', resource_type='package', and metadata containing previous owner ID, new owner ID, and confirmation timestamp
3. WHEN a publisher role is changed, THE Audit_Interceptor SHALL record an audit event with operation='publisher.change', resource_type='user', and metadata containing user ID, previous role, and new role

### Requirement 4: Audit Event Capture for Signature Operations

**User Story:** As a security auditor, I want cryptographic signature operations logged, so that I can verify the integrity of the supply chain.

#### Acceptance Criteria

1. WHEN a signing key is registered, THE Audit_Interceptor SHALL record an audit event with operation='signing_key.register', resource_type='signing_key', and metadata containing key ID, algorithm, and owner ID
2. WHEN a signing key is rotated, THE Audit_Interceptor SHALL record an audit event with operation='signing_key.rotate', resource_type='signing_key', and metadata containing old key ID, new key ID, and rotation reason
3. WHEN a signing key is revoked, THE Audit_Interceptor SHALL record an audit event with operation='signing_key.revoke', resource_type='signing_key', and metadata containing key ID, revocation timestamp, and revocation reason
4. WHEN a package signature is stored, THE Audit_Interceptor SHALL record an audit event with operation='signature.store', resource_type='signature', and metadata containing package ID, key ID, and signature verification result

### Requirement 5: Audit Event Capture for Administrative Actions

**User Story:** As a compliance officer, I want administrative actions logged, so that I can ensure proper use of elevated privileges.

#### Acceptance Criteria

1. WHEN a user role is changed, THE Audit_Interceptor SHALL record an audit event with operation='user.role_change', resource_type='user', and metadata containing user ID, previous role, new role, and administrator ID
2. WHEN an administrative action is performed that does not fall under other categories, THE Audit_Interceptor SHALL record an audit event with operation='admin.action', resource_type matching the affected entity type, and metadata containing action description and affected resource details

### Requirement 6: Tamper-Evident Chain Hash

**User Story:** As a security architect, I want each audit record cryptographically linked to its predecessor, so that tampering attempts can be detected.

#### Acceptance Criteria

1. WHEN an audit event is recorded, THE Audit_Logger SHALL compute chain_hash as SHA-256(previous_chain_hash || operation || resource_id || created_at_iso8601)
2. WHEN the audit_logs table is empty, THE Audit_Logger SHALL use 'genesis' as the previous_chain_hash for the first record
3. WHEN computing the chain hash, THE Audit_Logger SHALL fetch the most recent chain_hash value from the database via query: `SELECT chain_hash FROM audit_logs ORDER BY id DESC LIMIT 1`
4. THE Audit_Logger SHALL encode the computed SHA-256 digest as lowercase hexadecimal (64 characters)

### Requirement 7: Sensitive Data Redaction

**User Story:** As a privacy officer, I want sensitive fields automatically redacted from audit metadata, so that secrets are never persisted in audit logs.

#### Acceptance Criteria

1. WHEN audit metadata contains keys matching sensitive patterns (authorization, auth, token, access_token, refresh_token, api_key, secret, password, private_key, seed, mnemonic, cert_signature, signature), THE Audit_Logger SHALL replace their values with the string "[REDACTED]"
2. THE Audit_Logger SHALL recursively redact sensitive keys in nested JSON objects up to depth 8
3. THE Audit_Logger SHALL preserve non-sensitive metadata fields without modification
4. WHEN redacting metadata, THE Audit_Logger SHALL perform case-insensitive key matching for sensitive field detection

### Requirement 8: Admin API Query by Actor

**User Story:** As a security investigator, I want to query all actions performed by a specific user, so that I can investigate suspicious activity.

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint GET /api/admin/audit-logs?actor_id={actor_id}&limit={limit}
2. WHEN querying by actor_id, THE Admin_API SHALL return audit events ordered by created_at DESC
3. WHEN the limit parameter is provided, THE Admin_API SHALL clamp the value between 1 and 500
4. THE Admin_API SHALL return JSON array of audit events with fields: id, actor_id, actor_email, actor_ip, request_id, operation, resource_type, resource_id, metadata, status, error_message, chain_hash, created_at
5. THE Admin_API SHALL require authentication and admin authorization for this endpoint

### Requirement 9: Admin API Query by Action Type

**User Story:** As a compliance auditor, I want to retrieve all operations of a specific type, so that I can review category-specific activities (e.g., all deletions).

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint GET /api/admin/audit-logs?operation={operation}&limit={limit}
2. WHEN querying by operation, THE Admin_API SHALL return audit events matching the exact operation string
3. WHEN querying by operation, THE Admin_API SHALL order results by created_at DESC
4. THE Admin_API SHALL support filtering by multiple operation types via comma-separated values: GET /api/admin/audit-logs?operation={op1},{op2}
5. THE Admin_API SHALL require authentication and admin authorization for this endpoint

### Requirement 10: Admin API Query by Target Resource

**User Story:** As an incident responder, I want to view the complete audit trail for a specific resource, so that I can understand its history during investigations.

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint GET /api/admin/audit-logs?resource_type={type}&resource_id={id}&limit={limit}
2. WHEN querying by resource, THE Admin_API SHALL return audit events matching both resource_type and resource_id
3. WHEN querying by resource, THE Admin_API SHALL order results by created_at DESC
4. THE Admin_API SHALL support partial resource_id matching when the query parameter includes wildcard suffix: resource_id={prefix}*
5. THE Admin_API SHALL require authentication and admin authorization for this endpoint

### Requirement 11: Admin API Query by Time Range

**User Story:** As a security analyst, I want to filter audit logs by date range, so that I can focus investigations on specific time windows.

#### Acceptance Criteria

1. THE Admin_API SHALL support query parameters `since` and `until` for time-based filtering: GET /api/admin/audit-logs?since={iso8601_timestamp}&until={iso8601_timestamp}
2. WHEN the since parameter is provided, THE Admin_API SHALL return only audit events where created_at >= since
3. WHEN the until parameter is provided, THE Admin_API SHALL return only audit events where created_at <= until
4. WHEN both since and until are provided, THE Admin_API SHALL return audit events within the closed interval [since, until]
5. THE Admin_API SHALL parse timestamps in ISO 8601 format (e.g., "2026-01-15T10:30:00Z")
6. WHEN an invalid timestamp format is provided, THE Admin_API SHALL return HTTP 400 with error message describing the expected format

### Requirement 12: Admin API Pagination

**User Story:** As an administrator querying large audit datasets, I want paginated results, so that I can navigate through audit history without overwhelming the client or server.

#### Acceptance Criteria

1. THE Admin_API SHALL support pagination via query parameters `page` and `per_page`: GET /api/admin/audit-logs?page={page_number}&per_page={page_size}
2. WHEN page is not provided, THE Admin_API SHALL default to page=1
3. WHEN per_page is not provided, THE Admin_API SHALL default to per_page=50
4. THE Admin_API SHALL clamp per_page to the range [1, 500]
5. THE Admin_API SHALL compute the offset as (page - 1) * per_page
6. THE Admin_API SHALL return response headers including X-Total-Count (total matching records), X-Page (current page), X-Per-Page (records per page), X-Total-Pages (total pages)

### Requirement 13: Admin API Export All Logs

**User Story:** As a compliance officer, I want to export all audit logs for regulatory reporting, so that I can provide complete audit trails to auditors.

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint GET /api/admin/audit-logs/export
2. WHEN the export endpoint is called, THE Admin_API SHALL return all audit logs ordered by created_at DESC
3. THE Admin_API SHALL return the export in JSON format as an array of audit event objects
4. THE Admin_API SHALL require authentication and admin authorization for this endpoint
5. WHEN the export contains more than 10,000 records, THE Admin_API SHALL log a warning indicating large export size

### Requirement 14: Automated Retention Policy Enforcement

**User Story:** As a database administrator, I want old audit logs automatically cleaned up, so that storage costs remain manageable while maintaining compliance retention periods.

#### Acceptance Criteria

1. THE Audit_Subsystem SHALL provide an automated retention cleanup job that deletes audit records where created_at < (NOW() - retention_period)
2. THE Audit_Subsystem SHALL default the retention_period to 1 year (365 days)
3. THE Audit_Subsystem SHALL make the retention_period configurable via environment variable AUDIT_RETENTION_DAYS
4. WHEN the retention cleanup job runs, THE Audit_Subsystem SHALL log the number of deleted records
5. THE Audit_Subsystem SHALL execute the retention cleanup job daily at 02:00 UTC via scheduled task or cron

### Requirement 15: Manual Retention Cleanup Endpoint

**User Story:** As a system administrator, I want to manually trigger retention cleanup, so that I can free storage immediately when needed.

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint POST /api/admin/audit-logs/cleanup
2. WHEN the cleanup endpoint is called, THE Admin_API SHALL execute the retention policy deletion query
3. THE Admin_API SHALL return JSON response containing the number of deleted records: `{"deleted_count": N}`
4. THE Admin_API SHALL require authentication and admin authorization for this endpoint
5. THE Admin_API SHALL log the cleanup operation as an audit event with operation='admin.action', resource_type='audit_logs', metadata containing deleted_count

### Requirement 16: Audit Logger Integration in Application Code

**User Story:** As a backend developer, I want the Audit_Logger available throughout the application, so that I can easily record audit events from any handler or service.

#### Acceptance Criteria

1. THE Audit_Subsystem SHALL expose the Audit_Logger via the AppState shared state structure
2. THE Audit_Logger SHALL provide an async `log` method accepting an AuditEvent struct
3. THE Audit_Logger SHALL return the inserted audit event ID after successful logging
4. WHEN an audit event logging fails, THE Audit_Logger SHALL log the error via tracing::error and return an sqlx::Error
5. THE Audit_Logger SHALL be cloneable and safe for concurrent use across multiple request handlers

### Requirement 17: Audit Event Status Tracking

**User Story:** As a security analyst, I want to distinguish between successful and failed operations, so that I can identify patterns in failures that may indicate attacks.

#### Acceptance Criteria

1. THE AuditEvent SHALL include a status field of type AuditStatus enum with variants Success and Failure
2. WHEN an operation completes successfully, THE Audit_Interceptor SHALL set status=Success
3. WHEN an operation fails, THE Audit_Interceptor SHALL set status=Failure and populate error_message with the failure reason
4. THE Audit_Logger SHALL persist status as a string value ('success' or 'failure') in the database
5. THE Admin_API SHALL support filtering by status: GET /api/admin/audit-logs?status={success|failure}

### Requirement 18: Request Context Propagation

**User Story:** As a security engineer, I want audit logs to include request identifiers and client IP addresses, so that I can correlate events with application logs and network traffic.

#### Acceptance Criteria

1. WHEN an audit event is recorded for an HTTP request, THE Audit_Interceptor SHALL extract the request_id from the request headers or generate a UUID if absent
2. WHEN an audit event is recorded for an HTTP request, THE Audit_Interceptor SHALL extract the client IP address from the X-Forwarded-For header or the connection remote address
3. THE AuditEvent SHALL include optional fields actor_ip and request_id
4. THE Admin_API SHALL support filtering by request_id: GET /api/admin/audit-logs?request_id={uuid}
5. WHEN multiple operations occur within the same HTTP request, THE Audit_Interceptor SHALL use the same request_id for all related audit events

### Requirement 19: Audit Chain Integrity Verification

**User Story:** As a security auditor, I want to verify the integrity of the audit chain, so that I can detect if records have been tampered with or deleted.

#### Acceptance Criteria

1. THE Admin_API SHALL provide endpoint GET /api/admin/audit-logs/verify-chain
2. WHEN chain verification is requested, THE Admin_API SHALL fetch all audit records ordered by id ASC
3. FOR EACH audit record after the first, THE Admin_API SHALL recompute the expected chain_hash using the previous record's chain_hash
4. WHEN a chain_hash mismatch is detected, THE Admin_API SHALL include the record ID in the verification failure response
5. THE Admin_API SHALL return JSON response with structure: `{"valid": true|false, "total_records": N, "broken_links": [id1, id2, ...]}`
6. THE Admin_API SHALL require authentication and admin authorization for this endpoint

### Requirement 20: Performance Monitoring and Query Optimization

**User Story:** As a platform engineer, I want audit log queries to execute quickly even with millions of records, so that admin operations remain responsive.

#### Acceptance Criteria

1. WHEN querying audit logs with appropriate filters (actor_id, operation, resource, created_at), THE Admin_API SHALL execute queries in under 200ms for datasets up to 1 million records
2. THE Audit_Subsystem SHALL use database indexes for all filterable fields (actor_id, operation, resource_type, resource_id, created_at)
3. WHEN audit log queries exceed 500ms execution time, THE Audit_Subsystem SHALL emit a warning metric via OpenTelemetry
4. THE Audit_Subsystem SHALL expose Prometheus metrics for audit logging operations: audit_events_total (counter), audit_query_duration_seconds (histogram), audit_retention_deleted_total (counter)
5. WHEN the audit_logs table exceeds 10 million records, THE Audit_Subsystem SHALL recommend partitioning by created_at range in application logs

### Requirement 21: Error Handling and Resilience

**User Story:** As a reliability engineer, I want audit logging failures to not crash the main application, so that the system remains available even if audit logging encounters issues.

#### Acceptance Criteria

1. WHEN audit event logging fails due to database errors, THE Audit_Logger SHALL log the error via tracing::error and return the error to the caller
2. THE Audit_Interceptor SHALL treat audit logging failures as non-critical and allow the primary operation to succeed
3. WHEN an audit logging failure occurs, THE Audit_Interceptor SHALL emit a metric: audit_logging_failures_total (counter)
4. THE Audit_Logger SHALL implement retry logic with exponential backoff for transient database errors (connection timeouts, temporary unavailability)
5. THE Audit_Logger SHALL limit retries to 3 attempts with delays of 100ms, 300ms, and 900ms

### Requirement 22: Documentation and Operation Constants

**User Story:** As a backend developer, I want well-documented operation constants, so that I can use standardized operation names when logging audit events.

#### Acceptance Criteria

1. THE Audit_Subsystem SHALL provide a module `audit::ops` containing string constants for all supported operations
2. THE `audit::ops` module SHALL include constants: CONTRACT_VERIFY, CONTRACT_PUBLISH, CONTRACT_DELETE, CONTRACT_DEPRECATE, CONTRACT_UNDEPRECATE, PUBLISHER_CHANGE, USER_ROLE_CHANGE, ADMIN_ACTION, SIGNING_KEY_REGISTER, SIGNING_KEY_ROTATE, SIGNING_KEY_REVOKE, SIGNATURE_STORE, OWNERSHIP_TRANSFER_CREATE, OWNERSHIP_TRANSFER_CONFIRM
3. THE Audit_Logger module documentation SHALL include examples of creating and logging AuditEvent instances
4. THE Admin_API documentation SHALL list all supported query parameters with descriptions and examples
5. THE Audit_Subsystem README SHALL include architecture diagram showing Audit_Interceptor → Audit_Logger → Database flow

### Requirement 23: Testing and Validation

**User Story:** As a quality assurance engineer, I want comprehensive tests for audit logging, so that I can verify correctness and prevent regressions.

#### Acceptance Criteria

1. THE Audit_Subsystem SHALL include unit tests verifying chain_hash computation is deterministic and produces 64-character hexadecimal strings
2. THE Audit_Subsystem SHALL include unit tests verifying sensitive field redaction works for all configured sensitive keys
3. THE Audit_Subsystem SHALL include integration tests verifying audit events are persisted correctly for all mutating operations
4. THE Audit_Subsystem SHALL include integration tests verifying Admin_API endpoints return correct results for various filter combinations
5. THE Audit_Subsystem SHALL include integration tests verifying retention cleanup deletes records older than the configured period
6. THE Audit_Subsystem SHALL include integration tests verifying chain integrity verification detects tampered records
7. THE Audit_Subsystem SHALL achieve minimum 80% code coverage for audit.rs module

