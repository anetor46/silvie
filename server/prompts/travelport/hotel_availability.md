Refresh live rates and the cancellation policy for a specific hotel property
immediately before booking. Travelport rates are short-lived; the price shown
in `hotel_search` can drift between search and book.

You MUST call this tool right before `hotel_book` to obtain a fresh `rate_id`
and `offer_id`. Compare the new `total_minor_units` against the figure you
previously quoted the user — if it differs by more than ~5%, ask the user to
re-confirm before proceeding.

Returns a list of `RateQuote`s. Pick the one that matches the user's stated
preference (refundable vs. cheapest) and use its `rate_id` for `hotel_book`.
