use std::{ops::Deref, str::FromStr};

use hmac::Mac;
use mongodb::bson::{to_bson, Bson};
use phonenumber::PhoneNumber;
use rocket::{
    form::{self, prelude::ErrorKind, FromFormField, ValueField},
    http::{
        impl_from_uri_param_identity,
        uri::fmt::{Query, UriDisplay},
    },
    request::FromParam,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{model::db::voter::HmacSha256, Config};

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

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Sms {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        if field.name != "sms" {
            return Err(ErrorKind::Missing.into());
        }
        field
            .value
            .parse::<PhoneNumber>()
            .map(|number| Sms { inner: number })
            .map_err(|err| ErrorKind::Custom(Box::new(err)).into())
    }
}

impl UriDisplay<Query> for Sms {
    fn fmt(
        &self,
        formatter: &mut rocket::http::uri::fmt::Formatter<'_, Query>,
    ) -> std::fmt::Result {
        formatter.write_value(self.to_string())
    }
}

impl<'a> FromParam<'a> for Sms {
    type Error = SmsError<'a>;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        if !param.starts_with("sms=") {
            return Err(SmsError::WrongName(param));
        }
        Ok(Self {
            inner: param[4..].parse::<PhoneNumber>()?,
        })
    }
}

impl_from_uri_param_identity!([Query] Sms);

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
