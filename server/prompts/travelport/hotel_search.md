Search for available hotels via the Travelport GDS. Returns a ranked list of
offers — each carrying an `offer_id` and `property_id` you must pass verbatim
into `hotel_details`, `hotel_availability`, and ultimately `hotel_book`.

Use an IATA city code for the destination (e.g. `PAR`, `LON`, `NYC`). Dates
must be `YYYY-MM-DD`.

Optional filters: cap the per-night rate (`max_rate_per_night`, in major
units), require a minimum star rating (`star_rating_min`), or limit results
(`max_results`, default 10). For a "luxury only" request, pass
`star_rating_min: 4` or `5`.

Prices are returned in minor units (e.g. cents) — multiply by 100 / divide by
100 when talking to the user.
