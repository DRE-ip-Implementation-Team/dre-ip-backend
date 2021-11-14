use crate::{
    model::{
        otp::Otp,
        user::{Claims, Sms, User},
    },
    AdminPassword,
};
use mongodb::{
    bson::{doc, to_bson},
    options::ReplaceOptions,
    Collection,
};
use rocket::{
    form::{Form, Strict},
    http::{Cookie, CookieJar, Status},
    Route, State,
};

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
        let claims = Claims::for_admin();
        let token = claims.encode().unwrap();
        cookies.add_private(Cookie::new("auth_token", token));
        Status::Ok
    } else {
        Status::Unauthorized
    }
}

// TODO: Separate endpoints for:
// - Non-existent user
// - Existing unconfirmed user (contains `expiredAt`)
// - Existing confirmed user

#[post("/login/voter/request-otp", data = "<request>")]
async fn login_voter_request_otp(
    request: Form<Strict<OtpRequest>>,
    cookies: &CookieJar<'_>,
    users: &State<Collection<User>>,
    otps: &State<Collection<Otp>>,
) -> Status {
    // `if let` looks ugly but avoids the alternative `match` nesting
    let (user, _otp) = if let Some(user) = users
        .find_one(doc! { "sms": to_bson(&request.sms).unwrap() }, None)
        .await
        .unwrap()
    {
        // User already exists, so re-generate and upsert an OTP for their confirmation state

        let otp = match user.expire_at() {
            Some(_) => Otp::to_register(&user),
            None => Otp::to_authenticate(&user),
        }
        .unwrap();

        // TODO: Mitigate DoS attacks

        otps.replace_one(
            doc! { "userId": user.id },
            &otp,
            ReplaceOptions::builder().upsert(true).build(),
        )
        .await
        .unwrap();

        (user, otp)
    } else {
        // User does not exist, so create them, generate a registration OTP and insert the pair

        let mut user = User::new(request.sms.clone());
        let user_id = users
            .insert_one(&user, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap();
        user.id = Some(user_id);

        let otp = Otp::to_register(&user).unwrap();
        otps.insert_one(&otp, None).await.unwrap();

        (user, otp)
    };

    // TODO: Send OTP via SMS server

    let claims = Claims::for_user(&user).unwrap();
    let token = claims.encode().unwrap();
    cookies.add_private(Cookie::new("auth_token", token));
    Status::Ok
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

        cookies.add_private(Cookie::new("auth_token", "abc.123.xyz"));

        Status::Ok
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
