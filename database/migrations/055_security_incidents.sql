-- Security incident tracking system (Issue #504)

CREATE TYPE incident_severity AS ENUM ('critical', 'high', 'medium', 'low');
CREATE TYPE incident_status   AS ENUM ('reported', 'investigating', 'mitigating', 'resolved', 'closed');

-- Core incidents table
CREATE TABLE security_incidents (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    title           TEXT            NOT NULL,
    description     TEXT            NOT NULL,
    severity        incident_severity NOT NULL,
    status          incident_status   NOT NULL DEFAULT 'reported',
    reporter        TEXT            NOT NULL,      -- stellar address or username
    assigned_to     TEXT,
    cve_id          TEXT,                          -- optional CVE identifier
    reported_at     TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_incidents_severity ON security_incidents(severity);
CREATE INDEX idx_security_incidents_status   ON security_incidents(status);
CREATE INDEX idx_security_incidents_reported_at ON security_incidents(reported_at DESC);

-- Junction table linking incidents to affected contracts
CREATE TABLE incident_affected_contracts (
    incident_id     UUID    NOT NULL REFERENCES security_incidents(id) ON DELETE CASCADE,
    contract_id     UUID    NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    PRIMARY KEY (incident_id, contract_id)
);

CREATE INDEX idx_incident_affected_contracts_contract ON incident_affected_contracts(contract_id);

-- Timeline entries: status changes, comments, and internal updates
CREATE TABLE incident_updates (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id     UUID            NOT NULL REFERENCES security_incidents(id) ON DELETE CASCADE,
    author          TEXT            NOT NULL,
    message         TEXT            NOT NULL,
    -- When not null, this update records a status transition
    status_change   incident_status,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_incident_updates_incident_id ON incident_updates(incident_id);

-- Published security advisories (a subset of incidents that are made public)
CREATE TABLE security_advisories (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id     UUID            REFERENCES security_incidents(id) ON DELETE SET NULL,
    title           TEXT            NOT NULL,
    summary         TEXT            NOT NULL,
    details         TEXT            NOT NULL,
    severity        incident_severity NOT NULL,
    -- Comma-separated affected version ranges (e.g. "<1.2.3,>=2.0.0")
    affected_versions TEXT,
    mitigation      TEXT,
    published_at    TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_advisories_severity    ON security_advisories(severity);
CREATE INDEX idx_security_advisories_published_at ON security_advisories(published_at DESC);

-- Tracks which users were notified about which incidents
CREATE TABLE incident_notification_log (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id     UUID        NOT NULL REFERENCES security_incidents(id) ON DELETE CASCADE,
    contract_id     UUID        REFERENCES contracts(id) ON DELETE SET NULL,
    recipient       TEXT        NOT NULL,   -- stellar address
    channel         TEXT        NOT NULL DEFAULT 'in_app',
    message         TEXT        NOT NULL,
    sent_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_incident_notification_log_incident ON incident_notification_log(incident_id);
CREATE INDEX idx_incident_notification_log_recipient ON incident_notification_log(recipient);
