//! Data-access layer. Every module here owns the Diesel models *and* the
//! query functions for one logical table or aggregate. HTTP handlers live in
//! [`crate::api`] and call into these.

pub mod conversations;
pub mod hotel_bookings;
pub mod integrations;
pub mod payments;
pub mod user_info;
pub mod users;
