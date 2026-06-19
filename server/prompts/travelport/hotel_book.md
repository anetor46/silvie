Books a hotel room via the Travelport GDS using the user's stored payment method. A single-use virtual card is issued by Stripe for the exact booking amount, charged to the user's card on file, and passed to the GDS as the Form of Payment.

Use this tool only after the user has explicitly confirmed they want to complete the booking, and only when you have a specific hotel from a hotel_search result (with a valid hotel_id).

Before calling this tool you must have:
- hotel_id (from hotel_search)
- check_in and check_out dates
- total_price_minor_units (the total stay cost in the currency's smallest unit — multiply the displayed price by 100)
- currency code (lowercase ISO 4217, e.g. "usd", "eur")

Always state the total price, hotel name, and dates to the user before booking. Ask for confirmation unless the user's message unambiguously requested to book (e.g. "yes, book it" or "go ahead and book").

On success, report the confirmation number and card last4 to the user so they have a record.
