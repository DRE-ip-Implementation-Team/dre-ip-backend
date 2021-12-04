use crate::{
    conf,
    error::Error,
    model::{
        otp::{Code, Otp},
        sms::Sms,
        user::{Claims, User},
    },
    AdminPassword,
};
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::ReplaceOptions,
    Collection,
};
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

// TODO: Separate endpoints for:
// - Non-existent user
// - Existing unconfirmed user (contains `expiredAt`)
// - Existing confirmed user

// TODO: Mitigate DoS attacks

#[post("/login/voter/request-otp", data = "<request>")]
async fn login_voter_request_otp(
    request: Form<Strict<OtpRequest>>,
    cookies: &CookieJar<'_>,
    users: &State<Collection<User>>,
    otps: &State<Collection<Otp>>,
) -> Result<Status, Error> {
    let (user, otp) = if let Some(user) = users.find_one(doc! { "sms": &request.sms }, None).await?
    {
        // User already exists, so re-generate and upsert an OTP for their confirmation state

        let otp = match user.expire_at() {
            Some(_) => Otp::to_register(&user),
            None => Otp::to_authenticate(&user),
        }
        // Valid because `to_register` and `to_authenticate` will succeed:
        //  - `to_register` and `to_authenticate` require `id` (user came from DB)
        //  - `to_register` requires `expire_at` (checked in match)
        .unwrap();

        otps.replace_one(
            doc! { "userId": user.id },
            &otp,
            ReplaceOptions::builder().upsert(true).build(),
        )
        .await?;

        (user, otp)
    } else {
        // User does not exist, so create them, generate a registration OTP and insert the pair

        let mut user = User::new(request.sms.clone());
        let user_id = users
            .insert_one(&user, None)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap(); // Valid because `inserted_id` came from DB
        user.id = Some(user_id);

        let otp = Otp::to_register(&user).unwrap(); // Valid because `id` and `expire_at` exist
        otps.insert_one(&otp, None).await?;

        (user, otp)
    };

    // TODO: Send OTP via SMS server
    println!("{:?}", otp.code);

    cookies.add_private(
        Cookie::build("user_id", user.id.unwrap().to_string()) // Valid because upserted user has `id`
            .max_age(time::Duration::new(conf!(otp_ttl) as i64, 0))
            .finish(),
    );
    Ok(Status::Ok)
}

#[post("/login/voter/submit-otp", data = "<submission>")]
async fn login_voter_submit_otp(
    submission: Form<Strict<OtpSubmission>>,
    cookies: &CookieJar<'_>,
    users: &State<Collection<User>>,
    otps: &State<Collection<Otp>>,
) -> Result<Status, Error> {
    let user_id = cookies
        .get_private("user_id")
        .ok_or(Error::BadRequest("Missing `user_id` cookie".to_string()))?
        .value()
        .parse::<ObjectId>()?;

    let otp = otps
        .find_one(doc! { "userId": user_id }, None)
        .await?
        .ok_or(Error::BadRequest(format!(
            "no user found for `user_id` {}",
            user_id
        )))?;

    // Verify submitted OTP code
    if otp.code != submission.code {
        Err(Error::Unauthorized(format!(
            "incorrect OTP code {:?}",
            submission.code
        )))?
    }

    // Cancel user expiry
    users
        .update_one(
            doc! { "_id": user_id },
            doc! { "$unset": { "expireAt": "" } },
            None,
        )
        .await?;

    // Disallow OTP reuse
    otps.delete_one(doc! { "_id": otp.id }, None).await?;

    // Set authentication token and remove `user_id` cookie
    cookies.add(Claims::for_user_id(user_id).into());
    cookies.remove_private(Cookie::named("user_id"));

    Ok(Status::Ok)
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
