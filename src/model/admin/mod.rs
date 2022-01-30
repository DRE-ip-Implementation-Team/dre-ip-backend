use argon2::{hash_encoded, verify_encoded, Config};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

pub mod db;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admin {
    username: String,
    password_hash: String,
}

impl Admin {
    pub fn new(username: String, password_hash: String) -> Self {
        Self {
            username,
            password_hash,
        }
    }

    pub fn username(&self) -> &String {
        &self.username
    }

    pub fn verify_password<S: AsRef<str>>(&self, password: S) -> bool {
        // Valid because we only ever store correctly formatted encoded hashe
        verify_encoded(&self.password_hash, password.as_ref().as_bytes()).unwrap()
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Credentials<'a> {
    username: &'a str,
    password: &'a str,
}

impl Credentials<'_> {
    pub fn username(&self) -> &str {
        self.username
    }

    pub fn password(&self) -> &str {
        self.password
    }

    /// # Panics
    ///
    /// This method relies on `ThreadRng` to fill the salt array with random bytes.
    /// `ThreadRng` relies on `OsRng` in turn, which is "highly unlikely" to fail.
    ///
    /// Panicking is desirable in this case, as it is more efficient than busy-looping
    /// indeterminately and is still secure.
    pub fn into_admin(self) -> Admin {
        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let mut salt = [0_u8; 16];
        thread_rng().fill(&mut salt);
        Admin::new(
            self.username.to_string(),
            hash_encoded(
                self.password.as_bytes(),
                &salt,
                &Config::default(), // TODO: Tune hash configuration
            )
            .unwrap(), // Valid because the default `Config` is valid
        )
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    impl Admin {
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

    impl Credentials<'_> {
        pub fn example() -> Self {
            Self {
                username: "coordinator",
                password: "coordinator",
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "coordinator2",
                password: "coordinator2",
            }
        }

        pub fn empty() -> Self {
            Self {
                username: "",
                password: "",
            }
        }
    }
}
