use crate::conf;
use crate::model::sms::Sms;
use jsonwebtoken::{self as jwt, errors::Error as JwtError, DecodingKey, EncodingKey};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    error::Error as DbError,
    Collection,
};
use rocket::{
    http::{Cookie, Status},
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expire_at: Option<DateTime>,
    pub sms: Sms,
}

impl User {
    /// Returns a [`User`] with an `sms` that expires a number of seconds from the current system time.
    ///
    /// See [`Config`] to customise the number of seconds until expiry.
    pub fn new(sms: Sms) -> Self {
        Self {
            id: None,
            expire_at: Some(DateTime::from_system_time(
                SystemTime::now() + Duration::from_secs(conf!(otp_ttl)),
            )),
            sms,
        }
    }

    pub fn expire_at(&self) -> Option<DateTime> {
        self.expire_at
    }
}

#[derive(Debug)]
pub enum UserAuthError {
    NoCookie,
    BadCookie(String),
    NoUser,
    DbError(DbError),
    JwtError(JwtError),
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = UserAuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let users: &State<Collection<User>> = req.guard().await.unwrap();
        if let Some(cookie) = req.cookies().get("auth_token") {
            let token = cookie.value();
            match token.parse::<Claims>() {
                Ok(claims) => {
                    let user_id = claims.user_id.unwrap();
                    match users.find_one(doc! { "_id": user_id }, None).await {
                        Ok(result) => match result {
                            Some(user) => Outcome::Success(user),
                            None => {
                                // No user found
                                Outcome::Failure((
                                    Status::InternalServerError,
                                    UserAuthError::NoUser,
                                ))
                            }
                        },
                        Err(db_err) => {
                            // DB failed to fetch user
                            Outcome::Failure((
                                Status::InternalServerError,
                                UserAuthError::DbError(db_err),
                            ))
                        }
                    }
                }
                Err(jwt_err) => {
                    return Outcome::Failure((Status::BadRequest, UserAuthError::JwtError(jwt_err)))
                }
            }
        } else {
            // No `user_id` cookie set
            Outcome::Failure((Status::Unauthorized, UserAuthError::NoCookie))
        }
    }
}

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
