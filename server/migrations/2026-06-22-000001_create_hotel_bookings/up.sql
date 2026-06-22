-- ── hotel_bookings ─────────────────────────────────────────────────────────
-- One row per hotel booking initiated through the assistant. We persist our
-- own UUID so we can:
--   * link the booking to the Stripe Issuing card row (via entity_id on
--     issuing_card_log) and to the PaymentIntent used to pre-charge the user
--   * keep an audit trail independent of Travelport's reservation id (which
--     only exists after the supplier confirms the booking)
--   * support cancel / retrieve flows by our id rather than chasing the
--     supplier's id through the LLM
--
-- The cancellation_policy is stored as JSONB so we can preserve whatever
-- supplier-specific shape Travelport returns at book time without forcing it
-- into a normalised column set right now. `status` is text + CHECK to match
-- the project-wide pattern (see issuing_card_log.currency) and avoid pulling
-- in diesel-derive-enum for a 4-value field.
CREATE TABLE hotel_bookings (
    id                          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                     uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id             uuid        REFERENCES conversations(id) ON DELETE SET NULL,
    travelport_reservation_id   text,
    travelport_property_id      text        NOT NULL,
    travelport_offer_id         text,
    hotel_name                  text        NOT NULL,
    check_in                    date        NOT NULL,
    check_out                   date        NOT NULL,
    guests                      integer     NOT NULL,
    rooms                       integer     NOT NULL DEFAULT 1,
    total_amount_minor_units    bigint      NOT NULL,
    currency                    text        NOT NULL CHECK (currency IN ('USD', 'GBP', 'EUR')),
    cancellation_policy         jsonb,
    status                      text        NOT NULL DEFAULT 'pending'
                                                CHECK (status IN ('pending', 'confirmed', 'cancelled', 'failed')),
    failure_reason              text,
    payment_method_id           text,
    stripe_payment_intent_id    text,
    refunded_amount_minor_units bigint,
    created_at                  timestamptz NOT NULL DEFAULT now(),
    confirmed_at                timestamptz,
    cancelled_at                timestamptz
);

CREATE INDEX hotel_bookings_user_idx
    ON hotel_bookings (user_id, check_in);

CREATE UNIQUE INDEX hotel_bookings_reservation_unique
    ON hotel_bookings (travelport_reservation_id)
    WHERE travelport_reservation_id IS NOT NULL;
