Create a new event in the user's primary Google Calendar. Use this when the user asks to schedule a meeting, add an appointment, or block time.

Capabilities:
- Standard timed events: pass `start_time` and `end_time` as ISO 8601 with UTC offset.
- All-day events: pass `start_time` and `end_time` as `YYYY-MM-DD` (date only, no time component). For a single all-day event on Thursday, set `start_time` to Thursday's date and `end_time` to Friday's date (end is exclusive).
- Google Meet video link: pass `add_conference: true` and a Meet URL will be generated automatically and returned in `meet_link`.
- Recurring events: pass an `recurrence` array of RRULE strings (e.g. `["RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=10"]` for a weekly Monday meeting limited to 10 occurrences).

Always confirm the key details (title, time, attendees, conference) with the user before calling this tool unless they were stated unambiguously.
