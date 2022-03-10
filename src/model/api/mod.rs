//! API-compatible types.
//!
//! The types in this module are serialised in an API-friendly way, e.g.:
//!
//! - IDs are serialised as hex strings.
//! - Datetimes are serialised as timestamps.

pub mod auth;
pub mod otp;
pub mod pagination;
pub mod sms;
