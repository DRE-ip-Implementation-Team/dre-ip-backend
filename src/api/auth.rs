use crate::{
    error::{Error, Result},
    model::{
        otp::{challenge::Challenge, code::Code},
        sms::Sms,
        user::{claims::Claims, User, Users},
    },
    AdminPassword,
};

use mongodb::bson::doc;
use rocket::{
    form::{Form, Strict},
    http::{Cookie, CookieJar, Status},
    Route, State,
};

pub fn routes() -> Vec<Route> {
    routes![authenticate_admin, request_otp, authenticate_user, logout]
}

#[post("/auth/admin", data = "<login>")]
fn authenticate_admin(
    cookies: &CookieJar,
    login: Form<Strict<AdminLogin>>,
    admin_password: &State<AdminPassword>,
) -> Status {
    if login.password != admin_password.0 {
        return Status::Unauthorized;
    }
    cookies.add(Claims::for_admin().into());
    Status::Ok
}

#[get("/auth/voter?<sms>")]
fn request_otp(sms: Sms, cookies: &CookieJar<'_>) {
    cookies.add_private(Challenge::cookie(sms));
}

#[post("/auth/voter", data = "<code>")]
async fn authenticate_user(
    code: Form<Strict<Code>>,
    cookies: &CookieJar<'_>,
    users: &State<Users>,
) -> Result<()> {
    let challenge = cookies
        .get_private("challenge")
        .ok_or_else(|| Error::BadRequest("Missing `challenge` cookie".to_string()))?
        .value()
        .parse::<Challenge>()?;

    if challenge.code() != **code {
        return Err(Error::Unauthorized(format!(
            "Incorrect OTP code {:?}",
            code
        )));
    }

    let user_id = users
        .insert_one(User::new(challenge.sms()), None)
        .await?
        .inserted_id
        .as_object_id()
        .unwrap(); // Valid because the ID comes directly from the database

    cookies.add(Claims::for_user_id(user_id).into());
    cookies.remove(Cookie::named("challenge"));

    Ok(())
}

#[delete("/auth")]
fn logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named("auth_token"));
    Status::Ok
}

#[derive(FromForm)]
struct AdminLogin<'a> {
    password: &'a str,
}
