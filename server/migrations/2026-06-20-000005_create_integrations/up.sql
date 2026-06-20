-- ── integrations ───────────────────────────────────────────────────────────
-- Generic OAuth-credential store for third-party services the LLM agent can
-- call on the user's behalf (Google Calendar, Gmail, Microsoft Outlook,
-- Slack, etc.). One row per (user, provider, external account).
--
-- Per CLAUDE.md's encryption policy we do NOT KMS-encrypt these columns —
-- we rely on Postgres encryption-at-rest. The OAuth client_id/secret used to
-- refresh these tokens lives in env vars on the server, not in the DB.
CREATE TABLE integrations (
    id                      uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Provider slug. Examples:
    --   google_calendar, google_gmail, google_drive,
    --   microsoft_outlook, microsoft_calendar,
    --   slack, notion, salesforce
    provider                text        NOT NULL,
    -- External stable ID — Google `sub`, Microsoft `oid`, Slack user ID, etc.
    -- Required: a triple (user, provider, account_id) is what identifies a
    -- specific connection. Two Google accounts on one user = two rows.
    provider_account_id     text        NOT NULL,
    -- Display only.
    provider_account_email  text,
    access_token            text        NOT NULL,
    refresh_token           text,
    token_expiry            timestamptz,
    scopes                  text[]      NOT NULL DEFAULT '{}',
    status                  text        NOT NULL DEFAULT 'active'
                              CHECK (status IN ('active', 'expired', 'revoked')),
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX integrations_user_provider_account_unique
    ON integrations (user_id, provider, provider_account_id);

CREATE INDEX integrations_user_idx ON integrations (user_id);

-- For background "find tokens approaching expiry" sweeps later. Partial so the
-- index stays small.
CREATE INDEX integrations_token_expiry_idx
    ON integrations (token_expiry) WHERE status = 'active';
