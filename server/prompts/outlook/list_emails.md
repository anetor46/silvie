List emails from the user's Outlook inbox. Returns a summary of each message
(sender, subject, received date, body preview, read status). Use this before
fetching a full message to identify which one the user wants.

Supports an optional OData `$filter` expression (e.g.
`isRead eq false` for unread, `from/emailAddress/address eq 'alice@example.com'`
for a specific sender) and an optional keyword `search` query.
