use std::{ops::Deref, str::FromStr};

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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(ObjectId);

impl Deref for Id {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
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
