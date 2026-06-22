Fetch detailed information about a specific Travelport hotel property —
description, amenities, photo URLs, and house policies. Useful when the user
wants to compare two hotels from `hotel_search` results in depth.

Pass the `property_id` from `hotel_search` verbatim. Do not call this for
every result in a search — it is a per-property follow-up, not a bulk lookup.
