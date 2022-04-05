use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

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

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl AdminCore {
        pub fn example() -> Self {
            Self {
                username: "coordinator".to_string(),
                password_hash: "$argon2i$v=19$m=4096,t=2,p=1$VzJlNzBsa0ZUeGFCNVVucA$01vYAqN0vTeqhZEzW7q9PWmrZlXtzQ/Ns7NkCNE2mA0".to_string(),
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "coordinator2".to_string(),
                password_hash: "$argon2i$v=19$m=4096,t=3,p=1$QW1mQXRkU2h5NGpMYW52dw$/8gyud7gTZlB1ythrBFhVCWTR374g27cO9A+Ri0t/bQ".to_string(),
            }
        }
    }
}
