use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use mongodb::bson::oid::ObjectId;
use rocket::{
    data::ToByteUnit,
    form::{self, prelude::ErrorKind, DataField, FromFormField, ValueField},
    http::{
        impl_from_uri_param_identity,
        uri::fmt::{Path, UriDisplay},
    },
    request::FromParam,
};
use serde::{Deserialize, Serialize};

/// A unique ID for an object in the database.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(ObjectId);

impl Id {
    /// Generate a random ID.
    #[allow(clippy::new_without_default)] // A non-deterministic default would be odd.
    pub fn new() -> Self {
        Id(ObjectId::new())
    }

    /// Get the raw bytes of the ID.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.bytes().to_vec()
    }
}

impl Deref for Id {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        id.to_string()
    }
}

impl FromStr for Id {
    type Err = mongodb::bson::oid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse::<ObjectId>()?))
    }
}

impl From<ObjectId> for Id {
    fn from(id: ObjectId) -> Self {
        Self(id)
    }
}

impl<'a> FromParam<'a> for Id {
    type Error = mongodb::bson::oid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.parse::<Id>()
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Id {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        field.value.parse::<ObjectId>().map(Id).map_err(|err| {
            let error = ErrorKind::Custom(Box::new(err));
            error.into()
        })
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        field
            .data
            .open(12.bytes())
            .into_string()
            .await?
            .into_inner()
            .parse::<ObjectId>()
            .map(Id)
            .map_err(|err| {
                let error = ErrorKind::Custom(Box::new(err));
                error.into()
            })
    }
}

impl UriDisplay<Path> for Id {
    fn fmt(&self, formatter: &mut rocket::http::uri::fmt::Formatter<'_, Path>) -> std::fmt::Result {
        formatter.write_value(self.to_string())
    }
}

impl_from_uri_param_identity!([Path] Id);

/// Serde (de)serialization for HashMap<K, V> as HashMap<String, V> where K implements
/// both [`ToString`] and [`FromStr`]. Use via the attribute `#[serde(with = ...)]`.
/// This is useful for BSON, since document keys must be strings but we may want
/// to use different key types internally.
/// In other words, any `HashMap<K, V>` we want to store in mongodb must either
/// have `K = String` or be annotated with this module.
pub mod serde_string_map {
    use std::collections::HashMap;
    use std::hash::Hash;

    use super::*;

    pub fn serialize<K, V, S>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        K: ToString,
        V: Serialize,
        S: serde::Serializer,
    {
        let string_map = map
            .iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<HashMap<_, _>>();

        serde::Serialize::serialize(&string_map, serializer)
    }

    pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
    where
        K: FromStr + Eq + Hash,
        V: Deserialize<'de>,
        D: serde::Deserializer<'de>,
    {
        HashMap::<String, V>::deserialize(deserializer).and_then(|string_map| {
            string_map
                .into_iter()
                .map(|(s, v)| s.parse().map(|k| (k, v)))
                .collect::<Result<_, _>>()
                .map_err(|_| serde::de::Error::custom("failed to parse key"))
        })
    }
}
