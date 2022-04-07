use argon2::Config;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::model::db::admin::NewAdmin;

pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Raw admin credentials, received from a user. These are never stored directly,
/// since the password is in plaintext.
#[derive(Clone, Deserialize, Serialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password: String,
}

impl TryFrom<AdminCredentials> for NewAdmin {
    type Error = ();

    /// Convert [`AdminCredentials`] to a new [`Admin`] by hashing the password.
    /// This enforces that the username is non-empty, and the password meets minimum length.
    fn try_from(cred: AdminCredentials) -> Result<Self, Self::Error> {
        // Check credentials are acceptable.
        if cred.username.is_empty() || cred.password.len() < MIN_PASSWORD_LENGTH {
            return Err(());
        }

        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let mut salt = [0_u8; 16];
        rand::thread_rng().fill(&mut salt);
        let password_hash =
            argon2::hash_encoded(cred.password.as_bytes(), &salt, &Config::default()).unwrap(); // Safe because the default `Config` is valid.
        Ok(Self {
            username: cred.username,
            password_hash,
        })
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    impl AdminCredentials {
        pub fn example1() -> Self {
            Self {
                username: "alice112".into(),
                password: "dreip4lyfe".into(),
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "bobthesuperadmin".into(),
                password: "totallysecurepassword".into(),
            }
        }

        pub fn example3() -> Self {
            Self {
                username: "monsieur-foo".into(),
                password: "foobarbaz".into(),
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
