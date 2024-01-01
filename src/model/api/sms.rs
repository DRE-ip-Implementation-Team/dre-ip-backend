use std::{ops::Deref, str::FromStr};

use hmac::Mac;
use mongodb::bson::{to_bson, Bson};
use phonenumber::PhoneNumber;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{config::Config, model::db::voter::HmacSha256};

/// A voter's SMS number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sms {
    inner: PhoneNumber,
}

impl Deref for Sms {
    type Target = PhoneNumber;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Sms {
    pub fn into_hmac(self, config: &Config) -> Vec<u8> {
        let mut hmac = HmacSha256::new_from_slice(config.hmac_secret())
            .expect("HMAC can take key of any size");
        hmac.update(self.to_string().as_bytes());
        hmac.finalize().into_bytes().to_vec()
    }
}

impl FromStr for Sms {
    type Err = phonenumber::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Sms {
            inner: s.parse::<PhoneNumber>()?,
        })
    }
}

impl TryFrom<String> for Sms {
    type Error = phonenumber::ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Sms> for String {
    fn from(sms: Sms) -> Self {
        sms.to_string()
    }
}

#[derive(Debug, Error)]
pub enum SmsError<'a> {
    #[error("Expected `sms=<sms>`, got {0}")]
    WrongName(&'a str),
    #[error(transparent)]
    Parse(#[from] phonenumber::ParseError),
}

impl From<Sms> for Bson {
    fn from(sms: Sms) -> Self {
        to_bson(&sms).unwrap() // Valid because `PhoneNumber` serialization doesn't fail
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use rocket::local::asynchronous::Client;

    use super::*;

    impl Sms {
        pub fn example() -> Self {
            "+441234567890".parse().unwrap()
        }

        pub fn example_hmac(client: &Client) -> Vec<u8> {
            Self::example().into_hmac(client.rocket().state::<Config>().unwrap())
        }
    }
}
