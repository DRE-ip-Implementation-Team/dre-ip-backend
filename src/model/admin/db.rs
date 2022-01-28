use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{bson::Id, entity::DbEntity};

use super::Admin;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAdmin {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    admin: Admin,
}

impl Deref for DbAdmin {
    type Target = Admin;

    fn deref(&self) -> &Self::Target {
        &self.admin
    }
}

impl DbEntity for DbAdmin {
    fn id(&self) -> Id {
        self.id
    }
}
