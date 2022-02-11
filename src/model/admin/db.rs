use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::admin_core::AdminCore;

/// An admin user from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
pub struct Admin {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    admin: AdminCore,
}

impl Deref for Admin {
    type Target = AdminCore;

    fn deref(&self) -> &Self::Target {
        &self.admin
    }
}

impl DbEntity for Admin {
    fn id(&self) -> Id {
        self.id
    }
}
