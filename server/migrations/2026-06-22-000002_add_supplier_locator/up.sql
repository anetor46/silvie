-- Travelport's cancel endpoint requires BOTH the aggregator locator (in the
-- URL path) and the supplier locator (as a query param). The booking
-- response returns both — we now persist the supplier locator alongside.
ALTER TABLE hotel_bookings
    ADD COLUMN travelport_supplier_locator text;
