use std::ops::{Deref, DerefMut};

use mongodb::error::Error as DbError;
use serde::{Deserialize, Serialize};

use crate::model::{
    api::admin::AdminCredentials,
    mongodb::{Coll, Id},
};

pub const DEFAULT_ADMIN_USERNAME: &str = "replace-this-admin-asap";
pub const DEFAULT_ADMIN_PASSWORD: &str = "insecure";

/// Core admin user data.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminCore {
    pub username: String,
    pub password_hash: String,
}

impl AdminCore {
    /// Check whether the given password is correct.
    pub fn verify_password<T: AsRef<[u8]>>(&self, password: T) -> bool {
        // Unwrap safe because the only way to create an AdminCore is via
        // From<AdminCredentials>, so the hash is always well-formed.
        argon2::verify_encoded(&self.password_hash, password.as_ref()).unwrap()
    }
}

impl Default for AdminCore {
    fn default() -> Self {
        AdminCredentials {
            username: DEFAULT_ADMIN_USERNAME.to_string(),
            password: DEFAULT_ADMIN_PASSWORD.to_string(),
        }
        .try_into()
        .unwrap() // Default credentials are valid.
    }
}

/// An admin without an ID.
pub type NewAdmin = AdminCore;

/// An admin user from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
pub struct Admin {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub admin: AdminCore,
}

impl Deref for Admin {
    type Target = AdminCore;

    fn deref(&self) -> &Self::Target {
        &self.admin
    }
}

impl DerefMut for Admin {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.admin
    }
}

/// Ensure at least one admin user exists. If none do, create a default admin
/// with known credentials, allowing the system to be bootstrapped.
pub async fn ensure_admin_exists(admins: &Coll<NewAdmin>) -> Result<(), DbError> {
    debug!("Ensuring at least one admin user exists");
    let num_admins = admins.count_documents(None, None).await?;
    if num_admins == 0 {
        let admin = NewAdmin::default();
        admins.insert_one(admin, None).await?;
        warn!("Created default admin user - this must be replaced ASAP");
    }

    Ok(())
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl AdminCore {
        pub fn example() -> Self {
            Self {
                username: "alice112".to_string(),
                password_hash: "$argon2i$v=19$m=4096,t=2,p=1$T1pCQllCT2hGRTR0M2N0MQ$WEW073jjInrJFZ6h2kLX6hxqBCDFGh/NNJhbhWP/Dlo".to_string(),
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "bobthesuperadmin".to_string(),
                password_hash: "$argon2i$v=19$m=4096,t=2,p=1$T1pCQllCT2hGRTR0M2N0MQ$ixygmz+0rD8rpITYQ5tZYHtBhR7UJrCSx/8MzYg8NqM".to_string(),
            }
        }
    }
}
