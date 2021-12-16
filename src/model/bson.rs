use std::ops::Deref;

use mongodb::bson::oid::ObjectId;
use rocket::request::FromParam;

#[derive(Debug)]
pub struct Id(ObjectId);

impl Deref for Id {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> FromParam<'a> for Id {
    type Error = mongodb::bson::oid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(Self(param.parse::<ObjectId>()?))
    }
}
