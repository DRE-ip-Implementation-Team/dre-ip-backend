use aws_sdk_sns::Client as SnsClient;
use dre_ip::Serializable;
use mongodb::bson::doc;
use rocket::{
    http::{Cookie, CookieJar, Status},
    serde::json::Json,
    Route, State,
};

use crate::{
    error::{Error, Result},
    model::{
        api::{
            admin::AdminCredentials,
            auth::{AuthRequest, AuthToken, AUTH_TOKEN_COOKIE},
            otp::{Challenge, Code, CHALLENGE_COOKIE},
        },
        db::{
            admin::Admin,
            voter::{NewVoter, Voter},
        },
        mongodb::{Coll, Id},
    },
    Config,
};

pub fn routes() -> Vec<Route> {
    routes![authenticate, challenge, verify, logout]
}

#[post("/auth/admin", data = "<credentials>", format = "json")]
pub async fn authenticate(
    cookies: &CookieJar<'_>,
    credentials: Json<AdminCredentials>,
    admins: Coll<Admin>,
    config: &State<Config>,
) -> Result<()> {
    let with_username = doc! {
        "username": &credentials.username
    };

    let admin = admins
        .find_one(with_username, None)
        .await?
        .filter(|admin| admin.verify_password(&credentials.password))
        .ok_or_else(|| {
            Error::Status(
                Status::Unauthorized,
                "No admin found with the provided username and password combination.".to_string(),
            )
        })?;

    let token = AuthToken::new(&admin);
    cookies.add(token.into_cookie(config));

    Ok(())
}

#[cfg_attr(any(not(feature = "otp"), test), allow(unused_variables))]
#[post("/auth/voter/challenge", data = "<auth_request>", format = "json")]
pub async fn challenge(
    auth_request: Json<AuthRequest>,
    cookies: &CookieJar<'_>,
    config: &State<Config>,
    sender: &State<SnsClient>,
) -> Result<()> {
    // Verify the reCAPTCHA.
    let sms = auth_request
        .0
        .verify()
        .await
        .ok_or_else(|| Error::Status(Status::Unauthorized, "Invalid reCAPTCHA".to_string()))?;

    // Choose the OTP.
    let challenge = Challenge::new(sms);

    // Send the OTP.
    #[cfg(all(feature = "otp", not(test)))]
    sender
        .publish()
        .phone_number(challenge.sms.to_string())
        .message(format!("Voter registration code: {}", challenge.code))
        .send()
        .await
        .map_err(|_| {
            Error::Status(
                Status::InternalServerError,
                "Failed to send message".to_string(),
            )
        })?;

    // Set the cookie.
    cookies.add_private(challenge.into_cookie(config));

    Ok(())
}

#[cfg_attr(not(feature = "otp"), allow(unused_variables))]
#[post("/auth/voter/verify", data = "<code>", format = "json")]
pub async fn verify(
    code: Json<Code>,
    challenge: Challenge,
    cookies: &CookieJar<'_>,
    voters: Coll<Voter>,
    new_voters: Coll<NewVoter>,
    config: &State<Config>,
) -> Result<()> {
    #[cfg(feature = "otp")]
    if challenge.code != *code {
        // Submitted code is invalid and so the verification fails
        return Err(Error::Status(
            Status::Unauthorized,
            format!("Incorrect OTP code {:?}", code),
        ));
    }

    let voter = NewVoter::new(challenge.sms, config);

    let with_sms_hmac = doc! {
        "sms_hmac": voter.sms_hmac.to_bytestring(),
    };

    // We need an id to associate with the voter's interactions to ensure for instance that they
    // have not already voted for a certain question
    let db_voter = if let Some(voter) = voters.find_one(with_sms_hmac, None).await? {
        // Voter already exists.
        voter
    } else {
        // Voter doesn't exist yet.
        let new_id: Id = new_voters
            .insert_one(&voter, None)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap() // Safe because the ID comes directly from the database.
            .into();
        voters.find_one(new_id.as_doc(), None).await?.unwrap()
    };

    // Ensure the voter is authenticated
    let claims = AuthToken::new(&db_voter);
    cookies.add(claims.into_cookie(config));

    // We no longer need the OTP challenge
    cookies.remove(Cookie::named(CHALLENGE_COOKIE));

    Ok(())
}

#[delete("/auth")]
pub fn logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named(AUTH_TOKEN_COOKIE));
    Status::Ok
}

#[cfg(test)]
mod tests {
    use rocket::{http::ContentType, local::asynchronous::Client, serde::json::serde_json::json};

    use crate::model::{
        api::{
            otp::{Challenge, CODE_LENGTH},
            sms::Sms,
        },
        db::admin::NewAdmin,
    };

    use super::*;

    #[backend_test]
    async fn admin_authenticate_valid(client: Client, admins: Coll<NewAdmin>) {
        // Ensure there is an admin to login as
        admins.insert_one(NewAdmin::example(), None).await.unwrap();

        // Use valid credentials to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(json!(AdminCredentials::example1()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(client.cookies().get(AUTH_TOKEN_COOKIE).is_some());
    }

    #[backend_test]
    async fn admin_authenticate_invalid(client: Client, admins: Coll<NewAdmin>) {
        // Ensure there is an admin to fail to login as
        admins.insert_one(NewAdmin::example(), None).await.unwrap();

        // Use invalid username to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(json!(AdminCredentials::empty()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
        assert_eq!(None, client.cookies().get(AUTH_TOKEN_COOKIE));

        // Use invalid password to attempt admin login
        let response = client
            .post(uri!(authenticate))
            .header(ContentType::JSON)
            .body(
                json! ({
                    "username": &NewAdmin::example().username,
                    "password": "",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
        assert_eq!(None, client.cookies().get(AUTH_TOKEN_COOKIE));
    }

    #[backend_test]
    async fn voter_authenticate(client: Client, voters: Coll<NewVoter>) {
        // Request challenge
        let response = client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();

        // Submit verification
        let challenge = Challenge::from_cookie(&cookie, client.rocket().state().unwrap()).unwrap();
        let response = client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(challenge.code).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(client.cookies().get(AUTH_TOKEN_COOKIE).is_some());

        // Check voter was inserted
        let voter = voters
            .find_one(
                doc! { "sms_hmac": Sms::example_hmac(&client).to_bytestring() },
                None,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            NewVoter::example(client.rocket().state::<Config>().unwrap()),
            voter
        );
    }

    #[backend_test]
    async fn unique_challenges(client: Client) {
        // Request challenge
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example()).to_string())
            .dispatch()
            .await;
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let challenge_value = cookie.value();

        // Re-request challenge
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example()).to_string())
            .dispatch()
            .await;
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let next_challenge_value = cookie.value();

        assert_ne!(challenge_value, next_challenge_value);
    }

    #[backend_test]
    async fn invalid_voter_sms(client: Client) {
        let mut body = json!(AuthRequest::example());
        body["sms"] = json!("5555555555");
        let response = client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(body.to_string())
            .dispatch()
            .await;

        assert_eq!(Status::UnprocessableEntity, response.status());
    }

    #[backend_test]
    async fn invalid_recaptcha(client: Client) {
        let response = client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example_invalid()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
    }

    #[backend_test]
    async fn invalid_otp_code(client: Client) {
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example()).to_string())
            .dispatch()
            .await;
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let code = Challenge::from_cookie(&cookie, client.rocket().state().unwrap())
            .unwrap()
            .code;

        let mut wrong_code = [0; CODE_LENGTH];
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

    #[backend_test(admin)]
    async fn logout_admin(client: Client) {
        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert_eq!(None, client.cookies().get(AUTH_TOKEN_COOKIE));
    }

    #[backend_test]
    async fn logout_voter(client: Client) {
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(AuthRequest::example()).to_string())
            .dispatch()
            .await;

        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let code = Challenge::from_cookie(&cookie, client.rocket().state().unwrap())
            .unwrap()
            .code;

        client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(code).to_string())
            .dispatch()
            .await;

        assert!(client.cookies().get(AUTH_TOKEN_COOKIE).is_some());

        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert_eq!(None, client.cookies().get(AUTH_TOKEN_COOKIE));
    }

    #[backend_test]
    async fn logout_not_logged_in(client: Client) {
        let response = client.delete(uri!(logout)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
    }
}
