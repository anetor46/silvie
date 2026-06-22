Retrieve the current state of a hotel booking — our stored row plus a live
supplier-status check if a Travelport reservation id is on file.

`booking_id` is the UUID returned by `hotel_book` (not the Travelport
confirmation number — that's `reservation_id` in the response).

Use this when the user asks "what's the status of my booking?", "is my hotel
confirmed?", or before invoking `hotel_cancel_booking` to confirm refundability.
