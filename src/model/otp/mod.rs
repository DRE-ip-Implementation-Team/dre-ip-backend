use crate::conf;
use crate::model::user::User;

use mongodb::bson::{doc, oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

use std::time::{Duration, SystemTime};

use self::code::Code;

pub mod code;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Otp {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    expire_at: DateTime,
    pub code: Code,
}

impl Otp {
    /// Returns a random [`Otp`] for the `user` that expires a number of seconds from the current system time.
    ///
    /// If the `user`'s `id` is populated, `Some(otp)` is returned, else [`None`] is.
    ///
    /// See [`Config`] to customise the number of seconds until expiry.
    pub fn to_authenticate(user: &User) -> Option<Self> {
        Some(Self {
            id: None,
            expire_at: DateTime::from_system_time(
                SystemTime::now() + Duration::from_secs(conf!(otp_ttl)),
            ),
            code: Code::new(),
            user_id: user.id?,
        })
    }
    /// Returns a random [`Otp`] for the `user` that expires when the `user` does.
    ///
    /// If the `id` and `expire_at` fields of the user are populated, `Some(otp)` is returned, else [`None`] is.
    pub fn to_register(user: &User) -> Option<Self> {
        Some(Self {
            id: None,
            expire_at: user.expire_at()?,
            code: Code::new(),
            user_id: user.id?,
        })
    }

    pub fn expire_at(&self) -> DateTime {
        self.expire_at
    }
}
