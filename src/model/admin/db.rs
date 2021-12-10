use mongodb::{bson::oid::ObjectId, Collection};
use serde::{Deserialize, Serialize};

use super::Admin;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAdmin {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(flatten)]
    admin: Admin,
}

impl DbAdmin {
    pub fn id(&self) -> ObjectId {
        self.id
    }
}

pub type GetAdmins = Collection<DbAdmin>;
