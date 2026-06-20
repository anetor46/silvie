-- ── conversations ──────────────────────────────────────────────────────────
-- One row per chat thread the user has with the assistant. The most-recent
-- index is the hot path — that's what powers the sidebar.
CREATE TABLE conversations (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id uuid,
    title           text,                      -- auto-generated from first user msg
    model           text,                      -- e.g. 'gemini-2.0-flash'
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    deleted_at      timestamptz
);

CREATE INDEX conversations_user_recent_idx
    ON conversations (user_id, updated_at DESC) WHERE deleted_at IS NULL;

-- ── messages ───────────────────────────────────────────────────────────────
-- ON DELETE CASCADE so removing a conversation wipes the message tail. All
-- explicit columns rather than a jsonb metadata blob (per the no-NoSQL call
-- we made earlier). Persisted roles: user/assistant for now; the backend can
-- start recording system/tool messages later without a schema change.
CREATE TABLE messages (
    id                  uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id     uuid        NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role                text        NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content             text        NOT NULL,
    tool_name           text,
    tool_call_id        text,
    prompt_tokens       integer,
    completion_tokens   integer,
    latency_ms          integer,
    created_at          timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX messages_conversation_idx ON messages (conversation_id, created_at);
