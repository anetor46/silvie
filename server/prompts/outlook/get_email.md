Fetch the full content of a specific Outlook email by its ID. Returns the
complete body text (truncated at 6 000 characters if very long), along with
sender, recipients, CC, subject, received date, and the internet message ID
(needed for replies to preserve threading).

Use `list_outlook_emails` first to discover the message ID.
