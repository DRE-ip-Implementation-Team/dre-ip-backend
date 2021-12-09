use mongodb::bson::{oid::ObjectId, Bson};
use rocket::request::FromParam;

pub struct Id(ObjectId);

impl From<Id> for Bson {
    fn from(id: Id) -> Self {
        id.0.into()
    }
}

impl<'a> FromParam<'a> for Id {
    type Error = mongodb::bson::oid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(Self(param.parse::<ObjectId>()?))
    }
}
