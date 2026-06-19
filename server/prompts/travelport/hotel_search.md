Search for available hotels via the Travelport GDS. Returns a list of hotel properties with rates for the requested destination and dates.

Use IATA city codes for the destination field (e.g. "PAR" for Paris, "LON" for London, "NYC" for New York). Check-in and check-out must be in YYYY-MM-DD format.

Optional filters: limit results by minimum star rating or maximum rate per night. If the user asks for "budget" options, set a low max_rate_per_night. If they want "luxury", set star_rating_min to 4 or 5.

Each result includes the hotel ID needed for future booking operations.
