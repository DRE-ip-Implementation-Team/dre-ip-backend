use mongodb::bson::doc;
use rocket::{
    http::{Cookie, CookieJar, Status},
    serde::json::Json,
    Route, State,
};

use crate::{
    error::{Error, Result},
    model::{
        admin::{db::DbAdmin, Admin, Credentials},
        auth::token::AuthToken,
        mongodb::{collection::Coll, entity::DbEntity},
        otp::{challenge::Challenge, code::Code},
        sms::Sms,
        voter::{db::DbVoter, Voter},
    },
    Config,
};

pub fn routes() -> Vec<Route> {
    routes![authenticate, challenge, verify, logout]
}

#[post("/admins/authenticate", data = "<credentials>", format = "json")]
async fn authenticate(
    cookies: &CookieJar<'_>,
    credentials: Json<Credentials<'_>>,
    admins: Coll<DbAdmin>,
    config: &State<Config>,
) -> Result<()> {
    let with_username = doc! {
        "username": credentials.username()
    };

    let admin = admins
        .find_one(with_username, None)
        .await?
        .filter(|admin| admin.verify_password(credentials.password()))
        .ok_or_else(|| {
            Error::Status(
                Status::Unauthorized,
                "No admin found with the provided username and password combination.".to_string(),
            )
        })?;

    let token = AuthToken::<Admin>::for_user(&admin);
    cookies.add(token.into_cookie(config));

    Ok(())
}

#[get("/voter/challenge?<sms>")]
fn challenge(sms: Sms, cookies: &CookieJar<'_>, config: &State<Config>) {
    let challenge = Challenge::for_sms(sms);
    cookies.add_private(challenge.into_cookie(config));
}

#[post("/voter/verify", data = "<code>", format = "json")]
async fn verify(
    code: Json<Code>,
    challenge: Challenge,
    cookies: &CookieJar<'_>,
    voters: Coll<Voter>,
    db_voters: Coll<DbVoter>,
    config: &State<Config>,
) -> Result<()> {
    if challenge.code() != *code {
        // Submitted code is invalid and so the verification fails
        return Err(Error::Status(
            Status::Unauthorized,
            format!("Incorrect OTP code {:?}", code),
        ));
    }

    let voter = Voter::new(challenge.sms());

    let with_sms = doc! {
        "sms": voter.sms()
    };

    // We need an id to associate with the voter's interactions to ensure for instance that they
    // have not already voted for a certain question
    let id = if let Some(voter) = db_voters.find_one(with_sms, None).await? {
        // Voter already exists and so already has an id we can use
        voter.id()
    } else {
        // Voter does not exist and so must be inserted to retrieve an id
        voters
            .insert_one(&voter, None)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap() // Valid because the ID comes directly from the database
            .into()
    };

    // Ensure the voter is authenticated
    let claims = AuthToken::<Voter>::for_user(&DbVoter::new(id, voter));
    cookies.add(claims.into_cookie(config));

    // We no longer need the OTP challenge
    cookies.remove(Cookie::named("challenge"));

    Ok(())
}

#[delete("/auth")]
pub fn logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named("auth_token"));
    Status::Ok
}

#[cfg(test)]
pub async fn login_as_admin(client: &rocket::local::asynchronous::Client, db: &mongodb::Database) {
    use rocket::{http::ContentType, serde::json::serde_json::json};

    Coll::<Admin>::from_db(&db)
        .insert_one(Admin::example(), None)
        .await
        .unwrap();

    client
        .post(uri!(authenticate))
        .header(ContentType::JSON)
        .body(json!(Credentials::example()).to_string())
        .dispatch()
        .await;
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{http::ContentType, local::asynchronous::Client, serde::json::serde_json::json};

    use crate::{
        client_and_db,
        model::{
            admin::Admin,
            otp::{self, challenge, code::LENGTH},
        },
    };

    use super::*;

    #[db_test]
    async fn admin_authenticate_valid(client: Client, db: Database) {
        // Ensure there is an admin to login as
        let admins = Coll::<Admin>::from_db(&db);
        admins.insert_one(Admin::example(), None).await.unwrap();

        // Use valid credentials to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(json!(Credentials::example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(client.cookies().get("auth_token").is_some());
    }

    #[rocket::async_test]
    async fn admin_authenticate_invalid() {
        let (client, db) = client_and_db().await;

        // Ensure there is an admin to fail to login as
        let admins = Coll::<Admin>::from_db(&db);
        admins.insert_one(Admin::example(), None).await.unwrap();

        // Use invalid username to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(
                json! ({
                    "username": "",
                    "password": "",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
        assert_eq!(None, client.cookies().get("auth_token"));

        // Use invalid password to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(
                json! ({
                    "username": Admin::example().username(),
                    "password": "",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
        assert_eq!(None, client.cookies().get("auth_token"));

        // Clean up admin
        admins
            .delete_one(doc! { "username": Admin::example().username() }, None)
            .await
            .unwrap();
    }

    #[rocket::async_test]
    async fn voter_authenticate() {
        let (client, db) = client_and_db().await;

        // Request challenge
        let response = client.get(uri!(challenge(Sms::example()))).dispatch().await;

        let cookies = client.cookies();
        let possible_cookie = cookies.get_private("challenge");

        assert_eq!(Status::Ok, response.status());
        assert!(cookies.get_private("challenge").is_some());

        let cookie = possible_cookie.unwrap();
        let raw_claims = cookie.value();

        // Submit verification
        let challenge = challenge::Claims::from_str(raw_claims, client.rocket().state().unwrap())
            .unwrap()
            .into_challenge();
        let response = client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(challenge.code()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(client.cookies().get("auth_token").is_some());

        // Check voter was inserted
        let voters = Coll::<Voter>::from_db(&db);
        let voter = voters
            .find_one(doc! { "sms": Sms::example() }, None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(Voter::example(), voter);

        // Clean up voter
        voters
            .delete_one(doc! { "sms": Sms::example() }, None)
            .await
            .unwrap();
    }

    #[rocket::async_test]
    async fn unique_challenges() {
        let (client, _) = client_and_db().await;

        // Request challenge
        client.get(uri!(challenge(Sms::example()))).dispatch().await;
        let cookie = client.cookies().get_private("challenge").unwrap();
        let challenge_value = cookie.value();

        // Re-request challenge
        client.get(uri!(challenge(Sms::example()))).dispatch().await;
        let cookie = client.cookies().get_private("challenge").unwrap();
        let next_challenge_value = cookie.value();

        assert_ne!(challenge_value, next_challenge_value);
    }

    #[rocket::async_test]
    async fn invalid_voter_sms() {
        let (client, _) = client_and_db().await;

        let response = client.get("/voter/challenge?5555555555").dispatch().await;

        assert_eq!(Status::NotFound, response.status());
    }

    #[rocket::async_test]
    async fn invalid_otp_code() {
        let (client, _) = client_and_db().await;

        client.get(uri!(challenge(Sms::example()))).dispatch().await;
        let cookie = client.cookies().get_private("challenge").unwrap();
        let raw_claims = cookie.value();
        let code = otp::challenge::Claims::from_str(raw_claims, client.rocket().state().unwrap())
            .unwrap()
            .into_challenge()
            .code();

        let mut wrong_code = [0; LENGTH];
        wrong_code[0] = if code[0] == 0 { 1 } else { code[0] - 1 };
        let wrong_code = wrong_code
            .into_iter()
            .map(|digit| char::from_digit(digit as u32, 10).unwrap())
            .collect::<String>();

        let response = client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!({ "code": wrong_code }).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
    }

    #[db_test]
    async fn logout_admin(client: Client, db: Database) {
        login_as_admin(&client, &db).await;

        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert_eq!(None, client.cookies().get("auth_token"));
    }

    #[rocket::async_test]
    async fn logout_voter() {
        let (client, db) = client_and_db().await;

        client.get(uri!(challenge(Sms::example()))).dispatch().await;

        let cookie = client.cookies().get_private("challenge").unwrap();
        let raw_claims = cookie.value();
        let code = otp::challenge::Claims::from_str(raw_claims, client.rocket().state().unwrap())
            .unwrap()
            .into_challenge()
            .code();

        client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(code).to_string())
            .dispatch()
            .await;

        assert!(client.cookies().get("auth_token").is_some());

        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert_eq!(None, client.cookies().get("auth_token"));

        // Clean up voter
        let voters = Coll::<Voter>::from_db(&db);
        voters
            .delete_one(doc! { "sms": Sms::example() }, None)
            .await
            .unwrap();
    }

    #[rocket::async_test]
    async fn logout_not_logged_in() {
        let (client, _) = client_and_db().await;

        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
    }
}
