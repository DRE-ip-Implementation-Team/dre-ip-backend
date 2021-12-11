use argon2::{hash_encoded, verify_encoded, Config};
use mongodb::Collection;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use super::auth::token::{Rights, User};

pub mod db;

#[derive(Serialize, Deserialize)]
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

    pub fn verify_password<S: AsRef<str>>(&self, password: S) -> bool {
        // Valid because we only ever store correctly formatted encoded hashe
        verify_encoded(&self.password_hash, password.as_ref().as_bytes()).unwrap()
    }
}

impl User for Admin {
    fn rights() -> Rights {
        Rights::Admin
    }
}

pub type PutAdmins = Collection<Admin>;

#[derive(Clone, Copy, FromForm)]
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
