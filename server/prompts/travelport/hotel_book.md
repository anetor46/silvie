Book a hotel reservation via the Travelport GDS. This is a paid action: the
user's stored Stripe payment method is authorised for the booking amount, a
single-use virtual card is issued, and Travelport books with the virtual card.
On success the customer is charged; on failure the authorisation is released.

Before calling this tool you MUST have:
- `property_id` (composite, from hotel_search), `offer_id` (CatalogOffering
  Identifier from the most recent `hotel_availability`), and `rate_id`
  (the `bookingCode` from the same availability response). Travelport's
  cached offers expire ~30 minutes after the Availability call — if you've
  waited longer, re-run availability before booking.
- `hotel_name`, `check_in`, `check_out`, `guests`.
- `guest_given_name` and `guest_surname` — Travelport requires the lead
  guest's name split into two fields. Ask the user if you don't have both.
- `total_price_minor_units` — the exact total returned by
  `hotel_availability.rates[].total_minor_units`. Do NOT re-derive from
  per-night numbers.
- `currency` — uppercase ISO 4217 (`USD`, `EUR`, `GBP`).

Always state the price, hotel, dates, and refund policy to the user and obtain
explicit confirmation in chat ("yes, book it", "go ahead") before invoking
this tool. The confirmation UI shown to the user is your safety net, not a
replacement for stating the details.

On success, report the `reservation_id` and our `booking_id` so the user has
a record. The `booking_id` is what `hotel_retrieve_booking` and
`hotel_cancel_booking` take.
