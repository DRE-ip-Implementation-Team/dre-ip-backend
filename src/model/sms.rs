use mongodb::bson::{to_bson, Bson};
use phonenumber::PhoneNumber;
use rocket::form::{self, error::ErrorKind, FromFormField, ValueField};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sms {
    #[serde(with = "phone_number")]
    inner: PhoneNumber,
}

mod phone_number {
    use phonenumber::PhoneNumber;
    use serde::{de::Visitor, Deserializer, Serializer};

    pub fn serialize<S>(phone_number: &PhoneNumber, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&phone_number.to_string())
    }

    struct StrVisitor;

    impl Visitor<'_> for StrVisitor {
        type Value = PhoneNumber;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a valid phone number string")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            s.parse::<PhoneNumber>().map_err(|err| E::custom(err))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PhoneNumber, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrVisitor)
    }
}

impl Display for Sms {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.inner.fmt(formatter)
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

impl From<Sms> for Bson {
    fn from(sms: Sms) -> Self {
        to_bson(&sms).unwrap() // Valid because `PhoneNumber` serialization doesn't fail
    }
}
