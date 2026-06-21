List calendar events from the user's Outlook calendar within a time window.
Returns event subject, start/end times (ISO 8601 UTC), location, organizer,
attendee list, and a body preview for each event.

Both `start_time` and `end_time` are required. Use ISO 8601 format
(e.g. `2026-06-22T00:00:00Z`). Defaults to a 7-day window from now if omitted.
