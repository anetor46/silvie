-- ── user_profiles ────────────────────────────────────────────────────────────
-- 1:1 with users. All fields optional — created on demand by the API.
CREATE TABLE user_profiles (
    user_id              uuid        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    first_name           text,
    last_name            text,
    phone                text,                    -- E.164 recommended, not enforced
    nationality          text,                    -- ISO 3166-1 alpha-2 (e.g. 'FR')
    country_of_residence text,                    -- ISO 3166-1 alpha-2
    preferred_currency   text,                    -- ISO 4217 (e.g. 'EUR')
    preferred_language   text,                    -- BCP 47 (e.g. 'en-GB')
    timezone             text,                    -- IANA (e.g. 'Europe/Paris')
    meal_preference      text,
    seat_preference      text,
    cabin_class_preference text,
    updated_at           timestamptz NOT NULL DEFAULT now()
);

-- ── addresses ────────────────────────────────────────────────────────────────
-- N:1 → users. Polymorphic by `type` ('home' / 'billing' / 'work' / 'other').
-- For now the UI uses only 'home', but the table supports multiple per user.
CREATE TABLE addresses (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id uuid,                          -- nullable; for future multi-tenancy
    type            text        NOT NULL,
    label           text,                          -- user-visible label (e.g. "London Office")
    line1           text,
    line2           text,
    city            text,
    state           text,
    postal_code     text,
    country         text,                          -- ISO 3166-1 alpha-2
    is_default      boolean     NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    deleted_at      timestamptz
);

CREATE INDEX addresses_user_type_idx ON addresses (user_id, type) WHERE deleted_at IS NULL;

-- Enforce one row per (user_id, type) for the simple "home / billing / work"
-- model. If we later need multiple addresses of the same type, replace this
-- with `is_default` semantics.
CREATE UNIQUE INDEX addresses_user_type_unique
    ON addresses (user_id, type)
    WHERE deleted_at IS NULL;

-- ── travel_documents ─────────────────────────────────────────────────────────
-- N:1 → users. Polymorphic by `type` ('passport' / 'national_id' / 'visa' / …).
-- Per the data-encryption policy in CLAUDE.md, document_number is stored as
-- plain text — we rely on Postgres encryption-at-rest, not application-level
-- encryption. Revisit if compliance requirements change.
CREATE TABLE travel_documents (
    id               uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id  uuid,                         -- nullable; for future multi-tenancy
    type             text        NOT NULL,
    document_number  text,
    issuing_country  text,                         -- ISO 3166-1 alpha-2
    nationality      text,                         -- for passports
    issue_date       date,
    expiry_date      date,
    is_primary       boolean     NOT NULL DEFAULT false,
    notes            text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    deleted_at       timestamptz
);

CREATE INDEX travel_documents_user_type_idx
    ON travel_documents (user_id, type) WHERE deleted_at IS NULL;

CREATE INDEX travel_documents_user_expiry_idx
    ON travel_documents (user_id, expiry_date) WHERE deleted_at IS NULL;

-- Only one primary document per user per type (the one used for bookings).
CREATE UNIQUE INDEX travel_documents_user_type_primary_unique
    ON travel_documents (user_id, type)
    WHERE is_primary AND deleted_at IS NULL;
