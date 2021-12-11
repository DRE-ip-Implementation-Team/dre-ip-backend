use crate::{
    error::{Error, Result},
    model::{
        admin::{db::GetAdmins, Credentials},
        auth::claims::Claims,
        otp::{challenge::Challenge, code::Code},
        sms::Sms,
        voter::{
            db::{DbVoter, GetVoters},
            PutVoters, Voter,
        },
    },
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

#[post("/auth/admin", data = "<credentials>")]
async fn authenticate_admin(
    cookies: &CookieJar<'_>,
    credentials: Form<Strict<Credentials<'_>>>,
    admins: &State<GetAdmins>,
) -> Result<()> {
    let admin = admins
        .find_one(doc! { "username": credentials.username() }, None)
        .await?
        .filter(|admin| admin.verify_password(credentials.password()))
        .ok_or_else(|| {
            Error::NotFound(
                "No admin found with the provided username and password combination.".to_string(),
            )
        })?;
    cookies.add(Claims::for_admin(&admin));
    Ok(())
}

#[get("/auth/voter?<sms>")]
fn request_otp(sms: Sms, cookies: &CookieJar<'_>) {
    cookies.add_private(Challenge::cookie(sms));
}

#[post("/auth/voter", data = "<code>")]
async fn authenticate_user(
    code: Form<Strict<Code>>,
    challenge: Challenge,
    cookies: &CookieJar<'_>,
    put_voters: &State<PutVoters>,
    get_voters: &State<GetVoters>,
) -> Result<()> {
    if challenge.code() != **code {
        return Err(Error::Unauthorized(format!(
            "Incorrect OTP code {:?}",
            code
        )));
    }

    let sms = challenge.sms();
    let voter = Voter::new(sms.clone());
    let id = if let Some(voter) = get_voters.find_one(doc! { "sms": sms }, None).await? {
        voter.id()
    } else {
        put_voters
            .insert_one(&voter, None)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap() // Valid because the ID comes directly from the database
    };

    cookies.add(Claims::for_voter(&DbVoter::new(id, voter)));
    cookies.remove(Cookie::named("challenge"));

    Ok(())
}

#[delete("/auth")]
fn logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named("auth_token"));
    Status::Ok
}
