Reply to an existing email thread in the user's Gmail account.

To reply properly you need the `thread_id` (from `list_emails`) and optionally
the `message_id_header` (the `Message-ID` header value from `get_email`, used
for In-Reply-To threading). If you have not yet fetched the original email,
call `get_email` first to get the `message_id_header`.

IMPORTANT: Always summarize the reply to the user and get explicit confirmation
before calling this tool. Never send a reply without user approval.

Returns the sent message ID on success.
