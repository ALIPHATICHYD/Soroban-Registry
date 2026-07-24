-- Add 'unverified' value to the verification_status enum.
--
-- This MUST be isolated in its own migration: PostgreSQL forbids using a newly
-- added enum value in the same transaction that adds it ("unsafe use of new
-- value ... of enum type"). sqlx runs each migration file in its own
-- transaction, so the value is committed here and first used in the follow-up
-- migration 20260329093001_add_contract_verification_columns.sql.
ALTER TYPE verification_status ADD VALUE IF NOT EXISTS 'unverified';
