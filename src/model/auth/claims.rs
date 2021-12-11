use std::str::FromStr;

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use mongodb::bson::oid::ObjectId;
use rocket::http::Cookie;
use serde::{Deserialize, Serialize};
use time;

use crate::{
    conf,
    model::{admin::db::DbAdmin, voter::db::DbVoter},
};

use super::token::Rights;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    id: ObjectId,
    #[serde(rename = "rgt")]
    rights: Rights,
    #[serde(rename = "exp")]
    expire_at: u64,
}

impl Claims {
    pub fn rights(&self) -> Rights {
        self.rights
    }

    pub fn permits(&self, target: Rights) -> bool {
        self.rights >= target
    }

    pub fn for_voter(voter: &DbVoter) -> Cookie<'static> {
        Self {
            id: voter.id(),
            rights: Rights::Voter,
            expire_at: Self::expire_at(),
        }
        .into()
    }

    pub fn for_admin(admin: &DbAdmin) -> Cookie<'static> {
        Self {
            id: admin.id(),
            rights: Rights::Admin,
            expire_at: Self::expire_at(),
        }
        .into()
    }

    fn expire_at() -> u64 {
        (Utc::now() + Duration::seconds(conf!(auth_ttl) as i64)).timestamp() as u64
    }
}

impl From<Claims> for Cookie<'_> {
    fn from(claims: Claims) -> Self {
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(conf!(jwt_secret)),
        )
        .unwrap(); // Valid because Claims serialization never fails
        Cookie::build("auth_token", token)
            .max_age(time::Duration::seconds(conf!(auth_ttl) as i64))
            // .secure(true) // TODO: Uncomment when communicating via HTTPS
            .finish()
    }
}

impl FromStr for Claims {
    type Err = JwtError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        decode(
            string,
            &DecodingKey::from_secret(conf!(jwt_secret)),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
    }
}
