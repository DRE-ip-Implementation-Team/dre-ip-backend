use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

/// An API-friendly ID that serializes to a string rather than a nested struct.
/// This is needed for any struct that gets serialized into an API *response*.
/// If a struct is only in API *requests*, then we can skip this step and
/// deserialize directly to `Id`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String")]
pub struct ApiId(Id);

impl Debug for ApiId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Display for ApiId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ApiId {
    type Err = mongodb::bson::oid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse::<Id>()?))
    }
}

impl From<ApiId> for String {
    fn from(id: ApiId) -> Self {
        id.to_string()
    }
}

impl From<Id> for ApiId {
    fn from(id: Id) -> Self {
        Self(id)
    }
}

impl Deref for ApiId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ApiId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
