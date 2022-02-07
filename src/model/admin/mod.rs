pub use admin_core::AdminCredentials;
pub use db::Admin;

mod admin_core;
mod db;

/// A new admin ready for DB insertion is just an Admin without an ID, i.e. an AdminCore.
pub type NewAdmin = admin_core::AdminCore;
