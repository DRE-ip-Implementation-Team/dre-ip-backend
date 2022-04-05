use argon2::Config;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::model::db::admin::NewAdmin;

/// Raw admin credentials, received from a user. These are never stored directly,
/// since the password is in plaintext.
#[derive(Clone, Deserialize, Serialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password: String,
}

impl From<AdminCredentials> for NewAdmin {
    /// Convert [`AdminCredentials`] to a new [`Admin`] by hashing the password.
    fn from(cred: AdminCredentials) -> Self {
        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let mut salt = [0_u8; 16];
        rand::thread_rng().fill(&mut salt);
        let password_hash =
            argon2::hash_encoded(cred.password.as_bytes(), &salt, &Config::default()).unwrap(); // Safe because the default `Config` is valid.
        Self {
            username: cred.username,
            password_hash,
        }
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    impl AdminCredentials {
        pub fn example() -> Self {
            Self {
                username: "coordinator".into(),
                password: "coordinator".into(),
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "coordinator2".into(),
                password: "coordinator2".into(),
            }
        }

        pub fn empty() -> Self {
            Self {
                username: "".into(),
                password: "".into(),
            }
        }
    }
}
