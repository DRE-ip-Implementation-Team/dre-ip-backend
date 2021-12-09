use rand::distributions::{Distribution, Uniform};
use rocket::form::Errors;
use rocket::form::{self, prelude::ErrorKind, FromFormField, ValueField};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::borrow::Cow;
use std::convert::TryInto;
use std::str::FromStr;

const LENGTH: usize = 6;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Code {
    #[serde(with = "code")]
    inner: [u8; LENGTH],
}

impl Code {
    pub fn new() -> Self {
        let mut inner = [0; LENGTH];
        let digit_dist = Uniform::from(0..=9);
        let mut rng = rand::thread_rng();
        for digit in &mut inner {
            *digit = digit_dist.sample(&mut rng);
        }
        Self { inner }
    }
}

impl FromStr for Code {
    type Err = CodeParseError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let len = string.len();
        if len != LENGTH {
            Err(Self::Err::InvalidLength(len))?
        }
        let digits = string
            .chars()
            .map(|c| match c {
                '0'..='9' => Ok(c as u8 - 48),
                _ => Err(Self::Err::InvalidChar(c)),
            })
            .collect::<Result<Vec<u8>, Self::Err>>()?;
        Ok(Self {
            inner: digits.try_into().unwrap(),
        })
    }
}

#[derive(Error, Debug)]
pub enum CodeParseError {
    #[error("code must contain exactly 6 characters")]
    InvalidLength(usize),
    #[error("code must contain only digits")]
    InvalidChar(char),
}

impl From<CodeParseError> for form::error::ErrorKind<'_> {
    fn from(err: CodeParseError) -> Self {
        match err {
            CodeParseError::InvalidLength(_) => form::error::ErrorKind::InvalidLength {
                min: Some(LENGTH as u64),
                max: Some(LENGTH as u64),
            },
            CodeParseError::InvalidChar(c) => {
                form::error::ErrorKind::Validation(Cow::Owned(format!(
                    "OTP code must only consist of digit characters. Found non-digit character: '{}'",
                    c
                )))
            }
        }
    }
}

mod code {
    use serde::{de::Visitor, Deserializer, Serializer};

    use crate::model::otp::code::LENGTH;

    use super::Code;

    pub fn serialize<S>(code: &[u8; 6], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&code.iter().map(|n| (n + 48) as char).collect::<String>())
    }

    struct StrVisitor;

    impl Visitor<'_> for StrVisitor {
        type Value = Code;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a string of {} numbers", LENGTH)
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            s.parse::<Self::Value>().map_err(|err| E::custom(err))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 6], D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_str(StrVisitor)
            .map(|code| code.inner)
    }
}

impl<'r> FromFormField<'r> for Code {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        if field.name != "code" {
            Err(Errors::from(ErrorKind::Missing))?
        }
        let value = field.value;
        value.parse::<Code>().map_err(|err| err.into())
    }
}
