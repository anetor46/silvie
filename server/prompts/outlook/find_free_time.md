Check free/busy availability for the user's Outlook calendar over a given
time range. Returns a list of busy blocks and the overall availability view,
which lets you suggest open meeting slots.

Provide `start_time` and `end_time` as ISO 8601 UTC strings. The optional
`interval_minutes` controls the granularity of the availability view
(default 30 minutes).
