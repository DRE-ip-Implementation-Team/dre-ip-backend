use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use mongodb::bson::{doc, oid::ObjectId, Bson, Document};
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

    /// Create from raw bytes.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Option<Self> {
        let bytes: [u8; 12] = bytes.as_ref().try_into().ok()?;
        Some(Self(ObjectId::from_bytes(bytes)))
    }

    /// Conversion to a `MongoDB` query
    pub fn as_doc(&self) -> Document {
        doc! { "_id": self.0 }
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

impl From<Id> for ObjectId {
    fn from(id: Id) -> Self {
        id.0
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

impl From<Id> for Bson {
    fn from(id: Id) -> Self {
        id.0.into()
    }
}

/// Ser/deserialize a [`HashMap<K, V>`](std::collections::HashMap) as a
/// [`HashMap<String, V>`](std::collections::HashMap) where `K: Display + FromStr`.
///
/// Use via the attribute `#[serde(with = ...)]`.
/// This is useful for BSON, since document keys must be strings but we may want
/// to use different key types internally.
/// In other words, any [`HashMap<K, V>`](std::collections::HashMap) we want to store in `MongoDB`
/// must either have `K = String` or be annotated with this module.
pub mod serde_string_map {
    use std::hash::Hash;
    use std::{collections::HashMap, fmt::Display};

    use super::{Deserialize, FromStr, Serialize};

    pub fn serialize<K, V, S>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        K: Display,
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

/// Convert a u32 unique ID to a filter document.
pub fn u32_id_filter(id: u32) -> Document {
    doc! {
        "_id": id
    }
}
