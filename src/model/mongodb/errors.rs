//! For some reason, the mongodb crate doesn't provide error code constants.
//! This module fills in the gaps.

use mongodb::error::{Error as DbError, ErrorKind, WriteFailure};

pub const DUPLICATE_KEY: i32 = 11000;

/// Return true if the given result is a duplicate key write error.
pub fn is_duplicate_key_error<T>(result: Result<T, &DbError>) -> bool {
    if let Err(err) = result {
        if let ErrorKind::Write(WriteFailure::WriteError(ref e)) = *err.kind {
            return e.code == DUPLICATE_KEY;
        }
    }
    false
}
