CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    auth0_sub   text        NOT NULL UNIQUE,
    email       text        NOT NULL,
    name        text        NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    deleted_at  timestamptz
);

CREATE INDEX users_auth0_sub_idx ON users (auth0_sub) WHERE deleted_at IS NULL;
