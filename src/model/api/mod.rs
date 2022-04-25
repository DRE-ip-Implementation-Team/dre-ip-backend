//! API-compatible types.
//!
//! The types in this module are serialised in an API-friendly way, e.g.:
//!
//! - IDs are serialised as hex strings.
//! - Datetimes are serialised as timestamps.

pub mod admin;
pub mod auth;
pub mod ballot;
pub mod candidate_totals;
pub mod election;
pub mod otp;
pub mod pagination;
pub mod receipt;
pub mod sms;
