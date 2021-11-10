use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    Collection,
};
use phonenumber::PhoneNumber;
use rocket::{
    form::{self, prelude::ErrorKind, FromFormField, ValueField},
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Display;
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
    /// Returns a [`User`] with an `sms` that expires 5 minutes from the current system time.
    pub fn new(sms: Sms) -> Self {
        Self {
            id: None,
            expire_at: Some(DateTime::from_system_time(
                SystemTime::now() + Duration::from_secs(300),
            )),
            sms,
        }
    }
}

#[derive(Debug)]
pub enum UserAuthError {
    BadCookie(String),
    NoUser,
    DbError,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = UserAuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let users: &State<Collection<User>> = req.guard().await.unwrap();
        if let Some(cookie) = req.cookies().get_private("user_id") {
            let cookie_value = cookie.value();
            if let Ok(user_id) = ObjectId::parse_str(cookie_value) {
                match users.find_one(doc! { "_id": user_id }, None).await {
                    Ok(result) => match result {
                        Some(user) => Outcome::Success(user),
                        None => {
                            Outcome::Failure((Status::InternalServerError, UserAuthError::NoUser))
                        }
                    },
                    Err(_) => {
                        Outcome::Failure((Status::InternalServerError, UserAuthError::DbError))
                    }
                }
            } else {
                Outcome::Failure((
                    Status::BadRequest,
                    UserAuthError::BadCookie(cookie_value.to_string()),
                ))
            }
        } else {
            Outcome::Forward(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Otp {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    expire_at: DateTime,
    pub code: String,
}

impl Otp {
    /// Returns a random [`Otp`] that expires when the `user` does.
    ///
    /// If the `id` and `expire_at` fields of the user are populated, `Some(otp)` is returned, else [`None`] is.
    pub fn for_user(user: &User) -> Option<Self> {
        Some(Self {
            id: None,
            expire_at: user.expire_at?,
            // TODO: Generate random OTP
            code: "123456".to_string(),
            user_id: user.id?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sms(PhoneNumber);

impl Display for Sms {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.fmt(formatter)
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Sms {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        if field.name != "sms" {
            return Err(ErrorKind::InvalidChoice {
                choices: Cow::Owned(vec!["sms".into()]),
            }
            .into());
        }
        phonenumber::parse(None, field.value)
            .map(Sms)
            .map_err(|err| ErrorKind::Custom(Box::new(err)).into())
    }
}
