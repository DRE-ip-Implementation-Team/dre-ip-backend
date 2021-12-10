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
    routes![
        login_admin,
        login_voter_request_otp,
        login_voter_submit_otp,
        login_voter_logout
    ]
}

#[post("/login/admin", data = "<login>")]
fn login_admin(
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

#[post("/login/voter/request-otp", data = "<request>")]
fn login_voter_request_otp(request: Form<Strict<OtpRequest>>, cookies: &CookieJar<'_>) {
    cookies.add_private(Challenge::cookie(request.into_inner().into_inner().sms));
}

#[post("/login/voter/submit-otp", data = "<submission>")]
async fn login_voter_submit_otp(
    submission: Form<Strict<OtpSubmission>>,
    cookies: &CookieJar<'_>,
    users: &State<Users>,
) -> Result<()> {
    let challenge = cookies
        .get_private("challenge")
        .ok_or_else(|| Error::BadRequest("Missing `challenge` cookie".to_string()))?
        .value()
        .parse::<Challenge>()?;

    // Verify submitted OTP code
    if challenge.code() != submission.code {
        return Err(Error::Unauthorized(format!(
            "Incorrect OTP code {:?}",
            submission.code
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

#[post("/login/voter/logout")]
fn login_voter_logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named("auth_token"));
    Status::Ok
}

#[derive(FromForm)]
struct AdminLogin<'a> {
    password: &'a str,
}

#[derive(FromForm)]
struct OtpRequest {
    sms: Sms,
}

#[derive(FromForm)]
struct OtpSubmission {
    code: Code,
}
