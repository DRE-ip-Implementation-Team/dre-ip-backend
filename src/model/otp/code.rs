use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use rand::distributions::{Distribution, Uniform};
use rocket::form::{self, prelude::ErrorKind, Errors, FromFormField, ValueField};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LENGTH: usize = 6;

/// A one-time-password code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Code {
    #[serde(with = "serialize_code")]
    code: [u8; LENGTH],
}

impl Code {
    /// Generate a random code.
    pub fn random() -> Self {
        let mut code = [0; LENGTH];
        let digit_dist = Uniform::from(0..=9);
        let mut rng = rand::thread_rng();
        for digit in &mut code {
            *digit = digit_dist.sample(&mut rng);
        }
        Self { code }
    }
}

impl Deref for Code {
    type Target = [u8; LENGTH];

    fn deref(&self) -> &Self::Target {
        &self.code
    }
}

/// (De)serialisation for OTP codes.
mod serialize_code {
    use serde::{
        de::{Error, Unexpected, Visitor},
        Deserializer, Serializer,
    };

    use crate::model::otp::code::LENGTH;

    pub fn serialize<S>(code: &[u8; 6], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&code.iter().map(|n| (n + 48) as char).collect::<String>())
    }

    struct StrVisitor;

    impl<'de> Visitor<'de> for StrVisitor {
        type Value = [u8; LENGTH];

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a string of {} digits", LENGTH)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if v.len() != LENGTH {
                return Err(E::invalid_length(
                    v.len(),
                    &format!("a string of {} digit characters", LENGTH).as_str(),
                ));
            }

            v.chars()
                .map(|c| {
                    c.to_digit(10)
                        .map(|digit| digit as u8)
                        .ok_or_else(|| E::invalid_value(Unexpected::Char(c), &"a digit character"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|digits| digits.try_into().unwrap()) // Valid because the input length has been checked
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 6], D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrVisitor)
    }
}

impl Display for Code {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            self.code
                .iter()
                .map(|digit| char::from_digit(*digit as u32, 10).unwrap())
                .collect::<String>()
        )
    }
}

impl FromStr for Code {
    type Err = ParseError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let len = string.len();
        if len != LENGTH {
            return Err(Self::Err::InvalidLength(len));
        }
        let digits = string
            .chars()
            .map(|c| match c {
                '0'..='9' => Ok(c as u8 - 48),
                _ => Err(Self::Err::InvalidChar(c)),
            })
            .collect::<Result<Vec<u8>, Self::Err>>()?;
        Ok(Self {
            code: digits.try_into().unwrap(), // Valid because digits.len() == LENGTH
        })
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("code must contain exactly 6 characters")]
    InvalidLength(usize),
    #[error("code must contain only digits")]
    InvalidChar(char),
}

impl From<ParseError> for form::error::ErrorKind<'_> {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::InvalidLength(_) => form::error::ErrorKind::InvalidLength {
                min: Some(LENGTH as u64),
                max: Some(LENGTH as u64),
            },
            ParseError::InvalidChar(c) => form::error::ErrorKind::Validation(Cow::Owned(format!(
                "OTP code must only consist of digit characters. Found non-digit character: '{}'",
                c
            ))),
        }
    }
}

impl<'r> FromFormField<'r> for Code {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        if field.name != "code" {
            return Err(Errors::from(ErrorKind::Missing));
        }
        let value = field.value;
        value.parse::<Code>().map_err(Into::into)
    }
}
