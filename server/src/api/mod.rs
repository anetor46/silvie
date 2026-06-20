//! HTTP layer. Handlers only — they read request data, call into
//! [`crate::repos`] / [`crate::services`], and shape the response.

pub mod chat;
pub mod conversations;
pub mod integrations;
pub mod payments;
pub mod user_info;
pub mod users;
