use argon2::Error as Argon2Error;
use mongodb::Collection;
use serde::{Deserialize, Serialize};

use crate::conf;

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
        Ok(Admin::new(
            self.username.to_string(),
            argon2::hash_encoded(
                self.password.as_bytes(),
                conf!(salt),
                &argon2::Config::default(),
            )?,
        ))
    }
}
