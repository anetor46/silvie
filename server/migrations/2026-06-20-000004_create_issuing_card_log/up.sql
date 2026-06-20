-- ── issuing_card_log ───────────────────────────────────────────────────────
-- Audit trail for every Stripe Issuing virtual card created on behalf of a
-- user (currently only by the hotel-booking flow).
--
-- The PAN/CVC are never stored — they exist in process memory only for the
-- duration of a single booking request. We persist:
--   * which user / payment method funded the card
--   * the Stripe card ID (ic_xxx) so we can correlate with Stripe's dashboard
--   * the spending limit we set on the card
--   * the entity (booking) the card was created for (polymorphic; populated
--     once we have local booking tables)
--   * when the card was cancelled after use
--
-- `user_id` / `payment_method_id` are SET NULL on cascade so financial audit
-- records survive user deletion. Currency restricted to USD/GBP/EUR per the
-- earlier schema decision.
CREATE TABLE issuing_card_log (
    id                       uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                  uuid        REFERENCES users(id) ON DELETE SET NULL,
    payment_method_id        uuid        REFERENCES payment_methods(id) ON DELETE SET NULL,
    stripe_issuing_card_id   text        NOT NULL,
    amount_minor_units       bigint      NOT NULL,
    currency                 text        NOT NULL CHECK (currency IN ('USD', 'GBP', 'EUR')),
    entity_type              text,
    entity_id                uuid,
    created_at               timestamptz NOT NULL DEFAULT now(),
    cancelled_at             timestamptz
);

CREATE UNIQUE INDEX issuing_card_log_stripe_card_unique
    ON issuing_card_log (stripe_issuing_card_id);

CREATE INDEX issuing_card_log_user_idx
    ON issuing_card_log (user_id) WHERE user_id IS NOT NULL;

CREATE INDEX issuing_card_log_entity_idx
    ON issuing_card_log (entity_type, entity_id)
    WHERE entity_type IS NOT NULL AND entity_id IS NOT NULL;
