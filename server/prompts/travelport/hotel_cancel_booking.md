Cancel an existing hotel reservation. This calls Travelport's cancel API
and, when the cancellation policy allows, issues a refund against the
original Stripe charge.

`booking_id` is our UUID (returned by `hotel_book`). `confirm` MUST be `true`
— the user must have explicitly approved the cancellation in chat.

Before calling, retrieve the booking with `hotel_retrieve_booking` so you can
quote the refundable amount and refund deadline to the user, then ask for
clear confirmation. The user's approval gate in the chat UI is your safety
net, but you should still state the consequences (full / partial / no refund)
before triggering the tool.

If a booking is non-refundable or past its refund deadline, the tool will
still cancel the reservation with the supplier but no money will be returned —
make sure the user understands that before approving.
