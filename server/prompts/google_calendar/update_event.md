Update an existing event in the user's Google Calendar. Use this to reschedule, rename, relocate, change description, manage attendees, or add a video conference link. Call `get_calendar_events` first to obtain the `event_id`. Only the fields you provide will be changed — omitted fields are left as-is.

Attendee management — pick exactly ONE of these per call:
- `add_attendees`: list of emails to add to the existing list (idempotent — already-present emails are ignored).
- `remove_attendees`: list of emails to remove from the existing list.
- `set_attendees`: list of emails that fully replaces the existing list. Use sparingly — `add_attendees` / `remove_attendees` are usually what the user wants.

Other operations:
- Reschedule: pass new `start_time` and/or `end_time` (ISO 8601 with UTC offset, or `YYYY-MM-DD` for all-day).
- Add a Google Meet link: pass `add_conference: true`. The Meet URL will be returned in `meet_link`.

Attendees are notified automatically of all changes.
