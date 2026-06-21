Search or list emails from the user's Gmail inbox.

Returns up to `max_results` messages (default 10, max 25) matching the query,
with sender, subject, date, snippet, and message ID. Use `get_email` to fetch
the full body of a specific message.

Gmail query syntax examples:
- `is:unread` — unread messages
- `from:alice@example.com` — from a specific sender
- `subject:invoice` — subject contains "invoice"
- `after:2026/06/01 before:2026/06/15` — date range
- `has:attachment` — messages with attachments
- Combine with spaces: `from:boss@company.com is:unread`
