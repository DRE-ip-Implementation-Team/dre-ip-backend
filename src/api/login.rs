use crate::model::{Otp, Sms, User};
use crate::AdminPassword;
use mongodb::bson::{doc, to_bson};
use mongodb::Collection;
use rocket::form::{Form, Strict};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::{Route, State};

pub fn routes() -> Vec<Route> {
    routes![login_admin, login_voter_request_otp, login_voter_submit_otp,]
}

#[post("/login/admin", data = "<login>")]
fn login_admin(
    cookies: &CookieJar,
    login: Form<Strict<AdminLogin>>,
    admin_password: &State<AdminPassword>,
) -> Status {
    if login.password == admin_password.0 {
        // TODO: Generate legitimate JWT
        cookies.add_private(Cookie::new("access_token", "abc.123.xyz"));
        Status::Accepted
    } else {
        Status::Unauthorized
    }
}

#[post("/login/voter/request-otp", data = "<request>")]
async fn login_voter_request_otp(
    request: Form<Strict<OtpRequest>>,
    cookies: &CookieJar<'_>,
    users: &State<Collection<User>>,
    otps: &State<Collection<Otp>>,
) -> Status {
    // If user already exists, reject request
    if let Some(_) = users
        .find_one(doc! { "sms": to_bson(&request.sms).unwrap() }, None)
        .await
        .unwrap()
    {
        return Status::BadRequest;
    }

    // Insert user and OTP
    let mut user = User::new(request.sms.clone());
    let user_id = users
        .insert_one(&user, None)
        .await
        .unwrap()
        .inserted_id
        .as_object_id()
        .unwrap();
    user.id = Some(user_id);
    let otp = Otp::for_user(&user).unwrap();
    otps.insert_one(otp, None).await.unwrap();

    // TODO: Send OTP via SMS server

    cookies.add_private(Cookie::new("user_id", user_id.to_hex()));
    Status::Accepted
}

#[post("/login/voter/submit-otp", data = "<submission>")]
async fn login_voter_submit_otp(
    submission: Form<Strict<OtpSubmission<'_>>>,
    cookies: &CookieJar<'_>,
    users: &State<Collection<User>>,
    user: User,
    otps: &State<Collection<Otp>>,
) -> Status {
    // Verify submitted OTP code
    let otp = otps
        .find_one(doc! { "userId": user.id }, None)
        .await
        .unwrap()
        .unwrap();
    if otp.code == submission.code {
        // Cancel user expiry
        users
            .update_one(
                doc! { "_id": user.id },
                doc! { "$unset": { "expireAt": "" } },
                None,
            )
            .await
            .unwrap();

        // Disallow OTP reuse
        otps.delete_one(doc! { "_id": otp.id }, None).await.unwrap();
        // TODO: Generate legitimate JWT
        cookies.add_private(Cookie::new("access_token", "abc.123.xyz"));
        Status::Accepted
    } else {
        // Wrong code
        Status::Unauthorized
    }
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
struct OtpSubmission<'a> {
    #[field(validate = len(6..=6))]
    // XXX: can't use `char::is_numeric`: it accepts non-decimal characters, e.g. the "3/4" character
    #[field(validate = with(|otp| otp.chars().all(|c| ('0'..='9').contains(&c)), "OTP must consist of numbers only"))]
    code: &'a str,
}
