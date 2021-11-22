use phonenumber::PhoneNumber;
use rocket::form::{self, error::ErrorKind, FromFormField, ValueField};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sms {
    #[serde(serialize_with = "serialize_phone_number")]
    inner: PhoneNumber,
}

fn serialize_phone_number<S>(phone_number: &PhoneNumber, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&phone_number.to_string())
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
