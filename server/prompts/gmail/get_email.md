Fetch the full content of a single email by its message ID.

Returns the sender, recipients, CC, subject, date, and decoded plain-text body.
Use `list_emails` first to find the relevant message ID.

The body is truncated at 6 000 characters if the email is very long. Look for
`truncated: true` in the response if the full email was cut.
