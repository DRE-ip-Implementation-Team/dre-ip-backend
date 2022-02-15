use argon2::Config;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Core admin user data, as stored in the database.
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

/// Raw admin credentials, received from a user. These are never stored directly,
/// since the password is in plaintext.
#[derive(Clone, Deserialize, Serialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password: String,
}

impl From<AdminCredentials> for AdminCore {
    /// Convert [`AdminCredentials`] to a new [`Admin`] by hashing the password.
    fn from(cred: AdminCredentials) -> Self {
        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let mut salt = [0_u8; 16];
        rand::thread_rng().fill(&mut salt);
        let password_hash = argon2::hash_encoded(
            cred.password.as_bytes(),
            &salt,
            &Config::default(), // TODO: see if a custom config is useful.
        )
        .unwrap(); // Safe because the default `Config` is valid.
        Self {
            username: cred.username,
            password_hash,
        }
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
