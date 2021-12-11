use argon2::{hash_encoded, Config, Error as Argon2Error};
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

    pub fn into_admin(self) -> Result<Admin, Argon2Error> {
        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let salt = [0u8; 16];
        // Valid because `OsRng` is "highly unlikely" to fail
        // Also, panicking here is still secure and more efficient than indeterminately busy-looping
        thread_rng().fill(&mut salt);
        Ok(Admin::new(
            self.username.to_string(),
            hash_encoded(
                self.password.as_bytes(),
                &salt,
                &Config::default(), // TODO: Tune hash configuration
            )?,
        ))
    }
}
