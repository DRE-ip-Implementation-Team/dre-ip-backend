use std::{
    str::FromStr,
    time::{Duration, SystemTime},
};

use jsonwebtoken as jwt;
use jwt::{errors::Error as JwtError, DecodingKey, EncodingKey};
use mongodb::bson::oid::ObjectId;
use rocket::http::Cookie;
use serde::{Deserialize, Serialize};

use crate::conf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(rename = "uid", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<ObjectId>,
    #[serde(rename = "adm")]
    pub is_admin: bool,
    #[serde(rename = "exp", with = "timestamp")]
    pub expire_at: SystemTime,
}

mod timestamp {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(time.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UNIX_EPOCH + Duration::from_secs(u64::deserialize(deserializer)?))
    }
}

impl Claims {
    pub fn for_admin() -> Self {
        Self {
            user_id: None,
            is_admin: true,
            expire_at: Claims::expire_at(),
        }
    }

    pub fn for_user_id(user_id: ObjectId) -> Self {
        Self {
            user_id: Some(user_id),
            is_admin: false,
            expire_at: Claims::expire_at(),
        }
    }

    /// Returns a time at which the JWT represented by the `Claims` will cease to be valid.
    ///
    /// See [`Config`] to customise the number of seconds until the JWT's expiry.
    fn expire_at() -> SystemTime {
        SystemTime::now() + Duration::from_secs(conf!(jwt_duration))
    }

    /// Encodes the `Claims` as a JWT string with a standard header.
    ///
    /// See [`Config`] to customise the secret key used to encrypt the token.
    fn encode(&self) -> String {
        jwt::encode(
            &jwt::Header::default(),
            &self,
            &EncodingKey::from_secret(conf!(jwt_secret)),
        )
        // Valid because:
        //  - Secret is formatted as default signing algorithm expects
        //  - Serialisation does not fail
        .unwrap()
    }
}

impl FromStr for Claims {
    type Err = JwtError;

    fn from_str(token: &str) -> Result<Self, Self::Err> {
        Ok(jwt::decode(
            token,
            &DecodingKey::from_secret(conf!(jwt_secret)),
            &jwt::Validation::new(jwt::Algorithm::HS256),
        )?
        .claims)
    }
}

impl From<Claims> for Cookie<'_> {
    fn from(claims: Claims) -> Self {
        // TODO: Set `Secure` flag for HTTPS-only use
        Cookie::build("auth_token", claims.encode())
            .http_only(true)
            .finish()
    }
}
