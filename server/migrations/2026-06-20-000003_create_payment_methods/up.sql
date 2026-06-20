-- ── payment_methods ─────────────────────────────────────────────────────────
-- N:1 → users. Stores references to Stripe Customer + PaymentMethod IDs plus
-- display-safe metadata (last4, brand, expiry). The card number itself is
-- never stored — Stripe holds it. Billing address is normalised into the
-- existing `addresses` table via `billing_address_id` (typically a row with
-- type='billing').
--
-- The current UI manages a single primary card; the table is N:1 so future
-- multi-card support is a UI change, not a schema migration.
CREATE TABLE payment_methods (
    id                       uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                  uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id          uuid,                                       -- future multi-tenancy
    stripe_customer_id       text        NOT NULL,
    stripe_payment_method_id text        NOT NULL,
    last4                    text,
    brand                    text,
    exp_month                smallint,
    exp_year                 smallint,
    label                    text,
    is_default               boolean     NOT NULL DEFAULT false,
    billing_address_id       uuid        REFERENCES addresses(id) ON DELETE SET NULL,
    created_at               timestamptz NOT NULL DEFAULT now(),
    updated_at               timestamptz NOT NULL DEFAULT now(),
    deleted_at               timestamptz
);

CREATE INDEX payment_methods_user_idx
    ON payment_methods (user_id) WHERE deleted_at IS NULL;

-- A given Stripe PaymentMethod can only be linked to one row (across users).
CREATE UNIQUE INDEX payment_methods_stripe_pm_unique
    ON payment_methods (stripe_payment_method_id) WHERE deleted_at IS NULL;

-- At most one default payment method per user.
CREATE UNIQUE INDEX payment_methods_user_default_unique
    ON payment_methods (user_id) WHERE is_default AND deleted_at IS NULL;
