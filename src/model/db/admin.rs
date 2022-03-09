use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::{admin::NewAdmin, mongodb::Id};

/// An admin user from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
pub struct Admin {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub admin: NewAdmin,
}

impl Deref for Admin {
    type Target = NewAdmin;

    fn deref(&self) -> &Self::Target {
        &self.admin
    }
}

impl DerefMut for Admin {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.admin
    }
}
