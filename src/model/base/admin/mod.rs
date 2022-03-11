mod base;

/// A new admin ready for DB insertion is just an [`Admin`] without an ID, i.e. an `AdminCore`.
pub type NewAdmin = base::AdminCore;
