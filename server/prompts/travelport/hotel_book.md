Book a hotel reservation via the Travelport GDS. This is a paid action: the
user's stored Stripe payment method is authorised for the booking amount, a
single-use virtual card is issued, and Travelport books with the virtual card.
On success the customer is charged; on failure the authorisation is released.

Before calling this tool you MUST have:
- `property_id`, `offer_id`, `rate_id` from the most recent `hotel_availability`
  call (rates older than that may be stale).
- `hotel_name`, `check_in`, `check_out`, `guests`, `guest_name`.
- `total_price_minor_units` — the exact total returned by
  `hotel_availability.total_minor_units`. Do NOT re-derive from per-night
  numbers.
- `currency` — uppercase ISO 4217 (`USD`, `EUR`, `GBP`).

Always state the price, hotel, dates, and refund policy to the user and obtain
explicit confirmation in chat ("yes, book it", "go ahead") before invoking
this tool. The confirmation UI shown to the user is your safety net, not a
replacement for stating the details.

On success, report the `reservation_id` and our `booking_id` so the user has
a record. The `booking_id` is what `hotel_retrieve_booking` and
`hotel_cancel_booking` take.
