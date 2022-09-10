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
            auth::{AuthToken, VoterChallengeRequest, VoterVerifyRequest, AUTH_TOKEN_COOKIE},
            otp::{Challenge, CHALLENGE_COOKIE},
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
    routes![
        check_auth_admin,
        check_auth_voter,
        check_auth_none,
        authenticate,
        challenge,
        verify,
        logout,
    ]
}

#[get("/auth/check", rank = 1)]
async fn check_auth_admin(_token: AuthToken<Admin>) -> &'static str {
    "Admin"
}

#[get("/auth/check", rank = 2)]
async fn check_auth_voter(_token: AuthToken<Voter>) -> &'static str {
    "Voter"
}

#[get("/auth/check", rank = 3)]
async fn check_auth_none() -> &'static str {
    "Unauthenticated"
}

#[post("/auth/admin", data = "<credentials>", format = "json")]
async fn authenticate(
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
async fn challenge(
    auth_request: Json<VoterChallengeRequest>,
    cookies: &CookieJar<'_>,
    config: &State<Config>,
    sender: &State<SnsClient>,
) -> Result<()> {
    // Verify the reCAPTCHA.
    let sms = auth_request
        .0
        .verify(config.recaptcha_secret(), config.hostname())
        .await?;

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
#[post("/auth/voter/verify", data = "<auth_request>", format = "json")]
async fn verify(
    auth_request: Json<VoterVerifyRequest>,
    challenge: Challenge,
    cookies: &CookieJar<'_>,
    voters: Coll<Voter>,
    new_voters: Coll<NewVoter>,
    config: &State<Config>,
) -> Result<()> {
    #[cfg(feature = "otp")]
    {
        let code = auth_request
            .0
            .verify(config.recaptcha_secret(), config.hostname())
            .await?;
        if challenge.code != code {
            // Submitted code is invalid and so the verification fails
            return Err(Error::Status(
                Status::Unauthorized,
                format!("Incorrect OTP code {:?}", code),
            ));
        }
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
fn logout(cookies: &CookieJar) -> Status {
    cookies.remove(Cookie::named(AUTH_TOKEN_COOKIE));
    Status::Ok
}

#[cfg(test)]
mod tests {
    use rocket::{http::ContentType, local::asynchronous::Client, serde::json::serde_json::json};
    use std::str::FromStr;

    use crate::model::{
        api::{
            otp::{Challenge, Code, CODE_LENGTH},
            sms::Sms,
        },
        db::admin::NewAdmin,
    };

    use super::*;

    #[backend_test(admin)]
    async fn auth_check_admin(client: Client) {
        let response = client.get(uri!(check_auth_admin)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        let body = response.into_string().await.unwrap();
        assert_eq!(body, "Admin");
    }

    #[backend_test(voter)]
    async fn auth_check_voter(client: Client) {
        let response = client.get(uri!(check_auth_voter)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        let body = response.into_string().await.unwrap();
        assert_eq!(body, "Voter");
    }

    #[backend_test]
    async fn auth_check_none(client: Client) {
        let response = client.get(uri!(check_auth_none)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert_eq!(response.into_string().await.unwrap(), "Unauthenticated");
    }

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
            .body(json!(VoterChallengeRequest::example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        // Submit verification
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let code = Challenge::from_cookie(&cookie, client.rocket().state().unwrap())
            .unwrap()
            .code;
        let verify_request = VoterVerifyRequest::example(code);
        let response = client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(verify_request).to_string())
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
            .body(json!(VoterChallengeRequest::example()).to_string())
            .dispatch()
            .await;
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let challenge_value = cookie.value();

        // Re-request challenge
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(VoterChallengeRequest::example()).to_string())
            .dispatch()
            .await;
        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let next_challenge_value = cookie.value();

        assert_ne!(challenge_value, next_challenge_value);
    }

    #[backend_test]
    async fn invalid_voter_sms(client: Client) {
        let mut body = json!(VoterChallengeRequest::example());
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
    async fn invalid_recaptcha_challenge(client: Client) {
        let response = client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(VoterChallengeRequest::example_invalid()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Unauthorized, response.status());
    }

    #[backend_test]
    async fn invalid_otp_code(client: Client) {
        client
            .post(uri!(challenge))
            .header(ContentType::JSON)
            .body(json!(VoterChallengeRequest::example()).to_string())
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
        let verify_request = VoterVerifyRequest::example(Code::from_str(&wrong_code).unwrap());

        let response = client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(verify_request).to_string())
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
            .body(json!(VoterChallengeRequest::example()).to_string())
            .dispatch()
            .await;

        let cookie = client.cookies().get_private(CHALLENGE_COOKIE).unwrap();
        let code = Challenge::from_cookie(&cookie, client.rocket().state().unwrap())
            .unwrap()
            .code;
        let verify_request = VoterVerifyRequest::example(code);

        client
            .post(uri!(verify))
            .header(ContentType::JSON)
            .body(json!(verify_request).to_string())
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
